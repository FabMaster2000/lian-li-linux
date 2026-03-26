import { useCallback, useEffect, useMemo, useState } from "react";
import {
  LIVE_STATUS_REFRESH_INTERVAL_MS,
  useBackgroundRefresh,
} from "./useBackgroundRefresh";
import { useBackendEventSubscription } from "./useBackendEventSubscription";
import { listDevices } from "../services/devices";
import {
  applyLightingWorkbench,
  getLightingState,
} from "../services/lighting";
import type {
  LightingStateResponse,
} from "../types/api";
import {
  buildPairedClusters,
  getLightingApplyDefaults,
  resolveRequestedClusterId,
  summarizeLightingState,
  type MvpCluster,
} from "../features/mvpClusters";
import { isMvpRgbEffect } from "../features/lighting";

export type RgbEffectChoice = "Static" | "Meteor" | "Runway";

const METEOR_DEFAULT_SPEED = 10;
const METEOR_FIXED_BRIGHTNESS = 100;

function toErrorMessage(error: unknown, fallback: string) {
  return error instanceof Error ? error.message : fallback;
}

export function useMvpRgbPageData(
  requestedClusterId: string | null,
  requestedDeviceId: string | null,
) {
  const [clusters, setClusters] = useState<MvpCluster[]>([]);
  const [selectedClusterId, setSelectedClusterId] = useState("");
  const [lightingState, setLightingState] = useState<LightingStateResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [stateLoading, setStateLoading] = useState(false);
  const [stateRefreshing, setStateRefreshing] = useState(false);
  const [applying, setApplying] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [effect, setEffect] = useState<RgbEffectChoice>("Meteor");
  const [baselineEffect, setBaselineEffect] = useState<RgbEffectChoice>("Meteor");
  const [color, setColor] = useState("#5ec7ff");
  const [baselineColor, setBaselineColor] = useState("#5ec7ff");
  const [speed, setSpeed] = useState(METEOR_DEFAULT_SPEED);
  const [baselineSpeed, setBaselineSpeed] = useState(METEOR_DEFAULT_SPEED);

  const selectedCluster = useMemo(
    () => clusters.find((cluster) => cluster.id === selectedClusterId) ?? null,
    [clusters, selectedClusterId],
  );

  const dirty = color !== baselineColor || effect !== baselineEffect || speed !== baselineSpeed;

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
        const nextLightingState = await getLightingState(cluster.primaryDeviceId);
        const nextColor = nextLightingState.zones[0]?.colors[0] ?? "#5ec7ff";
        const liveEffect = nextLightingState.zones[0]?.effect ?? "Meteor";
        const nextEffect: RgbEffectChoice = isMvpRgbEffect(liveEffect) ? (liveEffect as RgbEffectChoice) : "Static";
        const nextSpeed = nextLightingState.zones[0]?.speed ?? METEOR_DEFAULT_SPEED;

        setLightingState(nextLightingState);
        setBaselineColor(nextColor);
        setBaselineEffect(nextEffect);
        setBaselineSpeed(nextSpeed);
        if (syncDraft) {
          setColor(nextColor);
          setEffect(nextEffect);
          setSpeed(nextSpeed);
        }
      } catch (nextError) {
        if (!background) {
          setLightingState(null);
          setBaselineColor("#5ec7ff");
          setBaselineEffect("Meteor");
          setBaselineSpeed(METEOR_DEFAULT_SPEED);
          if (syncDraft) {
            setColor("#5ec7ff");
            setEffect("Meteor");
            setSpeed(METEOR_DEFAULT_SPEED);
          }
        }
        setError(toErrorMessage(nextError, "RGB status could not be loaded"));
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
    async (options: {
      background?: boolean;
      preserveDraft?: boolean;
    } = {}) => {
      const {
        background = false,
        preserveDraft = dirty,
      } = options;

      setError(null);

      if (background) {
        setRefreshing(true);
      } else {
        setLoading(true);
      }

      try {
        const devices = await listDevices();
        const nextClusters = buildPairedClusters(devices);

        setClusters(nextClusters);

        const resolvedClusterId =
          resolveRequestedClusterId(requestedClusterId, requestedDeviceId, nextClusters) ||
          (selectedClusterId &&
          nextClusters.some((cluster) => cluster.id === selectedClusterId)
            ? selectedClusterId
            : nextClusters[0]?.id ?? "");

        setSelectedClusterId(resolvedClusterId);

        if (!resolvedClusterId) {
          setLightingState(null);
          setBaselineColor("#5ec7ff");
          setBaselineEffect("Meteor");
          setBaselineSpeed(METEOR_DEFAULT_SPEED);
          if (!preserveDraft) {
            setColor("#5ec7ff");
            setEffect("Meteor");
            setSpeed(METEOR_DEFAULT_SPEED);
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
        setError(toErrorMessage(nextError, "RGB data could not be loaded"));
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
          event.type === "lighting.changed" ||
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

  const applyChanges = useCallback(
    async (applyToAll = false) => {
      if (!selectedCluster) {
        setError("Kein Cluster ausgewählt");
        return false;
      }

      if (selectedCluster.status === "offline") {
        setError("Offline-Cluster können nicht aktualisiert werden");
        return false;
      }

      const targetClusters = applyToAll ? clusters : [selectedCluster];
      const targetDeviceIds = targetClusters.flatMap((cluster) => cluster.deviceIds);
      const defaults = getLightingApplyDefaults(lightingState);

      setApplying(true);
      setError(null);
      setSuccess(null);

      try {
        if (effect === "Meteor" || effect === "Runway") {
          await applyLightingWorkbench({
            target_mode: "route",
            device_ids: [],
            zone_mode: "all_zones",
            effect: effect,
            brightness: METEOR_FIXED_BRIGHTNESS,
            speed,
            colors: [{ hex: color }],
            scope: "All",
            sync_selected: false,
          });
        } else {
          await applyLightingWorkbench({
            target_mode: "selected",
            device_id: selectedCluster.primaryDeviceId,
            device_ids: targetDeviceIds,
            zone_mode: "all_zones",
            effect: "Static",
            brightness: defaults.brightness,
            speed: defaults.speed,
            colors: [{ hex: color }],
            direction: defaults.direction,
            scope: defaults.scope,
            sync_selected: false,
          });
        }

        await refresh({ preserveDraft: false });
        setSuccess(
          effect === "Meteor" || effect === "Runway"
            ? `${effect}-Effekt wurde auf alle Lüfter angewendet.`
            : applyToAll
              ? "RGB-Einstellung wurde auf alle Cluster übertragen."
              : "RGB-Einstellung wurde erfolgreich gespeichert.",
        );
        return true;
      } catch (nextError) {
        setError(toErrorMessage(nextError, "RGB-Einstellung konnte nicht gespeichert werden"));
        return false;
      } finally {
        setApplying(false);
      }
    },
    [clusters, color, effect, lightingState, refresh, selectedCluster],
  );

  const resetDraft = useCallback(() => {
    setColor(baselineColor);
    setEffect(baselineEffect);
    setSpeed(baselineSpeed);
    setError(null);
    setSuccess(null);
  }, [baselineColor, baselineEffect, baselineSpeed]);

  return {
    clusters,
    selectedClusterId,
    setSelectedClusterId,
    selectedCluster,
    lightingState,
    loading,
    refreshing,
    stateLoading,
    stateRefreshing,
    applying,
    error,
    success,
    effect,
    setEffect,
    color,
    setColor,
    speed,
    setSpeed,
    dirty,
    rgbSummary: summarizeLightingState(lightingState),
    refresh: async () => {
      await refresh({ preserveDraft: true });
    },
    applyChanges,
    resetDraft,
  };
}
