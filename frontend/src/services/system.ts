import { apiClient } from "./api";
import type {
  BackendEventEnvelope,
  DaemonStatusResponse,
  HealthResponse,
  JsonValue,
  RuntimeResponse,
  VersionResponse,
} from "../types/api";

export function getHealth() {
  return apiClient.get<HealthResponse>("/health");
}

export function getVersion() {
  return apiClient.get<VersionResponse>("/version");
}

export function getRuntime() {
  return apiClient.get<RuntimeResponse>("/runtime");
}

export function getDaemonStatus() {
  return apiClient.get<DaemonStatusResponse>("/daemon/status");
}

type BackendEventHandlers<TData extends JsonValue = JsonValue> = {
  onMessage: (event: BackendEventEnvelope<TData>) => void;
  onOpen?: () => void;
  onClose?: (event: CloseEvent) => void;
  onError?: (event: Event) => void;
  onParseError?: (raw: string, error: unknown) => void;
};

export function connectBackendEvents<TData extends JsonValue = JsonValue>(
  handlers: BackendEventHandlers<TData>,
) {
  return apiClient.connectEvents<TData>(handlers);
}
