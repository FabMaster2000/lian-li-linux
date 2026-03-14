import { useCallback, useEffect } from "react";
import { useBackendEventSubscription } from "./useBackendEventSubscription";
import type { DaemonStatusResponse, DeviceView, RuntimeResponse } from "../types/api";
import { listDevices } from "../services/devices";
import { getDaemonStatus, getRuntime } from "../services/system";
import { useServerResource } from "../state/server/useServerResource";

type DashboardDataState = {
  devices: DeviceView[];
  daemonStatus: DaemonStatusResponse | null;
  runtime: RuntimeResponse | null;
  loading: boolean;
  refreshing: boolean;
  error: string | null;
  lastUpdated: string | null;
  refresh: () => Promise<void>;
};

export function useDashboardData(): DashboardDataState {
  const loadDashboardSnapshot = useCallback(async () => {
    const [devices, daemonStatus, runtime] = await Promise.all([
      listDevices(),
      getDaemonStatus(),
      getRuntime(),
    ]);

    return {
      devices,
      daemonStatus,
      runtime,
    };
  }, []);

  const snapshot = useServerResource({
    initialData: {
      devices: [] as DeviceView[],
      daemonStatus: null as DaemonStatusResponse | null,
      runtime: null as RuntimeResponse | null,
    },
    load: loadDashboardSnapshot,
    loadErrorMessage: "Dashboard data could not be loaded",
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
        if (
          event.type === "daemon.connected" ||
          event.type === "daemon.disconnected" ||
          event.type === "device.updated" ||
          event.type === "fan.changed"
        ) {
          void snapshot.refresh({ background: true });
        }
      },
      [snapshot.refresh],
    ),
  );

  return {
    devices: snapshot.data.devices,
    daemonStatus: snapshot.data.daemonStatus,
    runtime: snapshot.data.runtime,
    loading: snapshot.loading,
    refreshing: snapshot.refreshing,
    error: snapshot.error,
    lastUpdated: snapshot.lastUpdated,
    refresh,
  };
}
