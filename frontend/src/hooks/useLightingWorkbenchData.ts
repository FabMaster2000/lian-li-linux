import { useCallback, useEffect, useMemo, useState } from "react";
import type { Dispatch, SetStateAction } from "react";
import { useBackendEventSubscription } from "./useBackendEventSubscription";
import { listDevices } from "../services/devices";
import { getLightingState, setLightingEffect } from "../services/lighting";
import type { DeviceView, LightingStateResponse, LightingZoneState } from "../types/api";

export type LightingFormState = {
  zone: number;
  effect: string;
  color: string;
  brightness: number;
};

type LightingWorkbenchState = {
  devices: DeviceView[];
  selectedDeviceId: string;
  setSelectedDeviceId: (deviceId: string) => void;
  lightingState: LightingStateResponse | null;
  activeZone: LightingZoneState | null;
  form: LightingFormState;
  setForm: Dispatch<SetStateAction<LightingFormState>>;
  loading: boolean;
  stateLoading: boolean;
  stateRefreshing: boolean;
  submitting: boolean;
  error: string | null;
  success: string | null;
  refresh: () => Promise<void>;
  applyChanges: () => Promise<void>;
};

const defaultFormState: LightingFormState = {
  zone: 0,
  effect: "Static",
  color: "#ffffff",
  brightness: 100,
};

function toErrorMessage(error: unknown, fallback: string) {
  return error instanceof Error ? error.message : fallback;
}

function normalizeHexColor(input: string | undefined) {
  if (!input) {
    return "#ffffff";
  }

  const trimmed = input.trim();
  if (/^#[0-9a-fA-F]{6}$/.test(trimmed)) {
    return trimmed.toLowerCase();
  }

  return "#ffffff";
}

function formFromZone(zone: LightingZoneState | null): LightingFormState {
  if (!zone) {
    return defaultFormState;
  }

  return {
    zone: zone.zone,
    effect: zone.effect,
    color: normalizeHexColor(zone.colors[0]),
    brightness: zone.brightness_percent,
  };
}

type LoadLightingStateOptions = {
  syncForm?: boolean;
  background?: boolean;
  preferredZone?: number;
};

export function useLightingWorkbenchData(
  requestedDeviceId: string | null,
): LightingWorkbenchState {
  const [devices, setDevices] = useState<DeviceView[]>([]);
  const [selectedDeviceId, setSelectedDeviceId] = useState(requestedDeviceId ?? "");
  const [lightingState, setLightingState] = useState<LightingStateResponse | null>(null);
  const [form, setForm] = useState<LightingFormState>(defaultFormState);
  const [loading, setLoading] = useState(true);
  const [stateLoading, setStateLoading] = useState(false);
  const [stateRefreshing, setStateRefreshing] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  const rgbDevices = useMemo(
    () => devices.filter((device) => device.capabilities.has_rgb),
    [devices],
  );

  const activeZone = useMemo(
    () =>
      lightingState?.zones.find((zone) => zone.zone === form.zone) ??
      lightingState?.zones[0] ??
      null,
    [form.zone, lightingState],
  );

  const loadLightingState = useCallback(
    async (deviceId: string, options: LoadLightingStateOptions = {}) => {
      const { syncForm = true, background = false, preferredZone } = options;

      if (background) {
        setStateRefreshing(true);
      } else {
        setStateLoading(true);
        setError(null);
        setSuccess(null);

        if (syncForm) {
          setLightingState(null);
          setForm(defaultFormState);
        }
      }

      try {
        const nextState = await getLightingState(deviceId);
        const nextZone =
          nextState.zones.find((zone) => zone.zone === preferredZone) ?? nextState.zones[0] ?? null;

        setLightingState(nextState);

        if (syncForm) {
          setForm(formFromZone(nextZone));
        } else if (nextZone) {
          setForm((current) =>
            current.zone === nextZone.zone
              ? current
              : {
                  ...current,
                  zone: nextZone.zone,
                },
          );
        } else {
          setForm(defaultFormState);
        }

        setError(null);
      } catch (err) {
        if (!background) {
          setLightingState(null);
          setForm(defaultFormState);
        }

        setError(toErrorMessage(err, "Lighting state could not be loaded"));
      } finally {
        if (background) {
          setStateRefreshing(false);
        } else {
          setStateLoading(false);
        }
      }
    },
    [],
  );

  const initialize = useCallback(async () => {
    setLoading(true);
    setError(null);

    try {
      const deviceItems = await listDevices();
      const nextRgbDevices = deviceItems.filter((device) => device.capabilities.has_rgb);
      setDevices(deviceItems);

      setSelectedDeviceId((current) => {
        const preferredDeviceId =
          (requestedDeviceId &&
            nextRgbDevices.find((device) => device.id === requestedDeviceId)?.id) ||
          (current && nextRgbDevices.find((device) => device.id === current)?.id) ||
          "";

        return preferredDeviceId;
      });

      if (nextRgbDevices.length === 0) {
        setLightingState(null);
        setForm(defaultFormState);
      }
    } catch (err) {
      setError(toErrorMessage(err, "Lighting devices could not be loaded"));
      setLightingState(null);
      setForm(defaultFormState);
    } finally {
      setLoading(false);
    }
  }, [requestedDeviceId]);

  useEffect(() => {
    void initialize();
  }, [initialize]);

  useEffect(() => {
    if (!selectedDeviceId) {
      setLightingState(null);
      setForm(defaultFormState);
      return;
    }

    void loadLightingState(selectedDeviceId, {
      syncForm: true,
    });
  }, [loadLightingState, selectedDeviceId]);

  const refresh = useCallback(async () => {
    setError(null);

    try {
      const deviceItems = await listDevices();
      const nextRgbDevices = deviceItems.filter((device) => device.capabilities.has_rgb);
      setDevices(deviceItems);

      const nextSelectedDeviceId =
        (requestedDeviceId &&
          nextRgbDevices.find((device) => device.id === requestedDeviceId)?.id) ||
        (selectedDeviceId &&
          nextRgbDevices.find((device) => device.id === selectedDeviceId)?.id) ||
        "";

      const selectionChanged = nextSelectedDeviceId !== selectedDeviceId;
      setSelectedDeviceId(nextSelectedDeviceId);

      if (nextRgbDevices.length === 0) {
        setLightingState(null);
        setForm(defaultFormState);
      } else if (!selectionChanged && nextSelectedDeviceId) {
        await loadLightingState(nextSelectedDeviceId, {
          background: true,
          preferredZone: form.zone,
          syncForm: false,
        });
      }
    } catch (err) {
      setError(toErrorMessage(err, "Lighting devices could not be loaded"));
    }
  }, [form.zone, loadLightingState, requestedDeviceId, selectedDeviceId]);

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
          (event.type === "lighting.changed" || event.type === "config.changed")
        ) {
          void loadLightingState(selectedDeviceId, {
            background: true,
            preferredZone: form.zone,
            syncForm: false,
          });
        }
      },
      [form.zone, loadLightingState, refresh, selectedDeviceId],
    ),
  );

  const applyChanges = useCallback(async () => {
    if (!selectedDeviceId) {
      setError("No RGB-capable device is selected");
      return;
    }

    setSubmitting(true);
    setError(null);
    setSuccess(null);

    try {
      const nextState = await setLightingEffect(selectedDeviceId, {
        zone: form.zone,
        effect: form.effect,
        brightness: form.brightness,
        color: { hex: normalizeHexColor(form.color) },
      });

      setLightingState(nextState);
      const nextZone =
        nextState.zones.find((zone) => zone.zone === form.zone) ?? nextState.zones[0] ?? null;
      setForm(formFromZone(nextZone));
      setSuccess("Lighting state applied");
    } catch (err) {
      setError(toErrorMessage(err, "Lighting changes could not be applied"));
    } finally {
      setSubmitting(false);
    }
  }, [form, selectedDeviceId]);

  return {
    devices: rgbDevices,
    selectedDeviceId,
    setSelectedDeviceId,
    lightingState,
    activeZone,
    form,
    setForm,
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
