import { apiClient } from "./api";
import type { ConfigDocument } from "../types/api";

export function getConfig() {
  return apiClient.get<ConfigDocument>("/config");
}

export function saveConfig(config: ConfigDocument) {
  return apiClient.post<ConfigDocument, ConfigDocument>("/config", config);
}
