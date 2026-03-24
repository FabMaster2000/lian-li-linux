import { Link } from "react-router-dom";
import { Card } from "../ui/Card";
import { StatusBadge } from "../ui/StatusBadge";
import type { DeviceView } from "../../types/api";
import { getInventoryDeviceStatus } from "../../features/wirelessSync";

export type DeviceCardAction = {
  label: string;
  to: string;
  tone?: "primary" | "secondary";
};

type DeviceCardProps = {
  device: DeviceView;
  actions: DeviceCardAction[];
};

function capabilityItems(device: DeviceView) {
  return device.capability_summary.split(" | ");
}

function telemetryItems(device: DeviceView) {
  const items: string[] = [device.current_mode_summary];

  if (device.state.fan_rpms && device.state.fan_rpms.length > 0) {
    items.push(`${device.state.fan_rpms.join(" / ")} RPM`);
  }

  if (typeof device.state.coolant_temp === "number") {
    items.push(`${device.state.coolant_temp.toFixed(1)} C coolant`);
  }

  items.push(device.health.summary);

  return items;
}

export function DeviceCard({ device, actions }: DeviceCardProps) {
  const capabilities = capabilityItems(device);
  const telemetry = telemetryItems(device);
  const status = getInventoryDeviceStatus(device);

  return (
    <Card className="device-card">
      <div className="device-card__header">
        <div>
          <p className="device-card__eyebrow">{device.family}</p>
          <h2>{device.display_name}</h2>
          <p className="inventory-device-card__subline">{device.controller.label}</p>
        </div>
        <StatusBadge tone={status.tone}>{status.label}</StatusBadge>
      </div>

      <p className="device-card__id">{device.id}</p>

      <div className="chip-row">
        <span className="capability-chip">{device.physical_role}</span>
        {device.wireless?.channel !== null ? (
          <span className="capability-chip">channel {device.wireless?.channel}</span>
        ) : null}
      </div>

      <div className="device-card__section">
        <p className="device-card__label">Capabilities</p>
        <div className="chip-row">
          {capabilities.map((item) => (
            <span className="capability-chip" key={item}>
              {item}
            </span>
          ))}
        </div>
      </div>

      <div className="device-card__section">
        <p className="device-card__label">Live state</p>
        <ul className="telemetry-list">
          {telemetry.map((item) => (
            <li key={item}>{item}</li>
          ))}
        </ul>
      </div>

      <div className="device-card__actions">
        {actions.map((action) => (
          <Link
            className={
              action.tone === "primary" ? "button-link button-link--primary" : "button-link"
            }
            key={`${device.id}-${action.label}`}
            to={action.to}
          >
            {action.label}
          </Link>
        ))}
      </div>
    </Card>
  );
}
