import { useCallback, useEffect, useMemo, useState } from "react";
import type { FanCurvePointDocument, FanStateResponse } from "../types/api";
import {
  LIVE_STATUS_REFRESH_INTERVAL_MS,
  useBackgroundRefresh,
} from "./useBackgroundRefresh";
import { useBackendEventSubscription } from "./useBackendEventSubscription";
import { listDevices } from "../services/devices";
import {
  applyFanWorkbench,
  createFanCurve,
  getFanState,
  listFanCurves,
  updateFanCurve,
} from "../services/fans";
import {
  buildMvpFanCurveName,
  buildMvpFanCurvePoints,
  buildPairedClusters,
  clampPercent,
  draftFromFanState,
  normalizeFanCurveSource,
  resolveRequestedClusterId,
  type FanCurveSource,
  type MvpCluster,
} from "../features/mvpClusters";

type FanMvpDraft = {
  mode: "manual" | "curve";
  manualPercent: number;
  curveSource: FanCurveSource;
  points: FanCurvePointDocument[];
};

const defaultDraft: FanMvpDraft = {
  mode: "manual",
  manualPercent: 50,
  curveSource: "cpu",
  points: buildMvpFanCurvePoints(null),
};

function sortCurvePoints(points: FanCurvePointDocument[]) {
  return [...points]
    .map((point) => ({
      temperature_celsius: Math.round(point.temperature_celsius * 10) / 10,
      percent: clampPercent(point.percent),
    }))
    .sort((left, right) => left.temperature_celsius - right.temperature_celsius);
}

function toErrorMessage(error: unknown, fallback: string) {
  return error instanceof Error ? error.message : fallback;
}

export function useMvpFansPageData(
  requestedClusterId: string | null,
  requestedDeviceId: string | null,
) {
  const [clusters, setClusters] = useState<MvpCluster[]>([]);
  const [selectedClusterId, setSelectedClusterId] = useState("");
  const [fanState, setFanState] = useState<FanStateResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [stateLoading, setStateLoading] = useState(false);
  const [stateRefreshing, setStateRefreshing] = useState(false);
  const [applying, setApplying] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [draft, setDraft] = useState<FanMvpDraft>(defaultDraft);
  const [baseline, setBaseline] = useState<FanMvpDraft>(defaultDraft);

  const selectedCluster = useMemo(
    () => clusters.find((cluster) => cluster.id === selectedClusterId) ?? null,
    [clusters, selectedClusterId],
  );

  const dirty = useMemo(
    () => JSON.stringify(draft) !== JSON.stringify(baseline),
    [baseline, draft],
  );

  const loadClusterState = useCallback(
    async (
      cluster: MvpCluster,
      options: { background?: boolean; syncDraft?: boolean } = {},
    ) => {
      const { background = false, syncDraft = true } = options;

      if (background) {
        setStateRefreshing(true);
      } else {
        setStateLoading(true);
      }

      try {
        const [nextFanState, curves] = await Promise.all([
          getFanState(cluster.primaryDeviceId),
          listFanCurves(),
        ]);
        const preferredCurveName =
          nextFanState.active_curve ?? buildMvpFanCurveName(cluster.id);
        const currentCurve = curves.find((curve) => curve.name === preferredCurveName) ?? null;
        const nextBaseline = {
          ...draftFromFanState(nextFanState, currentCurve?.points),
          points: sortCurvePoints(
            currentCurve?.points ?? buildMvpFanCurvePoints(null),
          ),
        };

        setFanState(nextFanState);
        setBaseline(nextBaseline);
        if (syncDraft) {
          setDraft(nextBaseline);
        }
      } catch (nextError) {
        if (!background) {
          setFanState(null);
          setBaseline(defaultDraft);
          if (syncDraft) {
            setDraft(defaultDraft);
          }
        }
        setError(toErrorMessage(nextError, "Fan status could not be loaded"));
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

  const refresh = useCallback(
    async (options: { background?: boolean; preserveDraft?: boolean } = {}) => {
      const { background = false, preserveDraft = dirty } = options;

      setError(null);

      if (background) {
        setRefreshing(true);
      } else {
        setLoading(true);
      }

      try {
        const devices = await listDevices();
        const nextClusters = buildPairedClusters(devices);
        const resolvedClusterId =
          resolveRequestedClusterId(requestedClusterId, requestedDeviceId, nextClusters) ||
          (selectedClusterId &&
          nextClusters.some((cluster) => cluster.id === selectedClusterId)
            ? selectedClusterId
            : nextClusters[0]?.id ?? "");

        setClusters(nextClusters);
        setSelectedClusterId(resolvedClusterId);

        if (!resolvedClusterId) {
          setFanState(null);
          setBaseline(defaultDraft);
          if (!preserveDraft) {
            setDraft(defaultDraft);
          }
          return;
        }

        const cluster =
          nextClusters.find((item) => item.id === resolvedClusterId) ?? null;
        if (!cluster) {
          return;
        }

        await loadClusterState(cluster, {
          background,
          syncDraft: !preserveDraft || resolvedClusterId !== selectedClusterId,
        });
      } catch (nextError) {
        setError(toErrorMessage(nextError, "Fan data could not be loaded"));
      } finally {
        if (background) {
          setRefreshing(false);
        } else {
          setLoading(false);
        }
      }
    },
    [
      dirty,
      loadClusterState,
      requestedClusterId,
      requestedDeviceId,
      selectedClusterId,
    ],
  );

  useEffect(() => {
    void refresh({ preserveDraft: false });
  }, [requestedClusterId, requestedDeviceId]);

  useBackendEventSubscription(
    useCallback(
      (event) => {
        if (
          event.type === "daemon.connected" ||
          event.type === "daemon.disconnected" ||
          event.type === "device.updated" ||
          event.type === "fan.changed" ||
          event.type === "config.changed"
        ) {
          void refresh({ background: true, preserveDraft: true });
        }
      },
      [refresh],
    ),
  );

  useBackgroundRefresh(
    useCallback(async () => {
      await refresh({ background: true, preserveDraft: true });
    }, [refresh]),
    LIVE_STATUS_REFRESH_INTERVAL_MS,
  );

  const applyChanges = useCallback(async () => {
    if (!selectedCluster) {
      setError("Kein Cluster ausgewählt");
      return false;
    }

    if (selectedCluster.status === "offline") {
      setError("Offline-Cluster können nicht aktualisiert werden");
      return false;
    }

    setApplying(true);
    setError(null);
    setSuccess(null);

    try {
      if (draft.mode === "curve") {
        const curveName = buildMvpFanCurveName(selectedCluster.id);
        const payload = {
          name: curveName,
          temperature_source: draft.curveSource,
          points: sortCurvePoints(draft.points),
        };

        const curves = await listFanCurves();
        const existingCurve = curves.find((curve) => curve.name === curveName) ?? null;

        if (existingCurve) {
          await updateFanCurve(existingCurve.name, payload);
        } else {
          await createFanCurve(payload);
        }

        await applyFanWorkbench({
          target_mode: "selected",
          device_id: selectedCluster.primaryDeviceId,
          device_ids: selectedCluster.deviceIds,
          mode: "curve",
          curve: curveName,
        });
      } else {
        await applyFanWorkbench({
          target_mode: "selected",
          device_id: selectedCluster.primaryDeviceId,
          device_ids: selectedCluster.deviceIds,
          mode: "manual",
          percent: clampPercent(draft.manualPercent),
        });
      }

      await loadClusterState(selectedCluster, { syncDraft: true });
      setSuccess("Lüftereinstellungen wurden erfolgreich gespeichert.");
      return true;
    } catch (nextError) {
      setError(toErrorMessage(nextError, "Lüftereinstellungen konnten nicht gespeichert werden"));
      return false;
    } finally {
      setApplying(false);
    }
  }, [draft, loadClusterState, selectedCluster]);

  const resetDraft = useCallback(() => {
    setDraft(baseline);
    setError(null);
    setSuccess(null);
  }, [baseline]);

  const updateCurvePoint = useCallback(
    (index: number, field: keyof FanCurvePointDocument, value: number) => {
      setDraft((current) => ({
        ...current,
        points: current.points.map((point, pointIndex) =>
          pointIndex === index
            ? {
                ...point,
                [field]:
                  field === "percent"
                    ? clampPercent(value)
                    : Math.round(value * 10) / 10,
              }
            : point,
        ),
      }));
    },
    [],
  );

  const addCurvePoint = useCallback(() => {
    setDraft((current) => {
      const lastPoint = current.points[current.points.length - 1];
      const nextPoint = lastPoint
        ? {
            temperature_celsius: Math.round((lastPoint.temperature_celsius + 10) * 10) / 10,
            percent: clampPercent(lastPoint.percent + 10),
          }
        : {
            temperature_celsius: 60,
            percent: 70,
          };

      return {
        ...current,
        points: [...current.points, nextPoint].slice(0, 5),
      };
    });
  }, []);

  const removeCurvePoint = useCallback((index: number) => {
    setDraft((current) => ({
      ...current,
      points:
        current.points.length <= 2
          ? current.points
          : current.points.filter((_, pointIndex) => pointIndex !== index),
    }));
  }, []);

  const setMode = useCallback((mode: "manual" | "curve") => {
    setDraft((current) => ({
      ...current,
      mode,
    }));
  }, []);

  const setManualPercent = useCallback((manualPercent: number) => {
    setDraft((current) => ({
      ...current,
      manualPercent: clampPercent(manualPercent),
    }));
  }, []);

  const setCurveSource = useCallback((curveSource: FanCurveSource) => {
    setDraft((current) => ({
      ...current,
      curveSource: normalizeFanCurveSource(curveSource),
    }));
  }, []);

  return {
    clusters,
    selectedClusterId,
    setSelectedClusterId,
    selectedCluster,
    fanState,
    loading,
    refreshing,
    stateLoading,
    stateRefreshing,
    applying,
    error,
    success,
    dirty,
    draft,
    refresh: async () => {
      await refresh({ preserveDraft: true });
    },
    applyChanges,
    resetDraft,
    setMode,
    setManualPercent,
    setCurveSource,
    updateCurvePoint,
    addCurvePoint,
    removeCurvePoint,
  };
}
