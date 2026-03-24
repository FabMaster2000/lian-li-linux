import { useCallback, useEffect, useState } from "react";
import {
  LIVE_STATUS_REFRESH_INTERVAL_MS,
  useBackgroundRefresh,
} from "./useBackgroundRefresh";
import { useBackendEventSubscription } from "./useBackendEventSubscription";
import {
  connectWirelessDevice,
  disconnectWirelessDevice,
  listDevices,
  refreshWirelessDiscovery,
} from "../services/devices";
import { getDaemonStatus } from "../services/system";
import { useServerResource } from "../state/server/useServerResource";
import type { DaemonStatusResponse } from "../types/api";
import {
  buildAvailableClusters,
  buildPairedClusters,
  type MvpCluster,
} from "../features/mvpClusters";

type WirelessSnapshot = {
  pairedClusters: MvpCluster[];
  availableClusters: MvpCluster[];
  daemonStatus: DaemonStatusResponse | null;
};

function toErrorMessage(error: unknown, fallback: string) {
  return error instanceof Error ? error.message : fallback;
}

export function useMvpWirelessSyncData() {
  const [actionError, setActionError] = useState<string | null>(null);
  const [actionSuccess, setActionSuccess] = useState<string | null>(null);
  const [searching, setSearching] = useState(false);
  const [connectingClusterId, setConnectingClusterId] = useState<string | null>(null);
  const [disconnectingClusterId, setDisconnectingClusterId] = useState<string | null>(null);

  const loadSnapshot = useCallback(async () => {
    const [devices, daemonStatus] = await Promise.all([listDevices(), getDaemonStatus()]);

    return {
      pairedClusters: buildPairedClusters(devices),
      availableClusters: buildAvailableClusters(devices),
      daemonStatus,
    };
  }, []);

  const resource = useServerResource<WirelessSnapshot>({
    initialData: {
      pairedClusters: [],
      availableClusters: [],
      daemonStatus: null,
    },
    load: loadSnapshot,
    loadErrorMessage: "Wireless sync data could not be loaded",
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

  const searchForDevices = useCallback(async () => {
    setSearching(true);
    setActionError(null);
    setActionSuccess(null);

    try {
      const result = await refreshWirelessDiscovery();
      await resource.refresh({ background: true });
      setActionSuccess(
        `Gerätesuche abgeschlossen. ${result.device_count} ${result.device_count === 1 ? "Gerät" : "Geräte"} gemeldet.`,
      );
    } catch (error) {
      setActionError(
        toErrorMessage(error, "Gerätesuche konnte nicht gestartet werden"),
      );
    } finally {
      setSearching(false);
    }
  }, [resource.refresh]);

  const connectCluster = useCallback(
    async (clusterId: string) => {
      const cluster =
        resource.data.availableClusters.find((item) => item.id === clusterId) ?? null;
      if (!cluster) {
        setActionError("Koppelbares Gerät konnte nicht gefunden werden");
        return false;
      }

      if (cluster.status !== "healthy") {
        setActionError("Offline-Geräte können nicht gekoppelt werden");
        return false;
      }

      setConnectingClusterId(clusterId);
      setActionError(null);
      setActionSuccess(null);

      try {
        await Promise.all(
          cluster.deviceIds.map((deviceId) => connectWirelessDevice(deviceId)),
        );
        await resource.refresh({ background: true });
        setActionSuccess(`${cluster.label} wurde erfolgreich gekoppelt.`);
        return true;
      } catch (error) {
        setActionError(
          toErrorMessage(error, "Gerät konnte nicht gekoppelt werden"),
        );
        return false;
      } finally {
        setConnectingClusterId(null);
      }
    },
    [resource.data.availableClusters, resource.refresh],
  );

  const disconnectCluster = useCallback(
    async (clusterId: string) => {
      const cluster =
        resource.data.pairedClusters.find((item) => item.id === clusterId) ?? null;
      if (!cluster) {
        setActionError("Gekoppeltes Gerät konnte nicht gefunden werden");
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
          toErrorMessage(error, "Gerät konnte nicht entkoppelt werden"),
        );
        return false;
      } finally {
        setDisconnectingClusterId(null);
      }
    },
    [resource.data.pairedClusters, resource.refresh],
  );

  return {
    pairedClusters: resource.data.pairedClusters,
    availableClusters: resource.data.availableClusters,
    daemonStatus: resource.data.daemonStatus,
    loading: resource.loading,
    refreshing: resource.refreshing,
    error: resource.error,
    lastUpdated: resource.lastUpdated,
    actionError,
    actionSuccess,
    searching,
    connectingClusterId,
    disconnectingClusterId,
    refresh,
    searchForDevices,
    connectCluster,
    disconnectCluster,
  };
}
