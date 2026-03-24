import { useCallback, useEffect, useState } from "react";
import {
  LIVE_STATUS_REFRESH_INTERVAL_MS,
  useBackgroundRefresh,
} from "./useBackgroundRefresh";
import { useBackendEventSubscription } from "./useBackendEventSubscription";
import { disconnectWirelessDevice, listDevices } from "../services/devices";
import { getFanState } from "../services/fans";
import { getLightingState } from "../services/lighting";
import { useServerResource } from "../state/server/useServerResource";
import type { FanStateResponse, LightingStateResponse } from "../types/api";
import { buildPairedClusters, type MvpCluster } from "../features/mvpClusters";

type DashboardSnapshot = {
  clusters: MvpCluster[];
  fanStates: Record<string, FanStateResponse | null>;
  lightingStates: Record<string, LightingStateResponse | null>;
};

function toErrorMessage(error: unknown, fallback: string) {
  return error instanceof Error ? error.message : fallback;
}

async function loadStateMap<TState>(
  clusters: MvpCluster[],
  canLoad: (cluster: MvpCluster) => boolean,
  load: (cluster: MvpCluster) => Promise<TState>,
) {
  const entries = await Promise.all(
    clusters.map(async (cluster) => {
      if (!canLoad(cluster)) {
        return [cluster.id, null] as const;
      }

      try {
        return [cluster.id, await load(cluster)] as const;
      } catch {
        return [cluster.id, null] as const;
      }
    }),
  );

  return Object.fromEntries(entries) as Record<string, TState | null>;
}

export function useMvpDashboardData() {
  const [actionError, setActionError] = useState<string | null>(null);
  const [actionSuccess, setActionSuccess] = useState<string | null>(null);
  const [disconnectingClusterId, setDisconnectingClusterId] = useState<string | null>(null);

  const loadSnapshot = useCallback(async () => {
    const devices = await listDevices();
    const clusters = buildPairedClusters(devices);
    const [fanStates, lightingStates] = await Promise.all([
      loadStateMap(
        clusters,
        (cluster) => cluster.primaryDevice.capabilities.has_fan,
        (cluster) => getFanState(cluster.primaryDeviceId),
      ),
      loadStateMap(
        clusters,
        (cluster) => cluster.primaryDevice.capabilities.has_rgb,
        (cluster) => getLightingState(cluster.primaryDeviceId),
      ),
    ]);

    return {
      clusters,
      fanStates,
      lightingStates,
    };
  }, []);

  const resource = useServerResource<DashboardSnapshot>({
    initialData: {
      clusters: [],
      fanStates: {},
      lightingStates: {},
    },
    load: loadSnapshot,
    loadErrorMessage: "Dashboard data could not be loaded",
  });

  const refresh = useCallback(async () => {
    await resource.refresh();
  }, [resource.refresh]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useBackendEventSubscription(
    useCallback(
      (event) => {
        if (
          event.type === "daemon.connected" ||
          event.type === "daemon.disconnected" ||
          event.type === "device.updated" ||
          event.type === "fan.changed" ||
          event.type === "lighting.changed" ||
          event.type === "config.changed"
        ) {
          void resource.refresh({ background: true });
        }
      },
      [resource.refresh],
    ),
  );

  useBackgroundRefresh(
    useCallback(async () => {
      await resource.refresh({ background: true });
    }, [resource.refresh]),
    LIVE_STATUS_REFRESH_INTERVAL_MS,
  );

  const disconnectCluster = useCallback(
    async (clusterId: string) => {
      const cluster = resource.data.clusters.find((item) => item.id === clusterId) ?? null;
      if (!cluster) {
        setActionError("Cluster could not be found");
        return false;
      }

      setDisconnectingClusterId(clusterId);
      setActionError(null);
      setActionSuccess(null);

      try {
        await Promise.all(
          cluster.deviceIds.map((deviceId) => disconnectWirelessDevice(deviceId)),
        );
        await resource.refresh({ background: true });
        setActionSuccess(`${cluster.label} wurde erfolgreich entkoppelt.`);
        return true;
      } catch (error) {
        setActionError(
          toErrorMessage(error, "Cluster could not be disconnected"),
        );
        return false;
      } finally {
        setDisconnectingClusterId(null);
      }
    },
    [resource.data.clusters, resource.refresh],
  );

  return {
    clusters: resource.data.clusters,
    fanStates: resource.data.fanStates,
    lightingStates: resource.data.lightingStates,
    loading: resource.loading,
    refreshing: resource.refreshing,
    error: resource.error,
    lastUpdated: resource.lastUpdated,
    actionError,
    actionSuccess,
    disconnectingClusterId,
    refresh,
    disconnectCluster,
  };
}
