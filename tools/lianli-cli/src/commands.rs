use crate::daemon_client::DaemonClient;
use anyhow::{anyhow, bail, Context, Result};
use clap::{Parser, Subcommand};
use lianli_shared::config::AppConfig;
use lianli_shared::fan::{FanConfig, FanGroup, FanSpeed};
use lianli_shared::ipc::{DeviceInfo, IpcRequest, IpcResponse, TelemetrySnapshot};
use lianli_shared::rgb::{RgbDirection, RgbEffect, RgbMode, RgbScope, MAX_EFFECT_SPEED};
use serde::de::DeserializeOwned;
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

const SLINF_ZONE_SEGMENTS: [(u16, u16); 5] = [
    (0, 9),
    (9, 9),
    (18, 9),
    (27, 9),
    (36, 8),
];

#[derive(Parser, Debug)]
#[command(name = "lianli-cli", about = "Headless IPC client for lianli-daemon")]
pub struct Cli {
    /// Path to daemon Unix socket
    #[arg(long)]
    pub socket: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Ping the daemon IPC socket
    Ping,

    /// List devices known to the daemon
    Devices,

    /// Show device status (telemetry + info). If no device_id, prints all.
    DeviceStatus {
        /// Device ID (from `devices`)
        device_id: Option<String>,
    },

    /// Set static color for a device/zone
    SetColor {
        /// Device ID
        device_id: String,
        /// RGB zone index (default 0)
        #[arg(long, default_value_t = 0)]
        zone: u8,
        /// Hex color, e.g. #ff00aa
        #[arg(long)]
        hex: Option<String>,
        /// RGB values, e.g. --rgb 255 0 170
        #[arg(long, num_args = 3)]
        rgb: Option<Vec<u8>>,
    },

    /// Set RGB effect mode for a device/zone
    SetEffect {
        /// Device ID
        device_id: String,
        /// Effect name (e.g. Static, Rainbow, Breathing)
        effect: String,
        /// RGB zone index (default 0)
        #[arg(long, default_value_t = 0)]
        zone: u8,
        /// Optional speed (0-20)
        #[arg(long)]
        speed: Option<u8>,
        /// Optional brightness (0-4)
        #[arg(long)]
        brightness: Option<u8>,
        /// Optional hex color (for colorized effects)
        #[arg(long)]
        hex: Option<String>,
        /// Optional RGB values (for colorized effects)
        #[arg(long, num_args = 3)]
        rgb: Option<Vec<u8>>,
    },

    /// Set brightness (0-100 percent) for a device/zone
    SetBrightness {
        /// Device ID
        device_id: String,
        /// Brightness percent (0-100)
        percent: u8,
        /// RGB zone index (default 0)
        #[arg(long, default_value_t = 0)]
        zone: u8,
    },

    /// Set fixed fan speed percent for a device (config-driven)
    SetFan {
        /// Device ID (including :portN if applicable)
        device_id: String,
        /// Fan speed percent (0-100)
        percent: u8,
        /// Optional fan slot (1-4). If omitted, applies to all slots.
        #[arg(long)]
        slot: Option<u8>,
    },

    /// Get daemon config (JSON)
    GetConfig,

    /// Save config from a JSON file
    SaveConfig {
        /// Path to JSON config file
        file: PathBuf,
    },

    /// Probe LEDs in Meteor traversal order (route x zones x LEDs)
    ProbeMeteorOrder {
        /// Duration each LED stays lit in milliseconds
        #[arg(long, default_value_t = 1000)]
        step_ms: u64,
        /// Probe color as hex, e.g. #ff00aa
        #[arg(long)]
        hex: Option<String>,
        /// Probe color as RGB triplet, e.g. --rgb 255 0 170
        #[arg(long, num_args = 3)]
        rgb: Option<Vec<u8>>,
        /// Print steps without sending probe commands
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },
}

pub fn default_socket_path() -> PathBuf {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(runtime_dir).join("lianli-daemon.sock")
}

pub fn execute(cli: Cli) -> Result<()> {
    let socket_path = cli.socket.unwrap_or_else(default_socket_path);
    let client = DaemonClient::new(socket_path);

    match cli.command {
        Command::Ping => handle_ping(&client),
        Command::Devices => handle_devices(&client),
        Command::DeviceStatus { device_id } => handle_device_status(&client, device_id),
        Command::SetColor {
            device_id,
            zone,
            hex,
            rgb,
        } => handle_set_color(&client, device_id, zone, hex, rgb),
        Command::SetEffect {
            device_id,
            effect,
            zone,
            speed,
            brightness,
            hex,
            rgb,
        } => handle_set_effect(&client, device_id, effect, zone, speed, brightness, hex, rgb),
        Command::SetBrightness {
            device_id,
            percent,
            zone,
        } => handle_set_brightness(&client, device_id, percent, zone),
        Command::SetFan {
            device_id,
            percent,
            slot,
        } => handle_set_fan(&client, device_id, percent, slot),
        Command::GetConfig => handle_get_config(&client),
        Command::SaveConfig { file } => handle_save_config(&client, file),
        Command::ProbeMeteorOrder {
            step_ms,
            hex,
            rgb,
            dry_run,
        } => handle_probe_meteor_order(&client, step_ms, hex, rgb, dry_run),
    }
}

fn handle_ping(client: &DaemonClient) -> Result<()> {
    let response = client.send(&IpcRequest::Ping)?;
    match response {
        IpcResponse::Ok { data } => {
            if let Some(s) = data.as_str() {
                println!("{s}");
            } else {
                println!("{data}");
            }
            Ok(())
        }
        IpcResponse::Error { message } => bail!("daemon error: {message}"),
    }
}

fn handle_devices(client: &DaemonClient) -> Result<()> {
    let response = client.send(&IpcRequest::ListDevices)?;
    let devices: Vec<DeviceInfo> = unwrap_response(response)?;
    println!("{}", serde_json::to_string_pretty(&devices)?);
    Ok(())
}

fn handle_device_status(client: &DaemonClient, device_id: Option<String>) -> Result<()> {
    let devices: Vec<DeviceInfo> = unwrap_response(client.send(&IpcRequest::ListDevices)?)?;
    let telemetry: TelemetrySnapshot = unwrap_response(client.send(&IpcRequest::GetTelemetry)?)?;

    if let Some(id) = device_id {
        let device = devices
            .iter()
            .find(|d| d.device_id == id)
            .cloned()
            .ok_or_else(|| anyhow!("device not found: {id}"))?;

        let fan_rpms = telemetry.fan_rpms.get(&device.device_id).cloned();
        let coolant_temp = telemetry.coolant_temps.get(&device.device_id).cloned();

        let out = json!({
            "device": device,
            "telemetry": {
                "fan_rpms": fan_rpms,
                "coolant_temp": coolant_temp,
                "streaming_active": telemetry.streaming_active,
                "openrgb_status": telemetry.openrgb_status,
            }
        });

        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    let list: Vec<_> = devices
        .into_iter()
        .map(|d| {
            let fan_rpms = telemetry.fan_rpms.get(&d.device_id).cloned();
            let coolant_temp = telemetry.coolant_temps.get(&d.device_id).cloned();
            json!({
                "device": d,
                "telemetry": {
                    "fan_rpms": fan_rpms,
                    "coolant_temp": coolant_temp,
                }
            })
        })
        .collect();

    let out = json!({
        "streaming_active": telemetry.streaming_active,
        "openrgb_status": telemetry.openrgb_status,
        "devices": list,
    });
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

fn handle_set_color(
    client: &DaemonClient,
    device_id: String,
    zone: u8,
    hex: Option<String>,
    rgb: Option<Vec<u8>>,
) -> Result<()> {
    let color = parse_color(hex, rgb)?;
    let mut effect = base_effect(client, &device_id, zone).unwrap_or_default();
    effect.mode = RgbMode::Static;
    effect.colors = vec![color];
    send_rgb_effect(client, device_id, zone, effect)
}

fn handle_set_effect(
    client: &DaemonClient,
    device_id: String,
    effect_name: String,
    zone: u8,
    speed: Option<u8>,
    brightness: Option<u8>,
    hex: Option<String>,
    rgb: Option<Vec<u8>>,
) -> Result<()> {
    let mode = parse_rgb_mode(&effect_name)?;
    let mut effect = base_effect(client, &device_id, zone).unwrap_or_default();
    effect.mode = mode;

    if let Some(spd) = speed {
        if spd > MAX_EFFECT_SPEED {
            bail!("speed must be between 0 and {MAX_EFFECT_SPEED}");
        }
        effect.speed = spd;
    }

    if let Some(bri) = brightness {
        if bri > 4 {
            bail!("brightness must be between 0 and 4");
        }
        effect.brightness = bri;
    }

    if hex.is_some() || rgb.is_some() {
        let color = parse_color(hex, rgb)?;
        effect.colors = vec![color];
    }

    send_rgb_effect(client, device_id, zone, effect)
}

fn handle_set_brightness(
    client: &DaemonClient,
    device_id: String,
    percent: u8,
    zone: u8,
) -> Result<()> {
    if percent > 100 {
        bail!("brightness percent must be 0-100");
    }
    let level = brightness_from_percent(percent);
    let mut effect = base_effect(client, &device_id, zone).unwrap_or_default();
    effect.brightness = level;
    send_rgb_effect(client, device_id, zone, effect)
}

fn handle_set_fan(
    client: &DaemonClient,
    device_id: String,
    percent: u8,
    slot: Option<u8>,
) -> Result<()> {
    if percent > 100 {
        bail!("fan percent must be 0-100");
    }
    if let Some(slot) = slot {
        if !(1..=4).contains(&slot) {
            bail!("slot must be in range 1-4");
        }
    }

    let pwm = ((percent as f32 / 100.0) * 255.0).round() as u8;
    let mut fan_cfg = load_fan_config(client).unwrap_or_else(|_| default_fan_config());

    let group = match fan_cfg
        .speeds
        .iter_mut()
        .find(|g| g.device_id.as_deref() == Some(&device_id))
    {
        Some(g) => g,
        None => {
            fan_cfg.speeds.push(FanGroup {
                device_id: Some(device_id.clone()),
                speeds: default_fan_speeds(0),
            });
            fan_cfg.speeds.last_mut().unwrap()
        }
    };

    if let Some(slot) = slot {
        let idx = (slot - 1) as usize;
        let mut speeds = group.speeds.clone();
        speeds[idx] = FanSpeed::Constant(pwm);
        group.speeds = speeds;
    } else {
        group.speeds = default_fan_speeds(pwm);
    }

    let response = client.send(&IpcRequest::SetFanConfig { config: fan_cfg })?;
    match response {
        IpcResponse::Ok { .. } => {
            println!("ok");
            Ok(())
        }
        IpcResponse::Error { message } => bail!("daemon error: {message}"),
    }
}

fn handle_get_config(client: &DaemonClient) -> Result<()> {
    let response = client.send(&IpcRequest::GetConfig)?;
    let config: AppConfig = unwrap_response(response)?;
    println!("{}", serde_json::to_string_pretty(&config)?);
    Ok(())
}

fn handle_save_config(client: &DaemonClient, file: PathBuf) -> Result<()> {
    let raw = fs::read_to_string(&file)
        .with_context(|| format!("read config file {}", file.display()))?;
    let config: AppConfig = serde_json::from_str(&raw)
        .with_context(|| "parse config JSON")?;
    let response = client.send(&IpcRequest::SetConfig { config })?;
    match response {
        IpcResponse::Ok { .. } => {
            println!("ok");
            Ok(())
        }
        IpcResponse::Error { message } => bail!("daemon error: {message}"),
    }
}

fn handle_probe_meteor_order(
    client: &DaemonClient,
    step_ms: u64,
    hex: Option<String>,
    rgb: Option<Vec<u8>>,
    dry_run: bool,
) -> Result<()> {
    if step_ms == 0 {
        bail!("step_ms must be greater than zero");
    }

    let color = if hex.is_some() || rgb.is_some() {
        parse_color(hex, rgb)?
    } else {
        [255, 255, 255]
    };

    let devices: Vec<DeviceInfo> = unwrap_response(client.send(&IpcRequest::ListDevices)?)?;
    let device_map = devices
        .into_iter()
        .map(|device| (device.device_id.clone(), device))
        .collect::<std::collections::HashMap<_, _>>();

    let config: AppConfig = unwrap_response(client.send(&IpcRequest::GetConfig)?)?;
    let Some(rgb_cfg) = config.rgb else {
        bail!("config has no rgb section");
    };

    if rgb_cfg.effect_route.is_empty() {
        bail!("rgb.effect_route is empty");
    }

    let mut restore_effects = std::collections::HashMap::<(String, u8), RgbEffect>::new();
    let mut steps = 0usize;
    let sleep_duration = Duration::from_millis(step_ms);

    for entry in rgb_cfg.effect_route {
        let Some(device) = device_map.get(&entry.device_id) else {
            eprintln!(
                "skip route entry {} fan {}: device not present",
                entry.device_id, entry.fan_index
            );
            continue;
        };

        if !entry.device_id.starts_with("wireless:") {
            eprintln!(
                "skip route entry {} fan {}: not a wireless device",
                entry.device_id, entry.fan_index
            );
            continue;
        }

        let fan_count = device.fan_count.unwrap_or(0);
        if fan_count == 0 || entry.fan_index == 0 || entry.fan_index > fan_count {
            eprintln!(
                "skip route entry {} fan {}: fan out of range (device fan_count={fan_count})",
                entry.device_id, entry.fan_index
            );
            continue;
        }

        for (zone_offset, _) in SLINF_ZONE_SEGMENTS.iter().enumerate() {
            let zone = zone_offset as u8;
            restore_effects
                .entry((entry.device_id.clone(), zone))
                .or_insert_with(|| base_effect(client, &entry.device_id, zone).unwrap_or_default());
        }

        let max_zone_leds = SLINF_ZONE_SEGMENTS
            .iter()
            .map(|(_, zone_len)| *zone_len)
            .max()
            .unwrap_or(0);

        for led_offset in 0..max_zone_leds {
            for (zone_offset, (zone_start, zone_len)) in SLINF_ZONE_SEGMENTS.iter().enumerate() {
                if led_offset >= *zone_len {
                    continue;
                }

                let zone = zone_offset as u8;
                let raw_led_index = zone_start + led_offset;
                steps += 1;
                println!(
                    "step={steps} device={} fan={} zone={} zone_led={} raw_led={} color=#{:02x}{:02x}{:02x}",
                    entry.device_id,
                    entry.fan_index,
                    zone + 1,
                    led_offset,
                    raw_led_index,
                    color[0],
                    color[1],
                    color[2]
                );

                if !dry_run {
                    let response = client.send(&IpcRequest::ProbeRgbLed {
                        device_id: entry.device_id.clone(),
                        fan_index: entry.fan_index,
                        led_index: raw_led_index,
                        color,
                    })?;

                    if let IpcResponse::Error { message } = response {
                        eprintln!(
                            "probe failed on device={} fan={} zone={} zone_led={} raw_led={}: {message}",
                            entry.device_id,
                            entry.fan_index,
                            zone + 1,
                            led_offset,
                            raw_led_index
                        );
                    }

                    thread::sleep(sleep_duration);
                }
            }
        }
    }

    if !dry_run {
        for ((device_id, zone), effect) in restore_effects {
            let response = client.send(&IpcRequest::SetRgbEffect {
                device_id,
                zone,
                effect,
            })?;

            if let IpcResponse::Error { message } = response {
                eprintln!("restore failed: {message}");
            }
        }
    }

    println!("completed steps={steps} dry_run={dry_run}");
    Ok(())
}

fn send_rgb_effect(client: &DaemonClient, device_id: String, zone: u8, effect: RgbEffect) -> Result<()> {
    let response = client.send(&IpcRequest::SetRgbEffect {
        device_id,
        zone,
        effect,
    })?;
    match response {
        IpcResponse::Ok { .. } => {
            println!("ok");
            Ok(())
        }
        IpcResponse::Error { message } => bail!("daemon error: {message}"),
    }
}

fn unwrap_response<T: DeserializeOwned>(response: IpcResponse) -> Result<T> {
    match response {
        IpcResponse::Ok { data } => {
            serde_json::from_value(data).with_context(|| "response parse error")
        }
        IpcResponse::Error { message } => bail!("daemon error: {message}"),
    }
}

fn parse_color(hex: Option<String>, rgb: Option<Vec<u8>>) -> Result<[u8; 3]> {
    if let Some(hex) = hex {
        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 {
            bail!("hex color must be 6 digits (RRGGBB)");
        }
        let r = u8::from_str_radix(&hex[0..2], 16)?;
        let g = u8::from_str_radix(&hex[2..4], 16)?;
        let b = u8::from_str_radix(&hex[4..6], 16)?;
        return Ok([r, g, b]);
    }

    if let Some(rgb) = rgb {
        if rgb.len() != 3 {
            bail!("rgb requires 3 values");
        }
        return Ok([rgb[0], rgb[1], rgb[2]]);
    }

    bail!("color required: use --hex or --rgb")
}

fn parse_rgb_mode(input: &str) -> Result<RgbMode> {
    let norm = input
        .to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-' && *c != '_')
        .collect::<String>();

    let mode = match norm.as_str() {
        "off" => RgbMode::Off,
        "direct" => RgbMode::Direct,
        "static" => RgbMode::Static,
        "rainbow" => RgbMode::Rainbow,
        "rainbowmorph" => RgbMode::RainbowMorph,
        "breathing" => RgbMode::Breathing,
        "runway" => RgbMode::Runway,
        "meteor" => RgbMode::Meteor,
        "colorcycle" => RgbMode::ColorCycle,
        "staggered" => RgbMode::Staggered,
        "tide" => RgbMode::Tide,
        "mixing" => RgbMode::Mixing,
        "voice" => RgbMode::Voice,
        "door" => RgbMode::Door,
        "render" => RgbMode::Render,
        "ripple" => RgbMode::Ripple,
        "reflect" => RgbMode::Reflect,
        "tailchasing" => RgbMode::TailChasing,
        "paint" => RgbMode::Paint,
        "pingpong" => RgbMode::PingPong,
        "stack" => RgbMode::Stack,
        "covercycle" => RgbMode::CoverCycle,
        "wave" => RgbMode::Wave,
        "racing" => RgbMode::Racing,
        "lottery" => RgbMode::Lottery,
        "intertwine" => RgbMode::Intertwine,
        "meteorshower" => RgbMode::MeteorShower,
        "collide" => RgbMode::Collide,
        "electriccurrent" => RgbMode::ElectricCurrent,
        "kaleidoscope" => RgbMode::Kaleidoscope,
        "bigbang" => RgbMode::BigBang,
        "vortex" => RgbMode::Vortex,
        "pump" => RgbMode::Pump,
        "colorsmorph" => RgbMode::ColorsMorph,
        _ => bail!("unknown effect: {input}"),
    };

    Ok(mode)
}

fn brightness_from_percent(percent: u8) -> u8 {
    let scaled = ((percent as f32 / 100.0) * 4.0).round() as i32;
    scaled.clamp(0, 4) as u8
}

fn base_effect(client: &DaemonClient, device_id: &str, zone: u8) -> Result<RgbEffect> {
    let response = client.send(&IpcRequest::GetConfig)?;
    let cfg: AppConfig = unwrap_response(response)?;
    if let Some(rgb) = cfg.rgb {
        if let Some(dev) = rgb.devices.into_iter().find(|d| d.device_id == device_id) {
            if let Some(z) = dev.zones.into_iter().find(|z| z.zone_index == zone) {
                return Ok(z.effect);
            }
        }
    }
    Ok(RgbEffect {
        mode: RgbMode::Static,
        colors: vec![[255, 255, 255]],
        speed: 2,
        brightness: 4,
        direction: RgbDirection::Clockwise,
        scope: RgbScope::All,
        smoothness_ms: 0,
    })
}

fn load_fan_config(client: &DaemonClient) -> Result<FanConfig> {
    let response = client.send(&IpcRequest::GetConfig)?;
    let cfg: AppConfig = unwrap_response(response)?;
    if let Some(fans) = cfg.fans {
        Ok(fans)
    } else {
        Ok(default_fan_config())
    }
}

fn default_fan_config() -> FanConfig {
    FanConfig {
        speeds: Vec::new(),
        update_interval_ms: 1000,
    }
}

fn default_fan_speeds(pwm: u8) -> [FanSpeed; 4] {
    [
        FanSpeed::Constant(pwm),
        FanSpeed::Constant(pwm),
        FanSpeed::Constant(pwm),
        FanSpeed::Constant(pwm),
    ]
}
