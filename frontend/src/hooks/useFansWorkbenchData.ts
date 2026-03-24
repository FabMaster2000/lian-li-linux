import { useCallback, useEffect, useMemo, useState } from "react";
import type { Dispatch, SetStateAction } from "react";
import {
  LIVE_STATUS_REFRESH_INTERVAL_MS,
  useBackgroundRefresh,
} from "./useBackgroundRefresh";
import { useBackendEventSubscription } from "./useBackendEventSubscription";
import { listDevices } from "../services/devices";
import {
  applyFanWorkbench,
  createFanCurve,
  deleteFanCurve,
  getFanState,
  listFanCurves,
  previewFanTemperatureSource,
  updateFanCurve,
} from "../services/fans";
import {
  clampFanPercent,
  controlDraftFromFanState,
  createBlankFanCurveDraft,
  createDuplicateFanCurveDraft,
  curveDraftFromCurve,
  defaultFanControlDraft,
  fanModeOptions,
  summarizeFanControlDraft,
  validateFanCurveDraft,
  type FanControlDraft,
  type FanCurveDraft,
  type FanCurveValidationIssue,
  type FanTargetMode,
} from "../features/fans";
import { isInventoryVisibleDevice } from "../features/wirelessSync";
import type {
  DeviceView,
  FanApplyResponse,
  FanCurveDocument,
  FanCurvePointDocument,
  FanTemperaturePreview,
  FanStateResponse,
} from "../types/api";

export type FansWorkbenchNotice = {
  id: string;
  tone: "warning" | "info" | "error";
  title: string;
  message: string;
};

export type FansWorkbenchState = {
  devices: DeviceView[];
  selectedDeviceId: string;
  setSelectedDeviceId: (deviceId: string) => void;
  selectedDeviceIds: string[];
  toggleSelectedDevice: (deviceId: string) => void;
  targetMode: FanTargetMode;
  setTargetMode: Dispatch<SetStateAction<FanTargetMode>>;
  fanState: FanStateResponse | null;
  curves: FanCurveDocument[];
  selectedCurveName: string;
  selectCurve: (name: string) => void;
  controlDraft: FanControlDraft;
  setControlDraft: Dispatch<SetStateAction<FanControlDraft>>;
  curveDraft: FanCurveDraft;
  setCurveDraft: Dispatch<SetStateAction<FanCurveDraft>>;
  temperaturePreview: FanTemperaturePreview | null;
  temperaturePreviewLoading: boolean;
  temperaturePreviewError: string | null;
  loading: boolean;
  stateLoading: boolean;
  stateRefreshing: boolean;
  submitting: boolean;
  savingCurve: boolean;
  deletingCurve: boolean;
  error: string | null;
  success: string | null;
  dirty: boolean;
  curveDirty: boolean;
  notices: FansWorkbenchNotice[];
  curveValidationIssues: FanCurveValidationIssue[];
  previewSummary: string;
  targetSummary: string;
  lastApplySummary: FanApplyResponse | null;
  refresh: () => Promise<void>;
  applyChanges: (overrideTargetMode?: FanTargetMode) => Promise<void>;
  resetDraft: () => void;
  restoreDefaults: () => void;
  newCurve: () => void;
  duplicateCurve: () => void;
  saveCurve: () => Promise<void>;
  deleteSelectedCurve: () => Promise<void>;
  addCurvePoint: () => void;
  updateCurvePoint: (
    index: number,
    field: keyof FanCurvePointDocument,
    value: number,
  ) => void;
  removeCurvePoint: (index: number) => void;
  modeOptions: typeof fanModeOptions;
};

function toErrorMessage(error: unknown, fallback: string) {
  return error instanceof Error ? error.message : fallback;
}

function summarizeFanUpdate(response: FanApplyResponse) {
  const applied = response.applied_devices.length;
  const skipped = response.skipped_devices.length;
  if (skipped > 0) {
    return `Applied fan settings to ${applied} device${applied === 1 ? "" : "s"}; ${skipped} target${skipped === 1 ? " was" : "s were"} skipped.`;
  }

  return `Applied fan settings to ${applied} device${applied === 1 ? "" : "s"}.`;
}

function sortCurves(curves: FanCurveDocument[]) {
  return [...curves].sort((left, right) => left.name.localeCompare(right.name));
}

function draftMatchesCurve(
  curveDraft: FanCurveDraft,
  curveName: string,
  curves: FanCurveDocument[],
) {
  const curve = curves.find((item) => item.name === curveName);
  if (!curve) {
    return false;
  }

  return JSON.stringify(curveDraft) === JSON.stringify(curveDraftFromCurve(curve));
}

export function useFansWorkbenchData(
  requestedDeviceId: string | null,
): FansWorkbenchState {
  const [devices, setDevices] = useState<DeviceView[]>([]);
  const [selectedDeviceId, setSelectedDeviceId] = useState(requestedDeviceId ?? "");
  const [selectedDeviceIds, setSelectedDeviceIds] = useState<string[]>(
    requestedDeviceId ? [requestedDeviceId] : [],
  );
  const [targetMode, setTargetMode] = useState<FanTargetMode>("single");
  const [fanState, setFanState] = useState<FanStateResponse | null>(null);
  const [curves, setCurves] = useState<FanCurveDocument[]>([]);
  const [selectedCurveName, setSelectedCurveName] = useState("");
  const [controlDraft, setControlDraft] = useState<FanControlDraft>(defaultFanControlDraft);
  const [baselineDraft, setBaselineDraft] = useState<FanControlDraft>(defaultFanControlDraft);
  const [curveDraft, setCurveDraft] = useState<FanCurveDraft>(createBlankFanCurveDraft([]));
  const [temperaturePreview, setTemperaturePreview] = useState<FanTemperaturePreview | null>(null);
  const [temperaturePreviewLoading, setTemperaturePreviewLoading] = useState(false);
  const [temperaturePreviewError, setTemperaturePreviewError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [stateLoading, setStateLoading] = useState(false);
  const [stateRefreshing, setStateRefreshing] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [savingCurve, setSavingCurve] = useState(false);
  const [deletingCurve, setDeletingCurve] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [lastApplySummary, setLastApplySummary] = useState<FanApplyResponse | null>(null);

  const fanDevices = useMemo(
    () => devices.filter((device) => device.capabilities.has_fan && isInventoryVisibleDevice(device)),
    [devices],
  );

  const selectedDevice = useMemo(
    () => fanDevices.find((device) => device.id === selectedDeviceId) ?? null,
    [fanDevices, selectedDeviceId],
  );

  const selectedCurve = useMemo(
    () => curves.find((curve) => curve.name === selectedCurveName) ?? null,
    [curves, selectedCurveName],
  );

  const targetDevices = useMemo(() => {
    if (targetMode === "all") {
      return fanDevices;
    }

    if (targetMode === "selected") {
      return fanDevices.filter((device) => selectedDeviceIds.includes(device.id));
    }

    return selectedDevice ? [selectedDevice] : [];
  }, [fanDevices, selectedDevice, selectedDeviceIds, targetMode]);

  const dirty = useMemo(
    () => JSON.stringify(controlDraft) !== JSON.stringify(baselineDraft),
    [baselineDraft, controlDraft],
  );

  const curveValidationIssues = useMemo(
    () => validateFanCurveDraft(curveDraft),
    [curveDraft],
  );

  const curveDirty = useMemo(() => {
    if (selectedCurve) {
      return JSON.stringify(curveDraft) !== JSON.stringify(curveDraftFromCurve(selectedCurve));
    }

    return JSON.stringify(curveDraft) !== JSON.stringify(createBlankFanCurveDraft(curves));
  }, [curveDraft, curves, selectedCurve]);

  const previewSummary = useMemo(() => summarizeFanControlDraft(controlDraft), [controlDraft]);

  const targetSummary = useMemo(() => {
    if (targetMode === "all") {
      return `Applying to all compatible fan devices (${targetDevices.length}).`;
    }

    if (targetMode === "selected") {
      return `Applying to ${targetDevices.length} selected device${targetDevices.length === 1 ? "" : "s"}.`;
    }

    return selectedDevice
      ? `Applying to ${selectedDevice.display_name}.`
      : "Choose a primary fan-capable device before applying changes.";
  }, [selectedDevice, targetDevices.length, targetMode]);

  const notices = useMemo(() => {
    const nextNotices: FansWorkbenchNotice[] = [];

    if (!selectedDeviceId) {
      nextNotices.push({
        id: "select-device",
        tone: "warning",
        title: "Primary device required",
        message:
          "Select a fan-capable device first. The workbench uses that device as the editing baseline and live telemetry source.",
      });
    }

    if (targetMode === "selected" && targetDevices.length === 0) {
      nextNotices.push({
        id: "selected-empty",
        tone: "warning",
        title: "No selected targets",
        message: "Choose at least one compatible target before applying in selected-devices mode.",
      });
    }

    if (controlDraft.mode === "manual" && controlDraft.manualPercent >= 85) {
      nextNotices.push({
        id: "high-manual",
        tone: "warning",
        title: "High fixed speed",
        message:
          "High manual percentages can increase noise and power draw. Double-check that a fixed speed is the right choice before applying broadly.",
      });
    }

    if (controlDraft.mode === "curve" && !controlDraft.curveName) {
      nextNotices.push({
        id: "curve-required",
        tone: "warning",
        title: "Curve selection required",
        message: "Choose or save a named curve before applying curve mode.",
      });
    }

    if (controlDraft.mode === "curve" && curveDirty && draftMatchesCurve(curveDraft, controlDraft.curveName, curves)) {
      nextNotices.push({
        id: "curve-unsaved",
        tone: "info",
        title: "Curve changes are not saved yet",
        message: "Save the curve draft before applying if you want the latest point edits to be used by curve mode.",
      });
    }

    if (controlDraft.mode === "mb_sync") {
      const unsupportedTargets = targetDevices.filter(
        (device) => !device.capabilities.mb_sync_support,
      );
      if (unsupportedTargets.length > 0) {
        nextNotices.push({
          id: "mb-sync-support",
          tone: "warning",
          title: "Motherboard sync will skip some targets",
          message: `${unsupportedTargets.length} target device${unsupportedTargets.length === 1 ? " does" : "s do"} not expose motherboard RPM sync support in the current backend model.`,
        });
      }

      nextNotices.push({
        id: "mb-sync-info",
        tone: "info",
        title: "External sync delegates speed control",
        message:
          "When motherboard sync is active, the web workbench documents the state but does not own the RPM decisions anymore.",
      });
    }

    if (controlDraft.mode === "start_stop") {
      nextNotices.push({
        id: "start-stop-unavailable",
        tone: "warning",
        title: "Start / stop is not available yet",
        message:
          "The backend currently reports start / stop as unsupported, so apply requests in this mode will be skipped rather than partially faked in the UI.",
      });
    }

    if (fanState?.mb_sync_enabled && controlDraft.mode !== "mb_sync") {
      nextNotices.push({
        id: "current-mb-sync",
        tone: "info",
        title: "Current live state uses motherboard sync",
        message:
          "The selected device currently reports motherboard sync as active. Applying manual or curve mode would replace that state when the backend accepts the request.",
      });
    }

    for (const issue of curveValidationIssues) {
      nextNotices.push({
        id: issue.id,
        tone: issue.tone,
        title: "Curve validation",
        message: issue.message,
      });
    }

    return nextNotices;
  }, [
    controlDraft.curveName,
    controlDraft.manualPercent,
    controlDraft.mode,
    curveDirty,
    curveDraft,
    curveValidationIssues,
    curves,
    fanState?.mb_sync_enabled,
    selectedDeviceId,
    targetDevices,
    targetMode,
  ]);

  const reconcileCurveSelection = useCallback(
    (nextCurves: FanCurveDocument[], syncDraft: boolean) => {
      const resolvedCurve =
        nextCurves.find((curve) => curve.name === selectedCurveName) ??
        nextCurves.find((curve) => curve.name === controlDraft.curveName) ??
        nextCurves[0] ??
        null;

      if (!resolvedCurve) {
        setSelectedCurveName("");
        if (syncDraft || !curveDirty) {
          setCurveDraft(createBlankFanCurveDraft(nextCurves));
        }
        return;
      }

      setSelectedCurveName(resolvedCurve.name);
        if (syncDraft || !curveDirty) {
          setCurveDraft(curveDraftFromCurve(resolvedCurve));
        }
    },
    [controlDraft.curveName, curveDirty, selectedCurveName],
  );

  const loadFanState = useCallback(
    async (
      deviceId: string,
      options: { syncDraft?: boolean; background?: boolean } = {},
    ) => {
      const { syncDraft = true, background = false } = options;

      if (background) {
        setStateRefreshing(true);
      } else {
        setStateLoading(true);
        setError(null);
        setSuccess(null);
      }

      try {
        const nextState = await getFanState(deviceId);
        setFanState(nextState);
        const nextBaseline = controlDraftFromFanState(nextState);
        setBaselineDraft(nextBaseline);

        if (syncDraft) {
          setControlDraft(nextBaseline);
          if (nextBaseline.curveName) {
            const matchingCurve = curves.find((curve) => curve.name === nextBaseline.curveName);
            if (matchingCurve) {
              setSelectedCurveName(matchingCurve.name);
              setCurveDraft(curveDraftFromCurve(matchingCurve));
            }
          }
        }

        setError(null);
      } catch (nextError) {
        if (!background) {
          setFanState(null);
          setBaselineDraft(defaultFanControlDraft);
          setControlDraft(defaultFanControlDraft);
        }

        setError(toErrorMessage(nextError, "Fan state could not be loaded"));
      } finally {
        if (background) {
          setStateRefreshing(false);
        } else {
          setStateLoading(false);
        }
      }
    },
    [curves],
  );

  const initialize = useCallback(async () => {
    setLoading(true);
    setError(null);

    try {
      const [deviceItems, curveItems] = await Promise.all([listDevices(), listFanCurves()]);
      const nextFanDevices = deviceItems.filter((device) => device.capabilities.has_fan);
      setDevices(deviceItems);
      setCurves(sortCurves(curveItems));

      let resolvedSelectedDeviceId = "";
      setSelectedDeviceId((current) => {
        resolvedSelectedDeviceId =
          (requestedDeviceId &&
            nextFanDevices.find((device) => device.id === requestedDeviceId)?.id) ||
          (current && nextFanDevices.find((device) => device.id === current)?.id) ||
          "";
        return resolvedSelectedDeviceId;
      });

      setSelectedDeviceIds((current) => {
        const allowed = current.filter((deviceId) =>
          nextFanDevices.some((device) => device.id === deviceId),
        );
        if (allowed.length > 0) {
          return allowed;
        }
        return resolvedSelectedDeviceId ? [resolvedSelectedDeviceId] : [];
      });

      reconcileCurveSelection(sortCurves(curveItems), true);

      if (!resolvedSelectedDeviceId) {
        setFanState(null);
        setBaselineDraft(defaultFanControlDraft);
        setControlDraft(defaultFanControlDraft);
      }
    } catch (nextError) {
      setError(toErrorMessage(nextError, "Fan workbench data could not be loaded"));
      setFanState(null);
      setBaselineDraft(defaultFanControlDraft);
      setControlDraft(defaultFanControlDraft);
    } finally {
      setLoading(false);
    }
  }, [reconcileCurveSelection, requestedDeviceId]);

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
      setFanState(null);
      setBaselineDraft(defaultFanControlDraft);
      setControlDraft(defaultFanControlDraft);
      return;
    }

    void loadFanState(selectedDeviceId, { syncDraft: true });
  }, [loadFanState, selectedDeviceId]);

  useEffect(() => {
    const source = curveDraft.temperature_source.trim();
    if (!source) {
      setTemperaturePreview(null);
      setTemperaturePreviewError(null);
      setTemperaturePreviewLoading(false);
      return;
    }

    let active = true;
    const timer = window.setTimeout(() => {
      setTemperaturePreviewLoading(true);
      void previewFanTemperatureSource(source)
        .then((preview) => {
          if (!active) {
            return;
          }
          setTemperaturePreview(preview);
          setTemperaturePreviewError(null);
        })
        .catch((nextError) => {
          if (!active) {
            return;
          }
          setTemperaturePreview(null);
          setTemperaturePreviewError(
            toErrorMessage(nextError, "Temperature preview could not be loaded"),
          );
        })
        .finally(() => {
          if (!active) {
            return;
          }
          setTemperaturePreviewLoading(false);
        });
    }, 180);

    return () => {
      active = false;
      window.clearTimeout(timer);
    };
  }, [curveDraft.temperature_source]);

  const refresh = useCallback(async () => {
    setError(null);

    try {
      const [deviceItems, curveItems] = await Promise.all([listDevices(), listFanCurves()]);
      const nextFanDevices = deviceItems.filter((device) => device.capabilities.has_fan);
      const sortedCurves = sortCurves(curveItems);
      setDevices(deviceItems);
      setCurves(sortedCurves);
      reconcileCurveSelection(sortedCurves, false);

      const nextSelectedDeviceId =
        (requestedDeviceId && nextFanDevices.find((device) => device.id === requestedDeviceId)?.id) ||
        (selectedDeviceId && nextFanDevices.find((device) => device.id === selectedDeviceId)?.id) ||
        "";

      const selectionChanged = nextSelectedDeviceId !== selectedDeviceId;
      setSelectedDeviceId(nextSelectedDeviceId);
      setSelectedDeviceIds((current) => {
        const allowed = current.filter((deviceId) =>
          nextFanDevices.some((device) => device.id === deviceId),
        );
        if (allowed.length > 0) {
          return allowed;
        }
        return nextSelectedDeviceId ? [nextSelectedDeviceId] : [];
      });

      if (!nextSelectedDeviceId) {
        setFanState(null);
        setBaselineDraft(defaultFanControlDraft);
        if (!dirty) {
          setControlDraft(defaultFanControlDraft);
        }
      } else if (!selectionChanged) {
        await loadFanState(nextSelectedDeviceId, {
          background: true,
          syncDraft: !dirty,
        });
      }
    } catch (nextError) {
      setError(toErrorMessage(nextError, "Fan workbench data could not be refreshed"));
    }
  }, [dirty, loadFanState, reconcileCurveSelection, requestedDeviceId, selectedDeviceId]);

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

        if (event.type === "config.changed") {
          void refresh();
          return;
        }

        if (selectedDeviceId && isSelectedDeviceEvent && event.type === "fan.changed") {
          void loadFanState(selectedDeviceId, {
            background: true,
            syncDraft: false,
          });
        }
      },
      [loadFanState, refresh, selectedDeviceId],
    ),
  );

  useBackgroundRefresh(refresh, LIVE_STATUS_REFRESH_INTERVAL_MS);

  const selectCurve = useCallback(
    (name: string) => {
      const curve = curves.find((item) => item.name === name);
      if (!curve) {
        return;
      }

      setSelectedCurveName(curve.name);
      setCurveDraft(curveDraftFromCurve(curve));
      setControlDraft((current) =>
        current.mode === "curve"
          ? {
              ...current,
              curveName: curve.name,
            }
          : current,
      );
    },
    [curves],
  );

  const toggleSelectedDevice = useCallback((deviceId: string) => {
    setSelectedDeviceIds((current) => {
      if (current.includes(deviceId)) {
        return current.filter((id) => id !== deviceId);
      }

      return [...current, deviceId];
    });
  }, []);

  const newCurve = useCallback(() => {
    setSelectedCurveName("");
    setCurveDraft(createBlankFanCurveDraft(curves));
    setControlDraft((current) =>
      current.mode === "curve"
        ? {
            ...current,
            curveName: "",
          }
        : current,
    );
  }, [curves]);

  const duplicateCurve = useCallback(() => {
    setSelectedCurveName("");
    setCurveDraft((current) => createDuplicateFanCurveDraft(current, curves));
    setControlDraft((current) =>
      current.mode === "curve"
        ? {
            ...current,
            curveName: "",
          }
        : current,
    );
  }, [curves]);

  const saveCurve = useCallback(async () => {
    const blockingIssue = curveValidationIssues.find((issue) => issue.tone === "error");
    if (blockingIssue) {
      setError(blockingIssue.message);
      return;
    }

    const payload = {
      name: curveDraft.name.trim(),
      temperature_source: curveDraft.temperature_source.trim(),
      points: curveDraft.points.map((point) => ({
        temperature_celsius: Math.round(point.temperature_celsius * 10) / 10,
        percent: clampFanPercent(point.percent),
      })),
    };

    const existingCurve = curves.find((curve) => curve.name === selectedCurveName) ?? null;

    setSavingCurve(true);
    setError(null);
    setSuccess(null);

    try {
      const savedCurve = existingCurve
        ? await updateFanCurve(existingCurve.name, payload)
        : await createFanCurve(payload);

      setCurves((current) => {
        const withoutPrevious = existingCurve
          ? current.filter((curve) => curve.name !== existingCurve.name)
          : current;
        return sortCurves([...withoutPrevious, savedCurve]);
      });
      setSelectedCurveName(savedCurve.name);
      setCurveDraft(curveDraftFromCurve(savedCurve));
      setControlDraft((current) => {
        if (current.mode !== "curve" && current.curveName !== existingCurve?.name) {
          return current;
        }

        return {
          ...current,
          curveName: savedCurve.name,
        };
      });
      setSuccess(existingCurve ? "Fan curve updated" : "Fan curve created");
    } catch (nextError) {
      setError(toErrorMessage(nextError, "Fan curve could not be saved"));
    } finally {
      setSavingCurve(false);
    }
  }, [curveDraft, curveValidationIssues, curves, selectedCurveName]);

  const deleteSelectedCurve = useCallback(async () => {
    if (!selectedCurveName) {
      return;
    }

    setDeletingCurve(true);
    setError(null);
    setSuccess(null);

    try {
      await deleteFanCurve(selectedCurveName);
      const nextCurves = curves.filter((curve) => curve.name !== selectedCurveName);
      const nextSelectedCurve = nextCurves[0] ?? null;
      setCurves(nextCurves);
      setSelectedCurveName(nextSelectedCurve?.name ?? "");
      setCurveDraft(
        nextSelectedCurve
          ? curveDraftFromCurve(nextSelectedCurve)
          : createBlankFanCurveDraft(nextCurves),
      );
      setControlDraft((current) =>
        current.curveName === selectedCurveName
          ? {
              ...current,
              curveName: nextSelectedCurve?.name ?? "",
            }
          : current,
      );
      setSuccess("Fan curve deleted");
    } catch (nextError) {
      setError(toErrorMessage(nextError, "Fan curve could not be deleted"));
    } finally {
      setDeletingCurve(false);
    }
  }, [curves, selectedCurveName]);

  const applyChanges = useCallback(
    async (overrideTargetMode?: FanTargetMode) => {
      const effectiveTargetMode = overrideTargetMode ?? targetMode;

      if (!selectedDeviceId) {
        setError("No fan-capable primary device is selected");
        return;
      }

      if (effectiveTargetMode === "selected" && selectedDeviceIds.length === 0) {
        setError("Select at least one target device before applying selected-device fan changes");
        return;
      }

      if (controlDraft.mode === "curve" && !controlDraft.curveName) {
        setError("Choose or save a fan curve before applying curve mode");
        return;
      }

      if (
        controlDraft.mode === "curve" &&
        curveDirty &&
        draftMatchesCurve(curveDraft, controlDraft.curveName, curves)
      ) {
        setError("Save the curve draft before applying curve mode");
        return;
      }

      setSubmitting(true);
      setError(null);
      setSuccess(null);

      try {
        const response = await applyFanWorkbench({
          target_mode: effectiveTargetMode,
          device_id: selectedDeviceId,
          device_ids: selectedDeviceIds,
          mode: controlDraft.mode,
          percent: controlDraft.mode === "manual" ? clampFanPercent(controlDraft.manualPercent) : undefined,
          curve: controlDraft.mode === "curve" ? controlDraft.curveName : undefined,
        });

        setLastApplySummary(response);
        const nextSelectedState =
          response.applied_devices.find((device) => device.device_id === selectedDeviceId) ?? null;
        if (nextSelectedState) {
          setFanState(nextSelectedState);
          setBaselineDraft(controlDraftFromFanState(nextSelectedState));
        } else {
          setBaselineDraft(controlDraft);
        }
        setSuccess(summarizeFanUpdate(response));
      } catch (nextError) {
        setError(toErrorMessage(nextError, "Fan changes could not be applied"));
      } finally {
        setSubmitting(false);
      }
    },
    [
      controlDraft,
      curves,
      curveDirty,
      curveDraft,
      selectedDeviceId,
      selectedDeviceIds,
      targetMode,
    ],
  );

  const resetDraft = useCallback(() => {
    setControlDraft(baselineDraft);
    if (baselineDraft.curveName) {
      const baselineCurve = curves.find((curve) => curve.name === baselineDraft.curveName);
      if (baselineCurve) {
        setSelectedCurveName(baselineCurve.name);
        setCurveDraft(curveDraftFromCurve(baselineCurve));
      }
    }
  }, [baselineDraft, curves]);

  const restoreDefaults = useCallback(() => {
    setControlDraft(defaultFanControlDraft);
    setSuccess(null);
  }, []);

  const addCurvePoint = useCallback(() => {
    setCurveDraft((current) => {
      const lastPoint = current.points[current.points.length - 1];
      const nextPoint = lastPoint
        ? {
            temperature_celsius: lastPoint.temperature_celsius + 8,
            percent: clampFanPercent(lastPoint.percent + 10),
          }
        : {
            temperature_celsius: 40,
            percent: 50,
          };

      return {
        ...current,
        points: [...current.points, nextPoint],
      };
    });
  }, []);

  const updateCurvePoint = useCallback(
    (index: number, field: keyof FanCurvePointDocument, value: number) => {
      setCurveDraft((current) => ({
        ...current,
        points: current.points.map((point, pointIndex) =>
          pointIndex === index
            ? {
                ...point,
                [field]: field === "percent" ? clampFanPercent(value) : value,
              }
            : point,
        ),
      }));
    },
    [],
  );

  const removeCurvePoint = useCallback((index: number) => {
    setCurveDraft((current) => ({
      ...current,
      points: current.points.filter((_, pointIndex) => pointIndex !== index),
    }));
  }, []);

  return {
    devices: fanDevices,
    selectedDeviceId,
    setSelectedDeviceId,
    selectedDeviceIds,
    toggleSelectedDevice,
    targetMode,
    setTargetMode,
    fanState,
    curves,
    selectedCurveName,
    selectCurve,
    controlDraft,
    setControlDraft,
    curveDraft,
    setCurveDraft,
    temperaturePreview,
    temperaturePreviewLoading,
    temperaturePreviewError,
    loading,
    stateLoading,
    stateRefreshing,
    submitting,
    savingCurve,
    deletingCurve,
    error,
    success,
    dirty,
    curveDirty,
    notices,
    curveValidationIssues,
    previewSummary,
    targetSummary,
    lastApplySummary,
    refresh,
    applyChanges,
    resetDraft,
    restoreDefaults,
    newCurve,
    duplicateCurve,
    saveCurve,
    deleteSelectedCurve,
    addCurvePoint,
    updateCurvePoint,
    removeCurvePoint,
    modeOptions: fanModeOptions,
  };
}



