import { apiClient } from "./api";
import type {
  DevicePresentationUpdateRequest,
  DeviceView,
  WirelessConnectResponse,
  WirelessDiscoveryRefreshResponse,
  WirelessDisconnectResponse,
} from "../types/api";

function devicePath(deviceId: string) {
  return `/devices/${encodeURIComponent(deviceId)}`;
}

export function listDevices() {
  return apiClient.get<DeviceView[]>("/devices");
}

export function getDevice(deviceId: string) {
  return apiClient.get<DeviceView>(devicePath(deviceId));
}

export function updateDevicePresentation(
  deviceId: string,
  request: DevicePresentationUpdateRequest,
) {
  return apiClient.put<DeviceView, DevicePresentationUpdateRequest>(
    `${devicePath(deviceId)}/presentation`,
    request,
  );
}

export function disconnectWirelessDevice(deviceId: string) {
  return apiClient.post<WirelessDisconnectResponse, Record<string, never>>(
    `${devicePath(deviceId)}/wireless/disconnect`,
    {},
  );
}

export function connectWirelessDevice(deviceId: string) {
  return apiClient.post<WirelessConnectResponse, Record<string, never>>(
    `${devicePath(deviceId)}/wireless/connect`,
    {},
  );
}

export function refreshWirelessDiscovery() {
  return apiClient.post<WirelessDiscoveryRefreshResponse, Record<string, never>>(
    "/wireless/discovery/refresh",
    {},
  );
}
