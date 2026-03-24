import { apiClient } from "./api";
import type {
  FanApplyRequest,
  FanApplyResponse,
  FanCurveDocument,
  FanTemperaturePreview,
  FanCurveUpsertRequest,
  FanManualRequest,
  FanStateResponse,
} from "../types/api";

function fansBasePath(deviceId: string) {
  return `/devices/${encodeURIComponent(deviceId)}/fans`;
}

function fanCurvePath(name?: string) {
  return name ? `/fan-curves/${encodeURIComponent(name)}` : "/fan-curves";
}

export function getFanState(deviceId: string) {
  return apiClient.get<FanStateResponse>(fansBasePath(deviceId));
}

export function setManualFanSpeed(deviceId: string, request: FanManualRequest) {
  return apiClient.post<FanStateResponse, FanManualRequest>(
    `${fansBasePath(deviceId)}/manual`,
    request,
  );
}

export function listFanCurves() {
  return apiClient.get<FanCurveDocument[]>(fanCurvePath());
}

export function previewFanTemperatureSource(source: string) {
  return apiClient.get<FanTemperaturePreview>("/fan-temperatures/preview", {
    query: { source },
  });
}

export function createFanCurve(request: FanCurveUpsertRequest) {
  return apiClient.post<FanCurveDocument, FanCurveUpsertRequest>(fanCurvePath(), request);
}

export function updateFanCurve(existingName: string, request: FanCurveUpsertRequest) {
  return apiClient.put<FanCurveDocument, FanCurveUpsertRequest>(fanCurvePath(existingName), request);
}

export function deleteFanCurve(name: string) {
  return apiClient.delete<{ deleted: boolean; name: string }>(fanCurvePath(name));
}

export function applyFanWorkbench(request: FanApplyRequest) {
  return apiClient.post<FanApplyResponse, FanApplyRequest>("/fans/apply", request);
}
