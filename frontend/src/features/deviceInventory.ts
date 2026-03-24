import type { DeviceView, ProfileDocument } from "../types/api";

export type DeviceGroupingMode = "controller" | "family";
export type DeviceCapabilityFilter = "all" | "rgb" | "fan" | "lcd" | "pump" | "warning";

export type DeviceGroup = {
  id: string;
  label: string;
  description: string;
  devices: DeviceView[];
};

export type ControllerSummary = {
  id: string;
  label: string;
  kind: string;
  deviceCount: number;
  wirelessGroupCount: number;
  channelCount: number;
  warningCount: number;
  stateSummary: string;
};

export function filterDevices(
  devices: DeviceView[],
  searchTerm: string,
  typeFilter: string,
  capabilityFilter: DeviceCapabilityFilter,
) {
  const normalizedSearch = searchTerm.trim().toLowerCase();

  return devices.filter((device) => {
    const matchesSearch =
      normalizedSearch.length === 0 ||
      [
        device.display_name,
        device.name,
        device.id,
        device.family,
        device.controller.label,
        device.physical_role,
      ]
        .join(" ")
        .toLowerCase()
        .includes(normalizedSearch);

    const matchesType = typeFilter === "all" || device.family === typeFilter;
    const matchesCapability =
      capabilityFilter === "all" ||
      (capabilityFilter === "rgb" && device.capabilities.has_rgb) ||
      (capabilityFilter === "fan" && device.capabilities.has_fan) ||
      (capabilityFilter === "lcd" && device.capabilities.has_lcd) ||
      (capabilityFilter === "pump" && device.capabilities.has_pump) ||
      (capabilityFilter === "warning" && device.health.level !== "healthy");

    return matchesSearch && matchesType && matchesCapability;
  });
}

export function groupDevices(devices: DeviceView[], mode: DeviceGroupingMode): DeviceGroup[] {
  const groups = new Map<string, DeviceGroup>();

  for (const device of devices) {
    const key = mode === "controller" ? device.controller.id : device.family;
    const existing = groups.get(key);

    if (existing) {
      existing.devices.push(device);
      continue;
    }

    groups.set(key, {
      id: key,
      label: mode === "controller" ? device.controller.label : device.family,
      description:
        mode === "controller"
          ? `${device.controller.kind.replace(/_/g, " ")} topology`
          : `${device.family} family inventory`,
      devices: [device],
    });
  }

  return [...groups.values()]
    .map((group) => ({
      ...group,
      devices: [...group.devices].sort((left, right) => {
        return (
          left.ui_order - right.ui_order ||
          left.display_name.localeCompare(right.display_name) ||
          left.id.localeCompare(right.id)
        );
      }),
    }))
    .sort((left, right) => left.label.localeCompare(right.label));
}

export function buildControllerSummaries(devices: DeviceView[]): ControllerSummary[] {
  const groups = new Map<string, DeviceView[]>();

  for (const device of devices) {
    const existing = groups.get(device.controller.id);
    if (existing) {
      existing.push(device);
    } else {
      groups.set(device.controller.id, [device]);
    }
  }

  return [...groups.entries()]
    .map(([controllerId, controllerDevices]) => {
      const first = controllerDevices[0]!;
      const distinctChannels = new Set(
        controllerDevices
          .map((device) => device.wireless?.channel)
          .filter((channel): channel is number => typeof channel === "number"),
      );
      const wirelessGroups = controllerDevices.filter((device) => device.wireless !== null);
      const warningCount = controllerDevices.filter(
        (device) => device.health.level !== "healthy",
      ).length;

      const stateSummary =
        warningCount > 0
          ? `${warningCount} device(s) need attention`
          : distinctChannels.size > 0
            ? `${distinctChannels.size} active channel(s)`
            : "All associated devices healthy";

      return {
        id: controllerId,
        label: first.controller.label,
        kind: first.controller.kind,
        deviceCount: controllerDevices.length,
        wirelessGroupCount: wirelessGroups.length,
        channelCount: distinctChannels.size,
        warningCount,
        stateSummary,
      };
    })
    .sort((left, right) => left.label.localeCompare(right.label));
}

export function profilesForDevice(profiles: ProfileDocument[], deviceId: string) {
  return profiles.filter(
    (profile) =>
      profile.targets.mode === "all" || profile.targets.device_ids.includes(deviceId),
  );
}
