import { useCallback, useEffect, useMemo, useState } from "react";
import type { Dispatch, SetStateAction } from "react";
import { lightingEffectOptions } from "../features/lighting";
import { listDevices } from "../services/devices";
import {
  applyProfile,
  createProfile,
  deleteProfile,
  listProfiles,
} from "../services/profiles";
import type {
  DeviceView,
  ProfileApplyResponse,
  ProfileDocument,
  ProfileUpsertDocument,
} from "../types/api";

export type ProfileDraft = {
  id: string;
  name: string;
  description: string;
  targetMode: "all" | "devices";
  selectedDeviceIds: string[];
  lightingEnabled: boolean;
  lightingColor: string;
  lightingEffect: string;
  lightingBrightness: number;
  fanEnabled: boolean;
  fanMode: "manual";
  fanPercent: number;
};

type ProfilesWorkbenchState = {
  devices: DeviceView[];
  profiles: ProfileDocument[];
  draft: ProfileDraft;
  setDraft: Dispatch<SetStateAction<ProfileDraft>>;
  loading: boolean;
  submitting: boolean;
  deletingProfileId: string | null;
  applyingProfileId: string | null;
  error: string | null;
  success: string | null;
  applyResult: ProfileApplyResponse | null;
  refresh: () => Promise<void>;
  createDraftProfile: () => Promise<void>;
  removeProfile: (profileId: string) => Promise<void>;
  runProfile: (profileId: string) => Promise<void>;
};

const defaultDraft: ProfileDraft = {
  id: "",
  name: "",
  description: "",
  targetMode: "all",
  selectedDeviceIds: [],
  lightingEnabled: true,
  lightingColor: "#223366",
  lightingEffect: lightingEffectOptions[1]?.value ?? "Static",
  lightingBrightness: 25,
  fanEnabled: false,
  fanMode: "manual",
  fanPercent: 30,
};

function toErrorMessage(error: unknown, fallback: string) {
  return error instanceof Error ? error.message : fallback;
}

function clampPercent(value: number) {
  return Math.min(100, Math.max(0, Math.round(value)));
}

function normalizeHexColor(value: string) {
  return /^#[0-9a-fA-F]{6}$/.test(value) ? value.toLowerCase() : "#223366";
}

function validateDraft(draft: ProfileDraft) {
  if (!draft.id.trim()) {
    return "Profile id is required";
  }

  if (!/^[a-z0-9_-]+$/.test(draft.id.trim())) {
    return "Profile id must be lowercase and slug-like";
  }

  if (!draft.name.trim()) {
    return "Profile name is required";
  }

  if (!draft.lightingEnabled && !draft.fanEnabled) {
    return "Enable lighting or fans before creating a profile";
  }

  if (draft.targetMode === "devices" && draft.selectedDeviceIds.length === 0) {
    return "Select at least one device when using explicit device targets";
  }

  return null;
}

function buildPayload(draft: ProfileDraft): ProfileUpsertDocument {
  return {
    id: draft.id.trim(),
    name: draft.name.trim(),
    description: draft.description.trim() || undefined,
    targets: {
      mode: draft.targetMode,
      device_ids: draft.targetMode === "devices" ? draft.selectedDeviceIds : [],
    },
    lighting: draft.lightingEnabled
      ? {
          enabled: true,
          color: normalizeHexColor(draft.lightingColor),
          effect: draft.lightingEffect,
          brightness_percent: clampPercent(draft.lightingBrightness),
        }
      : undefined,
    fans: draft.fanEnabled
      ? {
          enabled: true,
          mode: draft.fanMode,
          percent: clampPercent(draft.fanPercent),
        }
      : undefined,
  };
}

export function useProfilesWorkbenchData(): ProfilesWorkbenchState {
  const [devices, setDevices] = useState<DeviceView[]>([]);
  const [profiles, setProfiles] = useState<ProfileDocument[]>([]);
  const [draft, setDraft] = useState<ProfileDraft>(defaultDraft);
  const [loading, setLoading] = useState(true);
  const [submitting, setSubmitting] = useState(false);
  const [deletingProfileId, setDeletingProfileId] = useState<string | null>(null);
  const [applyingProfileId, setApplyingProfileId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [applyResult, setApplyResult] = useState<ProfileApplyResponse | null>(null);

  const targetableDevices = useMemo(
    () => devices.filter((device) => device.capabilities.has_rgb || device.capabilities.has_fan),
    [devices],
  );

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);

    try {
      const [profileItems, deviceItems] = await Promise.all([listProfiles(), listDevices()]);
      setProfiles(profileItems);
      setDevices(deviceItems);
    } catch (err) {
      setError(toErrorMessage(err, "Profiles could not be loaded"));
      setProfiles([]);
      setDevices([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const createDraftProfile = useCallback(async () => {
    const validationError = validateDraft(draft);
    if (validationError) {
      setError(validationError);
      setSuccess(null);
      return;
    }

    setSubmitting(true);
    setError(null);
    setSuccess(null);
    setApplyResult(null);

    try {
      const createdProfile = await createProfile(buildPayload(draft));
      setProfiles((current) => [createdProfile, ...current]);
      setDraft(defaultDraft);
      setSuccess(`Profile '${createdProfile.name}' created`);
    } catch (err) {
      setError(toErrorMessage(err, "Profile could not be created"));
    } finally {
      setSubmitting(false);
    }
  }, [draft]);

  const removeProfile = useCallback(async (profileId: string) => {
    setDeletingProfileId(profileId);
    setError(null);
    setSuccess(null);
    setApplyResult(null);

    try {
      await deleteProfile(profileId);
      setProfiles((current) => current.filter((profile) => profile.id !== profileId));
      setSuccess(`Profile '${profileId}' deleted`);
    } catch (err) {
      setError(toErrorMessage(err, "Profile could not be deleted"));
    } finally {
      setDeletingProfileId(null);
    }
  }, []);

  const runProfile = useCallback(async (profileId: string) => {
    setApplyingProfileId(profileId);
    setError(null);
    setSuccess(null);
    setApplyResult(null);

    try {
      const result = await applyProfile(profileId);
      setApplyResult(result);
      setSuccess(`Profile '${result.profile_name}' applied`);
    } catch (err) {
      setError(toErrorMessage(err, "Profile could not be applied"));
    } finally {
      setApplyingProfileId(null);
    }
  }, []);

  return {
    devices: targetableDevices,
    profiles,
    draft,
    setDraft,
    loading,
    submitting,
    deletingProfileId,
    applyingProfileId,
    error,
    success,
    applyResult,
    refresh,
    createDraftProfile,
    removeProfile,
    runProfile,
  };
}
