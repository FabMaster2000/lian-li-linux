import { useCallback, useEffect, useMemo, useState } from "react";
import { useBackendEventSubscription } from "./useBackendEventSubscription";
import { listDevices } from "../services/devices";
import { getFanState, setManualFanSpeed } from "../services/fans";
import type { DeviceView, FanStateResponse } from "../types/api";

type FansWorkbenchState = {
  devices: DeviceView[];
  selectedDeviceId: string;
  setSelectedDeviceId: (deviceId: string) => void;
  fanState: FanStateResponse | null;
  formPercent: number;
  setFormPercent: (percent: number) => void;
  loading: boolean;
  stateLoading: boolean;
  stateRefreshing: boolean;
  submitting: boolean;
  error: string | null;
  success: string | null;
  refresh: () => Promise<void>;
  applyChanges: () => Promise<void>;
};

const defaultPercent = 50;

function toErrorMessage(error: unknown, fallback: string) {
  return error instanceof Error ? error.message : fallback;
}

function clampPercent(value: number) {
  return Math.min(100, Math.max(0, Math.round(value)));
}

function seedPercentFromState(fanState: FanStateResponse | null) {
  if (!fanState) {
    return defaultPercent;
  }

  const definedPercents = fanState.slots
    .map((slot) => slot.percent)
    .filter((percent): percent is number => typeof percent === "number");

  if (definedPercents.length === 0) {
    return defaultPercent;
  }

  const total = definedPercents.reduce((sum, percent) => sum + percent, 0);
  return clampPercent(total / definedPercents.length);
}

type LoadFanStateOptions = {
  syncForm?: boolean;
  background?: boolean;
};

export function useFansWorkbenchData(requestedDeviceId: string | null): FansWorkbenchState {
  const [devices, setDevices] = useState<DeviceView[]>([]);
  const [selectedDeviceId, setSelectedDeviceId] = useState(requestedDeviceId ?? "");
  const [fanState, setFanState] = useState<FanStateResponse | null>(null);
  const [formPercent, setFormPercent] = useState(defaultPercent);
  const [loading, setLoading] = useState(true);
  const [stateLoading, setStateLoading] = useState(false);
  const [stateRefreshing, setStateRefreshing] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  const fanDevices = useMemo(
    () => devices.filter((device) => device.capabilities.has_fan),
    [devices],
  );

  const loadFanState = useCallback(async (deviceId: string, options: LoadFanStateOptions = {}) => {
    const { syncForm = true, background = false } = options;

    if (background) {
      setStateRefreshing(true);
    } else {
      setStateLoading(true);
      setError(null);
      setSuccess(null);

      if (syncForm) {
        setFanState(null);
        setFormPercent(defaultPercent);
      }
    }

    try {
      const nextState = await getFanState(deviceId);
      setFanState(nextState);

      if (syncForm) {
        setFormPercent(seedPercentFromState(nextState));
      }

      setError(null);
    } catch (err) {
      if (!background) {
        setFanState(null);
        setFormPercent(defaultPercent);
      }

      setError(toErrorMessage(err, "Fan state could not be loaded"));
    } finally {
      if (background) {
        setStateRefreshing(false);
      } else {
        setStateLoading(false);
      }
    }
  }, []);

  const initialize = useCallback(async () => {
    setLoading(true);
    setError(null);

    try {
      const deviceItems = await listDevices();
      const nextFanDevices = deviceItems.filter((device) => device.capabilities.has_fan);
      setDevices(deviceItems);

      setSelectedDeviceId((current) => {
        const preferredDeviceId =
          (requestedDeviceId &&
            nextFanDevices.find((device) => device.id === requestedDeviceId)?.id) ||
          (current && nextFanDevices.find((device) => device.id === current)?.id) ||
          "";

        return preferredDeviceId;
      });

      if (nextFanDevices.length === 0) {
        setFanState(null);
        setFormPercent(defaultPercent);
      }
    } catch (err) {
      setError(toErrorMessage(err, "Fan devices could not be loaded"));
      setFanState(null);
      setFormPercent(defaultPercent);
    } finally {
      setLoading(false);
    }
  }, [requestedDeviceId]);

  useEffect(() => {
    void initialize();
  }, [initialize]);

  useEffect(() => {
    if (!selectedDeviceId) {
      setFanState(null);
      setFormPercent(defaultPercent);
      return;
    }

    void loadFanState(selectedDeviceId, {
      syncForm: true,
    });
  }, [loadFanState, selectedDeviceId]);

  const refresh = useCallback(async () => {
    setError(null);

    try {
      const deviceItems = await listDevices();
      const nextFanDevices = deviceItems.filter((device) => device.capabilities.has_fan);
      setDevices(deviceItems);

      const nextSelectedDeviceId =
        (requestedDeviceId &&
          nextFanDevices.find((device) => device.id === requestedDeviceId)?.id) ||
        (selectedDeviceId &&
          nextFanDevices.find((device) => device.id === selectedDeviceId)?.id) ||
        "";

      const selectionChanged = nextSelectedDeviceId !== selectedDeviceId;
      setSelectedDeviceId(nextSelectedDeviceId);

      if (nextFanDevices.length === 0) {
        setFanState(null);
        setFormPercent(defaultPercent);
      } else if (!selectionChanged && nextSelectedDeviceId) {
        await loadFanState(nextSelectedDeviceId, {
          background: true,
          syncForm: false,
        });
      }
    } catch (err) {
      setError(toErrorMessage(err, "Fan devices could not be loaded"));
    }
  }, [loadFanState, requestedDeviceId, selectedDeviceId]);

  useBackendEventSubscription(
    useCallback(
      (event) => {
        const isSelectedDeviceEvent =
          !event.device_id || event.device_id === selectedDeviceId;

        if (event.type === "daemon.connected" || event.type === "daemon.disconnected") {
          void refresh();
          return;
        }

        if (event.type === "device.updated") {
          if (!selectedDeviceId || isSelectedDeviceEvent) {
            void refresh();
          }
          return;
        }

        if (
          selectedDeviceId &&
          isSelectedDeviceEvent &&
          (event.type === "fan.changed" || event.type === "config.changed")
        ) {
          void loadFanState(selectedDeviceId, {
            background: true,
            syncForm: false,
          });
        }
      },
      [loadFanState, refresh, selectedDeviceId],
    ),
  );

  const applyChanges = useCallback(async () => {
    if (!selectedDeviceId) {
      setError("No fan-capable device is selected");
      return;
    }

    setSubmitting(true);
    setError(null);
    setSuccess(null);

    try {
      const nextState = await setManualFanSpeed(selectedDeviceId, {
        percent: clampPercent(formPercent),
      });
      const mergedState =
        nextState.rpms === null && fanState?.device_id === nextState.device_id
          ? {
              ...nextState,
              rpms: fanState.rpms,
            }
          : nextState;

      setFanState(mergedState);
      setFormPercent(seedPercentFromState(mergedState));
      setSuccess("Manual fan speed applied");
    } catch (err) {
      setError(toErrorMessage(err, "Fan changes could not be applied"));
    } finally {
      setSubmitting(false);
    }
  }, [fanState, formPercent, selectedDeviceId]);

  return {
    devices: fanDevices,
    selectedDeviceId,
    setSelectedDeviceId,
    fanState,
    formPercent,
    setFormPercent: (percent) => setFormPercent(clampPercent(percent)),
    loading,
    stateLoading,
    stateRefreshing,
    submitting,
    error,
    success,
    refresh,
    applyChanges,
  };
}
