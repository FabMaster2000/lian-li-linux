import { apiClient } from "./api";
import type { FanManualRequest, FanStateResponse } from "../types/api";

function fansBasePath(deviceId: string) {
  return `/devices/${encodeURIComponent(deviceId)}/fans`;
}

export function getFanState(deviceId: string) {
  return apiClient.get<FanStateResponse>(fansBasePath(deviceId));
}

export function setManualFanSpeed(
  deviceId: string,
  request: FanManualRequest,
) {
  return apiClient.post<FanStateResponse, FanManualRequest>(
    `${fansBasePath(deviceId)}/manual`,
    request,
  );
}
