import { Link } from "react-router-dom";
import { StatusBadge } from "./StatusBadge";
import type { DeviceView } from "../types/api";

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
  const items: string[] = [];

  if (device.capabilities.has_rgb) {
    items.push(
      device.capabilities.rgb_zone_count
        ? `${device.capabilities.rgb_zone_count} RGB zone${
            device.capabilities.rgb_zone_count === 1 ? "" : "s"
          }`
        : "RGB",
    );
  }

  if (device.capabilities.has_fan) {
    items.push(
      device.capabilities.fan_count
        ? `${device.capabilities.fan_count} fan slot${
            device.capabilities.fan_count === 1 ? "" : "s"
          }`
        : "Fan control",
    );
  }

  if (device.capabilities.has_lcd) {
    items.push("LCD");
  }

  if (device.capabilities.has_pump) {
    items.push("Pump");
  }

  if (device.capabilities.mb_sync_support) {
    items.push("MB sync");
  }

  return items.length > 0 ? items : ["No exposed capabilities"];
}

function telemetryItems(device: DeviceView) {
  const items: string[] = [];

  if (device.state.fan_rpms && device.state.fan_rpms.length > 0) {
    items.push(`${device.state.fan_rpms.join(" / ")} RPM`);
  }

  if (typeof device.state.coolant_temp === "number") {
    items.push(`${device.state.coolant_temp.toFixed(1)} C coolant`);
  }

  if (typeof device.state.streaming_active === "boolean") {
    items.push(device.state.streaming_active ? "Streaming active" : "Streaming idle");
  }

  return items.length > 0 ? items : ["No live telemetry reported"];
}

export function DeviceCard({ device, actions }: DeviceCardProps) {
  const capabilities = capabilityItems(device);
  const telemetry = telemetryItems(device);

  return (
    <article className="device-card">
      <div className="device-card__header">
        <div>
          <p className="device-card__eyebrow">{device.family}</p>
          <h2>{device.name}</h2>
        </div>
        <StatusBadge tone={device.online ? "online" : "offline"}>
          {device.online ? "online" : "offline"}
        </StatusBadge>
      </div>

      <p className="device-card__id">{device.id}</p>

      <div className="device-card__section">
        <p className="device-card__label">Capabilities</p>
        <div className="chip-row">
          {capabilities.map((item) => (
            <span key={item} className="capability-chip">
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
            key={`${device.id}-${action.label}`}
            className={
              action.tone === "primary" ? "button-link button-link--primary" : "button-link"
            }
            to={action.to}
          >
            {action.label}
          </Link>
        ))}
      </div>
    </article>
  );
}
