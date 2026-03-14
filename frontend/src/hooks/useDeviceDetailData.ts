import { useCallback, useEffect, useMemo } from "react";
import { useBackendEventSubscription } from "./useBackendEventSubscription";
import { getDevice } from "../services/devices";
import { getFanState } from "../services/fans";
import { getLightingState } from "../services/lighting";
import { useServerResource } from "../state/server/useServerResource";
import type {
  DeviceView,
  FanStateResponse,
  LightingStateResponse,
} from "../types/api";

type DeviceDetailState = {
  deviceId: string;
  device: DeviceView | null;
  lightingState: LightingStateResponse | null;
  fanState: FanStateResponse | null;
  loading: boolean;
  refreshing: boolean;
  error: string | null;
  lightingError: string | null;
  fanError: string | null;
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

    return {
      device,
      lightingState,
      fanState,
      lightingError,
      fanError,
    };
  }, [deviceId]);

  const snapshot = useServerResource({
    initialData: {
      device: null as DeviceView | null,
      lightingState: null as LightingStateResponse | null,
      fanState: null as FanStateResponse | null,
      lightingError: null as string | null,
      fanError: null as string | null,
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
          event.device_id === deviceId &&
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

  return {
    deviceId,
    device: snapshot.data.device,
    lightingState: snapshot.data.lightingState,
    fanState: snapshot.data.fanState,
    loading: snapshot.loading,
    refreshing: snapshot.refreshing,
    error: snapshot.error,
    lightingError: snapshot.data.lightingError,
    fanError: snapshot.data.fanError,
    refresh,
  };
}
