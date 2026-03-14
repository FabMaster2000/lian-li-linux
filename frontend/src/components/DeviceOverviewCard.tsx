import { DeviceCard } from "./DeviceCard";
import type { DeviceView } from "../types/api";

type DeviceOverviewCardProps = {
  device: DeviceView;
  detailsTo?: string;
  lightingTo?: string | null;
  fansTo?: string | null;
};

export function DeviceOverviewCard({
  device,
  detailsTo = "/devices",
  lightingTo = "/lighting",
  fansTo = "/fans",
}: DeviceOverviewCardProps) {
  const actions = [
    { label: "Details", to: detailsTo, tone: "primary" as const },
    ...(device.capabilities.has_rgb && lightingTo
      ? [{ label: "Lighting", to: lightingTo }]
      : []),
    ...(device.capabilities.has_fan && fansTo ? [{ label: "Fans", to: fansTo }] : []),
  ];

  return <DeviceCard actions={actions} device={device} />;
}
