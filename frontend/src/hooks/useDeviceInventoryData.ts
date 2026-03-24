import { useCallback, useEffect, useState } from "react";
import {
  LIVE_STATUS_REFRESH_INTERVAL_MS,
  useBackgroundRefresh,
} from "./useBackgroundRefresh";
import { useBackendEventSubscription } from "./useBackendEventSubscription";
import { applyProfile, listProfiles } from "../services/profiles";
import {
  listDevices,
  updateDevicePresentation,
} from "../services/devices";
import { setLightingColor } from "../services/lighting";
import type {
  DevicePresentationUpdateRequest,
  DeviceView,
  ProfileDocument,
} from "../types/api";

function toErrorMessage(error: unknown, fallback: string) {
  return error instanceof Error ? error.message : fallback;
}

type InventorySnapshot = {
  devices: DeviceView[];
  profiles: ProfileDocument[];
};

type DeviceInventoryDataState = {
  devices: DeviceView[];
  profiles: ProfileDocument[];
  loading: boolean;
  refreshing: boolean;
  error: string | null;
  profileError: string | null;
  actionError: string | null;
  actionSuccess: string | null;
  savingDeviceId: string | null;
  colorDeviceId: string | null;
  profileDeviceId: string | null;
  refresh: () => Promise<void>;
  savePresentation: (
    deviceId: string,
    request: DevicePresentationUpdateRequest,
  ) => Promise<boolean>;
  setStaticColorForDevice: (deviceId: string, color: string) => Promise<boolean>;
  applyProfileToDevice: (deviceId: string, profileId: string) => Promise<boolean>;
};

function mergeDevice(devices: DeviceView[], nextDevice: DeviceView) {
  return devices
    .map((device) => (device.id === nextDevice.id ? nextDevice : device))
    .sort((left, right) => {
      return (
        left.ui_order - right.ui_order ||
        left.controller.label.localeCompare(right.controller.label) ||
        left.display_name.localeCompare(right.display_name) ||
        left.id.localeCompare(right.id)
      );
    });
}

export function useDeviceInventoryData(): DeviceInventoryDataState {
  const [snapshot, setSnapshot] = useState<InventorySnapshot>({
    devices: [],
    profiles: [],
  });
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [profileError, setProfileError] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [actionSuccess, setActionSuccess] = useState<string | null>(null);
  const [savingDeviceId, setSavingDeviceId] = useState<string | null>(null);
  const [colorDeviceId, setColorDeviceId] = useState<string | null>(null);
  const [profileDeviceId, setProfileDeviceId] = useState<string | null>(null);

  const loadSnapshot = useCallback(async (background = false) => {
    if (background) {
      setRefreshing(true);
    } else {
      setLoading(true);
      setError(null);
    }

    try {
      const devices = await listDevices();
      setError(null);

      try {
        const profiles = await listProfiles();
        setSnapshot({ devices, profiles });
        setProfileError(null);
      } catch (nextError) {
        setSnapshot((current) => ({
          devices,
          profiles: background ? current.profiles : [],
        }));
        setProfileError(toErrorMessage(nextError, "Profiles could not be loaded"));
      }
    } catch (nextError) {
      setError(toErrorMessage(nextError, "Device inventory could not be loaded"));
      setProfileError(null);
      if (!background) {
        setSnapshot({ devices: [], profiles: [] });
      }
    } finally {
      if (background) {
        setRefreshing(false);
      } else {
        setLoading(false);
      }
    }
  }, []);

  useEffect(() => {
    void loadSnapshot(false);
  }, [loadSnapshot]);

  useBackendEventSubscription(
    useCallback(
      (event) => {
        if (
          event.type === "daemon.connected" ||
          event.type === "daemon.disconnected" ||
          event.type === "device.updated" ||
          event.type === "config.changed"
        ) {
          void loadSnapshot(true);
        }
      },
      [loadSnapshot],
    ),
  );

  useBackgroundRefresh(
    useCallback(async () => {
      await loadSnapshot(true);
    }, [loadSnapshot]),
    LIVE_STATUS_REFRESH_INTERVAL_MS,
  );

  const refresh = useCallback(async () => {
    await loadSnapshot(false);
  }, [loadSnapshot]);

  const savePresentation = useCallback(
    async (deviceId: string, request: DevicePresentationUpdateRequest) => {
      setSavingDeviceId(deviceId);
      setActionError(null);
      setActionSuccess(null);

      try {
        const nextDevice = await updateDevicePresentation(deviceId, request);
        setSnapshot((current) => ({
          ...current,
          devices: mergeDevice(current.devices, nextDevice),
        }));
        setActionSuccess(`Saved device presentation for ${nextDevice.display_name}.`);
        return true;
      } catch (nextError) {
        setActionError(toErrorMessage(nextError, "Device presentation could not be saved"));
        return false;
      } finally {
        setSavingDeviceId(null);
      }
    },
    [],
  );

  const setStaticColorForDevice = useCallback(async (deviceId: string, color: string) => {
    setColorDeviceId(deviceId);
    setActionError(null);
    setActionSuccess(null);

    try {
      await setLightingColor(deviceId, { color: { hex: color } });
      setActionSuccess(`Applied static color ${color} to ${deviceId}.`);
      await loadSnapshot(true);
      return true;
    } catch (nextError) {
      setActionError(toErrorMessage(nextError, "Static color could not be applied"));
      return false;
    } finally {
      setColorDeviceId(null);
    }
  }, [loadSnapshot]);

  const applyProfileToDevice = useCallback(
    async (deviceId: string, profileId: string) => {
      const matchingProfile = snapshot.profiles.find((profile) => profile.id === profileId) ?? null;
      if (!matchingProfile) {
        setActionError(`Unknown profile: ${profileId}`);
        return false;
      }

      const appliesToDevice =
        matchingProfile.targets.mode === "all" ||
        matchingProfile.targets.device_ids.includes(deviceId);
      if (!appliesToDevice) {
        setActionError(`Profile ${matchingProfile.name} does not target ${deviceId}.`);
        return false;
      }

      setProfileDeviceId(deviceId);
      setActionError(null);
      setActionSuccess(null);

      try {
        await applyProfile(profileId);
        setActionSuccess(`Applied profile ${matchingProfile.name} to the current inventory.`);
        await loadSnapshot(true);
        return true;
      } catch (nextError) {
        setActionError(toErrorMessage(nextError, "Profile could not be applied"));
        return false;
      } finally {
        setProfileDeviceId(null);
      }
    },
    [loadSnapshot, snapshot.profiles],
  );

  return {
    devices: snapshot.devices,
    profiles: snapshot.profiles,
    loading,
    refreshing,
    error,
    profileError,
    actionError,
    actionSuccess,
    savingDeviceId,
    colorDeviceId,
    profileDeviceId,
    refresh,
    savePresentation,
    setStaticColorForDevice,
    applyProfileToDevice,
  };
}
