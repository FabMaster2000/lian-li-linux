import { apiClient } from "./api";
import type {
  LightingApplyRequest,
  LightingApplyResponse,
  LightingBrightnessRequest,
  LightingColorRequest,
  LightingEffectRequest,
  LightingStateResponse,
} from "../types/api";

function lightingBasePath(deviceId: string) {
  return `/devices/${encodeURIComponent(deviceId)}/lighting`;
}

export function getLightingState(deviceId: string) {
  return apiClient.get<LightingStateResponse>(lightingBasePath(deviceId));
}

export function setLightingColor(
  deviceId: string,
  request: LightingColorRequest,
) {
  return apiClient.post<LightingStateResponse, LightingColorRequest>(
    `${lightingBasePath(deviceId)}/color`,
    request,
  );
}

export function setLightingEffect(
  deviceId: string,
  request: LightingEffectRequest,
) {
  return apiClient.post<LightingStateResponse, LightingEffectRequest>(
    `${lightingBasePath(deviceId)}/effect`,
    request,
  );
}

export function setLightingBrightness(
  deviceId: string,
  request: LightingBrightnessRequest,
) {
  return apiClient.post<LightingStateResponse, LightingBrightnessRequest>(
    `${lightingBasePath(deviceId)}/brightness`,
    request,
  );
}

export function applyLightingWorkbench(request: LightingApplyRequest) {
  return apiClient.post<LightingApplyResponse, LightingApplyRequest>(
    "/lighting/apply",
    request,
  );
}


