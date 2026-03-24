import { Link } from "react-router-dom";
import { Card } from "../ui/Card";
import { StatusBadge } from "../ui/StatusBadge";
import type { DeviceView } from "../../types/api";
import {
  formatWirelessClusterMembers,
  formatWirelessIdentityLabel,
  formatWirelessRpmSummary,
  getInventoryDeviceStatus,
} from "../../features/wirelessSync";

type InventoryDeviceCardProps = {
  device: DeviceView;
};

export function InventoryDeviceCard({ device }: InventoryDeviceCardProps) {
  const status = getInventoryDeviceStatus(device);
  const wirelessIdentity =
    device.wireless !== null ? formatWirelessIdentityLabel(device.id) : null;
  const fanSetup = device.capabilities.has_fan
    ? `${device.capabilities.fan_count ?? 0} fan${device.capabilities.fan_count === 1 ? "" : "s"}`
    : "No fan control";
  const rgbZones = device.capabilities.has_rgb
    ? `${device.capabilities.rgb_zone_count ?? 0} zone${device.capabilities.rgb_zone_count === 1 ? "" : "s"}`
    : "No RGB control";

  return (
    <Card className="inventory-device-card">
      <div className="inventory-device-card__header">
        <div>
          <p className="device-card__eyebrow">{device.family}</p>
          <h3>{device.display_name}</h3>
          <p className="inventory-device-card__subline">
            {device.wireless
              ? `${formatWirelessClusterMembers(device.capabilities.fan_count)} cluster | id ${wirelessIdentity}`
              : device.controller.label}
          </p>
        </div>
        <div className="inventory-device-card__badges">
          <StatusBadge tone={status.tone}>{status.label}</StatusBadge>
          <StatusBadge tone="info">order {device.ui_order}</StatusBadge>
        </div>
      </div>

      <div className="chip-row">
        <span className="capability-chip">{device.physical_role}</span>
        {device.wireless?.channel != null ? (
          <span className="capability-chip">channel {device.wireless?.channel}</span>
        ) : null}
        {device.wireless?.group_label ? (
          <span className="capability-chip">group {device.wireless.group_label}</span>
        ) : null}
      </div>

      <dl className="detail-list detail-list--compact inventory-device-card__details">
        <div>
          <dt>Device id</dt>
          <dd>{device.id}</dd>
        </div>
        <div>
          <dt>Controller</dt>
          <dd>{device.controller.label}</dd>
        </div>
        {device.wireless ? (
          <div>
            <dt>Wireless id</dt>
            <dd>{wirelessIdentity}</dd>
          </div>
        ) : null}
        <div>
          <dt>Capability summary</dt>
          <dd>{device.capability_summary}</dd>
        </div>
        <div>
          <dt>Fan setup</dt>
          <dd>{fanSetup}</dd>
        </div>
        <div>
          <dt>RGB zones</dt>
          <dd>{rgbZones}</dd>
        </div>
        <div>
          <dt>Current mode</dt>
          <dd>{device.current_mode_summary}</dd>
        </div>
        <div>
          <dt>Health</dt>
          <dd>{device.health.summary}</dd>
        </div>
        <div>
          <dt>Fan RPM</dt>
          <dd>{formatWirelessRpmSummary(device.state.fan_rpms)}</dd>
        </div>
      </dl>

      <div className="inventory-device-card__links">
        <Link className="button-link button-link--primary" to={`/devices/${encodeURIComponent(device.id)}`}>
          Details
        </Link>
        {device.capabilities.has_rgb ? (
          <Link className="button-link" to={`/lighting?device=${encodeURIComponent(device.id)}`}>
            Lighting
          </Link>
        ) : null}
        {device.capabilities.has_fan ? (
          <Link className="button-link" to={`/fans?device=${encodeURIComponent(device.id)}`}>
            Fans
          </Link>
        ) : null}
      </div>
    </Card>
  );
}

