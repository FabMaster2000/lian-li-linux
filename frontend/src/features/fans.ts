import type {
  DeviceView,
  FanCurveDocument,
  FanCurvePointDocument,
  FanTemperaturePreview,
  FanStateResponse,
} from "../types/api";

export type FanTargetMode = "single" | "selected" | "all";
export type FanWorkbenchMode = "manual" | "curve" | "mb_sync" | "start_stop";

export type FanControlDraft = {
  mode: FanWorkbenchMode;
  manualPercent: number;
  curveName: string;
};

export type FanCurveDraft = {
  name: string;
  temperature_source: string;
  points: FanCurvePointDocument[];
};

export type FanCurveValidationIssue = {
  id: string;
  tone: "warning" | "error";
  message: string;
};

export type FanTemperatureSourceInput = "cpu" | "gpu";
export type FanTemperatureSourceCombine = "single" | "max" | "avg" | "min";
export type FanTemperatureSourceDraft = {
  mode: "hardware" | "custom";
  inputs: FanTemperatureSourceInput[];
  combine: FanTemperatureSourceCombine;
  customCommand: string;
};

export type FanCurvePreviewMarker = {
  id: string;
  label: string;
  x: number;
  celsius: number;
  tone: "cpu" | "gpu" | "aggregate";
};

const defaultCurvePoints: FanCurvePointDocument[] = [
  { temperature_celsius: 28, percent: 25 },
  { temperature_celsius: 38, percent: 40 },
  { temperature_celsius: 52, percent: 62 },
  { temperature_celsius: 68, percent: 85 },
];

export const defaultFanControlDraft: FanControlDraft = {
  mode: "manual",
  manualPercent: 50,
  curveName: "",
};

export const singleWirelessSlInfManualMinimumPercent = 30;

export const fanTargetModeTabs = [
  { id: "single", label: "Single device" },
  { id: "selected", label: "Selected devices" },
  { id: "all", label: "All compatible" },
] as const;

export const fanModeOptions = [
  { value: "manual", label: "Manual fixed speed" },
  { value: "curve", label: "Curve mode" },
  { value: "mb_sync", label: "Motherboard sync" },
  { value: "start_stop", label: "Start / stop" },
] as const;

export const fanTemperatureSourceInputs = [
  { value: "cpu", label: "CPU" },
  { value: "gpu", label: "GPU" },
] as const;

export const fanTemperatureCombineOptions = [
  { value: "max", label: "Highest" },
  { value: "avg", label: "Average" },
  { value: "min", label: "Lowest" },
] as const;

export function clampFanPercent(value: number) {
  return Math.min(100, Math.max(0, Math.round(value)));
}

export function stableManualFloorForDevice(
  device: Pick<DeviceView, "family" | "wireless" | "capabilities"> | null,
) {
  if (
    device?.wireless?.transport === "wireless" &&
    device.family === "SlInf" &&
    device.capabilities.fan_count === 1
  ) {
    return singleWirelessSlInfManualMinimumPercent;
  }

  return null;
}

export function normalizeFanWorkbenchMode(value: string | undefined): FanWorkbenchMode {
  switch (value) {
    case "curve":
    case "mb_sync":
    case "start_stop":
      return value;
    default:
      return "manual";
  }
}

export function summarizeFanLiveMode(fanState: FanStateResponse | null) {
  if (!fanState) {
    return "n/a";
  }

  if (fanState.active_mode) {
    return fanState.active_mode;
  }

  const modes = [...new Set(fanState.slots.map((slot) => slot.mode))];
  return modes.length === 1 ? modes[0] : "mixed";
}

export function averageFanPercent(fanState: FanStateResponse | null) {
  if (!fanState) {
    return defaultFanControlDraft.manualPercent;
  }

  const percents = fanState.slots
    .map((slot) => slot.percent)
    .filter((percent): percent is number => typeof percent === "number");

  if (percents.length === 0) {
    return defaultFanControlDraft.manualPercent;
  }

  return clampFanPercent(
    percents.reduce((sum, percent) => sum + percent, 0) / percents.length,
  );
}

export function controlDraftFromFanState(fanState: FanStateResponse | null): FanControlDraft {
  if (!fanState) {
    return defaultFanControlDraft;
  }

  const mode = normalizeFanWorkbenchMode(summarizeFanLiveMode(fanState));
  return {
    mode,
    manualPercent: averageFanPercent(fanState),
    curveName: fanState.active_curve ?? fanState.slots.find((slot) => slot.curve)?.curve ?? "",
  };
}

export function curveDraftFromCurve(curve: FanCurveDocument): FanCurveDraft {
  return {
    name: curve.name,
    temperature_source: curve.temperature_source,
    points: curve.points.map((point) => ({ ...point })),
  };
}

function nextCurveName(existingNames: string[], baseName: string) {
  if (!existingNames.includes(baseName)) {
    return baseName;
  }

  let index = 2;
  while (existingNames.includes(`${baseName} ${index}`)) {
    index += 1;
  }

  return `${baseName} ${index}`;
}

export function createBlankFanCurveDraft(existingCurves: FanCurveDocument[]): FanCurveDraft {
  return {
    name: nextCurveName(
      existingCurves.map((curve) => curve.name),
      "Balanced curve",
    ),
    temperature_source: existingCurves[0]?.temperature_source ?? "cpu",
    points: defaultCurvePoints.map((point) => ({ ...point })),
  };
}

export function createDuplicateFanCurveDraft(
  source: FanCurveDraft,
  existingCurves: FanCurveDocument[],
): FanCurveDraft {
  return {
    ...source,
    name: nextCurveName(
      existingCurves.map((curve) => curve.name),
      `${source.name.trim() || "Curve"} Copy`,
    ),
    points: source.points.map((point) => ({ ...point })),
  };
}

export function validateFanCurveDraft(
  draft: FanCurveDraft,
): FanCurveValidationIssue[] {
  const issues: FanCurveValidationIssue[] = [];

  if (!draft.name.trim()) {
    issues.push({
      id: "name-required",
      tone: "error",
      message: "Curve name is required.",
    });
  }

  if (!draft.temperature_source.trim()) {
    issues.push({
      id: "temperature-source-required",
      tone: "error",
      message: "Temperature source is required.",
    });
  }

  if (draft.points.length < 2) {
    issues.push({
      id: "point-count",
      tone: "error",
      message: "Add at least two curve points before saving or applying curve mode.",
    });
  }

  let previousTemperature = Number.NEGATIVE_INFINITY;
  const seenTemperatures = new Set<number>();

  for (const [index, point] of draft.points.entries()) {
    if (!Number.isFinite(point.temperature_celsius)) {
      issues.push({
        id: `point-${index}-temp-finite`,
        tone: "error",
        message: `Point ${index + 1} temperature must be a valid number.`,
      });
    }

    if (!Number.isFinite(point.percent)) {
      issues.push({
        id: `point-${index}-percent-finite`,
        tone: "error",
        message: `Point ${index + 1} speed must be a valid percent.`,
      });
    }

    if (point.percent < 0 || point.percent > 100) {
      issues.push({
        id: `point-${index}-percent-range`,
        tone: "error",
        message: `Point ${index + 1} speed must stay between 0% and 100%.`,
      });
    }

    if (seenTemperatures.has(point.temperature_celsius)) {
      issues.push({
        id: `point-${index}-temp-duplicate`,
        tone: "error",
        message: `Point ${index + 1} reuses a temperature that already exists in this curve.`,
      });
    }

    if (point.temperature_celsius <= previousTemperature) {
      issues.push({
        id: `point-${index}-temp-order`,
        tone: "error",
        message: "Curve points must be ordered by ascending temperature.",
      });
    }

    previousTemperature = point.temperature_celsius;
    seenTemperatures.add(point.temperature_celsius);
  }

  return issues;
}

export function parseFanTemperatureSourceDraft(
  source: string,
): FanTemperatureSourceDraft {
  const trimmed = source.trim();
  if (!trimmed) {
    return {
      mode: "hardware",
      inputs: ["cpu"],
      combine: "single",
      customCommand: "",
    };
  }

  const normalized = trimmed.toLowerCase();
  if (normalized === "cpu" || normalized === "gpu") {
    return {
      mode: "hardware",
      inputs: [normalized as FanTemperatureSourceInput],
      combine: "single",
      customCommand: "",
    };
  }

  for (const combine of ["max", "avg", "min"] as const) {
    const prefix = `${combine}(`;
    if (normalized.startsWith(prefix) && normalized.endsWith(")")) {
      const inner = normalized.slice(prefix.length, -1);
      const inputs = inner
        .split(",")
        .map((part) => part.trim())
        .filter((part): part is FanTemperatureSourceInput => part === "cpu" || part === "gpu");
      const uniqueInputs = [...new Set(inputs)];
      if (uniqueInputs.length >= 2) {
        return {
          mode: "hardware",
          inputs: uniqueInputs.sort(),
          combine,
          customCommand: "",
        };
      }
    }
  }

  return {
    mode: "custom",
    inputs: ["cpu"],
    combine: "single",
    customCommand: source,
  };
}

export function buildFanTemperatureSource(
  draft: FanTemperatureSourceDraft,
) {
  if (draft.mode === "custom") {
    return draft.customCommand.trim();
  }

  const uniqueInputs = [...new Set(draft.inputs)].sort() as FanTemperatureSourceInput[];
  if (uniqueInputs.length <= 1) {
    return uniqueInputs[0] ?? "cpu";
  }

  const combine = draft.combine === "single" ? "max" : draft.combine;
  return `${combine}(${uniqueInputs.join(",")})`;
}

export function describeFanTemperatureSource(source: string | null | undefined) {
  if (!source?.trim()) {
    return "n/a";
  }

  const draft = parseFanTemperatureSourceDraft(source);
  if (draft.mode === "custom") {
    return "Custom command";
  }

  if (draft.inputs.length === 1) {
    return `${draft.inputs[0].toUpperCase()} temperature`;
  }

  const inputsLabel = draft.inputs.map((input) => input.toUpperCase()).join(" + ");
  switch (draft.combine) {
    case "avg":
      return `Average of ${inputsLabel}`;
    case "min":
      return `Lowest of ${inputsLabel}`;
    default:
      return `Highest of ${inputsLabel}`;
  }
}

export function fanTemperatureSourceDetail(source: string | null | undefined) {
  if (!source?.trim()) {
    return null;
  }

  const draft = parseFanTemperatureSourceDraft(source);
  return draft.mode === "custom" ? source : null;
}

export function fanCurvePreviewPolyline(
  points: FanCurvePointDocument[],
  width = 220,
  height = 120,
) {
  if (points.length === 0) {
    return "";
  }

  const sortedPoints = [...points].sort(
    (left, right) => left.temperature_celsius - right.temperature_celsius,
  );
  const temperatures = sortedPoints.map((point) => point.temperature_celsius);
  const minTemperature = Math.min(...temperatures);
  const maxTemperature = Math.max(...temperatures);
  const xRange = Math.max(1, maxTemperature - minTemperature);
  const padding = 12;
  const innerWidth = Math.max(1, width - padding * 2);
  const innerHeight = Math.max(1, height - padding * 2);

  return sortedPoints
    .map((point) => {
      const x = padding + ((point.temperature_celsius - minTemperature) / xRange) * innerWidth;
      const y = height - padding - (clampFanPercent(point.percent) / 100) * innerHeight;
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(" ");
}

export function fanCurvePreviewMarkers(
  points: FanCurvePointDocument[],
  preview: FanTemperaturePreview | null,
  width = 220,
): FanCurvePreviewMarker[] {
  if (!preview || points.length === 0) {
    return [];
  }

  const sortedPoints = [...points].sort(
    (left, right) => left.temperature_celsius - right.temperature_celsius,
  );
  const temperatures = sortedPoints.map((point) => point.temperature_celsius);
  const minTemperature = Math.min(...temperatures);
  const maxTemperature = Math.max(...temperatures);
  const xRange = Math.max(1, maxTemperature - minTemperature);
  const padding = 12;
  const innerWidth = Math.max(1, width - padding * 2);
  const markers: FanCurvePreviewMarker[] = [];
  const seen = new Set<string>();

  for (const component of preview.components) {
    if (!component.available || component.celsius === null) {
      continue;
    }
    markers.push({
      id: component.key,
      label: component.label,
      celsius: component.celsius,
      x:
        padding +
        ((component.celsius - minTemperature) / xRange) * innerWidth,
      tone:
        component.key === "cpu"
          ? "cpu"
          : component.key === "gpu"
            ? "gpu"
            : "aggregate",
    });
    seen.add(component.key);
  }

  if (preview.available && preview.celsius !== null && !seen.has("aggregate")) {
    markers.push({
      id: "aggregate",
      label: "Active source",
      celsius: preview.celsius,
      x: padding + ((preview.celsius - minTemperature) / xRange) * innerWidth,
      tone: "aggregate",
    });
  }

  return markers.map((marker) => ({
    ...marker,
    x: Math.min(width - padding, Math.max(padding, marker.x)),
  }));
}

export function summarizeFanControlDraft(draft: FanControlDraft) {
  switch (draft.mode) {
    case "curve":
      return draft.curveName ? `Curve ${draft.curveName}` : "Curve mode";
    case "mb_sync":
      return "Motherboard sync";
    case "start_stop":
      return "Start / stop";
    default:
      return `${draft.manualPercent}% fixed speed`;
  }
}
