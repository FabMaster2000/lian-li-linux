import type { DeviceView } from "../types/api";

export type WirelessHealthFilter = "all" | "healthy" | "attention" | "offline";

export type WirelessGroup = {
  id: string;
  label: string;
  channel: number | null;
  controllerId: string;
  controllerLabel: string;
  controllerKind: string;
  devices: DeviceView[];
  deviceCount: number;
  onlineCount: number;
  attentionCount: number;
  familySummary: string;
  capabilitySummary: string;
  statusTone: "online" | "warning" | "offline";
  statusLabel: string;
  stateSummary: string;
};

export type WirelessChannelSummary = {
  id: string;
  label: string;
  groupCount: number;
  deviceCount: number;
  controllerCount: number;
  attentionCount: number;
  offlineCount: number;
};

function compareDevices(left: DeviceView, right: DeviceView) {
  return (
    left.ui_order - right.ui_order ||
    left.display_name.localeCompare(right.display_name) ||
    left.id.localeCompare(right.id)
  );
}

function compareChannels(left: number | null, right: number | null) {
  if (left === right) {
    return 0;
  }

  if (left === null) {
    return 1;
  }

  if (right === null) {
    return -1;
  }

  return left - right;
}

function summarizeFamilies(devices: DeviceView[]) {
  return [...new Set(devices.map((device) => device.family))]
    .sort((left, right) => left.localeCompare(right))
    .join(" | ");
}

function summarizeCapabilities(devices: DeviceView[]) {
  const items = [];

  if (devices.some((device) => device.capabilities.has_rgb)) {
    items.push("RGB");
  }

  if (devices.some((device) => device.capabilities.has_fan)) {
    items.push("Fan");
  }

  if (devices.some((device) => device.capabilities.has_lcd)) {
    items.push("LCD");
  }

  if (devices.some((device) => device.capabilities.has_pump)) {
    items.push("Pump");
  }

  return items.length > 0 ? items.join(" | ") : "Inventory only";
}

export function isWirelessDevice(device: DeviceView) {
  return device.wireless !== null;
}

function resolvedWirelessBindingState(device: DeviceView) {
  if (!isWirelessDevice(device)) {
    return null;
  }

  // Older backends only reported already-connected wireless inventory and did
  // not include an explicit binding state yet. Treat those entries as connected
  // so the UI stays useful during mixed-version rollouts.
  return device.wireless?.binding_state ?? "connected";
}

export function isConnectedWirelessDevice(device: DeviceView) {
  return resolvedWirelessBindingState(device) === "connected";
}

export function isAvailableWirelessDevice(device: DeviceView) {
  return resolvedWirelessBindingState(device) === "available";
}

export function isForeignWirelessDevice(device: DeviceView) {
  return resolvedWirelessBindingState(device) === "foreign";
}

export function isInventoryVisibleDevice(device: DeviceView) {
  return !isWirelessDevice(device) || isConnectedWirelessDevice(device);
}

export function getInventoryDeviceStatus(device: DeviceView) {
  if (!device.online) {
    return {
      label: "offline",
      tone: "offline" as const,
    };
  }

  if (isAvailableWirelessDevice(device)) {
    return {
      label: "unpaired",
      tone: "warning" as const,
    };
  }

  if (isForeignWirelessDevice(device)) {
    return {
      label: "paired elsewhere",
      tone: "warning" as const,
    };
  }

  return {
    label: device.health.level,
    tone: device.health.level === "healthy" ? ("online" as const) : ("warning" as const),
  };
}

export function formatWirelessChannelLabel(channel: number | null) {
  return channel === null ? "Unassigned" : `Channel ${channel}`;
}

export function formatWirelessBindingStateLabel(
  bindingState: DeviceView["wireless"] extends infer T
    ? T extends { binding_state: infer S }
      ? S
      : never
    : never,
) {
  switch (bindingState) {
    case "connected":
      return "Connected";
    case "available":
      return "Ready to connect";
    case "foreign":
      return "Paired elsewhere";
    default:
      return "Connected";
  }
}

export function formatWirelessIdentityLabel(deviceId: string) {
  const tail = deviceId.startsWith("wireless:") ? deviceId.slice("wireless:".length) : deviceId;
  const segments = tail.split(":");

  if (segments.length >= 4) {
    return `${segments[0]}:${segments[1]}..${segments[segments.length - 2]}:${segments[segments.length - 1]}`;
  }

  if (segments.length >= 2) {
    const lastIndex = segments.length - 1;
    return `${segments[lastIndex - 1]}:${segments[lastIndex]}`;
  }

  return tail;
}

export function formatWirelessClusterMembers(fanCount: number | null) {
  if (fanCount === null) {
    return "Unknown cluster size";
  }

  return fanCount === 1 ? "1 fan" : `${fanCount} fans`;
}

export function formatWirelessRpmSummary(fanRpm: number[] | null) {
  return fanRpm && fanRpm.length > 0 ? fanRpm.join(" / ") : "n/a";
}

export function filterWirelessDevices(
  devices: DeviceView[],
  searchTerm: string,
  channelFilter: string,
  healthFilter: WirelessHealthFilter,
) {
  const normalizedSearch = searchTerm.trim().toLowerCase();

  return devices.filter((device) => {
    if (!isWirelessDevice(device)) {
      return false;
    }

    const matchesSearch =
      normalizedSearch.length === 0 ||
      [
        device.display_name,
        device.name,
        device.id,
        device.family,
        device.controller.label,
        device.wireless?.group_label ?? "",
        device.current_mode_summary,
        device.health.summary,
      ]
        .join(" ")
        .toLowerCase()
        .includes(normalizedSearch);

    const channelKey =
      device.wireless?.channel === null || device.wireless?.channel === undefined
        ? "unassigned"
        : String(device.wireless.channel);
    const matchesChannel = channelFilter === "all" || channelFilter === channelKey;

    const matchesHealth =
      healthFilter === "all" ||
      (healthFilter === "healthy" && device.online && device.health.level === "healthy") ||
      (healthFilter === "attention" &&
        (!device.online || device.health.level !== "healthy")) ||
      (healthFilter === "offline" && !device.online);

    return matchesSearch && matchesChannel && matchesHealth;
  });
}

export function buildWirelessGroups(devices: DeviceView[]): WirelessGroup[] {
  const groups = new Map<string, DeviceView[]>();

  for (const device of devices) {
    if (!isWirelessDevice(device)) {
      continue;
    }

    const groupId = device.wireless?.group_id ?? device.id;
    const existing = groups.get(groupId);

    if (existing) {
      existing.push(device);
    } else {
      groups.set(groupId, [device]);
    }
  }

  return [...groups.entries()]
    .map(([groupId, groupDevices]) => {
      const devicesInGroup = [...groupDevices].sort(compareDevices);
      const first = devicesInGroup[0]!;
      const offlineCount = devicesInGroup.filter((device) => !device.online).length;
      const attentionCount = devicesInGroup.filter(
        (device) => !device.online || device.health.level !== "healthy",
      ).length;
      const onlineCount = devicesInGroup.length - offlineCount;
      const statusTone: WirelessGroup["statusTone"] =
        offlineCount === devicesInGroup.length
          ? "offline"
          : attentionCount > 0
            ? "warning"
            : "online";
      const statusLabel =
        offlineCount === devicesInGroup.length
          ? "offline"
          : attentionCount > 0
            ? "attention"
            : "healthy";

      let stateSummary = "Wireless group healthy";
      if (offlineCount === devicesInGroup.length) {
        stateSummary = "All reported devices are offline";
      } else if (attentionCount > 0) {
        stateSummary = `${attentionCount} device${attentionCount === 1 ? "" : "s"} need attention`;
      } else if (devicesInGroup.length > 1) {
        stateSummary = `${devicesInGroup.length} devices share this wireless group`;
      }

      return {
        id: groupId,
        label: first.wireless?.group_label ?? first.display_name,
        channel: first.wireless?.channel ?? null,
        controllerId: first.controller.id,
        controllerLabel: first.controller.label,
        controllerKind: first.controller.kind,
        devices: devicesInGroup,
        deviceCount: devicesInGroup.length,
        onlineCount,
        attentionCount,
        familySummary: summarizeFamilies(devicesInGroup),
        capabilitySummary: summarizeCapabilities(devicesInGroup),
        statusTone,
        statusLabel,
        stateSummary,
      };
    })
    .sort((left, right) => {
      return (
        compareChannels(left.channel, right.channel) ||
        left.label.localeCompare(right.label) ||
        left.id.localeCompare(right.id)
      );
    });
}

export function buildWirelessChannelSummaries(groups: WirelessGroup[]): WirelessChannelSummary[] {
  const summaries = new Map<string, { channel: number | null; groups: WirelessGroup[] }>();

  for (const group of groups) {
    const key = group.channel === null ? "unassigned" : String(group.channel);
    const existing = summaries.get(key);

    if (existing) {
      existing.groups.push(group);
    } else {
      summaries.set(key, {
        channel: group.channel,
        groups: [group],
      });
    }
  }

  return [...summaries.entries()]
    .map(([key, summary]) => {
      const controllerIds = new Set(summary.groups.map((group) => group.controllerId));
      const offlineCount = summary.groups.filter((group) => group.statusTone === "offline").length;
      const attentionCount = summary.groups.filter((group) => group.attentionCount > 0).length;

      return {
        id: key,
        label: formatWirelessChannelLabel(summary.channel),
        groupCount: summary.groups.length,
        deviceCount: summary.groups.reduce((total, group) => total + group.deviceCount, 0),
        controllerCount: controllerIds.size,
        attentionCount,
        offlineCount,
      };
    })
    .sort((left, right) => {
      const leftChannel = left.id === "unassigned" ? null : Number(left.id);
      const rightChannel = right.id === "unassigned" ? null : Number(right.id);

      return compareChannels(leftChannel, rightChannel) || left.label.localeCompare(right.label);
    });
}
