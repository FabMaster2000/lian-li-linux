import { useCallback, useEffect, useMemo } from "react";
import {
  LIVE_STATUS_REFRESH_INTERVAL_MS,
  useBackgroundRefresh,
} from "./useBackgroundRefresh";
import { useBackendEventSubscription } from "./useBackendEventSubscription";
import { getDevice } from "../services/devices";
import { getFanState } from "../services/fans";
import { getLightingState } from "../services/lighting";
import { listProfiles } from "../services/profiles";
import { useServerResource } from "../state/server/useServerResource";
import type {
  DeviceView,
  FanStateResponse,
  LightingStateResponse,
  ProfileDocument,
} from "../types/api";

type DeviceDetailState = {
  deviceId: string;
  device: DeviceView | null;
  lightingState: LightingStateResponse | null;
  fanState: FanStateResponse | null;
  profiles: ProfileDocument[];
  loading: boolean;
  refreshing: boolean;
  error: string | null;
  lightingError: string | null;
  fanError: string | null;
  profileError: string | null;
  refresh: () => Promise<void>;
};

function decodeDeviceId(input: string) {
  try {
    return decodeURIComponent(input);
  } catch {
    return input;
  }
}

function toErrorMessage(error: unknown, fallback: string) {
  return error instanceof Error ? error.message : fallback;
}

export function useDeviceDetailData(routeDeviceId: string): DeviceDetailState {
  const deviceId = useMemo(() => decodeDeviceId(routeDeviceId), [routeDeviceId]);
  const loadDeviceDetailSnapshot = useCallback(async () => {
    const device = await getDevice(deviceId);
    let lightingState: LightingStateResponse | null = null;
    let fanState: FanStateResponse | null = null;
    let lightingError: string | null = null;
    let fanError: string | null = null;
    let profiles: ProfileDocument[] = [];
    let profileError: string | null = null;

    if (device.capabilities.has_rgb) {
      try {
        lightingState = await getLightingState(deviceId);
      } catch (error) {
        lightingError = toErrorMessage(error, "Lighting state could not be loaded");
      }
    }

    if (device.capabilities.has_fan) {
      try {
        fanState = await getFanState(deviceId);
      } catch (error) {
        fanError = toErrorMessage(error, "Fan state could not be loaded");
      }
    }

    try {
      profiles = await listProfiles();
    } catch (error) {
      profileError = toErrorMessage(error, "Profile assignments could not be loaded");
    }

    return {
      device,
      lightingState,
      fanState,
      profiles,
      lightingError,
      fanError,
      profileError,
    };
  }, [deviceId]);

  const snapshot = useServerResource({
    initialData: {
      device: null as DeviceView | null,
      lightingState: null as LightingStateResponse | null,
      fanState: null as FanStateResponse | null,
      profiles: [] as ProfileDocument[],
      lightingError: null as string | null,
      fanError: null as string | null,
      profileError: null as string | null,
    },
    load: loadDeviceDetailSnapshot,
    loadErrorMessage: "Device detail could not be loaded",
  });

  const refresh = useCallback(async () => {
    await snapshot.refresh();
  }, [snapshot.refresh]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useBackendEventSubscription(
    useCallback(
      (event) => {
        if (event.type === "daemon.connected" || event.type === "daemon.disconnected") {
          void snapshot.refresh({ background: true });
          return;
        }

        if (
          (!event.device_id || event.device_id === deviceId) &&
          (event.type === "device.updated" ||
            event.type === "fan.changed" ||
            event.type === "lighting.changed" ||
            event.type === "config.changed")
        ) {
          void snapshot.refresh({ background: true });
        }
      },
      [deviceId, snapshot.refresh],
    ),
  );

  useBackgroundRefresh(
    useCallback(async () => {
      await snapshot.refresh({ background: true });
    }, [snapshot.refresh]),
    LIVE_STATUS_REFRESH_INTERVAL_MS,
  );

  return {
    deviceId,
    device: snapshot.data.device,
    lightingState: snapshot.data.lightingState,
    fanState: snapshot.data.fanState,
    profiles: snapshot.data.profiles,
    loading: snapshot.loading,
    refreshing: snapshot.refreshing,
    error: snapshot.error,
    lightingError: snapshot.data.lightingError,
    fanError: snapshot.data.fanError,
    profileError: snapshot.data.profileError,
    refresh,
  };
}
