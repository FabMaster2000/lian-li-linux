import { useCallback, useEffect, useMemo, useState } from "react";
import {
  LIVE_STATUS_REFRESH_INTERVAL_MS,
  useBackgroundRefresh,
} from "./useBackgroundRefresh";
import { useBackendEventSubscription } from "./useBackendEventSubscription";
import { listDevices } from "../services/devices";
import {
  applyLightingWorkbench,
  getLightingEffectRoute,
  getLightingState,
  saveLightingEffectRoute,
} from "../services/lighting";
import type {
  LightingEffectRouteEntryDocument,
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

export type RgbEffectChoice = "Static" | "Meteor";

const METEOR_FIXED_SPEED = 10;
const METEOR_FIXED_BRIGHTNESS = 100;

function toErrorMessage(error: unknown, fallback: string) {
  return error instanceof Error ? error.message : fallback;
}

export type RouteDraftEntry = {
  key: string;
  deviceId: string;
  fanIndex: number;
  label: string;
};

function routeEntryKey(deviceId: string, fanIndex: number) {
  return `${deviceId}::${fanIndex}`;
}

function buildAllFansRoute(clusters: MvpCluster[]): RouteDraftEntry[] {
  return clusters.flatMap((cluster) =>
    cluster.devices.flatMap((device) => {
      const fanCount = Math.max(0, device.capabilities.fan_count ?? 0);
      return Array.from({ length: fanCount }, (_, index) => {
        const fanIndex = index + 1;
        const prefix =
          device.display_name && device.display_name !== cluster.label
            ? `${cluster.label} · ${device.display_name}`
            : cluster.label;
        return {
          key: routeEntryKey(device.id, fanIndex),
          deviceId: device.id,
          fanIndex,
          label: `${prefix} · Lüfter ${fanIndex}`,
        };
      });
    }),
  );
}

function applySavedOrder(
  allFans: RouteDraftEntry[],
  saved: LightingEffectRouteEntryDocument[],
): RouteDraftEntry[] {
  if (saved.length === 0) return allFans;

  const fanMap = new Map(allFans.map((f) => [f.key, f]));
  const ordered: RouteDraftEntry[] = [];

  for (const entry of saved) {
    const key = routeEntryKey(entry.device_id, entry.fan_index);
    const fan = fanMap.get(key);
    if (fan) {
      ordered.push(fan);
      fanMap.delete(key);
    }
  }

  // Append any new fans that weren't in the saved route
  for (const fan of fanMap.values()) {
    ordered.push(fan);
  }

  return ordered;
}

function moveEntry(entries: RouteDraftEntry[], sourceKey: string, targetKey: string) {
  const sourceIndex = entries.findIndex((e) => e.key === sourceKey);
  const targetIndex = entries.findIndex((e) => e.key === targetKey);
  if (sourceIndex < 0 || targetIndex < 0 || sourceIndex === targetIndex) return entries;
  const next = [...entries];
  const [moved] = next.splice(sourceIndex, 1);
  next.splice(targetIndex, 0, moved);
  return next;
}

function routeOrderEqual(a: RouteDraftEntry[], b: RouteDraftEntry[]) {
  if (a.length !== b.length) return false;
  return a.every((entry, i) => entry.key === b[i]?.key);
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
  const [routeDraft, setRouteDraft] = useState<RouteDraftEntry[]>([]);
  const [baselineRoute, setBaselineRoute] = useState<RouteDraftEntry[]>([]);

  const selectedCluster = useMemo(
    () => clusters.find((cluster) => cluster.id === selectedClusterId) ?? null,
    [clusters, selectedClusterId],
  );

  const routeDirty = !routeOrderEqual(routeDraft, baselineRoute);
  const dirty = color !== baselineColor || effect !== baselineEffect || routeDirty;

  const reorderRouteEntry = useCallback((sourceKey: string, targetKey: string) => {
    setRouteDraft((current) => moveEntry(current, sourceKey, targetKey));
  }, []);

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

        setLightingState(nextLightingState);
        setBaselineColor(nextColor);
        setBaselineEffect(nextEffect);
        if (syncDraft) {
          setColor(nextColor);
          setEffect(nextEffect);
        }
      } catch (nextError) {
        if (!background) {
          setLightingState(null);
          setBaselineColor("#5ec7ff");
          setBaselineEffect("Meteor");
          if (syncDraft) {
            setColor("#5ec7ff");
            setEffect("Meteor");
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
      preserveRouteDraft?: boolean;
    } = {}) => {
      const {
        background = false,
        preserveDraft = dirty,
        preserveRouteDraft = routeDirty,
      } = options;

      setError(null);

      if (background) {
        setRefreshing(true);
      } else {
        setLoading(true);
      }

      try {
        const [devices, savedRoute] = await Promise.all([
          listDevices(),
          getLightingEffectRoute(),
        ]);
        const nextClusters = buildPairedClusters(devices);
        const allFans = buildAllFansRoute(nextClusters);
        const orderedRoute = applySavedOrder(allFans, savedRoute.route);

        setClusters(nextClusters);
        setBaselineRoute(orderedRoute);
        if (!preserveRouteDraft) {
          setRouteDraft(orderedRoute);
        }

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
          if (!preserveDraft) {
            setColor("#5ec7ff");
            setEffect("Meteor");
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
      routeDirty,
      loadClusterState,
      requestedClusterId,
      requestedDeviceId,
      selectedClusterId,
    ],
  );

  useEffect(() => {
    void refresh({ preserveDraft: false, preserveRouteDraft: false });
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
          void refresh({ background: true, preserveDraft: true, preserveRouteDraft: true });
        }
      },
      [refresh],
    ),
  );

  useBackgroundRefresh(
    useCallback(async () => {
      await refresh({ background: true, preserveDraft: true, preserveRouteDraft: true });
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
        if (effect === "Meteor") {
          // Save the route order first, then apply Meteor
          await saveLightingEffectRoute({
            route: routeDraft.map((entry) => ({
              device_id: entry.deviceId,
              fan_index: entry.fanIndex,
            })),
          });

          await applyLightingWorkbench({
            target_mode: "route",
            device_ids: [],
            zone_mode: "all_zones",
            effect: "Meteor",
            brightness: METEOR_FIXED_BRIGHTNESS,
            speed: METEOR_FIXED_SPEED,
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

        await refresh({ preserveDraft: false, preserveRouteDraft: false });
        setSuccess(
          effect === "Meteor"
            ? "Meteor-Effekt wurde auf alle Lüfter angewendet."
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
    [clusters, color, effect, lightingState, refresh, routeDraft, selectedCluster],
  );

  const resetDraft = useCallback(() => {
    setColor(baselineColor);
    setEffect(baselineEffect);
    setRouteDraft(baselineRoute);
    setError(null);
    setSuccess(null);
  }, [baselineColor, baselineEffect, baselineRoute]);

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
    routeDraft,
    reorderRouteEntry,
    dirty,
    rgbSummary: summarizeLightingState(lightingState),
    refresh: async () => {
      await refresh({ preserveDraft: true, preserveRouteDraft: true });
    },
    applyChanges,
    resetDraft,
  };
}
