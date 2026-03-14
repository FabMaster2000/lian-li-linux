import { apiClient } from "./api";
import type {
  ProfileApplyResponse,
  ProfileDocument,
  ProfileUpsertDocument,
} from "../types/api";

export function listProfiles() {
  return apiClient.get<ProfileDocument[]>("/profiles");
}

export function createProfile(profile: ProfileUpsertDocument) {
  return apiClient.post<ProfileDocument, ProfileUpsertDocument>(
    "/profiles",
    profile,
  );
}

export function updateProfile(id: string, profile: ProfileUpsertDocument) {
  return apiClient.put<ProfileDocument, ProfileUpsertDocument>(
    `/profiles/${encodeURIComponent(id)}`,
    profile,
  );
}

export function deleteProfile(id: string) {
  return apiClient.delete<{ deleted: boolean; id: string }>(
    `/profiles/${encodeURIComponent(id)}`,
  );
}

export function applyProfile(id: string) {
  return apiClient.post<ProfileApplyResponse, Record<string, never>>(
    `/profiles/${encodeURIComponent(id)}/apply`,
    {},
  );
}
