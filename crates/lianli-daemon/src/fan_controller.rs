use crate::temperature_sources;
use anyhow::Result;
use lianli_devices::traits::FanDevice;
use lianli_devices::wireless::{DiscoveredDevice, WirelessController, WirelessFanType};
use lianli_shared::fan::{FanConfig, FanCurve, FanSpeed};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

const SL_INF_SINGLE_FAN_STABLE_MIN_DUTY_PERCENT: u8 = 30;

pub struct FanController {
    config: FanConfig,
    curves: HashMap<String, FanCurve>,
    wireless: Option<Arc<WirelessController>>,
    wired_devices: Arc<HashMap<String, Box<dyn FanDevice>>>,
    stop_flag: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl FanController {
    pub fn new(
        config: FanConfig,
        curves: Vec<FanCurve>,
        wireless: Option<Arc<WirelessController>>,
        wired_devices: Arc<HashMap<String, Box<dyn FanDevice>>>,
    ) -> Self {
        let curves_map: HashMap<String, FanCurve> =
            curves.into_iter().map(|c| (c.name.clone(), c)).collect();

        Self {
            config,
            curves: curves_map,
            wireless,
            wired_devices,
            stop_flag: Arc::new(AtomicBool::new(false)),
            thread: None,
        }
    }

    pub fn start(&mut self) {
        let config = self.config.clone();
        let curves = self.curves.clone();
        let wireless = self.wireless.clone();
        let wired = Arc::clone(&self.wired_devices);
        let stop_flag = Arc::clone(&self.stop_flag);

        let thread = thread::spawn(move || {
            fan_control_thread(config, curves, wireless, wired, stop_flag);
        });

        self.thread = Some(thread);
    }

    pub fn stop(self) {
        self.stop_flag.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread {
            let _ = thread.join();
        }
    }
}

fn fan_control_thread(
    config: FanConfig,
    curves: HashMap<String, FanCurve>,
    wireless: Option<Arc<WirelessController>>,
    wired: Arc<HashMap<String, Box<dyn FanDevice>>>,
    stop_flag: Arc<AtomicBool>,
) {
    let update_interval = Duration::from_millis(config.update_interval_ms);
    let mut last_update = Instant::now() - update_interval;
    let mut last_applied: HashMap<String, [u8; 4]> = HashMap::new();

    // Wait briefly for wireless discovery if we have wireless
    if let Some(ref w) = wireless {
        info!("Fan control thread started, waiting for wireless discovery...");
        let discovery_start = Instant::now();
        while !stop_flag.load(Ordering::Relaxed)
            && discovery_start.elapsed() < Duration::from_secs(10)
        {
            if w.has_discovered_devices() {
                let devices = w.devices();
                info!(
                    "Wireless discovery complete: {} device(s)",
                    devices.len()
                );
                for dev in &devices {
                    info!(
                        "  {} — {:?}, {} fan(s)",
                        dev, dev.fan_type, dev.fan_count
                    );
                }
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
    }

    if !wired.is_empty() {
        let wired_names: Vec<&str> = wired.keys().map(|s| s.as_str()).collect();
        info!("Wired fan devices: {}", wired_names.join(", "));
    }

    if wireless.as_ref().map_or(true, |w| !w.has_discovered_devices()) && wired.is_empty() {
        warn!("No fan devices available — fan control disabled");
        return;
    }

    info!("Starting fan speed control loop ({} group(s))", config.speeds.len());

    // Initialize MB RPM sync state for all wired groups at startup.
    // Groups with MbSync speeds get sync enabled; others get it disabled.
    for (group_idx, group) in config.speeds.iter().enumerate() {
        let is_mb_sync = group.speeds.iter().any(|s| s.is_mb_sync());
        if let Some(ref device_id) = group.device_id {
            if let Some((base_id, port_str)) = device_id.rsplit_once(":port") {
                if let (Some(dev), Ok(port)) = (wired.get(base_id), port_str.parse::<u8>()) {
                    if dev.supports_mb_sync() {
                        if let Err(err) = dev.set_mb_rpm_sync(port, is_mb_sync) {
                            warn!("Failed to set MB sync for {device_id}: {err}");
                        } else if is_mb_sync {
                            info!("MB RPM sync enabled for {device_id}");
                        }
                    }
                }
            } else if let Some(dev) = wired.get(device_id) {
                if dev.supports_mb_sync() {
                    // For non-port devices, use port 0
                    if let Err(err) = dev.set_mb_rpm_sync(0, is_mb_sync) {
                        warn!("Failed to set MB sync for {device_id}: {err}");
                    } else if is_mb_sync {
                        info!("MB RPM sync enabled for {device_id}");
                    }
                }
            }
        }
        if is_mb_sync {
            debug!("Group {group_idx} ({}): MB RPM sync mode", group.device_id.as_deref().unwrap_or("none"));
        }
    }

    while !stop_flag.load(Ordering::Relaxed) {
        let now = Instant::now();
        if now.duration_since(last_update) < update_interval {
            thread::sleep(Duration::from_millis(100));
            continue;
        }
        last_update = now;

        for (group_idx, group) in config.speeds.iter().enumerate() {
            // MB RPM sync mode: wired hardware handles it natively, but wireless
            // devices need software relay of the motherboard PWM signal.
            if group.speeds.iter().any(|s| s.is_mb_sync()) {
                if let Some(ref device_id) = group.device_id {
                    if device_id.starts_with("wireless:") {
                        if let Some(ref w) = wireless {
                            if let Some(dev) = find_wireless_device(&wireless, device_id) {
                                if dev.fan_type.supports_hw_mobo_sync() {
                                    // SLV3: firmware reads its local PWM header
                                    let speeds = [6, 6, 6, 6];
                                    let effective_speeds =
                                        effective_wireless_speeds_for_id(&wireless, device_id, &speeds);
                                    if should_apply_wireless_speeds(
                                        &last_applied,
                                        &wireless,
                                        device_id,
                                        &effective_speeds,
                                    ) && apply_wireless_by_id(
                                        &wireless,
                                        device_id,
                                        &effective_speeds,
                                        group_idx,
                                    ) {
                                        remember_applied(
                                            &mut last_applied,
                                            device_id,
                                            &effective_speeds,
                                        );
                                    }
                                } else if let Some(pwm) = w.motherboard_pwm() {
                                    // RX dongle reports valid mobo PWM — relay it
                                    let speeds = [pwm, pwm, pwm, pwm];
                                    let effective_speeds =
                                        effective_wireless_speeds_for_id(&wireless, device_id, &speeds);
                                    if should_apply_wireless_speeds(
                                        &last_applied,
                                        &wireless,
                                        device_id,
                                        &effective_speeds,
                                    ) && apply_wireless_by_id(
                                        &wireless,
                                        device_id,
                                        &effective_speeds,
                                        group_idx,
                                    ) {
                                        remember_applied(
                                            &mut last_applied,
                                            device_id,
                                            &effective_speeds,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                continue;
            }

            let speeds = match calculate_fan_speeds(&group.speeds, &curves) {
                Ok(speeds) => speeds,
                Err(err) => {
                    warn!("Fan speed calculation failed for group {group_idx}: {err}");
                    continue;
                }
            };

            // Try to apply to the right device
            if let Some(ref device_id) = group.device_id {
                if device_id.starts_with("wireless:") {
                    let effective_speeds =
                        effective_wireless_speeds_for_id(&wireless, device_id, &speeds);
                    if should_apply_wireless_speeds(
                        &last_applied,
                        &wireless,
                        device_id,
                        &effective_speeds,
                    ) && apply_wireless_by_id(&wireless, device_id, &effective_speeds, group_idx)
                    {
                        remember_applied(
                            &mut last_applied,
                            device_id,
                            &effective_speeds,
                        );
                    }
                } else if let Some((base_id, port_str)) = device_id.rsplit_once(":port") {
                    // Per-port wired device (e.g. "Nuvoton:port0")
                    if let (Some(dev), Ok(port)) = (wired.get(base_id), port_str.parse::<u8>()) {
                        if !should_apply_speeds(&last_applied, device_id, &speeds) {
                            thread::sleep(Duration::from_millis(5));
                            continue;
                        }
                        if let Err(err) = dev.set_fan_speed(port, speeds[0]) {
                            warn!("Failed to set fan speed for {device_id}: {err}");
                        } else {
                            remember_applied(&mut last_applied, device_id, &speeds);
                        }
                    } else {
                        warn!("Fan group {group_idx}: device '{device_id}' not found");
                    }
                } else if let Some(dev) = wired.get(device_id) {
                    if !should_apply_speeds(&last_applied, device_id, &speeds) {
                        thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                    if let Err(err) = dev.set_fan_speeds(&speeds) {
                        warn!("Failed to set fan speeds for {device_id}: {err}");
                    } else {
                        remember_applied(&mut last_applied, device_id, &speeds);
                    }
                } else {
                    warn!("Fan group {group_idx}: device '{device_id}' not found");
                }
            } else {
                // Legacy: match by group index to wireless devices
                if let Some(ref w) = wireless {
                    let legacy_key = format!("wireless-group:{group_idx}");
                    if !should_apply_speeds(&last_applied, &legacy_key, &speeds) {
                        thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                    if let Err(err) = w.set_fan_speeds(group_idx as u8, &speeds) {
                        warn!("Failed to set fan speeds for wireless device {group_idx}: {err}");
                    } else {
                        remember_applied(&mut last_applied, &legacy_key, &speeds);
                    }
                }
            }

            thread::sleep(Duration::from_millis(5));
        }

        thread::sleep(Duration::from_millis(100));
    }

    info!("Fan control thread stopped");
}

fn apply_wireless_by_id(
    wireless: &Option<Arc<WirelessController>>,
    device_id: &str,
    speeds: &[u8; 4],
    group_idx: usize,
) -> bool {
    let Some(w) = wireless else {
        warn!("Fan group {group_idx}: wireless not available for device {device_id}");
        return false;
    };
    if let Some(dev) = find_wireless_device(wireless, device_id) {
        if let Err(err) = w.set_fan_speeds(dev.list_index, speeds) {
            warn!("Failed to set fan speeds for {device_id}: {err}");
            false
        } else {
            true
        }
    } else {
        debug!("Fan group {group_idx}: wireless device {device_id} is not connected");
        false
    }
}

fn should_apply_speeds(
    last_applied: &HashMap<String, [u8; 4]>,
    target_id: &str,
    speeds: &[u8; 4],
) -> bool {
    last_applied.get(target_id) != Some(speeds)
}

fn remember_applied(
    last_applied: &mut HashMap<String, [u8; 4]>,
    target_id: &str,
    speeds: &[u8; 4],
) {
    last_applied.insert(target_id.to_string(), *speeds);
}

fn find_wireless_device(
    wireless: &Option<Arc<WirelessController>>,
    device_id: &str,
) -> Option<DiscoveredDevice> {
    let w = wireless.as_ref()?;
    let master_mac = w.master_mac();
    let mac_str = device_id.strip_prefix("wireless:").unwrap_or(device_id);
    w.devices()
        .into_iter()
        .find(|device| {
            device.mac_str() == mac_str
                && !w.is_locally_detached(&device.mac)
                && !device.master_mac.iter().all(|&byte| byte == 0)
                && device.master_mac == master_mac
        })
}

fn effective_wireless_speeds_for_id(
    wireless: &Option<Arc<WirelessController>>,
    device_id: &str,
    speeds: &[u8; 4],
) -> [u8; 4] {
    find_wireless_device(wireless, device_id)
        .map(|device| normalize_wireless_speeds(&device, speeds))
        .unwrap_or(*speeds)
}

fn should_apply_wireless_speeds(
    last_applied: &HashMap<String, [u8; 4]>,
    wireless: &Option<Arc<WirelessController>>,
    device_id: &str,
    speeds: &[u8; 4],
) -> bool {
    let Some(device) = find_wireless_device(wireless, device_id) else {
        return false;
    };

    if last_applied.get(device_id) != Some(speeds) {
        return true;
    }

    device.current_pwm != *speeds
}

fn normalize_wireless_speeds(device: &DiscoveredDevice, speeds: &[u8; 4]) -> [u8; 4] {
    let mut normalized = *speeds;
    let min_pwm = ((stable_min_duty_percent(device) as f32 / 100.0) * 255.0) as u8;

    for (index, value) in normalized.iter_mut().enumerate() {
        if index as u8 >= device.fan_count {
            *value = 0;
            continue;
        }

        if *value > 0 && *value < min_pwm {
            *value = min_pwm;
        }

        if device.fan_type == WirelessFanType::Clv1 {
            match *value {
                153 | 154 => *value = 152,
                155 => *value = 156,
                _ => {}
            }
        }
    }

    normalized
}

fn stable_min_duty_percent(device: &DiscoveredDevice) -> u8 {
    match (device.fan_type, device.fan_count) {
        (WirelessFanType::SlInf, 1) => {
            SL_INF_SINGLE_FAN_STABLE_MIN_DUTY_PERCENT.max(device.fan_type.min_duty_percent())
        }
        _ => device.fan_type.min_duty_percent(),
    }
}

fn calculate_fan_speeds(
    fan_speeds: &[FanSpeed; 4],
    curves: &HashMap<String, FanCurve>,
) -> Result<[u8; 4]> {
    let mut pwm_values = [0u8; 4];

    for (i, fan_speed) in fan_speeds.iter().enumerate() {
        pwm_values[i] = match fan_speed {
            FanSpeed::Constant(value) => *value,
            FanSpeed::Curve(curve_name) => {
                let curve = curves
                    .get(curve_name)
                    .ok_or_else(|| anyhow::anyhow!("Curve '{curve_name}' not found"))?;

                let temp = temperature_sources::read_temperature(&curve.temp_command)?;
                let speed_percent = interpolate_curve(&curve.curve, temp);
                let pwm = (speed_percent * 2.55) as u8;

                debug!("Fan {i}: Temp {temp:.1}C, Speed {speed_percent:.0}%, PWM {pwm}");
                pwm
            }
        };
    }

    Ok(pwm_values)
}

fn interpolate_curve(curve: &[(f32, f32)], temp: f32) -> f32 {
    if curve.is_empty() {
        return 50.0;
    }

    if curve.len() == 1 {
        return curve[0].1;
    }

    let mut sorted_curve = curve.to_vec();
    sorted_curve.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    if temp <= sorted_curve[0].0 {
        return sorted_curve[0].1;
    }

    if temp >= sorted_curve[sorted_curve.len() - 1].0 {
        return sorted_curve[sorted_curve.len() - 1].1;
    }

    for i in 0..sorted_curve.len() - 1 {
        let (temp1, speed1) = sorted_curve[i];
        let (temp2, speed2) = sorted_curve[i + 1];

        if temp >= temp1 && temp <= temp2 {
            let ratio = (temp - temp1) / (temp2 - temp1);
            return speed1 + ratio * (speed2 - speed1);
        }
    }

    50.0
}
