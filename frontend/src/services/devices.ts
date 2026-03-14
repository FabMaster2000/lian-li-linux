import { apiClient } from "./api";
import type { DeviceView } from "../types/api";

function devicePath(deviceId: string) {
  return `/devices/${encodeURIComponent(deviceId)}`;
}

export function listDevices() {
  return apiClient.get<DeviceView[]>("/devices");
}

export function getDevice(deviceId: string) {
  return apiClient.get<DeviceView>(devicePath(deviceId));
}
