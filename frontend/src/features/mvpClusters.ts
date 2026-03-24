import type {
  DeviceView,
  FanCurvePointDocument,
  FanStateResponse,
  LightingStateResponse,
} from "../types/api";

export type MvpClusterStatus = "healthy" | "offline";

export type MvpCluster = {
  id: string;
  label: string;
  deviceIds: string[];
  primaryDeviceId: string;
  fanCount: number | null;
  fanType: string;
  status: MvpClusterStatus;
  devices: DeviceView[];
  primaryDevice: DeviceView;
};

export type FanCurveSource = "cpu" | "gpu";

const defaultCurvePoints: FanCurvePointDocument[] = [
  { temperature_celsius: 30, percent: 30 },
  { temperature_celsius: 50, percent: 55 },
  { temperature_celsius: 70, percent: 80 },
];

function compareDevices(left: DeviceView, right: DeviceView) {
  return (
    left.ui_order - right.ui_order ||
    left.display_name.localeCompare(right.display_name) ||
    left.id.localeCompare(right.id)
  );
}

function resolveBindingState(device: DeviceView) {
  return device.wireless?.binding_state ?? "connected";
}

function isOfflineDevice(device: DeviceView) {
  return device.online === false || device.health.level === "offline";
}

function toCluster(
  clusterId: string,
  clusterDevices: DeviceView[],
): MvpCluster | null {
  const devices = [...clusterDevices].sort(compareDevices);
  const primaryDevice = devices[0] ?? null;
  if (!primaryDevice) {
    return null;
  }

  return {
    id: clusterId,
    label: primaryDevice.wireless?.group_label ?? primaryDevice.display_name,
    deviceIds: devices.map((device) => device.id),
    primaryDeviceId: primaryDevice.id,
    fanCount: primaryDevice.capabilities.fan_count,
    fanType: primaryDevice.family,
    status: devices.every(isOfflineDevice) ? "offline" : "healthy",
    devices,
    primaryDevice,
  };
}

function buildClusters(
  devices: DeviceView[],
  bindingState: "connected" | "available",
  options: {
    includeOffline?: boolean;
  } = {},
) {
  const { includeOffline = true } = options;
  const groups = new Map<string, DeviceView[]>();

  for (const device of devices) {
    if (device.wireless === null) {
      continue;
    }

    if (resolveBindingState(device) !== bindingState) {
      continue;
    }

    if (!includeOffline && isOfflineDevice(device)) {
      continue;
    }

    const clusterId = device.wireless.group_id ?? device.id;
    const current = groups.get(clusterId);
    if (current) {
      current.push(device);
    } else {
      groups.set(clusterId, [device]);
    }
  }

  return [...groups.entries()]
    .map(([clusterId, clusterDevices]) => toCluster(clusterId, clusterDevices))
    .filter((cluster): cluster is MvpCluster => cluster !== null)
    .sort((left, right) => {
      return (
        left.label.localeCompare(right.label) ||
        left.id.localeCompare(right.id)
      );
    });
}

export function buildPairedClusters(devices: DeviceView[]) {
  return buildClusters(devices, "connected");
}

export function buildAvailableClusters(devices: DeviceView[]) {
  return buildClusters(devices, "available", { includeOffline: false });
}

export function resolveRequestedClusterId(
  requestedClusterId: string | null,
  requestedDeviceId: string | null,
  clusters: MvpCluster[],
) {
  if (requestedClusterId && clusters.some((cluster) => cluster.id === requestedClusterId)) {
    return requestedClusterId;
  }

  if (requestedDeviceId) {
    const cluster = clusters.find((item) => item.deviceIds.includes(requestedDeviceId));
    if (cluster) {
      return cluster.id;
    }
  }

  return "";
}

export function normalizeFanCurveSource(source: string | null | undefined): FanCurveSource {
  return source?.toLowerCase().includes("gpu") ? "gpu" : "cpu";
}

export function buildMvpFanCurvePoints(points: FanCurvePointDocument[] | null | undefined) {
  const source = points && points.length > 0 ? points : defaultCurvePoints;
  return source.map((point) => ({
    temperature_celsius: Math.round(point.temperature_celsius * 10) / 10,
    percent: clampPercent(point.percent),
  }));
}

export function draftFromFanState(
  fanState: FanStateResponse | null,
  curvePoints?: FanCurvePointDocument[] | null,
) {
  const manualPercent = averagePercent(fanState);
  const mode: "manual" | "curve" =
    fanState?.active_mode === "manual" ? "manual" : "curve";

  return {
    mode,
    manualPercent,
    curveSource: normalizeFanCurveSource(fanState?.temperature_source),
    points: buildMvpFanCurvePoints(curvePoints),
  };
}

export function buildMvpFanCurveName(clusterId: string) {
  return `__mvp_cluster__${sanitizeClusterId(clusterId)}`;
}

export function sanitizeClusterId(clusterId: string) {
  return clusterId.replace(/[^a-zA-Z0-9_-]+/g, "_");
}

export function clampPercent(value: number) {
  return Math.max(0, Math.min(100, Math.round(value)));
}

export function summarizeFanRpm(rpms: number[] | null | undefined) {
  return rpms && rpms.length > 0 ? rpms.join(" / ") : "n/a";
}

export function summarizeLightingState(lightingState: LightingStateResponse | null | undefined) {
  const firstZone = lightingState?.zones[0] ?? null;
  if (!firstZone) {
    return "n/a";
  }

  if (firstZone.effect === "Static" && firstZone.colors[0]) {
    return `Static ${firstZone.colors[0]}`;
  }

  return firstZone.effect;
}

export function getLightingApplyDefaults(lightingState: LightingStateResponse | null | undefined) {
  const firstZone = lightingState?.zones[0] ?? null;
  return {
    brightness: firstZone?.brightness_percent ?? 100,
    speed: firstZone?.speed ?? 2,
    direction: firstZone?.direction ?? "Clockwise",
    scope: firstZone?.scope ?? "All",
  };
}

function averagePercent(fanState: FanStateResponse | null) {
  if (!fanState) {
    return 50;
  }

  const values = fanState.slots
    .map((slot) => slot.percent)
    .filter((percent): percent is number => typeof percent === "number");

  if (values.length === 0) {
    return 50;
  }

  return clampPercent(values.reduce((sum, value) => sum + value, 0) / values.length);
}
