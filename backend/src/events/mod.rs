use crate::daemon::DaemonClient;
use lianli_shared::device_id::DeviceFamily;
use lianli_shared::ipc::{DeviceInfo, IpcRequest, IpcResponse, TelemetrySnapshot};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::thread;
use std::time::Duration;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tokio::sync::broadcast;

const EVENT_CHANNEL_CAPACITY: usize = 256;
const POLL_INTERVAL: Duration = Duration::from_secs(1);
const CONFIG_POLL_EVERY: u32 = 5;

#[derive(Clone, Debug)]
pub struct EventHub {
    tx: broadcast::Sender<WebEvent>,
}

impl EventHub {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(EVENT_CHANNEL_CAPACITY);
        Self { tx }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<WebEvent> {
        self.tx.subscribe()
    }

    pub fn publish(&self, event: WebEvent) {
        let _ = self.tx.send(event);
    }

    pub fn publish_fan_changed(&self, source: &str, device_id: &str, data: Value) {
        self.publish(WebEvent::new(
            "fan.changed",
            source,
            Some(device_id.to_string()),
            data,
        ));
    }

    pub fn publish_lighting_changed(&self, source: &str, device_id: &str, data: Value) {
        self.publish(WebEvent::new(
            "lighting.changed",
            source,
            Some(device_id.to_string()),
            data,
        ));
    }

    pub fn publish_config_changed(&self, source: &str, data: Value) {
        self.publish(WebEvent::new("config.changed", source, None, data));
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WebEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub timestamp: String,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
    #[serde(default)]
    pub data: Value,
}

impl WebEvent {
    pub fn new(event_type: &str, source: &str, device_id: Option<String>, data: Value) -> Self {
        Self {
            event_type: event_type.to_string(),
            timestamp: timestamp_now(),
            source: source.to_string(),
            device_id,
            data,
        }
    }
}

pub fn start_polling(daemon: DaemonClient, hub: EventHub) {
    thread::spawn(move || {
        let mut previous = PollSnapshot::default();
        let mut config_counter = 0;

        loop {
            let mut current = collect_live_snapshot(&daemon, previous.config_fingerprint.clone());
            if should_refresh_config(&previous, &current, config_counter) {
                current.config_fingerprint =
                    fetch_config_fingerprint(&daemon, current.config_fingerprint.clone());
            }

            for event in diff_poll_snapshot(&previous, &current) {
                hub.publish(event);
            }

            previous = current;
            config_counter = (config_counter + 1) % CONFIG_POLL_EVERY;
            thread::sleep(POLL_INTERVAL);
        }
    });
}

#[derive(Debug, Clone, Default)]
struct PollSnapshot {
    reachable: bool,
    error: Option<String>,
    devices: HashMap<String, DeviceSnapshot>,
    config_fingerprint: Option<String>,
    openrgb_status: Option<OpenRgbStatusSnapshot>,
}

#[derive(Debug, Clone, PartialEq)]
struct DeviceSnapshot {
    family: DeviceFamily,
    name: String,
    has_lcd: bool,
    has_fan: bool,
    has_pump: bool,
    has_rgb: bool,
    fan_count: Option<u8>,
    per_fan_control: Option<bool>,
    mb_sync_support: bool,
    rgb_zone_count: Option<u8>,
    fan_rpms: Option<Vec<u16>>,
    coolant_temp: Option<f32>,
}

#[derive(Debug, Clone, PartialEq)]
struct OpenRgbStatusSnapshot {
    enabled: bool,
    running: bool,
    port: Option<u16>,
    error: Option<String>,
}

fn collect_live_snapshot(
    daemon: &DaemonClient,
    previous_config_fingerprint: Option<String>,
) -> PollSnapshot {
    let ping_result = match daemon.send(&IpcRequest::Ping) {
        Ok(IpcResponse::Ok { .. }) => Ok(()),
        Ok(IpcResponse::Error { message }) => Err(message),
        Err(err) => Err(err.to_string()),
    };

    if let Err(error) = ping_result {
        return PollSnapshot {
            reachable: false,
            error: Some(error),
            devices: HashMap::new(),
            config_fingerprint: previous_config_fingerprint,
            openrgb_status: None,
        };
    }

    let devices: Vec<DeviceInfo> = match daemon.send(&IpcRequest::ListDevices) {
        Ok(response) => match unwrap_response::<Vec<DeviceInfo>>(response) {
            Ok(devices) => devices,
            Err(error) => {
                return PollSnapshot {
                    reachable: false,
                    error: Some(error),
                    devices: HashMap::new(),
                    config_fingerprint: previous_config_fingerprint,
                    openrgb_status: None,
                }
            }
        },
        Err(error) => {
            return PollSnapshot {
                reachable: false,
                error: Some(error.to_string()),
                devices: HashMap::new(),
                config_fingerprint: previous_config_fingerprint,
                openrgb_status: None,
            }
        }
    };

    let telemetry: TelemetrySnapshot = match daemon.send(&IpcRequest::GetTelemetry) {
        Ok(response) => match unwrap_response::<TelemetrySnapshot>(response) {
            Ok(telemetry) => telemetry,
            Err(error) => {
                return PollSnapshot {
                    reachable: false,
                    error: Some(error),
                    devices: HashMap::new(),
                    config_fingerprint: previous_config_fingerprint,
                    openrgb_status: None,
                }
            }
        },
        Err(error) => {
            return PollSnapshot {
                reachable: false,
                error: Some(error.to_string()),
                devices: HashMap::new(),
                config_fingerprint: previous_config_fingerprint,
                openrgb_status: None,
            }
        }
    };

    let devices = devices
        .iter()
        .map(|device| {
            (
                device.device_id.clone(),
                DeviceSnapshot::from_parts(device, &telemetry),
            )
        })
        .collect();

    PollSnapshot {
        reachable: true,
        error: None,
        devices,
        config_fingerprint: previous_config_fingerprint,
        openrgb_status: Some(OpenRgbStatusSnapshot {
            enabled: telemetry.openrgb_status.enabled,
            running: telemetry.openrgb_status.running,
            port: telemetry.openrgb_status.port,
            error: telemetry.openrgb_status.error,
        }),
    }
}

fn fetch_config_fingerprint(
    daemon: &DaemonClient,
    previous_config_fingerprint: Option<String>,
) -> Option<String> {
    match daemon.send(&IpcRequest::GetConfig) {
        Ok(response) => unwrap_response::<serde_json::Value>(response)
            .ok()
            .and_then(|cfg| serde_json::to_string(&cfg).ok())
            .or(previous_config_fingerprint),
        Err(_) => previous_config_fingerprint,
    }
}

fn should_refresh_config(previous: &PollSnapshot, current: &PollSnapshot, config_counter: u32) -> bool {
    current.reachable
        && (
            config_counter == 0
                || !previous.reachable
                || device_topology_changed(previous, current)
        )
}

fn device_topology_changed(previous: &PollSnapshot, current: &PollSnapshot) -> bool {
    previous.devices.len() != current.devices.len()
        || previous
            .devices
            .keys()
            .any(|device_id| !current.devices.contains_key(device_id))
}

fn diff_poll_snapshot(previous: &PollSnapshot, current: &PollSnapshot) -> Vec<WebEvent> {
    let mut events = Vec::new();

    if !previous.reachable && current.reachable {
        events.push(WebEvent::new("daemon.connected", "poll", None, json!({})));
    }

    if previous.reachable && !current.reachable {
        events.push(WebEvent::new(
            "daemon.disconnected",
            "poll",
            None,
            json!({ "error": current.error.clone() }),
        ));
    }

    if !current.reachable {
        return events;
    }

    let device_ids = previous
        .devices
        .keys()
        .chain(current.devices.keys())
        .cloned()
        .collect::<HashSet<_>>();

    for device_id in device_ids {
        match (previous.devices.get(&device_id), current.devices.get(&device_id)) {
            (None, Some(current_device)) => {
                events.push(WebEvent::new(
                    "device.updated",
                    "poll",
                    Some(device_id.clone()),
                    json!({
                        "reason": "discovered",
                        "online": true,
                        "name": current_device.name.clone(),
                        "family": format!("{:?}", current_device.family),
                    }),
                ));

                if let Some(rpms) = &current_device.fan_rpms {
                    events.push(WebEvent::new(
                        "fan.changed",
                        "poll",
                        Some(device_id.clone()),
                        json!({
                            "reason": "telemetry",
                            "rpms": rpms,
                        }),
                    ));
                }
            }
            (Some(_), None) => {
                events.push(WebEvent::new(
                    "device.updated",
                    "poll",
                    Some(device_id),
                    json!({
                        "reason": "removed",
                        "online": false,
                    }),
                ));
            }
            (Some(previous_device), Some(current_device)) if previous_device != current_device => {
                if previous_device.changed_without_fans(current_device) {
                    events.push(WebEvent::new(
                        "device.updated",
                        "poll",
                        Some(device_id.clone()),
                        json!({
                            "reason": "changed",
                            "online": true,
                            "coolant_temp": current_device.coolant_temp,
                        }),
                    ));
                }

                if previous_device.fan_rpms != current_device.fan_rpms {
                    events.push(WebEvent::new(
                        "fan.changed",
                        "poll",
                        Some(device_id),
                        json!({
                            "reason": "telemetry",
                            "rpms": current_device.fan_rpms.clone(),
                        }),
                    ));
                }
            }
            _ => {}
        }
    }

    if previous.reachable {
        if previous.openrgb_status != current.openrgb_status {
            events.push(WebEvent::new(
                "openrgb.changed",
                "poll",
                None,
                json!({
                    "enabled": current.openrgb_status.as_ref().map(|status| status.enabled),
                    "running": current.openrgb_status.as_ref().map(|status| status.running),
                    "port": current.openrgb_status.as_ref().and_then(|status| status.port),
                    "error": current.openrgb_status.as_ref().and_then(|status| status.error.clone()),
                }),
            ));
        }

        if let (Some(previous_config), Some(current_config)) =
            (&previous.config_fingerprint, &current.config_fingerprint)
        {
            if previous_config != current_config {
                events.push(WebEvent::new(
                    "config.changed",
                    "poll",
                    None,
                    json!({ "reason": "daemon_config_updated" }),
                ));
            }
        }
    }

    events
}

fn unwrap_response<T: serde::de::DeserializeOwned>(response: IpcResponse) -> Result<T, String> {
    match response {
        IpcResponse::Ok { data } => {
            serde_json::from_value(data).map_err(|err| format!("response parse error: {err}"))
        }
        IpcResponse::Error { message } => Err(message),
    }
}

impl DeviceSnapshot {
    fn from_parts(device: &DeviceInfo, telemetry: &TelemetrySnapshot) -> Self {
        Self {
            family: device.family,
            name: device.name.clone(),
            has_lcd: device.has_lcd,
            has_fan: device.has_fan,
            has_pump: device.has_pump,
            has_rgb: device.has_rgb,
            fan_count: device.fan_count,
            per_fan_control: device.per_fan_control,
            mb_sync_support: device.mb_sync_support,
            rgb_zone_count: device.rgb_zone_count,
            fan_rpms: telemetry.fan_rpms.get(&device.device_id).cloned(),
            coolant_temp: telemetry.coolant_temps.get(&device.device_id).copied(),
        }
    }

    fn changed_without_fans(&self, other: &Self) -> bool {
        self.family != other.family
            || self.name != other.name
            || self.has_lcd != other.has_lcd
            || self.has_fan != other.has_fan
            || self.has_pump != other.has_pump
            || self.has_rgb != other.has_rgb
            || self.fan_count != other.fan_count
            || self.per_fan_control != other.per_fan_control
            || self.mb_sync_support != other.mb_sync_support
            || self.rgb_zone_count != other.rgb_zone_count
            || self.coolant_temp != other.coolant_temp
    }
}

fn timestamp_now() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        device_topology_changed, diff_poll_snapshot, should_refresh_config, EventHub,
        OpenRgbStatusSnapshot, PollSnapshot, WebEvent,
    };
    use lianli_shared::device_id::DeviceFamily;
    use serde_json::json;
    use std::collections::HashMap;

    fn device_snapshot(rpms: Option<Vec<u16>>) -> super::DeviceSnapshot {
        super::DeviceSnapshot {
            family: DeviceFamily::SlInf,
            name: "Sim Device".to_string(),
            has_lcd: false,
            has_fan: true,
            has_pump: false,
            has_rgb: true,
            fan_count: Some(3),
            per_fan_control: Some(true),
            mb_sync_support: false,
            rgb_zone_count: Some(3),
            fan_rpms: rpms,
            coolant_temp: Some(31.5),
        }
    }

    #[test]
    fn diff_emits_connected_device_and_fan_events_on_first_successful_poll() {
        let previous = PollSnapshot::default();
        let current = PollSnapshot {
            reachable: true,
            error: None,
            devices: HashMap::from([(
                "wireless:test".to_string(),
                device_snapshot(Some(vec![900, 910, 920])),
            )]),
            config_fingerprint: Some("cfg-a".to_string()),
            openrgb_status: Some(super::OpenRgbStatusSnapshot {
                enabled: false,
                running: false,
                port: None,
                error: None,
            }),
        };

        let events = diff_poll_snapshot(&previous, &current);
        let types = events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>();

        assert!(types.contains(&"daemon.connected"));
        assert!(types.contains(&"device.updated"));
        assert!(types.contains(&"fan.changed"));
    }

    #[test]
    fn diff_skips_device_updated_for_rpm_only_changes() {
        let previous = PollSnapshot {
            reachable: true,
            error: None,
            devices: HashMap::from([(
                "wireless:test".to_string(),
                device_snapshot(Some(vec![900, 910, 920])),
            )]),
            config_fingerprint: Some("cfg-a".to_string()),
            openrgb_status: Some(OpenRgbStatusSnapshot {
                enabled: false,
                running: false,
                port: None,
                error: None,
            }),
        };
        let current = PollSnapshot {
            reachable: true,
            error: None,
            devices: HashMap::from([(
                "wireless:test".to_string(),
                device_snapshot(Some(vec![930, 940, 950])),
            )]),
            config_fingerprint: Some("cfg-a".to_string()),
            openrgb_status: Some(OpenRgbStatusSnapshot {
                enabled: false,
                running: false,
                port: None,
                error: None,
            }),
        };

        let events = diff_poll_snapshot(&previous, &current);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "fan.changed");
    }

    #[test]
    fn diff_emits_disconnected_when_daemon_becomes_unreachable() {
        let previous = PollSnapshot {
            reachable: true,
            error: None,
            devices: HashMap::new(),
            config_fingerprint: Some("cfg-a".to_string()),
            openrgb_status: None,
        };
        let current = PollSnapshot {
            reachable: false,
            error: Some("connection refused".to_string()),
            devices: HashMap::new(),
            config_fingerprint: Some("cfg-a".to_string()),
            openrgb_status: None,
        };

        let events = diff_poll_snapshot(&previous, &current);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "daemon.disconnected");
        assert_eq!(events[0].data, json!({ "error": "connection refused" }));
    }

    #[test]
    fn diff_emits_config_changed_when_fingerprint_changes() {
        let previous = PollSnapshot {
            reachable: true,
            error: None,
            devices: HashMap::new(),
            config_fingerprint: Some("cfg-a".to_string()),
            openrgb_status: Some(OpenRgbStatusSnapshot {
                enabled: false,
                running: false,
                port: None,
                error: None,
            }),
        };
        let current = PollSnapshot {
            reachable: true,
            error: None,
            devices: HashMap::new(),
            config_fingerprint: Some("cfg-b".to_string()),
            openrgb_status: Some(OpenRgbStatusSnapshot {
                enabled: false,
                running: false,
                port: None,
                error: None,
            }),
        };

        let events = diff_poll_snapshot(&previous, &current);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "config.changed");
    }

    #[test]
    fn refresh_policy_triggers_on_reconnect_and_topology_change() {
        let disconnected = PollSnapshot::default();
        let reconnected = PollSnapshot {
            reachable: true,
            error: None,
            devices: HashMap::new(),
            config_fingerprint: Some("cfg-a".to_string()),
            openrgb_status: Some(OpenRgbStatusSnapshot {
                enabled: false,
                running: false,
                port: None,
                error: None,
            }),
        };
        assert!(should_refresh_config(&disconnected, &reconnected, 3));

        let previous = PollSnapshot {
            reachable: true,
            error: None,
            devices: HashMap::new(),
            config_fingerprint: Some("cfg-a".to_string()),
            openrgb_status: Some(OpenRgbStatusSnapshot {
                enabled: false,
                running: false,
                port: None,
                error: None,
            }),
        };
        let current = PollSnapshot {
            reachable: true,
            error: None,
            devices: HashMap::from([(
                "wireless:test".to_string(),
                device_snapshot(None),
            )]),
            config_fingerprint: Some("cfg-a".to_string()),
            openrgb_status: Some(OpenRgbStatusSnapshot {
                enabled: false,
                running: false,
                port: None,
                error: None,
            }),
        };

        assert!(device_topology_changed(&previous, &current));
        assert!(should_refresh_config(&previous, &current, 4));
        assert!(!should_refresh_config(&current, &current, 4));
    }

    #[test]
    fn event_hub_helper_publishes_web_event_shape() {
        let hub = EventHub::new();
        let mut receiver = hub.subscribe();

        hub.publish_lighting_changed(
            "api",
            "wireless:test",
            json!({ "reason": "color_set", "zone": 0 }),
        );

        let event: WebEvent = receiver.try_recv().expect("receive event");
        assert_eq!(event.event_type, "lighting.changed");
        assert_eq!(event.source, "api");
        assert_eq!(event.device_id.as_deref(), Some("wireless:test"));
        assert_eq!(event.data["zone"], 0);
    }
}
