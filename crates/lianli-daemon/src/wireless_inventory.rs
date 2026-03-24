use lianli_devices::wireless::{DiscoveredDevice, WirelessController, WirelessFanType};
use lianli_shared::device_id::DeviceFamily;
use lianli_shared::ipc::{DeviceInfo, WirelessBindingState};
use std::collections::HashMap;

pub fn build_wireless_inventory(
    wireless: &WirelessController,
) -> (Vec<DeviceInfo>, HashMap<String, Vec<u16>>) {
    let master_mac = wireless.master_mac();
    let mut devices = Vec::new();
    let mut telemetry = HashMap::new();

    for dev in wireless.devices() {
        let device_id = format!("wireless:{}", dev.mac_str());
        let locally_detached = wireless.is_locally_detached(&dev.mac);
        let binding_state = wireless_binding_state(&master_mac, &dev, locally_detached);
        if dev.missed_polls == 0 {
            let rpms: Vec<u16> = dev.fan_rpms[..dev.fan_count as usize].to_vec();
            telemetry.insert(device_id.clone(), rpms);
        }
        devices.push(DeviceInfo {
            device_id,
            family: wireless_family(dev.fan_type),
            name: dev.fan_type.display_name().to_string(),
            serial: Some(dev.mac_str()),
            wireless_channel: Some(dev.channel),
            wireless_missed_polls: Some(dev.missed_polls),
            wireless_master_mac: (!locally_detached && !is_empty_mac(&dev.master_mac))
                .then(|| format_mac(&dev.master_mac)),
            wireless_binding_state: Some(binding_state),
            has_lcd: false,
            has_fan: dev.fan_count > 0,
            has_pump: false,
            has_rgb: true,
            fan_count: Some(dev.fan_count),
            per_fan_control: Some(true),
            mb_sync_support: dev.fan_type.supports_hw_mobo_sync() || wireless.motherboard_pwm().is_some(),
            rgb_zone_count: Some(match dev.fan_type {
                // SL-INF uses 5 fixed logical zones per fan (9+9+9+9+8 LEDs).
                // Must match SLINF_ZONE_SEGMENTS count in rgb_controller.
                WirelessFanType::SlInf => 5,
                _ => dev.fan_count,
            }),
            screen_width: None,
            screen_height: None,
        });
    }

    (devices, telemetry)
}

fn wireless_family(fan_type: WirelessFanType) -> DeviceFamily {
    match fan_type {
        WirelessFanType::Slv3Led => DeviceFamily::Slv3Led,
        WirelessFanType::Slv3Lcd => DeviceFamily::Slv3Lcd,
        WirelessFanType::Tlv2Lcd => DeviceFamily::Tlv2Lcd,
        WirelessFanType::Tlv2Led => DeviceFamily::Tlv2Led,
        WirelessFanType::SlInf => DeviceFamily::SlInf,
        WirelessFanType::Clv1 => DeviceFamily::Clv1,
        WirelessFanType::Unknown => DeviceFamily::Slv3Led,
    }
}

fn wireless_binding_state(
    master_mac: &[u8; 6],
    device: &DiscoveredDevice,
    locally_detached: bool,
) -> WirelessBindingState {
    if locally_detached || is_empty_mac(&device.master_mac) {
        WirelessBindingState::Available
    } else if &device.master_mac == master_mac {
        WirelessBindingState::Connected
    } else {
        WirelessBindingState::Foreign
    }
}

fn format_mac(mac: &[u8; 6]) -> String {
    format!(
        "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5],
    )
}

fn is_empty_mac(mac: &[u8; 6]) -> bool {
    mac.iter().all(|&byte| byte == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_device(master_mac: [u8; 6]) -> DiscoveredDevice {
        DiscoveredDevice {
            mac: [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01],
            master_mac,
            channel: 8,
            rx_type: 1,
            device_type: 0,
            fan_count: 1,
            fan_types: [36, 0, 0, 0],
            fan_rpms: [0, 0, 0, 0],
            current_pwm: [0, 0, 0, 0],
            cmd_seq: 0,
            fan_type: WirelessFanType::SlInf,
            list_index: 0,
            missed_polls: 0,
        }
    }

    #[test]
    fn locally_detached_device_is_reported_as_available() {
        let master = [1, 2, 3, 4, 5, 6];
        let device = test_device(master);

        assert_eq!(
            wireless_binding_state(&master, &device, true),
            WirelessBindingState::Available
        );
    }
}
