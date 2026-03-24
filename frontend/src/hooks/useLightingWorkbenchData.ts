import { useCallback, useEffect, useMemo, useState } from "react";
import type { Dispatch, SetStateAction } from "react";
import {
  LIVE_STATUS_REFRESH_INTERVAL_MS,
  useBackgroundRefresh,
} from "./useBackgroundRefresh";
import { useBackendEventSubscription } from "./useBackendEventSubscription";
import { listDevices } from "../services/devices";
import { applyLightingWorkbench, getLightingState } from "../services/lighting";
import {
  findLightingPreset,
  getLightingEffectDefinition,
  lightingPresetCatalog,
  type LightingPreset,
} from "../features/lighting";
import { isInventoryVisibleDevice } from "../features/wirelessSync";
import type {
  DeviceView,
  LightingApplyResponse,
  LightingStateResponse,
  LightingZoneState,
} from "../types/api";

export type LightingTargetMode = "single" | "selected" | "all";
export type LightingZoneMode = "active" | "all_zones";

export type LightingDraftState = {
  zone: number;
  zoneMode: LightingZoneMode;
  effect: string;
  colors: string[];
  brightness: number;
  speed: number;
  direction: string;
  scope: string;
};

export type LightingNotice = {
  id: string;
  tone: "warning" | "info";
  title: string;
  message: string;
};

type CustomLightingPreset = LightingPreset & {
  source: "custom";
};

type LightingWorkbenchState = {
  devices: DeviceView[];
  selectedDeviceId: string;
  setSelectedDeviceId: (deviceId: string) => void;
  selectedDeviceIds: string[];
  toggleSelectedDevice: (deviceId: string) => void;
  targetMode: LightingTargetMode;
  setTargetMode: Dispatch<SetStateAction<LightingTargetMode>>;
  syncSelected: boolean;
  setSyncSelected: Dispatch<SetStateAction<boolean>>;
  lightingState: LightingStateResponse | null;
  activeZone: LightingZoneState | null;
  draft: LightingDraftState;
  setDraft: Dispatch<SetStateAction<LightingDraftState>>;
  loading: boolean;
  stateLoading: boolean;
  stateRefreshing: boolean;
  submitting: boolean;
  error: string | null;
  success: string | null;
  dirty: boolean;
  preserveMultiTargetScope: boolean;
  notices: LightingNotice[];
  previewSummary: string;
  targetSummary: string;
  lastApplySummary: LightingApplyResponse | null;
  builtInPresets: LightingPreset[];
  customPresets: CustomLightingPreset[];
  presetName: string;
  setPresetName: Dispatch<SetStateAction<string>>;
  applyPreset: (presetId: string) => void;
  saveCurrentPreset: () => void;
  refresh: () => Promise<void>;
  applyChanges: (overrideTargetMode?: LightingTargetMode) => Promise<void>;
  resetDraft: () => void;
};

const customPresetStorageKey = "lighting-workbench-presets-v1";

const defaultDraftState: LightingDraftState = {
  zone: 0,
  zoneMode: "active",
  effect: "Static",
  colors: ["#ffffff"],
  brightness: 100,
  speed: 2,
  direction: "Clockwise",
  scope: "All",
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

function normalizePalette(colors: string[]) {
  const palette = colors
    .map((color) => normalizeHexColor(color))
    .filter((color, index, items) => items.indexOf(color) === index)
    .slice(0, 4);

  return palette.length > 0 ? palette : ["#ffffff"];
}

function draftFromZone(zone: LightingZoneState | null): LightingDraftState {
  if (!zone) {
    return defaultDraftState;
  }

  return {
    zone: zone.zone,
    zoneMode: "active",
    effect: zone.effect,
    colors: normalizePalette(zone.colors),
    brightness: zone.brightness_percent,
    speed: zone.speed,
    direction: zone.direction,
    scope: zone.scope,
  };
}

function applyZoneState(
  state: LightingStateResponse | null,
  preferredZone: number,
): LightingZoneState | null {
  return state?.zones.find((zone) => zone.zone === preferredZone) ?? state?.zones[0] ?? null;
}
function isWirelessSlInf(device: DeviceView | null | undefined) {
  return device?.family === "SlInf" && device.wireless !== null;
}

function loadCustomPresets(): CustomLightingPreset[] {
  try {
    const raw = window.localStorage.getItem(customPresetStorageKey);
    if (!raw) {
      return [];
    }

    const parsed = JSON.parse(raw) as CustomLightingPreset[];
    if (!Array.isArray(parsed)) {
      return [];
    }

    return parsed.map((preset) => ({
      ...preset,
      source: "custom",
      colors: normalizePalette(preset.colors ?? []),
      brightness: Math.max(0, Math.min(100, preset.brightness ?? 100)),
      speed: Math.max(0, Math.min(20, preset.speed ?? 2)),
      direction: preset.direction ?? "Clockwise",
      scope: preset.scope ?? "All",
    }));
  } catch {
    return [];
  }
}

function saveCustomPresetsToStorage(presets: CustomLightingPreset[]) {
  window.localStorage.setItem(customPresetStorageKey, JSON.stringify(presets));
}

function summarizeDraft(draft: LightingDraftState) {
  return `${draft.effect} • ${draft.brightness}% • ${draft.colors.length} color${draft.colors.length === 1 ? "" : "s"} • speed ${draft.speed}`;
}

export function useLightingWorkbenchData(
  requestedDeviceId: string | null,
): LightingWorkbenchState {
  const [devices, setDevices] = useState<DeviceView[]>([]);
  const [selectedDeviceId, setSelectedDeviceId] = useState(requestedDeviceId ?? "");
  const [selectedDeviceIds, setSelectedDeviceIds] = useState<string[]>(
    requestedDeviceId ? [requestedDeviceId] : [],
  );
  const [targetMode, setTargetMode] = useState<LightingTargetMode>("single");
  const [syncSelected, setSyncSelected] = useState(false);
  const [lightingState, setLightingState] = useState<LightingStateResponse | null>(null);
  const [draft, setDraft] = useState<LightingDraftState>(defaultDraftState);
  const [baselineDraft, setBaselineDraft] = useState<LightingDraftState>(defaultDraftState);
  const [loading, setLoading] = useState(true);
  const [stateLoading, setStateLoading] = useState(false);
  const [stateRefreshing, setStateRefreshing] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [lastApplySummary, setLastApplySummary] = useState<LightingApplyResponse | null>(null);
  const [customPresets, setCustomPresets] = useState<CustomLightingPreset[]>([]);
  const [presetName, setPresetName] = useState("");

  const rgbDevices = useMemo(
    () => devices.filter((device) => device.capabilities.has_rgb && isInventoryVisibleDevice(device)),
    [devices],
  );

  const selectedDevice = useMemo(
    () => rgbDevices.find((device) => device.id === selectedDeviceId) ?? null,
    [rgbDevices, selectedDeviceId],
  );

  const activeZone = useMemo(
    () => applyZoneState(lightingState, draft.zone),
    [draft.zone, lightingState],
  );

  const targetDevices = useMemo(() => {
    if (targetMode === "all") {
      return rgbDevices;
    }

    if (targetMode === "selected") {
      return rgbDevices.filter((device) => selectedDeviceIds.includes(device.id));
    }

    return selectedDevice ? [selectedDevice] : [];
  }, [rgbDevices, selectedDevice, selectedDeviceIds, targetMode]);

  const dirty = useMemo(
    () => JSON.stringify(draft) !== JSON.stringify(baselineDraft),
    [baselineDraft, draft],
  );
  const preserveMultiTargetScope = useMemo(
    () => targetDevices.length > 1 && targetDevices.some((device) => device.wireless !== null),
    [targetDevices],
  );
  const scopeLockedByDeviceModel = useMemo(
    () => targetDevices.some((device) => isWirelessSlInf(device)),
    [targetDevices],
  );

  const previewSummary = useMemo(() => {
    const scopeLabel = draft.zoneMode === "all_zones" ? "all zones" : `zone ${draft.zone}`;
    return `${summarizeDraft(draft)} • ${scopeLabel}`;
  }, [draft]);

  const targetSummary = useMemo(() => {
    if (targetMode === "all") {
      return `Applying to all compatible RGB devices (${targetDevices.length}).`;
    }

    if (targetMode === "selected") {
      return `Applying to ${targetDevices.length} selected device${targetDevices.length === 1 ? "" : "s"}.`;
    }

    return selectedDevice
      ? `Applying to ${selectedDevice.display_name}.`
      : "Choose a primary device to build the current lighting draft.";
  }, [selectedDevice, targetDevices.length, targetMode]);

  const notices = useMemo(() => {
    const nextNotices: LightingNotice[] = [];
    const effectDefinition = getLightingEffectDefinition(draft.effect);

    if (!selectedDeviceId) {
      nextNotices.push({
        id: "select-device",
        tone: "warning",
        title: "Primary device required",
        message:
          "Select an RGB-capable device first. The workbench uses that device as the editing baseline and live preview source.",
      });
    }

    if (targetMode === "selected" && targetDevices.length === 0) {
      nextNotices.push({
        id: "selected-empty",
        tone: "warning",
        title: "No selected targets",
        message: "Choose at least one target device before applying in selected-devices mode.",
      });
    }

    if (effectDefinition.pumpOnly) {
      const skipped = targetDevices.filter((device) => !device.capabilities.has_pump).length;
      if (skipped > 0) {
        nextNotices.push({
          id: "pump-only",
          tone: "warning",
          title: "Pump-only effect",
          message: `${skipped} targeted device${skipped === 1 ? " will" : "s will"} be skipped because ${effectDefinition.label} is limited to pump-capable devices in the current backend model.`,
        });
      }
    }

    if (effectDefinition.paletteMode === "none" && draft.colors.length > 1) {
      nextNotices.push({
        id: "palette-ignored",
        tone: "info",
        title: "Palette not used by this effect",
        message: "This effect keeps the extra palette values in the draft, but current backend handling is likely to ignore them.",
      });
    }

    if (draft.zoneMode === "all_zones") {
      nextNotices.push({
        id: "all-zones",
        tone: "info",
        title: "All-zones apply",
        message: "The current draft will be written to every known zone on each targeted device during apply.",
      });
    }

    if (preserveMultiTargetScope && effectDefinition.supportsScope) {
      nextNotices.push({
        id: "preserve-target-scope",
        tone: "info",
        title: "Per-device scope preserved",
        message:
          "Multi-device wireless applies keep each target's current scope so mixed clusters still react reliably. Use single-device mode if you want to change scope explicitly.",
      });
    }

    if (scopeLockedByDeviceModel && effectDefinition.supportsScope) {
      nextNotices.push({
        id: "wireless-cluster-zones",
        tone: "info",
        title: "Cluster zones active",
        message:
          "SL-INF wireless clusters expose direct lighting zones. Scope writes stay locked to All so Top and Bottom do not override the wrong layer.",
      });
    }

    if (syncSelected && targetMode === "selected") {
      nextNotices.push({
        id: "sync-selected",
        tone: "info",
        title: "One-time sync apply",
        message: "Sync selected devices currently means one identical config write across the current selection, not a persistent future linkage.",
      });
    }

    return nextNotices;
  }, [draft.colors.length, draft.effect, draft.zone, draft.zoneMode, preserveMultiTargetScope, scopeLockedByDeviceModel, selectedDeviceId, syncSelected, targetDevices, targetMode]);

  useEffect(() => {
    if (!scopeLockedByDeviceModel || draft.scope === "All") {
      return;
    }

    setDraft((current) => ({
      ...current,
      scope: "All",
    }));
  }, [draft.scope, scopeLockedByDeviceModel]);

  const loadLightingState = useCallback(
    async (
      deviceId: string,
      options: { syncDraft?: boolean; background?: boolean; preferredZone?: number } = {},
    ) => {
      const { syncDraft = true, background = false, preferredZone } = options;

      if (background) {
        setStateRefreshing(true);
      } else {
        setStateLoading(true);
        setError(null);
        setSuccess(null);
      }

      try {
        const nextState = await getLightingState(deviceId);
        setLightingState(nextState);
        const nextZone = applyZoneState(nextState, preferredZone ?? draft.zone);
        const nextBaseline = draftFromZone(nextZone);
        setBaselineDraft(nextBaseline);

        if (syncDraft) {
          setDraft(nextBaseline);
        } else if (nextZone) {
          setDraft((current) =>
            current.zone === nextZone.zone
              ? current
              : {
                  ...current,
                  zone: nextZone.zone,
                },
          );
        }

        setError(null);
      } catch (nextError) {
        if (!background) {
          setLightingState(null);
          setBaselineDraft(defaultDraftState);
          setDraft(defaultDraftState);
        }

        setError(toErrorMessage(nextError, "Lighting state could not be loaded"));
      } finally {
        if (background) {
          setStateRefreshing(false);
        } else {
          setStateLoading(false);
        }
      }
    },
    [draft.zone],
  );

  const initialize = useCallback(async () => {
    setLoading(true);
    setError(null);

    try {
      const deviceItems = await listDevices();
      const nextRgbDevices = deviceItems.filter((device) => device.capabilities.has_rgb);
      setDevices(deviceItems);
      setCustomPresets(loadCustomPresets());

      let resolvedSelectedDeviceId = "";
      setSelectedDeviceId((current) => {
        resolvedSelectedDeviceId =
          (requestedDeviceId && nextRgbDevices.find((device) => device.id === requestedDeviceId)?.id) ||
          (current && nextRgbDevices.find((device) => device.id === current)?.id) ||
          "";
        return resolvedSelectedDeviceId;
      });
      setSelectedDeviceIds((current) => {
        const allowed = current.filter((deviceId) => nextRgbDevices.some((device) => device.id === deviceId));
        if (allowed.length > 0) {
          return allowed;
        }
        return resolvedSelectedDeviceId ? [resolvedSelectedDeviceId] : [];
      });

      if (!resolvedSelectedDeviceId) {
        setLightingState(null);
        setBaselineDraft(defaultDraftState);
        setDraft(defaultDraftState);
      }
    } catch (nextError) {
      setError(toErrorMessage(nextError, "Lighting devices could not be loaded"));
      setLightingState(null);
      setBaselineDraft(defaultDraftState);
      setDraft(defaultDraftState);
    } finally {
      setLoading(false);
    }
  }, [requestedDeviceId]);

  useEffect(() => {
    void initialize();
  }, [initialize]);

  useEffect(() => {
    if (targetMode === "selected" && selectedDeviceId && !selectedDeviceIds.includes(selectedDeviceId)) {
      setSelectedDeviceIds((current) => [...current, selectedDeviceId]);
    }
  }, [selectedDeviceId, selectedDeviceIds, targetMode]);

  useEffect(() => {
    if (!selectedDeviceId) {
      setLightingState(null);
      setBaselineDraft(defaultDraftState);
      setDraft(defaultDraftState);
      return;
    }

    void loadLightingState(selectedDeviceId, { syncDraft: true, preferredZone: draft.zone });
  }, [loadLightingState, selectedDeviceId]);

  useEffect(() => {
    saveCustomPresetsToStorage(customPresets);
  }, [customPresets]);

  const refresh = useCallback(async () => {
    setError(null);

    try {
      const deviceItems = await listDevices();
      const nextRgbDevices = deviceItems.filter((device) => device.capabilities.has_rgb);
      setDevices(deviceItems);
      setSelectedDeviceIds((current) =>
        current.filter((deviceId) => nextRgbDevices.some((device) => device.id === deviceId)),
      );

      const nextSelectedDeviceId =
        (requestedDeviceId && nextRgbDevices.find((device) => device.id === requestedDeviceId)?.id) ||
        (selectedDeviceId && nextRgbDevices.find((device) => device.id === selectedDeviceId)?.id) ||
        "";

      const selectionChanged = nextSelectedDeviceId !== selectedDeviceId;
      setSelectedDeviceId(nextSelectedDeviceId);

      if (!nextSelectedDeviceId) {
        setLightingState(null);
        setBaselineDraft(defaultDraftState);
        setDraft(defaultDraftState);
      } else if (!selectionChanged) {
        await loadLightingState(nextSelectedDeviceId, {
          background: true,
          preferredZone: draft.zone,
          syncDraft: false,
        });
      }
    } catch (nextError) {
      setError(toErrorMessage(nextError, "Lighting devices could not be loaded"));
    }
  }, [draft.zone, loadLightingState, requestedDeviceId, selectedDeviceId]);

  useBackendEventSubscription(
    useCallback(
      (event) => {
        const isSelectedDeviceEvent = !event.device_id || event.device_id === selectedDeviceId;

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
            preferredZone: draft.zone,
            syncDraft: false,
          });
        }
      },
      [draft.zone, loadLightingState, refresh, selectedDeviceId],
    ),
  );

  useBackgroundRefresh(refresh, LIVE_STATUS_REFRESH_INTERVAL_MS);

  const toggleSelectedDevice = useCallback((deviceId: string) => {
    setSelectedDeviceIds((current) =>
      current.includes(deviceId)
        ? current.filter((item) => item !== deviceId)
        : [...current, deviceId],
    );
  }, []);

  const applyPreset = useCallback((presetId: string) => {
    const builtinPreset = findLightingPreset(presetId);
    const customPreset = customPresets.find((preset) => preset.id === presetId) ?? null;
    const preset = builtinPreset ?? customPreset;
    if (!preset) {
      return;
    }

    setDraft((current) => ({
      ...current,
      effect: preset.effect,
      colors: normalizePalette(preset.colors),
      brightness: preset.brightness,
      speed: preset.speed,
      direction: preset.direction,
      scope: preset.scope,
    }));
    setSuccess(`Loaded preset ${preset.label}.`);
    setError(null);
  }, [customPresets]);

  const saveCurrentPreset = useCallback(() => {
    const label = presetName.trim();
    if (!label) {
      setError("Preset name is required");
      return;
    }

    const preset: CustomLightingPreset = {
      id: `custom-${Date.now()}`,
      label,
      description: `Saved from the lighting workbench on ${new Date().toISOString()}.`,
      effect: draft.effect,
      colors: normalizePalette(draft.colors),
      brightness: draft.brightness,
      speed: draft.speed,
      direction: draft.direction,
      scope: draft.scope,
      source: "custom",
    };

    setCustomPresets((current) => [preset, ...current].slice(0, 12));
    setPresetName("");
    setSuccess(`Saved preset ${preset.label}.`);
    setError(null);
  }, [draft, presetName]);

  const resetDraft = useCallback(() => {
    setDraft(baselineDraft);
    setError(null);
    setSuccess("Reverted lighting draft to the last confirmed backend state.");
  }, [baselineDraft]);

  const applyChanges = useCallback(
    async (overrideTargetMode?: LightingTargetMode) => {
      if (!selectedDeviceId) {
        setError("No RGB-capable device is selected");
        return;
      }

      const effectiveTargetMode = overrideTargetMode ?? targetMode;
      const effectiveSelectedIds =
        effectiveTargetMode === "selected"
          ? selectedDeviceIds.filter((deviceId) => rgbDevices.some((device) => device.id === deviceId))
          : [];

      if (effectiveTargetMode === "selected" && effectiveSelectedIds.length === 0) {
        setError("Choose at least one selected target before applying lighting");
        return;
      }

      setSubmitting(true);
      setError(null);
      setSuccess(null);
      setLastApplySummary(null);

      try {
        const response = await applyLightingWorkbench({
          target_mode: effectiveTargetMode,
          device_id: selectedDeviceId,
          device_ids: effectiveSelectedIds,
          zone_mode: draft.zoneMode,
          zone: draft.zone,
          sync_selected: effectiveTargetMode === "selected" && syncSelected,
          effect: draft.effect,
          brightness: draft.brightness,
          speed: draft.speed,
          colors: normalizePalette(draft.colors).map((color) => ({ hex: color })),
          direction: draft.direction,
          scope: preserveMultiTargetScope ? null : scopeLockedByDeviceModel ? "All" : draft.scope,
        });

        setLastApplySummary(response);
        const selectedDeviceState =
          response.applied_devices.find((device) => device.device_id === selectedDeviceId) ?? null;
        if (selectedDeviceState) {
          const nextLightingState: LightingStateResponse = {
            device_id: selectedDeviceState.device_id,
            zones: selectedDeviceState.zones,
          };
          setLightingState(nextLightingState);
          const nextBaseline = draftFromZone(applyZoneState(nextLightingState, draft.zone));
          setBaselineDraft(nextBaseline);
          setDraft(nextBaseline);
        }

        if (response.applied_devices.length === 0) {
          setError("No compatible lighting targets could be updated.");
        } else if (response.skipped_devices.length > 0) {
          setSuccess(
            `Applied lighting to ${response.applied_devices.length} device${response.applied_devices.length === 1 ? "" : "s"}; ${response.skipped_devices.length} target${response.skipped_devices.length === 1 ? " was" : "s were"} skipped.`,
          );
        } else {
          setSuccess(
            `Applied lighting to ${response.applied_devices.length} device${response.applied_devices.length === 1 ? "" : "s"}.`,
          );
        }
      } catch (nextError) {
        setError(toErrorMessage(nextError, "Lighting changes could not be applied"));
      } finally {
        setSubmitting(false);
      }
    },
    [draft, preserveMultiTargetScope, rgbDevices, scopeLockedByDeviceModel, selectedDeviceId, selectedDeviceIds, syncSelected, targetMode],
  );

  return {
    devices: rgbDevices,
    selectedDeviceId,
    setSelectedDeviceId,
    selectedDeviceIds,
    toggleSelectedDevice,
    targetMode,
    setTargetMode,
    syncSelected,
    setSyncSelected,
    lightingState,
    activeZone,
    draft,
    setDraft,
    loading,
    stateLoading,
    stateRefreshing,
    submitting,
    error,
    success,
    dirty,
    preserveMultiTargetScope,
    notices,
    previewSummary,
    targetSummary,
    lastApplySummary,
    builtInPresets: lightingPresetCatalog,
    customPresets,
    presetName,
    setPresetName,
    applyPreset,
    saveCurrentPreset,
    refresh,
    applyChanges,
    resetDraft,
  };
}



