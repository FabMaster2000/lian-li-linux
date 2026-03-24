import { createContext, useCallback, useContext, useEffect } from "react";
import type { PropsWithChildren } from "react";
import { useBackendEventSubscription } from "../hooks/useBackendEventSubscription";
import { getDaemonStatus, getHealth } from "../services/system";
import { useServerResource } from "../state/server/useServerResource";
import type { DaemonStatusResponse } from "../types/api";

type SystemStatusSnapshot = {
  apiReachable: boolean;
  apiError: string | null;
  daemonStatus: DaemonStatusResponse | null;
  loading: boolean;
  refreshing: boolean;
  refresh: () => Promise<void>;
};

const SystemStatusContext = createContext<SystemStatusSnapshot>({
  apiReachable: true,
  apiError: null,
  daemonStatus: null,
  loading: true,
  refreshing: false,
  refresh: async () => undefined,
});

const pollIntervalMs = 15_000;

export function SystemStatusProvider({ children }: PropsWithChildren) {
  const loadSystemStatus = useCallback(async () => {
    await getHealth();
    const daemonStatus = await getDaemonStatus();

    return {
      daemonStatus,
    };
  }, []);

  const resource = useServerResource({
    initialData: {
      daemonStatus: null as DaemonStatusResponse | null,
    },
    load: loadSystemStatus,
    loadErrorMessage: "Backend API could not be reached",
  });

  useEffect(() => {
    void resource.refresh();
  }, [resource.refresh]);

  useEffect(() => {
    const timer = window.setInterval(() => {
      void resource.refresh({ background: true });
    }, pollIntervalMs);

    return () => {
      window.clearInterval(timer);
    };
  }, [resource.refresh]);

  useBackendEventSubscription(
    useCallback(
      (event) => {
        if (event.type === "daemon.connected" || event.type === "daemon.disconnected") {
          void resource.refresh({ background: true });
        }
      },
      [resource.refresh],
    ),
  );

  return (
    <SystemStatusContext.Provider
      value={{
        apiReachable: resource.error === null,
        apiError: resource.error,
        daemonStatus: resource.data.daemonStatus,
        loading: resource.loading,
        refreshing: resource.refreshing,
        refresh: async () => {
          await resource.refresh();
        },
      }}
    >
      {children}
    </SystemStatusContext.Provider>
  );
}

export function useSystemStatus() {
  return useContext(SystemStatusContext);
}
