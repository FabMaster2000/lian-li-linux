//! RGB controller: manages LED effects for all RGB-capable devices.
//!
//! Coordinates between native config effects and OpenRGB overrides.
//! Wired devices use the `RgbDevice` trait. Wireless devices stream
//! compressed per-LED frames via the `WirelessController`.

use lianli_devices::traits::RgbDevice;
use lianli_devices::wireless::{DiscoveredDevice, WirelessController, WirelessFanType};
use lianli_shared::rgb::{
    RgbAppConfig, RgbDeviceCapabilities, RgbDirection, RgbEffect, RgbEffectRouteEntry,
    RgbMode, RgbZoneInfo, MAX_EFFECT_SPEED,
};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::f32::consts::PI;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tracing::{debug, info, warn};

const WIRELESS_EFFECT_FRAMES: usize = 84;
const WIRELESS_EFFECT_INTERVAL_MS: u16 = 60;

#[derive(Clone)]
enum WirelessZoneSource {
    Effect(RgbEffect),
    Direct(Vec<[u8; 3]>),
}

#[derive(Clone)]
struct WirelessZoneLayout {
    name: String,
    led_indexes: Vec<usize>,
}

/// Tracks a wireless device's RGB state for `send_rgb_direct`.
struct WirelessRgbState {
    mac: [u8; 6],
    fan_count: u8,
    leds_per_fan: u8,
    fan_type: WirelessFanType,
    /// Per-LED color buffer - the full device LED state.
    /// Updated per-zone, then the whole buffer is sent via RF.
    led_state: Vec<[u8; 3]>,
    /// Monotonically increasing effect index (4 bytes, sent in RF header).
    effect_counter: u32,
    slinf_zone_layouts: Vec<Vec<WirelessZoneLayout>>,
    zone_sources: Vec<WirelessZoneSource>,
}

#[derive(Clone, Copy)]
struct WirelessZoneSegment {
    name: &'static str,
    start: usize,
    len: usize,
}

struct RouteLedSegment {
    device_id: String,
    fan_slot_index: usize,
    led_indexes: Vec<usize>,
}

struct WirelessRenderPlan {
    frames: Vec<Vec<[u8; 3]>>,
    animated: bool,
    interval_ms: u16,
}

const SLINF_ZONE_SEGMENTS: [WirelessZoneSegment; 5] = [
    // Fixed internal SL-INF mapping derived from L-Connect USB capture analysis.
    // Zone sizes (8,10,8,10,8) = 44 LEDs per fan.
    WirelessZoneSegment {
        name: "Zone 1",
        start: 0,
        len: 8,
    },
    WirelessZoneSegment {
        name: "Zone 2",
        start: 8,
        len: 10,
    },
    WirelessZoneSegment {
        name: "Zone 3",
        start: 18,
        len: 8,
    },
    WirelessZoneSegment {
        name: "Zone 4",
        start: 26,
        len: 10,
    },
    WirelessZoneSegment {
        name: "Zone 5",
        start: 36,
        len: 8,
    },
];

fn default_slinf_zone_layout() -> Vec<WirelessZoneLayout> {
    SLINF_ZONE_SEGMENTS
        .iter()
        .map(|segment| WirelessZoneLayout {
            name: segment.name.to_string(),
            led_indexes: (segment.start..segment.start + segment.len).collect(),
        })
        .collect()
}

fn default_slinf_zone_layouts(fan_count: u8) -> Vec<Vec<WirelessZoneLayout>> {
    (0..fan_count)
        .map(|_| default_slinf_zone_layout())
        .collect()
}

fn sync_slinf_zone_layouts(wireless_state: &mut HashMap<String, WirelessRgbState>) {
    for (_device_id, state) in wireless_state.iter_mut() {
        if state.fan_type != WirelessFanType::SlInf {
            continue;
        }

        let layouts = default_slinf_zone_layouts(state.fan_count);
        state.set_slinf_zone_layouts(layouts);
    }
}

impl WirelessRgbState {
    fn new(mac: [u8; 6], fan_count: u8, fan_type: WirelessFanType) -> Self {
        let leds_per_fan = fan_type.leds_per_fan();
        let total_leds = fan_count as usize * leds_per_fan as usize;
        let slinf_zone_layouts = if fan_type == WirelessFanType::SlInf {
            default_slinf_zone_layouts(fan_count)
        } else {
            Vec::new()
        };
        let zone_count = wireless_zone_count_for(fan_type, fan_count, &slinf_zone_layouts);
        Self {
            mac,
            fan_count,
            leds_per_fan,
            fan_type,
            led_state: vec![[0, 0, 0]; total_leds],
            effect_counter: 0,
            slinf_zone_layouts,
            zone_sources: vec![WirelessZoneSource::Effect(off_effect()); zone_count],
        }
    }

    #[cfg(test)]
    fn set_slinf_zone_layout(&mut self, layout: Vec<WirelessZoneLayout>) {
        self.set_slinf_zone_layouts((0..self.fan_count).map(|_| layout.clone()).collect());
    }

    fn set_slinf_zone_layouts(&mut self, layouts: Vec<Vec<WirelessZoneLayout>>) {
        self.slinf_zone_layouts = layouts;
        let zone_count = wireless_zone_count(self);
        let mut next_sources = vec![WirelessZoneSource::Effect(off_effect()); zone_count];
        for (index, source) in self.zone_sources.iter().cloned().enumerate().take(zone_count) {
            next_sources[index] = source;
        }
        self.zone_sources = next_sources;
    }
}

fn wireless_device_is_connected(
    wireless: &WirelessController,
    device: &DiscoveredDevice,
) -> bool {
    let master_mac = wireless.master_mac();
    !wireless.is_locally_detached(&device.mac)
        && !device.master_mac.iter().all(|&byte| byte == 0)
        && device.master_mac == master_mac
}

fn off_effect() -> RgbEffect {
    RgbEffect {
        mode: RgbMode::Off,
        colors: vec![[0, 0, 0]],
        ..RgbEffect::default()
    }
}

fn wireless_zone_count_for(
    fan_type: WirelessFanType,
    fan_count: u8,
    slinf_zone_layouts: &[Vec<WirelessZoneLayout>],
) -> usize {
    match fan_type {
        WirelessFanType::SlInf => slinf_zone_layouts
            .iter()
            .map(Vec::len)
            .max()
            .unwrap_or(0),
        _ => fan_count as usize,
    }
}

fn wireless_zone_count(state: &WirelessRgbState) -> usize {
    wireless_zone_count_for(state.fan_type, state.fan_count, &state.slinf_zone_layouts)
}

fn wireless_zone_led_count(state: &WirelessRgbState, zone_idx: usize) -> Option<usize> {
    match state.fan_type {
        WirelessFanType::SlInf => {
            let mut count = 0usize;
            let mut present = false;

            for fan_layout in &state.slinf_zone_layouts {
                if let Some(zone) = fan_layout.get(zone_idx) {
                    present = true;
                    count += zone.led_indexes.len();
                }
            }

            present.then_some(count)
        }
        _ => (zone_idx < state.fan_count as usize).then_some(state.leds_per_fan as usize),
    }
}

fn wireless_zone_info(state: &WirelessRgbState) -> Vec<RgbZoneInfo> {
    match state.fan_type {
        WirelessFanType::SlInf => (0..wireless_zone_count(state))
            .map(|zone_idx| {
                let name = state
                    .slinf_zone_layouts
                    .iter()
                    .find_map(|fan_layout| fan_layout.get(zone_idx).map(|zone| zone.name.clone()))
                    .unwrap_or_else(|| format!("Zone {}", zone_idx + 1));
                let led_count = state
                    .slinf_zone_layouts
                    .iter()
                    .filter_map(|fan_layout| fan_layout.get(zone_idx))
                    .map(|zone| zone.led_indexes.len() as u16)
                    .sum();

                RgbZoneInfo { name, led_count }
            })
            .collect(),
        _ => (0..state.fan_count)
            .map(|i| RgbZoneInfo {
                name: format!("Fan {}", i + 1),
                led_count: state.leds_per_fan as u16,
            })
            .collect(),
    }
}

fn wireless_supported_modes(fan_type: WirelessFanType) -> Vec<RgbMode> {
    let mut modes = vec![RgbMode::Off, RgbMode::Static, RgbMode::Direct];
    if fan_type == WirelessFanType::SlInf {
        modes.extend([
            RgbMode::Rainbow,
            RgbMode::RainbowMorph,
            RgbMode::Breathing,
            RgbMode::Runway,
            RgbMode::Meteor,
            RgbMode::ColorCycle,
            RgbMode::Staggered,
            RgbMode::Tide,
            RgbMode::Mixing,
            RgbMode::Door,
            RgbMode::Ripple,
            RgbMode::Reflect,
            RgbMode::TailChasing,
            RgbMode::Paint,
            RgbMode::PingPong,
            RgbMode::Stack,
            RgbMode::CoverCycle,
            RgbMode::Wave,
            RgbMode::Racing,
            RgbMode::Lottery,
            RgbMode::Intertwine,
            RgbMode::MeteorShower,
            RgbMode::Collide,
            RgbMode::ElectricCurrent,
            RgbMode::Kaleidoscope,
        ]);
    }
    modes
}

fn effect_is_route_aware(mode: RgbMode) -> bool {
    matches!(mode, RgbMode::Wave | RgbMode::TailChasing)
}

fn effect_is_animated(mode: RgbMode) -> bool {
    !matches!(mode, RgbMode::Off | RgbMode::Static | RgbMode::Direct)
}

fn source_is_animated(source: &WirelessZoneSource) -> bool {
    match source {
        WirelessZoneSource::Effect(effect) => effect_is_animated(effect.mode),
        WirelessZoneSource::Direct(_) => false,
    }
}

fn configured_route_segments(
    wireless_state: &HashMap<String, WirelessRgbState>,
    effect_route: &[RgbEffectRouteEntry],
    zone_idx: usize,
    full_fan_traversal: bool,
) -> Vec<RouteLedSegment> {
    effect_route
        .iter()
        .filter_map(|entry| {
            let state = wireless_state.get(&entry.device_id)?;
            if state.fan_type != WirelessFanType::SlInf {
                return None;
            }

            let fan_slot_index = entry.fan_index.checked_sub(1)? as usize;
            let led_indexes = if full_fan_traversal {
                slinf_full_fan_traversal_indexes(state, fan_slot_index)
            } else {
                let zone = state
                    .slinf_zone_layouts
                    .get(fan_slot_index)
                    .and_then(|fan_layout| fan_layout.get(zone_idx))?;
                zone.led_indexes.clone()
            };

            if led_indexes.is_empty() {
                return None;
            }

            Some(RouteLedSegment {
                device_id: entry.device_id.clone(),
                fan_slot_index,
                led_indexes,
            })
        })
        .collect()
}

fn slinf_full_fan_traversal_indexes(state: &WirelessRgbState, fan_slot_index: usize) -> Vec<usize> {
    let Some(fan_layout) = state.slinf_zone_layouts.get(fan_slot_index) else {
        return Vec::new();
    };

    let max_zone_len = fan_layout
        .iter()
        .map(|zone| zone.led_indexes.len())
        .max()
        .unwrap_or(0);

    let mut ordered = Vec::new();
    for led_offset in 0..max_zone_len {
        for zone in fan_layout {
            if let Some(led_index) = zone.led_indexes.get(led_offset) {
                ordered.push(*led_index);
            }
        }
    }

    ordered
}

fn build_wireless_route_render_plans(
    wireless_state: &HashMap<String, WirelessRgbState>,
    effect_route: &[RgbEffectRouteEntry],
    zone_idx: usize,
    effect: &RgbEffect,
) -> HashMap<String, WirelessRenderPlan> {
    let full_fan_traversal = effect.mode == RgbMode::Meteor;
    let segments = configured_route_segments(
        wireless_state,
        effect_route,
        zone_idx,
        full_fan_traversal,
    );
    if segments.is_empty() {
        return HashMap::new();
    }

    let animated_effect = effect_is_animated(effect.mode);
    let frame_count = if animated_effect {
        WIRELESS_EFFECT_FRAMES
    } else {
        1
    };
    let animated = frame_count > 1;

    let interval_ms = if effect.mode == RgbMode::Meteor || effect.mode == RgbMode::MeteorShower {
        meteor_interval_ms(effect.speed)
    } else {
        WIRELESS_EFFECT_INTERVAL_MS
    };

    let mut plans = HashMap::new();
    for segment in &segments {
        let Some(state) = wireless_state.get(&segment.device_id) else {
            continue;
        };
        plans
            .entry(segment.device_id.clone())
            .or_insert_with(|| WirelessRenderPlan {
                frames: (0..frame_count).map(|_| state.led_state.clone()).collect(),
                animated,
                interval_ms,
            });
    }

    if effect.mode == RgbMode::Meteor {
        let segment_count = segments.len().max(1);
        let base_frame_count = if animated_effect {
            (frame_count / segment_count).max(1)
        } else {
            1
        };
        let total_cycle_frames = frame_count;

        for (segment_idx, segment) in segments.iter().enumerate() {
            let Some(state) = wireless_state.get(&segment.device_id) else {
                continue;
            };
            let Some(plan) = plans.get_mut(&segment.device_id) else {
                continue;
            };

            let segment_frames = render_zone_frames(
                &WirelessZoneSource::Effect(effect.clone()),
                segment.led_indexes.len(),
                animated_effect,
            );

            let reverse_idx = segment_count - 1 - segment_idx;
            let window_start = reverse_idx * base_frame_count;
            let window_end = window_start + base_frame_count;

            for frame_idx in 0..frame_count {
                let cycle_frame = if animated_effect {
                    frame_idx % total_cycle_frames
                } else {
                    0
                };

                if animated_effect {
                    if cycle_frame < window_start || cycle_frame >= window_end {
                        continue;
                    }
                }

                let source_frame_idx = if animated_effect {
                    let local_idx = cycle_frame
                        .saturating_sub(window_start)
                        .min(base_frame_count.saturating_sub(1));
                    if base_frame_count <= 1 {
                        0
                    } else {
                        (local_idx * (WIRELESS_EFFECT_FRAMES - 1)) / (base_frame_count - 1)
                    }
                } else {
                    0
                };
                let frame = &segment_frames[source_frame_idx];
                for (offset, led_index) in segment.led_indexes.iter().enumerate() {
                    let dst_index = segment.fan_slot_index * state.leds_per_fan as usize + led_index;
                    if let Some(color) = frame.get(offset) {
                        plan.frames[frame_idx][dst_index] = *color;
                    }
                }
            }
        }

        return plans;
    }

    let total_led_count = segments
        .iter()
        .map(|segment| segment.led_indexes.len())
        .sum::<usize>();
    if total_led_count == 0 {
        return HashMap::new();
    }

    let route_frames = render_zone_frames(
        &WirelessZoneSource::Effect(effect.clone()),
        total_led_count,
        animated_effect,
    );

    for (frame_idx, route_frame) in route_frames.iter().enumerate() {
        let mut source_start = 0usize;

        for segment in &segments {
            let Some(state) = wireless_state.get(&segment.device_id) else {
                source_start += segment.led_indexes.len();
                continue;
            };
            let Some(plan) = plans.get_mut(&segment.device_id) else {
                source_start += segment.led_indexes.len();
                continue;
            };

            for (offset, led_index) in segment.led_indexes.iter().enumerate() {
                let dst_index = segment.fan_slot_index * state.leds_per_fan as usize + led_index;
                if let Some(color) = route_frame.get(source_start + offset) {
                    plan.frames[frame_idx][dst_index] = *color;
                }
            }

            source_start += segment.led_indexes.len();
        }
    }

    plans
}

fn brightness_scale(brightness: u8) -> f32 {
    (brightness as f32 / 4.0).clamp(0.0, 1.0)
}

fn scale_color(color: [u8; 3], scale: f32) -> [u8; 3] {
    [
        (color[0] as f32 * scale).clamp(0.0, 255.0) as u8,
        (color[1] as f32 * scale).clamp(0.0, 255.0) as u8,
        (color[2] as f32 * scale).clamp(0.0, 255.0) as u8,
    ]
}

fn mix_colors(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    [
        (a[0] as f32 + (b[0] as f32 - a[0] as f32) * t).round() as u8,
        (a[1] as f32 + (b[1] as f32 - a[1] as f32) * t).round() as u8,
        (a[2] as f32 + (b[2] as f32 - a[2] as f32) * t).round() as u8,
    ]
}

fn wrap01(value: f32) -> f32 {
    let wrapped = value % 1.0;
    if wrapped < 0.0 {
        wrapped + 1.0
    } else {
        wrapped
    }
}

fn wrap_distance(a: f32, b: f32) -> f32 {
    let delta = (a - b).abs();
    delta.min(1.0 - delta)
}

fn bounce01(value: f32) -> f32 {
    let wrapped = wrap01(value);
    if wrapped <= 0.5 {
        wrapped * 2.0
    } else {
        (1.0 - wrapped) * 2.0
    }
}

fn speed_factor(speed: u8) -> f32 {
    0.08 + speed.min(MAX_EFFECT_SPEED) as f32 * 0.03
}

const METEOR_FIXED_TRAIL_WIDTH: f32 = 0.525;

/// L-Connect Meteor brightness lookup table (12 discrete steps).
/// Derived from pixel-level analysis of L-Connect USB capture data.
/// Each value is the intensity relative to the head brightness (1.0 = head).
/// Index 0 = head (brightest), index 11 = tail tip (dimmest).
const METEOR_TRAIL_TABLE: [f32; 12] = [
    1.000, // head
    0.749, // 134/179
    0.497, // 89/179
    0.369, // 66/179
    0.246, // 44/179
    0.179, // 32/179
    0.151, // 27/179
    0.117, // 21/179
    0.084, // 15/179
    0.073, // 13/179
    0.056, // 10/179
    0.039, // 7/179 - tail tip
];

fn palette_from_effect(effect: &RgbEffect) -> Vec<[u8; 3]> {
    let scale = brightness_scale(effect.brightness);
    let mut palette = effect.colors.clone();
    if palette.is_empty() {
        palette.push([255, 255, 255]);
    }
    palette
        .into_iter()
        .map(|color| scale_color(color, scale))
        .collect()
}

fn palette_color(palette: &[[u8; 3]], position: f32) -> [u8; 3] {
    if palette.is_empty() {
        return [255, 255, 255];
    }

    if palette.len() == 1 {
        return palette[0];
    }

    let wrapped = wrap01(position);
    let scaled = wrapped * palette.len() as f32;
    let start = scaled.floor() as usize % palette.len();
    let end = (start + 1) % palette.len();
    let blend = scaled.fract();
    mix_colors(palette[start], palette[end], blend)
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [u8; 3] {
    let h = wrap01(h) * 6.0;
    let c = v * s;
    let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match h as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    [
        ((r + m) * 255.0).round() as u8,
        ((g + m) * 255.0).round() as u8,
        ((b + m) * 255.0).round() as u8,
    ]
}

fn nearest_scale_colors(colors: &[[u8; 3]], target_len: usize) -> Vec<[u8; 3]> {
    if target_len == 0 {
        return Vec::new();
    }
    if colors.is_empty() {
        return vec![[0, 0, 0]; target_len];
    }
    if colors.len() == target_len {
        return colors.to_vec();
    }
    if colors.len() == 1 {
        return vec![colors[0]; target_len];
    }

    let last_index = colors.len() - 1;
    (0..target_len)
        .map(|idx| {
            let pos = idx as f32 / (target_len.saturating_sub(1)).max(1) as f32;
            let source_idx = (pos * last_index as f32).round() as usize;
            colors[source_idx.min(last_index)]
        })
        .collect()
}

fn effect_phase(effect: &RgbEffect, frame_idx: usize, total_frames: usize) -> f32 {
    if total_frames <= 1 {
        return 0.0;
    }
    let mut phase = frame_idx as f32 / total_frames as f32;
    // Meteor/MeteorShower: full revolution across all frames; speed controls
    // interval_ms instead (see `meteor_interval_ms`).
    if effect.mode != RgbMode::Meteor && effect.mode != RgbMode::MeteorShower {
        phase *= speed_factor(effect.speed);
    }
    match effect.direction {
        RgbDirection::CounterClockwise | RgbDirection::Down | RgbDirection::Gather => -phase,
        _ => phase,
    }
}

/// Map Meteor speed (0..=20) to per-frame interval in milliseconds.
/// Calibrated from L-Connect captures: speed 0 -> 84 ms (7.1 s cycle),
/// speed 10 -> 60 ms (5.0 s cycle), speed 20 -> 36 ms (3.0 s cycle).
/// Linear fit: interval = 84 − 2.4 × speed.
fn meteor_interval_ms(speed: u8) -> u16 {
    let s = speed.min(MAX_EFFECT_SPEED) as f32;
    (84.0 - s * 2.4).round().clamp(36.0, 84.0) as u16
}

pub struct RgbController {
    /// Wired RGB devices keyed by device_id.
    wired: HashMap<String, Box<dyn RgbDevice>>,
    /// Wireless controller for RF-based LED control.
    wireless: Option<Arc<WirelessController>>,
    /// Wireless device state keyed by device_id ("wireless:xx:xx:xx:xx:xx:xx").
    wireless_state: HashMap<String, WirelessRgbState>,
    /// Current RGB config (from AppConfig).
    config: Option<RgbAppConfig>,
    /// When true, OpenRGB has active control — suppress native config application.
    openrgb_active: bool,
}

impl RgbController {
    pub fn new(
        wired: HashMap<String, Box<dyn RgbDevice>>,
        wireless: Option<Arc<WirelessController>>,
    ) -> Self {
        let mut wireless_state = HashMap::new();

        // Build wireless state from discovered devices
        if let Some(ref w) = wireless {
            for dev in w.devices().into_iter().filter(|dev| wireless_device_is_connected(w, dev)) {
                let device_id = format!("wireless:{}", dev.mac_str());
                wireless_state.insert(
                    device_id,
                    WirelessRgbState::new(dev.mac, dev.fan_count, dev.fan_type),
                );
            }
        }

        info!(
            "RGB controller: {} wired device(s), {} wireless device(s)",
            wired.len(),
            wireless_state.len()
        );

        Self {
            wired,
            wireless,
            wireless_state,
            config: None,
            openrgb_active: false,
        }
    }

    /// Apply an RGB config. Called on config load/change.
    pub fn apply_config(&mut self, config: &RgbAppConfig) {
        self.config = Some(config.clone());
        sync_slinf_zone_layouts(&mut self.wireless_state);

        if !config.enabled {
            info!("RGB control disabled in config");
            return;
        }

        if config.openrgb_server {
            debug!("Skipping native RGB config — OpenRGB server is enabled");
            return;
        }

        if self.openrgb_active {
            debug!("Skipping native RGB config — OpenRGB has active control");
            return;
        }

        for dev_cfg in &config.devices {
            for zone_cfg in &dev_cfg.zones {
                if let Err(e) =
                    self.set_effect(&dev_cfg.device_id, zone_cfg.zone_index, &zone_cfg.effect)
                {
                    warn!(
                        "Failed to apply RGB effect to {} zone {}: {e}",
                        dev_cfg.device_id, zone_cfg.zone_index
                    );
                }
                // Apply fan direction if the device supports it
                if zone_cfg.swap_lr || zone_cfg.swap_tb {
                    if let Err(e) = self.set_fan_direction(
                        &dev_cfg.device_id,
                        zone_cfg.zone_index,
                        zone_cfg.swap_lr,
                        zone_cfg.swap_tb,
                    ) {
                        warn!(
                            "Failed to apply fan direction to {} zone {}: {e}",
                            dev_cfg.device_id, zone_cfg.zone_index
                        );
                    }
                }
            }
        }
    }

    /// Set an effect on a specific device zone.
    pub fn set_effect(
        &mut self,
        device_id: &str,
        zone: u8,
        effect: &RgbEffect,
    ) -> anyhow::Result<()> {
        // Try wired first
        if let Some(dev) = self.wired.get(device_id) {
            dev.set_zone_effect(zone, effect)?;
            debug!("Set RGB effect on {device_id} zone {zone}: {:?}", effect.mode);
            return Ok(());
        }

        // Try wireless — update only the target zone's LEDs, then send the full buffer
        if let Some(ref wireless) = self.wireless {
            if self.wireless_state.contains_key(device_id) {
                let zone_idx = zone as usize;
                let cluster_led_count = {
                    let state = self
                        .wireless_state
                        .get(device_id)
                        .expect("wireless state checked above");
                    wireless_zone_led_count(state, zone_idx).ok_or_else(|| {
                        anyhow::anyhow!(
                            "Zone {zone} out of range (device has {} zones)",
                            wireless_zone_count(state)
                        )
                    })?
                };

                if effect_is_route_aware(effect.mode) {
                    let route_plans = self
                        .config
                        .as_ref()
                        .filter(|config| {
                            config
                                .effect_route
                                .iter()
                                .any(|entry| entry.device_id == device_id)
                        })
                        .map(|config| {
                            build_wireless_route_render_plans(
                                &self.wireless_state,
                                &config.effect_route,
                                zone_idx,
                                effect,
                            )
                        })
                        .unwrap_or_default();

                    if !route_plans.is_empty() {
                        for (route_device_id, plan) in route_plans {
                            let Some(route_state) = self.wireless_state.get_mut(&route_device_id)
                            else {
                                continue;
                            };
                            if zone_idx < route_state.zone_sources.len() {
                                route_state.zone_sources[zone_idx] =
                                    WirelessZoneSource::Effect(effect.clone());
                            }
                            send_wireless_render_plan(wireless, route_state, plan, 4)?;
                        }
                        debug!(
                            "Set routed wireless RGB on {device_id} zone {zone}: {:?}, {} cluster LEDs",
                            effect.mode, cluster_led_count
                        );
                        return Ok(());
                    }
                }

                let state = self
                    .wireless_state
                    .get_mut(device_id)
                    .expect("wireless state checked above");

                // SL-INF Meteor: the plan covers *all* fans/zones at once.
                // Set all zone_sources to the effect and send only once,
                // rather than rebuilding + re-sending for each of the 5 zones.
                if state.fan_type == WirelessFanType::SlInf && effect.mode == RgbMode::Meteor {
                    let already_applied = state.zone_sources.iter().all(|src| {
                        matches!(src, WirelessZoneSource::Effect(e) if e.mode == RgbMode::Meteor)
                    });
                    if already_applied {
                        // All zones already set to Meteor — skip redundant RF send
                        debug!(
                            "Skipped redundant Meteor send for {device_id} zone {zone}"
                        );
                        return Ok(());
                    }
                    // First zone to request Meteor: set ALL zones and send once
                    for zs in state.zone_sources.iter_mut() {
                        *zs = WirelessZoneSource::Effect(effect.clone());
                    }
                    let plan = rebuild_wireless_render_plan(state)?;
                    send_wireless_render_plan(wireless, state, plan, 4)?;
                    debug!(
                        "Set wireless RGB (full Meteor) on {device_id}: {:?}, {} cluster LEDs",
                        effect.mode, cluster_led_count
                    );
                    return Ok(());
                }

                state.zone_sources[zone_idx] = WirelessZoneSource::Effect(effect.clone());
                let plan = rebuild_wireless_render_plan(state)?;
                let animated = plan.animated;
                send_wireless_render_plan(wireless, state, plan, 4)?;
                debug!(
                    "Set wireless RGB on {device_id} zone {zone}: {:?}, {} cluster LEDs, animated={animated}",
                    effect.mode, cluster_led_count
                );
                return Ok(());
            }
        }

        anyhow::bail!("RGB device not found: {device_id}");
    }

    /// Set per-LED colors directly (used by OpenRGB `UpdateLEDs`).
    pub fn set_direct_colors(
        &mut self,
        device_id: &str,
        zone: u8,
        colors: &[[u8; 3]],
    ) -> anyhow::Result<()> {
        // Try wired
        if let Some(dev) = self.wired.get(device_id) {
            dev.set_direct_colors(zone, colors)?;
            return Ok(());
        }

        // Try wireless — update zone's LEDs and send full buffer
        if let (Some(ref wireless), Some(state)) =
            (&self.wireless, self.wireless_state.get_mut(device_id))
        {
            let zone_idx = zone as usize;
            let _ = wireless_zone_led_count(state, zone_idx).ok_or_else(|| {
                anyhow::anyhow!(
                    "Zone {zone} out of range (device has {} zones)",
                    wireless_zone_count(state)
                )
            })?;
            state.zone_sources[zone_idx] = WirelessZoneSource::Direct(colors.to_vec());
            let plan = rebuild_wireless_render_plan(state)?;
            send_wireless_render_plan(wireless, state, plan, 2)?;
            return Ok(());
        }

        anyhow::bail!("RGB device not found: {device_id}");
    }

    /// Light a single raw LED on a selected wireless fan for zone mapping.
    pub fn probe_led(
        &mut self,
        device_id: &str,
        fan_index: u8,
        led_index: u16,
        color: [u8; 3],
    ) -> anyhow::Result<()> {
        let (Some(ref wireless), Some(state)) =
            (&self.wireless, self.wireless_state.get_mut(device_id))
        else {
            anyhow::bail!("RGB device not found: {device_id}");
        };

        if fan_index == 0 || fan_index > state.fan_count {
            anyhow::bail!(
                "Fan {} out of range (device has {} fan(s))",
                fan_index,
                state.fan_count
            );
        }

        let led_index = led_index as usize;
        let leds_per_fan = state.leds_per_fan as usize;
        if led_index >= leds_per_fan {
            anyhow::bail!(
                "LED {} out of range (fan has {} addressable LEDs)",
                led_index,
                state.leds_per_fan
            );
        }

        let mut frame = vec![[0, 0, 0]; state.led_state.len()];
        let raw_index = (fan_index as usize - 1) * leds_per_fan + led_index;
        frame[raw_index] = color;
        send_wireless_render_plan(
            wireless,
            state,
            WirelessRenderPlan {
                frames: vec![frame],
                animated: false,
                interval_ms: WIRELESS_EFFECT_INTERVAL_MS,
            },
            2,
        )?;
        Ok(())
    }

    /// Return RGB capabilities for all connected devices.
    pub fn capabilities(&self) -> Vec<RgbDeviceCapabilities> {
        let mut caps = Vec::new();

        // Wired devices
        for (device_id, dev) in &self.wired {
            caps.push(RgbDeviceCapabilities {
                device_id: device_id.clone(),
                device_name: dev.device_name(),
                supported_modes: dev.supported_modes(),
                zones: dev.zone_info(),
                supports_direct: dev.supports_direct(),
                supports_mb_rgb_sync: dev.supports_mb_rgb_sync(),
                total_led_count: dev.total_led_count(),
                supported_scopes: dev.supported_scopes(),
                supports_direction: dev.supports_direction(),
            });
        }

        // Wireless devices
        for (device_id, state) in &self.wireless_state {
            let total_leds =
                state.fan_count as u16 * state.leds_per_fan as u16;

            caps.push(RgbDeviceCapabilities {
                device_id: device_id.clone(),
                device_name: state.fan_type.display_name().to_string(),
                supported_modes: wireless_supported_modes(state.fan_type),
                zones: wireless_zone_info(state),
                supports_direct: true,
                supports_mb_rgb_sync: false,
                total_led_count: total_leds,
                supported_scopes: vec![],
                supports_direction: false,
            });
        }

        caps
    }

    /// Enable or disable motherboard ARGB sync for a device.
    pub fn set_mb_rgb_sync(
        &self,
        device_id: &str,
        enabled: bool,
    ) -> anyhow::Result<()> {
        if let Some(dev) = self.wired.get(device_id) {
            if !dev.supports_mb_rgb_sync() {
                anyhow::bail!("Device {device_id} does not support MB RGB sync");
            }
            dev.set_mb_rgb_sync(enabled)?;
            info!("MB RGB sync {}: {device_id}", if enabled { "enabled" } else { "disabled" });
            return Ok(());
        }
        anyhow::bail!("RGB device not found: {device_id}");
    }

    /// Set fan direction (swap LR/TB) for a specific device zone.
    pub fn set_fan_direction(
        &self,
        device_id: &str,
        zone: u8,
        swap_lr: bool,
        swap_tb: bool,
    ) -> anyhow::Result<()> {
        if let Some(dev) = self.wired.get(device_id) {
            if !dev.supports_direction() {
                anyhow::bail!("Device {device_id} does not support fan direction");
            }
            dev.set_fan_direction(zone, swap_lr, swap_tb)?;
            debug!("Set fan direction on {device_id} zone {zone}: swap_lr={swap_lr} swap_tb={swap_tb}");
            return Ok(());
        }
        anyhow::bail!("RGB device not found: {device_id}");
    }

    /// Called when OpenRGB connects — suppress native config.
    pub fn set_openrgb_active(&mut self, active: bool) {
        if self.openrgb_active != active {
            self.openrgb_active = active;
            if active {
                info!("OpenRGB took control — suppressing native RGB config");
            } else {
                info!("OpenRGB released control");
                // Only restore native config if the OpenRGB server is disabled;
                // when the server is enabled, leave LEDs as-is so OpenRGB state persists.
                let server_enabled = self
                    .config
                    .as_ref()
                    .map(|c| c.openrgb_server)
                    .unwrap_or(false);
                if !server_enabled {
                    info!("Restoring native RGB config");
                    if let Some(config) = self.config.clone() {
                        self.apply_config(&config);
                    }
                }
            }
        }
    }

    /// Check if a device_id refers to a wireless device.
    pub fn is_wireless(&self, device_id: &str) -> bool {
        self.wireless_state.contains_key(device_id)
    }

    /// Refresh wireless device list (call after rediscovery / hot-plug).
    #[allow(dead_code)]
    pub fn refresh_wireless_devices(&mut self) {
        if let Some(ref w) = self.wireless {
            let mut new_state = HashMap::new();
            for dev in w.devices().into_iter().filter(|dev| wireless_device_is_connected(w, dev)) {
                let device_id = format!("wireless:{}", dev.mac_str());
                let (counter, led_state, zone_sources) = self
                    .wireless_state
                    .get(&device_id)
                    .map(|s| {
                        (
                            s.effect_counter,
                            Some(s.led_state.clone()),
                            Some(s.zone_sources.clone()),
                        )
                    })
                    .unwrap_or((0, None, None));

                let mut state = WirelessRgbState::new(dev.mac, dev.fan_count, dev.fan_type);
                state.effect_counter = counter;
                if let Some(leds) = led_state {
                    if leds.len() == state.led_state.len() {
                        state.led_state = leds;
                    }
                }
                if let Some(sources) = zone_sources {
                    if sources.len() == state.zone_sources.len() {
                        state.zone_sources = sources;
                    }
                }

                new_state.insert(device_id, state);
            }
            self.wireless_state = new_state;
            sync_slinf_zone_layouts(&mut self.wireless_state);
        }
    }
}

fn direction_inverts_gradient(direction: RgbDirection) -> bool {
    matches!(
        direction,
        RgbDirection::CounterClockwise | RgbDirection::Down | RgbDirection::Gather
    )
}

fn direction_mirrors_motion(direction: RgbDirection) -> bool {
    matches!(direction, RgbDirection::Spread | RgbDirection::Gather)
}

fn linear_position(index: usize, led_count: usize, mirrored: bool) -> f32 {
    if led_count <= 1 {
        return 0.0;
    }

    let position = index as f32 / (led_count - 1) as f32;
    if mirrored {
        (position - 0.5).abs() * 2.0
    } else {
        position
    }
}

fn render_motion_band(
    palette: &[[u8; 3]],
    position: f32,
    phase: f32,
    width: f32,
    tail: f32,
) -> [u8; 3] {
    let head = wrap01(phase);
    let distance = wrap_distance(position, head);
    let intensity = if distance <= width {
        1.0 - (distance / width).powf(tail.max(0.25))
    } else {
        0.0
    };
    scale_color(palette_color(palette, position + phase), intensity)
}

fn directed_wrap_distance(position: f32, head: f32, reverse: bool) -> f32 {
    let raw = if reverse {
        head - position
    } else {
        position - head
    };
    raw.rem_euclid(1.0)
}

fn render_meteor_pixel(
    palette: &[[u8; 3]],
    position: f32,
    phase: f32,
    reverse: bool,
    trail_width: f32,
    trail_curve: f32,
) -> [u8; 3] {
    let head = wrap01(phase);
    let distance = directed_wrap_distance(position, head, reverse);
    let tail = if distance <= trail_width {
        1.0 - (distance / trail_width).powf(trail_curve.max(0.25))
    } else {
        0.0
    };

    // Sharp bright tip at the very head (~2 LEDs on a 44-LED ring), then tail
    // carries most of the brightness (quadratic curve keeps it bright longer)
    let head_focus = (1.0 - (distance / 0.05).clamp(0.0, 1.0)).powf(3.0);
    let intensity = (tail * 0.85 + head_focus * 0.4).clamp(0.0, 1.0);
    let color = palette_color(palette, head);
    scale_color(color, intensity)
}

fn render_wireless_effect_frame(
    effect: &RgbEffect,
    led_count: usize,
    frame_idx: usize,
    total_frames: usize,
) -> Vec<[u8; 3]> {
    let palette = palette_from_effect(effect);
    let phase = effect_phase(effect, frame_idx, total_frames);
    let direction_enabled = effect.mode != RgbMode::Meteor;
    let mirrored = direction_enabled && direction_mirrors_motion(effect.direction);
    let invert = direction_enabled && direction_inverts_gradient(effect.direction);
    let reverse_motion = direction_enabled
        && matches!(
            effect.direction,
            RgbDirection::CounterClockwise | RgbDirection::Down | RgbDirection::Gather
        );

    (0..led_count)
        .map(|index| {
            let base_position = linear_position(index, led_count, mirrored);
            let position = if invert { 1.0 - base_position } else { base_position };

            match effect.mode {
                RgbMode::Off => [0, 0, 0],
                RgbMode::Static => palette[0],
                RgbMode::Rainbow => {
                    hsv_to_rgb(position + phase, 1.0, brightness_scale(effect.brightness))
                }
                RgbMode::RainbowMorph => {
                    hsv_to_rgb(phase + position * 0.15, 1.0, brightness_scale(effect.brightness))
                }
                RgbMode::Breathing => {
                    let intensity = (0.5 + 0.5 * (2.0 * PI * phase).sin()).clamp(0.1, 1.0);
                    scale_color(palette[0], intensity)
                }
                RgbMode::ColorCycle => palette_color(&palette, phase),
                RgbMode::Runway | RgbMode::Wave | RgbMode::TailChasing | RgbMode::Paint => {
                    palette_color(&palette, position + phase)
                }
                RgbMode::Meteor => {
                    render_meteor_pixel(
                        &palette,
                        position,
                        phase,
                        reverse_motion,
                        METEOR_FIXED_TRAIL_WIDTH,
                        2.0,
                    )
                }
                RgbMode::MeteorShower => {
                    let first =
                        render_meteor_pixel(&palette, position, phase, reverse_motion, 0.24, 1.15);
                    let second = render_meteor_pixel(
                        &palette,
                        position,
                        phase + 0.42,
                        reverse_motion,
                        0.2,
                        1.0,
                    );
                    mix_colors(first, second, 0.5)
                }
                RgbMode::Ripple => {
                    let center = bounce01(phase);
                    let wave = (1.0 - (position - center).abs() * 3.5).clamp(0.0, 1.0);
                    scale_color(palette_color(&palette, position), wave)
                }
                RgbMode::Reflect => {
                    let mirrored_position = (position - 0.5).abs() * 2.0;
                    palette_color(&palette, mirrored_position + phase)
                }
                RgbMode::PingPong => {
                    let head = bounce01(phase);
                    let distance = (position - head).abs();
                    let intensity = (1.0 - distance * 5.0).clamp(0.0, 1.0);
                    scale_color(palette_color(&palette, position), intensity)
                }
                RgbMode::Racing => {
                    let lead = render_motion_band(&palette, position, phase, 0.12, 1.0);
                    let chase = render_motion_band(&palette, position, phase + 0.5, 0.12, 1.0);
                    mix_colors(lead, chase, 0.5)
                }
                RgbMode::Collide => {
                    let left = render_motion_band(&palette, position, phase, 0.14, 1.0);
                    let right = render_motion_band(&palette, 1.0 - position, phase, 0.14, 1.0);
                    mix_colors(left, right, 0.5)
                }
                RgbMode::ElectricCurrent => {
                    let pulse =
                        ((position * 22.0 + phase * 2.0 * PI).sin() * 0.5 + 0.5).powf(3.0);
                    scale_color(palette_color(&palette, position + phase * 0.5), pulse)
                }
                RgbMode::Intertwine | RgbMode::Mixing => {
                    let stripe =
                        (((position + phase) * 8.0).floor() as usize) % palette.len().max(2);
                    palette[stripe % palette.len()]
                }
                RgbMode::Staggered | RgbMode::Stack | RgbMode::CoverCycle => {
                    let bucket = (((position + phase) * palette.len().max(2) as f32 * 2.0)
                        .floor() as usize)
                        % palette.len().max(2);
                    palette[bucket % palette.len()]
                }
                RgbMode::Tide | RgbMode::Door => {
                    let intensity =
                        (0.5 + 0.5 * (2.0 * PI * (position + phase)).sin()).clamp(0.0, 1.0);
                    scale_color(palette_color(&palette, position), intensity)
                }
                RgbMode::Lottery => {
                    let seed = ((position * 97.0) + phase * 131.0).sin().abs();
                    palette_color(&palette, seed)
                }
                RgbMode::Kaleidoscope => {
                    let spoke = ((position * 6.0 + phase * 3.0) % 1.0).abs();
                    palette_color(&palette, spoke)
                }
                _ => palette_color(&palette, position + phase),
            }
        })
        .collect()
}

fn render_zone_frames(
    source: &WirelessZoneSource,
    led_count: usize,
    animated: bool,
) -> Vec<Vec<[u8; 3]>> {
    match source {
        WirelessZoneSource::Direct(colors) => {
            let frame = nearest_scale_colors(colors, led_count);
            let frame_count = if animated { WIRELESS_EFFECT_FRAMES } else { 1 };
            vec![frame; frame_count]
        }
        WirelessZoneSource::Effect(effect) => {
            if !animated {
                return vec![render_wireless_effect_frame(effect, led_count, 0, 1)];
            }

            if !effect_is_animated(effect.mode) {
                let frame = render_wireless_effect_frame(effect, led_count, 0, 1);
                return vec![frame; WIRELESS_EFFECT_FRAMES];
            }

            (0..WIRELESS_EFFECT_FRAMES)
                .map(|frame_idx| {
                    render_wireless_effect_frame(
                        effect,
                        led_count,
                        frame_idx,
                        WIRELESS_EFFECT_FRAMES,
                    )
                })
                .collect()
        }
    }
}

fn apply_wireless_zone_frame(
    state: &WirelessRgbState,
    buffer: &mut [[u8; 3]],
    zone_idx: usize,
    colors: &[[u8; 3]],
) -> anyhow::Result<()> {
    match state.fan_type {
        WirelessFanType::SlInf => {
            let fan_zone_layouts = state
                .slinf_zone_layouts
                .iter()
                .map(|fan_layout| fan_layout.get(zone_idx))
                .collect::<Vec<_>>();
            if fan_zone_layouts.iter().all(Option::is_none) {
                anyhow::bail!("Zone {} out of range", zone_idx);
            }

            let cluster_len = fan_zone_layouts
                .iter()
                .flatten()
                .map(|zone| zone.led_indexes.len())
                .sum();
            let fitted = nearest_scale_colors(colors, cluster_len);

            let mut src_start = 0usize;
            for (fan_idx, zone) in fan_zone_layouts.into_iter().enumerate() {
                let Some(zone) = zone else {
                    continue;
                };

                for (offset, led_index) in zone.led_indexes.iter().enumerate() {
                    let dst_index = fan_idx * state.leds_per_fan as usize + led_index;
                    if let Some(color) = fitted.get(src_start + offset) {
                        buffer[dst_index] = *color;
                    }
                }
                src_start += zone.led_indexes.len();
            }

            Ok(())
        }
        _ => {
            if zone_idx >= state.fan_count as usize {
                anyhow::bail!(
                    "Zone {} out of range (device has {} zones)",
                    zone_idx,
                    wireless_zone_count(state)
                );
            }

            let start = zone_idx * state.leds_per_fan as usize;
            let end = start + state.leds_per_fan as usize;
            let fitted = nearest_scale_colors(colors, state.leds_per_fan as usize);
            buffer[start..end].copy_from_slice(&fitted);
            Ok(())
        }
    }
}

/// Number of round-robin rows per SL-INF fan in the Meteor sweep.
/// L-Connect packs 44 LEDs into 8 rows (zones with 10 LEDs contribute
/// 2 LEDs in certain rows, zones with 8 LEDs contribute exactly 1).
const METEOR_ROWS_PER_FAN: usize = 8;

/// Renders a Meteor effect for a SL-INF device using the L-Connect algorithm.
///
/// Instead of a rotating comet, L-Connect's Meteor is a sweep-wave:
/// 1. Build-up — the head advances one round-robin row per frame, lighting LEDs
///    with 12 discrete brightness levels forming a trail behind the head
/// 2. Plateau — head has swept across all fans, trail still visible
/// 3. Fade-out — trail dims and shortens until all LEDs are off again
/// 4. Dark phase — all LEDs off (remaining frames)
///
/// Parameters derived from pixel-level analysis of L-Connect USB captures
/// (both rx_type=5 / 2-fan and rx_type=6 / 3-fan).
fn rebuild_slinf_full_fan_meteor_plan(
    state: &mut WirelessRgbState,
    effect: &RgbEffect,
) -> anyhow::Result<WirelessRenderPlan> {
    let frame_count = WIRELESS_EFFECT_FRAMES;
    let total_leds = state.led_state.len();
    let palette = palette_from_effect(effect);
    let color = palette[0]; // Meteor uses a single color
    let fan_count = state.fan_count as usize;
    let leds_per_fan = state.leds_per_fan as usize;

    // Build per-fan rows. Each fan is an independent 8-row unit (no cross-fan
    // mixing). Fans are traversed in forward order (fan 0 → fan N-1), matching
    // L-Connect's 3-fan capture where the sweep goes from low buffer indices
    // to high.
    struct LedEntry {
        global_row: usize,
        dst_index: usize,
    }
    let mut led_entries: Vec<LedEntry> = Vec::with_capacity(total_leds);

    for fan_slot_index in 0..fan_count {
        let traversal = slinf_full_fan_traversal_indexes(state, fan_slot_index);
        let fan_base = fan_slot_index * leds_per_fan;
        let fan_row_offset = fan_slot_index * METEOR_ROWS_PER_FAN;
        let fan_led_count = traversal.len();

        for (t_idx, &led_index) in traversal.iter().enumerate() {
            // Distribute LEDs evenly into METEOR_ROWS_PER_FAN rows.
            let local_row = if fan_led_count > 0 {
                t_idx * METEOR_ROWS_PER_FAN / fan_led_count
            } else {
                0
            };
            led_entries.push(LedEntry {
                global_row: fan_row_offset + local_row,
                dst_index: fan_base + led_index,
            });
        }
    }

    if led_entries.is_empty() {
        return Ok(WirelessRenderPlan {
            frames: vec![vec![[0u8, 0, 0]; total_leds]; frame_count],
            animated: false,
            interval_ms: meteor_interval_ms(effect.speed),
        });
    }

    let total_rows = fan_count * METEOR_ROWS_PER_FAN;
    let trail_len = METEOR_TRAIL_TABLE.len(); // 12
    let reversed = effect.direction == RgbDirection::CounterClockwise;

    // Active phase: head crosses total_rows, then trail_len-1 more frames for
    // the trail to fully fade. Dark phase fills the remaining frames.
    // Forward: active at start (frames 0..active_frames).
    // Reversed: active at end (frames start_offset..start_offset+active_frames),
    //           matching L-Connect captures.
    let active_frames = (total_rows + trail_len - 1).min(frame_count);
    let start_offset = if reversed { frame_count.saturating_sub(active_frames) } else { 0 };

    let mut frames = vec![vec![[0u8, 0, 0]; total_leds]; frame_count];

    for step in 0..active_frames {
        let frame_idx = start_offset + step;
        let head_row = if reversed {
            // Sweep from last row to first.
            total_rows.saturating_sub(1).saturating_sub(step.min(total_rows.saturating_sub(1)))
        } else {
            step
        };

        for entry in &led_entries {
            // Distance from head: positive means the trail is behind the head.
            let distance = if reversed {
                entry.global_row as isize - head_row as isize
            } else {
                head_row as isize - entry.global_row as isize
            };

            if distance < 0 || distance >= trail_len as isize {
                continue; // Not yet lit, or already faded
            }

            let intensity = METEOR_TRAIL_TABLE[distance as usize];
            if entry.dst_index < total_leds {
                frames[frame_idx][entry.dst_index] = scale_color(color, intensity);
            }
        }
    }

    if let Some(last_frame) = frames.last().cloned() {
        state.led_state = last_frame;
    }

    Ok(WirelessRenderPlan { frames, animated: true, interval_ms: meteor_interval_ms(effect.speed) })
}

fn rebuild_wireless_render_plan(state: &mut WirelessRgbState) -> anyhow::Result<WirelessRenderPlan> {
    // SL-INF Meteor: use full-fan traversal so the animation sweeps all 44 LEDs
    // of each fan as one continuous ring rather than 5 independent zone meteors.
    if state.fan_type == WirelessFanType::SlInf {
        if let Some(meteor_effect) = state.zone_sources.iter().find_map(|src| {
            if let WirelessZoneSource::Effect(e) = src {
                if e.mode == RgbMode::Meteor { Some(e.clone()) } else { None }
            } else {
                None
            }
        }) {
            return rebuild_slinf_full_fan_meteor_plan(state, &meteor_effect);
        }
    }

    let animated = state.zone_sources.iter().any(source_is_animated);
    let frame_count = if animated { WIRELESS_EFFECT_FRAMES } else { 1 };
    let total_leds = state.led_state.len();
    let mut frames = vec![vec![[0, 0, 0]; total_leds]; frame_count];

    for zone_idx in 0..wireless_zone_count(state) {
        let zone_led_count = wireless_zone_led_count(state, zone_idx).ok_or_else(|| {
            anyhow::anyhow!(
                "Zone {} out of range (device has {} zones)",
                zone_idx,
                wireless_zone_count(state)
            )
        })?;
        let zone_frames = render_zone_frames(&state.zone_sources[zone_idx], zone_led_count, animated);
        for frame_idx in 0..frame_count {
            apply_wireless_zone_frame(state, &mut frames[frame_idx], zone_idx, &zone_frames[frame_idx])?;
        }
    }

    if let Some(last_frame) = frames.last().cloned() {
        state.led_state = last_frame;
    }

    let interval_ms = state.zone_sources.iter().find_map(|s| {
        if let WirelessZoneSource::Effect(e) = s {
            if e.mode == RgbMode::Meteor || e.mode == RgbMode::MeteorShower {
                return Some(meteor_interval_ms(e.speed));
            }
        }
        None
    }).unwrap_or(WIRELESS_EFFECT_INTERVAL_MS);

    Ok(WirelessRenderPlan { frames, animated, interval_ms })
}

fn send_wireless_render_plan(
    wireless: &WirelessController,
    state: &mut WirelessRgbState,
    plan: WirelessRenderPlan,
    header_repeats: u8,
) -> anyhow::Result<()> {
    state.effect_counter = state.effect_counter.wrapping_add(1);
    let idx = state.effect_counter.to_be_bytes();

    if plan.animated {
        wireless.send_rgb_frames(
            &state.mac,
            &plan.frames,
            plan.interval_ms,
            &idx,
            header_repeats,
        )?;
    } else {
        let frame = plan
            .frames
            .into_iter()
            .next()
            .unwrap_or_else(|| vec![[0, 0, 0]; state.led_state.len()]);
        state.led_state = frame.clone();
        wireless.send_rgb_direct(&state.mac, &frame, &idx, header_repeats)?;
    }

    Ok(())
}

/// Buffers per-device, per-zone direct color updates for async flushing.
///
/// The OpenRGB TCP handler writes latest colors here (fast, no device I/O).
/// A writer thread flushes dirty devices at ~30fps, dropping intermediate frames.
pub struct DirectColorBuffer {
    pending: HashMap<String, HashMap<u8, Vec<[u8; 3]>>>,
}

impl DirectColorBuffer {
    pub fn new() -> Self {
        Self {
            pending: HashMap::new(),
        }
    }

    /// Store colors for a device zone (overwrites any previous pending value).
    pub fn set(&mut self, device_id: String, zone: u8, colors: Vec<[u8; 3]>) {
        self.pending.entry(device_id).or_default().insert(zone, colors);
    }

    /// Take all pending updates, clearing the buffer.
    pub fn take_all(&mut self) -> HashMap<String, HashMap<u8, Vec<[u8; 3]>>> {
        std::mem::take(&mut self.pending)
    }
}

/// Spawns a background thread that flushes buffered direct colors.
///
/// Wired devices are processed first for lowest latency.
/// Wireless devices use single-frame direct sends.
pub fn start_direct_color_writer(
    rgb: Arc<Mutex<RgbController>>,
    buffer: Arc<Mutex<DirectColorBuffer>>,
    stop_flag: Arc<AtomicBool>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        debug!("Direct color writer started");

        loop {
            if stop_flag.load(Ordering::Relaxed) {
                break;
            }

            let updates = buffer.lock().take_all();

            if !updates.is_empty() {
                // Split into wired and wireless so wired always goes first
                let mut wired = Vec::new();
                let mut wireless = Vec::new();
                {
                    let rgb = rgb.lock();
                    for (device_id, zones) in updates {
                        if rgb.is_wireless(&device_id) {
                            wireless.push((device_id, zones));
                        } else {
                            wired.push((device_id, zones));
                        }
                    }
                }

                // Wired: flush immediately with minimal lock time
                if !wired.is_empty() {
                    let mut rgb = rgb.lock();
                    for (device_id, zones) in wired {
                        for (zone, colors) in zones {
                            if let Err(e) = rgb.set_direct_colors(&device_id, zone, &colors) {
                                debug!("Wired flush error for {device_id} zone {zone}: {e}");
                            }
                        }
                    }
                }

                // Wireless: send latest color state per device
                if !wireless.is_empty() {
                    let mut rgb = rgb.lock();
                    for (device_id, zones) in wireless {
                        for (zone, colors) in zones {
                            if let Err(e) = rgb.set_direct_colors(&device_id, zone, &colors) {
                                debug!("Wireless flush error for {device_id} zone {zone}: {e}");
                            }
                        }
                    }
                }
            } else {
                thread::sleep(Duration::from_millis(5));
            }
        }

        debug!("Direct color writer stopped");
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slinf_zone_led_count_spans_full_cluster() {
        let state = WirelessRgbState::new([0; 6], 3, WirelessFanType::SlInf);

        // Zone sizes (8,10,8,10,8) × 3 fans
        assert_eq!(wireless_zone_led_count(&state, 0), Some(24));
        assert_eq!(wireless_zone_led_count(&state, 1), Some(30));
        assert_eq!(wireless_zone_led_count(&state, 2), Some(24));
        assert_eq!(wireless_zone_led_count(&state, 3), Some(30));
        assert_eq!(wireless_zone_led_count(&state, 4), Some(24));
    }

    #[test]
    fn slinf_zone_frame_flows_across_fans() {
        let state = WirelessRgbState::new([0; 6], 2, WirelessFanType::SlInf);
        let mut frame = vec![[0, 0, 0]; state.led_state.len()];
        // Zone 1 has 10 LEDs per fan = 20 across 2 fans
        let colors: Vec<[u8; 3]> = (0..20).map(|idx| [idx as u8, 0, 0]).collect();

        apply_wireless_zone_frame(&state, &mut frame, 1, &colors).unwrap();

        // Zone 1 = LEDs 8-17 per fan
        assert_eq!(frame[7], [0, 0, 0]);
        assert_eq!(frame[8], [0, 0, 0]);
        assert_eq!(frame[17], [9, 0, 0]);
        assert_eq!(frame[52], [10, 0, 0]);
        assert_eq!(frame[61], [19, 0, 0]);
    }

    #[test]
    fn slinf_zone_frame_respects_per_fan_layouts() {
        let mut state = WirelessRgbState::new([0; 6], 2, WirelessFanType::SlInf);
        state.set_slinf_zone_layouts(vec![
            vec![WirelessZoneLayout {
                name: "Zone 1".to_string(),
                led_indexes: vec![2, 0],
            }],
            vec![WirelessZoneLayout {
                name: "Zone 1".to_string(),
                led_indexes: vec![1, 3],
            }],
        ]);

        let mut frame = vec![[0, 0, 0]; state.led_state.len()];
        let colors = vec![[10, 0, 0], [20, 0, 0], [30, 0, 0], [40, 0, 0]];

        apply_wireless_zone_frame(&state, &mut frame, 0, &colors).unwrap();

        assert_eq!(frame[2], [10, 0, 0]);
        assert_eq!(frame[0], [20, 0, 0]);
        assert_eq!(frame[45], [30, 0, 0]);
        assert_eq!(frame[47], [40, 0, 0]);
    }

    #[test]
    fn slinf_zone_frame_respects_configured_led_order() {
        let mut state = WirelessRgbState::new([0; 6], 1, WirelessFanType::SlInf);
        state.set_slinf_zone_layout(vec![WirelessZoneLayout {
            name: "Zone 1".to_string(),
            led_indexes: vec![2, 0, 1],
        }]);

        let mut frame = vec![[0, 0, 0]; state.led_state.len()];
        let colors = vec![[10, 0, 0], [20, 0, 0], [30, 0, 0]];

        apply_wireless_zone_frame(&state, &mut frame, 0, &colors).unwrap();

        assert_eq!(frame[2], [10, 0, 0]);
        assert_eq!(frame[0], [20, 0, 0]);
        assert_eq!(frame[1], [30, 0, 0]);
    }

    #[test]
    fn animated_slinf_effect_respects_configured_led_order() {
        let mut state = WirelessRgbState::new([0; 6], 1, WirelessFanType::SlInf);
        state.set_slinf_zone_layout(vec![WirelessZoneLayout {
            name: "Zone 1".to_string(),
            led_indexes: vec![2, 0, 3, 1],
        }]);

        let effect = RgbEffect {
            mode: RgbMode::Rainbow,
            colors: vec![[255, 255, 255]],
            speed: 2,
            brightness: 4,
            direction: RgbDirection::Clockwise,
            scope: lianli_shared::rgb::RgbScope::All,
            smoothness_ms: 0,
        };
        state.zone_sources[0] = WirelessZoneSource::Effect(effect.clone());

        let plan = rebuild_wireless_render_plan(&mut state).unwrap();
        let logical_frame = render_wireless_effect_frame(
            &effect,
            4,
            0,
            WIRELESS_EFFECT_FRAMES,
        );

        assert_eq!(plan.frames.len(), WIRELESS_EFFECT_FRAMES);
        assert_eq!(plan.frames[0][2], logical_frame[0]);
        assert_eq!(plan.frames[0][0], logical_frame[1]);
        assert_eq!(plan.frames[0][3], logical_frame[2]);
        assert_eq!(plan.frames[0][1], logical_frame[3]);
    }

    #[test]
    fn route_aware_effect_flows_across_wireless_devices_in_saved_order() {
        let mut device_a = WirelessRgbState::new([0; 6], 1, WirelessFanType::SlInf);
        device_a.set_slinf_zone_layout(vec![WirelessZoneLayout {
            name: "Zone 1".to_string(),
            led_indexes: vec![1, 0],
        }]);

        let mut device_b = WirelessRgbState::new([1; 6], 1, WirelessFanType::SlInf);
        device_b.set_slinf_zone_layout(vec![WirelessZoneLayout {
            name: "Zone 1".to_string(),
            led_indexes: vec![3, 2],
        }]);

        let mut wireless_state = HashMap::new();
        wireless_state.insert("wireless:a".to_string(), device_a);
        wireless_state.insert("wireless:b".to_string(), device_b);

        let route = vec![
            RgbEffectRouteEntry {
                device_id: "wireless:a".to_string(),
                fan_index: 1,
            },
            RgbEffectRouteEntry {
                device_id: "wireless:b".to_string(),
                fan_index: 1,
            },
        ];
        let effect = RgbEffect {
            mode: RgbMode::Wave,
            colors: vec![[0x11, 0x22, 0x33]],
            speed: 2,
            brightness: 4,
            direction: RgbDirection::Clockwise,
            scope: lianli_shared::rgb::RgbScope::All,
            smoothness_ms: 0,
        };

        let plans = build_wireless_route_render_plans(&wireless_state, &route, 0, &effect);
        let logical_frame = render_wireless_effect_frame(&effect, 4, 0, WIRELESS_EFFECT_FRAMES);

        assert_eq!(plans.len(), 2);
        assert_eq!(plans["wireless:a"].frames[0][1], logical_frame[0]);
        assert_eq!(plans["wireless:a"].frames[0][0], logical_frame[1]);
        assert_eq!(plans["wireless:b"].frames[0][3], logical_frame[2]);
        assert_eq!(plans["wireless:b"].frames[0][2], logical_frame[3]);
    }

    #[test]
    fn route_aware_effect_skips_missing_route_members_without_shifting_remaining_order() {
        let mut device_a = WirelessRgbState::new([0; 6], 1, WirelessFanType::SlInf);
        device_a.set_slinf_zone_layout(vec![WirelessZoneLayout {
            name: "Zone 1".to_string(),
            led_indexes: vec![2, 0],
        }]);

        let mut wireless_state = HashMap::new();
        wireless_state.insert("wireless:a".to_string(), device_a);

        let route = vec![
            RgbEffectRouteEntry {
                device_id: "wireless:missing".to_string(),
                fan_index: 1,
            },
            RgbEffectRouteEntry {
                device_id: "wireless:a".to_string(),
                fan_index: 1,
            },
        ];
        let effect = RgbEffect {
            mode: RgbMode::Meteor,
            colors: vec![[0xff, 0x00, 0x00]],
            speed: 2,
            brightness: 4,
            direction: RgbDirection::Clockwise,
            scope: lianli_shared::rgb::RgbScope::All,
            smoothness_ms: 0,
        };

        let plans = build_wireless_route_render_plans(&wireless_state, &route, 0, &effect);
        let logical_frame = render_wireless_effect_frame(&effect, 2, 0, WIRELESS_EFFECT_FRAMES);

        assert_eq!(plans.len(), 1);
        assert_eq!(plans["wireless:a"].frames[0][2], logical_frame[0]);
        assert_eq!(plans["wireless:a"].frames[0][0], logical_frame[1]);
    }

    #[test]
    fn route_aware_effect_compacts_missing_members_between_active_segments() {
        let mut device_a = WirelessRgbState::new([0; 6], 1, WirelessFanType::SlInf);
        device_a.set_slinf_zone_layout(vec![WirelessZoneLayout {
            name: "Zone 1".to_string(),
            led_indexes: vec![1, 0],
        }]);

        let mut device_b = WirelessRgbState::new([1; 6], 1, WirelessFanType::SlInf);
        device_b.set_slinf_zone_layout(vec![WirelessZoneLayout {
            name: "Zone 1".to_string(),
            led_indexes: vec![3, 2],
        }]);

        let mut wireless_state = HashMap::new();
        wireless_state.insert("wireless:a".to_string(), device_a);
        wireless_state.insert("wireless:b".to_string(), device_b);

        let route = vec![
            RgbEffectRouteEntry {
                device_id: "wireless:a".to_string(),
                fan_index: 1,
            },
            RgbEffectRouteEntry {
                device_id: "wireless:missing".to_string(),
                fan_index: 1,
            },
            RgbEffectRouteEntry {
                device_id: "wireless:b".to_string(),
                fan_index: 1,
            },
        ];
        let effect = RgbEffect {
            mode: RgbMode::Meteor,
            colors: vec![[0x77, 0xaa, 0xff]],
            speed: 2,
            brightness: 4,
            direction: RgbDirection::Clockwise,
            scope: lianli_shared::rgb::RgbScope::All,
            smoothness_ms: 0,
        };

        let plans = build_wireless_route_render_plans(&wireless_state, &route, 0, &effect);
        let logical_frame = render_wireless_effect_frame(&effect, 2, 0, WIRELESS_EFFECT_FRAMES);

        assert_eq!(plans.len(), 2);
        assert_eq!(plans["wireless:a"].frames.len(), WIRELESS_EFFECT_FRAMES);

        // Reversed and sequential: route segment B runs first, then segment A.
        assert_eq!(plans["wireless:b"].frames[0][3], logical_frame[0]);
        assert_eq!(plans["wireless:b"].frames[0][2], logical_frame[1]);
        assert_eq!(plans["wireless:a"].frames[0][1], [0, 0, 0]);
        assert_eq!(plans["wireless:a"].frames[0][0], [0, 0, 0]);

        assert_eq!(plans["wireless:a"].frames[WIRELESS_EFFECT_FRAMES / 2][1], logical_frame[0]);
        assert_eq!(plans["wireless:a"].frames[WIRELESS_EFFECT_FRAMES / 2][0], logical_frame[1]);
        assert_eq!(plans["wireless:b"].frames[WIRELESS_EFFECT_FRAMES / 2][3], [0, 0, 0]);
        assert_eq!(plans["wireless:b"].frames[WIRELESS_EFFECT_FRAMES / 2][2], [0, 0, 0]);
    }

    #[test]
    fn meteor_renders_single_direction_trail() {
        // smoothness_ms: 1500 gives trail_width ≈ 0.525, matching the original
        // hardcoded value and ensuring frame[2] out of 10 LEDs is still lit.
        let effect = RgbEffect {
            mode: RgbMode::Meteor,
            colors: vec![[255, 120, 40]],
            speed: 0,
            brightness: 4,
            direction: RgbDirection::Clockwise,
            scope: lianli_shared::rgb::RgbScope::All,
            smoothness_ms: 1500,
        };

        let frame = render_wireless_effect_frame(&effect, 10, 0, WIRELESS_EFFECT_FRAMES);

        let head = frame[0][0] as u16 + frame[0][1] as u16 + frame[0][2] as u16;
        let near_tail = frame[2][0] as u16 + frame[2][1] as u16 + frame[2][2] as u16;
        let far_ahead = frame[8][0] as u16 + frame[8][1] as u16 + frame[8][2] as u16;

        assert!(head > near_tail);
        assert!(near_tail > 0);
        assert_eq!(far_ahead, 0);
    }

    #[test]
    fn slinf_full_fan_traversal_indexes_follow_zone_led_round_robin_order() {
        let state = WirelessRgbState::new([0; 6], 1, WirelessFanType::SlInf);
        let traversal = slinf_full_fan_traversal_indexes(&state, 0);

        assert_eq!(traversal.len(), 44);
        // Zone layout (8,10,8,10,8): round-robin picks one LED per zone per offset.
        assert_eq!(traversal[0], 0);
        assert_eq!(traversal[1], 8);
        assert_eq!(traversal[2], 18);
        assert_eq!(traversal[3], 26);
        assert_eq!(traversal[4], 36);
        assert_eq!(traversal[5], 1);
        assert_eq!(traversal[6], 9);
        assert_eq!(traversal[7], 19);
        assert_eq!(traversal[8], 27);
        assert_eq!(traversal[9], 37);
        // Offsets 8-9 only yield 2 LEDs each (from 10-LED zones)
        assert_eq!(traversal[40], 16);
        assert_eq!(traversal[41], 34);
        assert_eq!(traversal[42], 17);
        assert_eq!(traversal[43], 35);
        assert!(traversal.contains(&43));
        assert_eq!(*traversal.last().unwrap(), 35);
    }

    #[test]
    fn route_segments_use_full_fan_traversal_for_meteor() {
        let state = WirelessRgbState::new([0; 6], 1, WirelessFanType::SlInf);
        let mut wireless_state = HashMap::new();
        wireless_state.insert("wireless:a".to_string(), state);

        let route = vec![RgbEffectRouteEntry {
            device_id: "wireless:a".to_string(),
            fan_index: 1,
        }];

        let segments = configured_route_segments(&wireless_state, &route, 0, true);
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].led_indexes.len(), 44);
        assert_eq!(segments[0].led_indexes[0], 0);
        assert_eq!(segments[0].led_indexes[1], 8);
        assert_eq!(segments[0].led_indexes[2], 18);
    }
}
