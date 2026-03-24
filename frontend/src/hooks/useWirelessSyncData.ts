import { useCallback, useEffect, useState } from "react";
import {
  LIVE_STATUS_REFRESH_INTERVAL_MS,
  useBackgroundRefresh,
} from "./useBackgroundRefresh";
import {
  isAvailableWirelessDevice,
  isConnectedWirelessDevice,
  isForeignWirelessDevice,
  isWirelessDevice,
} from "../features/wirelessSync";
import {
  connectWirelessDevice,
  disconnectWirelessDevice,
  listDevices,
  refreshWirelessDiscovery,
  updateDevicePresentation,
} from "../services/devices";
import { getDaemonStatus, getRuntime } from "../services/system";
import { useServerResource } from "../state/server/useServerResource";
import type {
  DaemonStatusResponse,
  DeviceView,
  RuntimeResponse,
} from "../types/api";
import { useBackendEventSubscription } from "./useBackendEventSubscription";

function toErrorMessage(error: unknown, fallback: string) {
  return error instanceof Error ? error.message : fallback;
}

type WirelessSyncSnapshot = {
  devices: DeviceView[];
  daemonStatus: DaemonStatusResponse | null;
  runtime: RuntimeResponse | null;
};

export type WirelessGroupPresentationRequest = {
  controllerLabel: string;
  clusterLabel: string;
};

type WirelessSyncDataState = {
  devices: DeviceView[];
  wirelessDevices: DeviceView[];
  connectedWirelessDevices: DeviceView[];
  availableWirelessDevices: DeviceView[];
  foreignWirelessDevices: DeviceView[];
  daemonStatus: DaemonStatusResponse | null;
  runtime: RuntimeResponse | null;
  loading: boolean;
  refreshing: boolean;
  searching: boolean;
  error: string | null;
  lastUpdated: string | null;
  actionError: string | null;
  actionSuccess: string | null;
  savingGroupId: string | null;
  connectingDeviceId: string | null;
  disconnectingGroupId: string | null;
  refresh: () => Promise<void>;
  searchForDevices: () => Promise<void>;
  saveGroupPresentation: (
    groupId: string,
    request: WirelessGroupPresentationRequest,
  ) => Promise<boolean>;
  connectDevice: (deviceId: string) => Promise<boolean>;
  disconnectGroup: (groupId: string) => Promise<boolean>;
};

export function useWirelessSyncData(): WirelessSyncDataState {
  const [actionError, setActionError] = useState<string | null>(null);
  const [actionSuccess, setActionSuccess] = useState<string | null>(null);
  const [savingGroupId, setSavingGroupId] = useState<string | null>(null);
  const [searching, setSearching] = useState(false);
  const [connectingDeviceId, setConnectingDeviceId] = useState<string | null>(null);
  const [disconnectingGroupId, setDisconnectingGroupId] = useState<string | null>(null);

  const loadSnapshot = useCallback(async () => {
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

  const snapshot = useServerResource<WirelessSyncSnapshot>({
    initialData: {
      devices: [],
      daemonStatus: null,
      runtime: null,
    },
    load: loadSnapshot,
    loadErrorMessage: "Wireless sync data could not be loaded",
  });

  const refresh = useCallback(async () => {
    await snapshot.refresh();
  }, [snapshot.refresh]);

  const searchForDevices = useCallback(async () => {
    setSearching(true);
    setActionError(null);
    setActionSuccess(null);

    try {
      const result = await refreshWirelessDiscovery();
      await snapshot.refresh({ background: true });
      setActionSuccess(
        `Wireless scan completed. ${result.device_count} device${result.device_count === 1 ? "" : "s"} currently reported.`,
      );
    } catch (error) {
      setActionError(toErrorMessage(error, "Wireless scan could not be completed"));
    } finally {
      setSearching(false);
    }
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
          event.type === "config.changed"
        ) {
          void snapshot.refresh({ background: true });
        }
      },
      [snapshot.refresh],
    ),
  );

  useBackgroundRefresh(
    useCallback(async () => {
      await snapshot.refresh({ background: true });
    }, [snapshot.refresh]),
    LIVE_STATUS_REFRESH_INTERVAL_MS,
  );

  const saveGroupPresentation = useCallback(
    async (groupId: string, request: WirelessGroupPresentationRequest) => {
      setSavingGroupId(groupId);
      setActionError(null);
      setActionSuccess(null);

      try {
        const targetDevices = snapshot.data.devices.filter(
          (device) => isWirelessDevice(device) && (device.wireless?.group_id ?? device.id) === groupId,
        );

        if (targetDevices.length === 0) {
          setActionError(`Unknown wireless group: ${groupId}`);
          return false;
        }

        await Promise.all(
          targetDevices.map((device) =>
            updateDevicePresentation(device.id, {
              display_name: device.display_name,
              ui_order: device.ui_order,
              physical_role: device.physical_role,
              controller_label: request.controllerLabel,
              cluster_label: request.clusterLabel,
            }),
          ),
        );

        await snapshot.refresh({ background: true });

        setActionSuccess(
          `Saved wireless labels for ${targetDevices.length} device${targetDevices.length === 1 ? "" : "s"}.`,
        );
        return true;
      } catch (error) {
        setActionError(toErrorMessage(error, "Wireless labels could not be saved"));
        return false;
      } finally {
        setSavingGroupId(null);
      }
    },
    [snapshot.data.devices, snapshot.refresh],
  );

  const disconnectGroup = useCallback(
    async (groupId: string) => {
      setDisconnectingGroupId(groupId);
      setActionError(null);
      setActionSuccess(null);

      try {
        const targetDevices = snapshot.data.devices.filter(
          (device) => isWirelessDevice(device) && (device.wireless?.group_id ?? device.id) === groupId,
        );

        if (targetDevices.length === 0) {
          setActionError(`Unknown wireless group: ${groupId}`);
          return false;
        }

        for (const device of targetDevices) {
          await disconnectWirelessDevice(device.id);
        }

        await snapshot.refresh({ background: true });

        setActionSuccess(
          `Disconnected ${targetDevices.length} wireless device${targetDevices.length === 1 ? "" : "s"} from the dongle.`,
        );
        return true;
      } catch (error) {
        setActionError(toErrorMessage(error, "Wireless disconnect could not be completed"));
        return false;
      } finally {
        setDisconnectingGroupId(null);
      }
    },
    [snapshot.data.devices, snapshot.refresh],
  );

  const connectDevice = useCallback(
    async (deviceId: string) => {
      setConnectingDeviceId(deviceId);
      setActionError(null);
      setActionSuccess(null);

      try {
        await connectWirelessDevice(deviceId);
        await snapshot.refresh({ background: true });
        setActionSuccess(`Connected ${deviceId} to the active wireless dongle.`);
        return true;
      } catch (error) {
        setActionError(toErrorMessage(error, "Wireless connect could not be completed"));
        return false;
      } finally {
        setConnectingDeviceId(null);
      }
    },
    [snapshot.refresh],
  );

  const wirelessDevices = snapshot.data.devices.filter(isWirelessDevice);
  const connectedWirelessDevices = wirelessDevices.filter(isConnectedWirelessDevice);
  const availableWirelessDevices = wirelessDevices.filter(isAvailableWirelessDevice);
  const foreignWirelessDevices = wirelessDevices.filter(isForeignWirelessDevice);

  return {
    devices: snapshot.data.devices,
    wirelessDevices,
    connectedWirelessDevices,
    availableWirelessDevices,
    foreignWirelessDevices,
    daemonStatus: snapshot.data.daemonStatus,
    runtime: snapshot.data.runtime,
    loading: snapshot.loading,
    refreshing: snapshot.refreshing,
    searching,
    error: snapshot.error,
    lastUpdated: snapshot.lastUpdated,
    actionError,
    actionSuccess,
    savingGroupId,
    connectingDeviceId,
    disconnectingGroupId,
    refresh,
    searchForDevices,
    saveGroupPresentation,
    connectDevice,
    disconnectGroup,
  };
}
