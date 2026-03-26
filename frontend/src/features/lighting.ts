export type LightingEffectOption = {
  value: string;
  label: string;
};

export type LightingPaletteMode = "none" | "single" | "multi";

export type LightingEffectDefinition = {
  value: string;
  label: string;
  paletteMode: LightingPaletteMode;
  supportsSpeed: boolean;
  supportsDirection: boolean;
  supportsScope: boolean;
  pumpOnly: boolean;
  note?: string;
};

export type LightingPreset = {
  id: string;
  label: string;
  description: string;
  effect: string;
  colors: string[];
  brightness: number;
  speed: number;
  direction: string;
  scope: string;
};

const effectOptions: LightingEffectOption[] = [
  { value: "Meteor", label: "Meteor" },
  { value: "Runway", label: "Runway" },
];

const noPaletteEffects = new Set<string>();
const multiPaletteEffects = new Set<string>();
const noSpeedEffects = new Set<string>();
const directionalEffects = new Set(["Meteor", "Runway"]);
const scopedEffects = new Set(["Meteor", "Runway"]);
const pumpOnlyEffects = new Set<string>();

export const lightingEffectOptions = effectOptions;

export const mvpRgbEffectValues = [
  "Meteor",
  "Runway",
] as const;

const mvpRgbEffectValueSet = new Set<string>(mvpRgbEffectValues);

export const mvpRgbEffectOptions = mvpRgbEffectValues
  .map((value) => effectOptions.find((option) => option.value === value) ?? null)
  .filter((option): option is LightingEffectOption => option !== null);

export function isMvpRgbEffect(effect: string) {
  return mvpRgbEffectValueSet.has(effect);
}

export const lightingDirectionOptions = [
  { value: "Clockwise", label: "Clockwise" },
  { value: "CounterClockwise", label: "Counter-clockwise" },
  { value: "Up", label: "Up" },
  { value: "Down", label: "Down" },
  { value: "Spread", label: "Spread" },
  { value: "Gather", label: "Gather" },
];

export const lightingScopeOptions = [
  { value: "All", label: "All LEDs" },
  { value: "Top", label: "Top" },
  { value: "Bottom", label: "Bottom" },
  { value: "Inner", label: "Inner" },
  { value: "Outer", label: "Outer" },
];

export const lightingPresetCatalog: LightingPreset[] = [
  {
    id: "meteor-default",
    label: "Meteor",
    description: "Meteor trail animation across all fans.",
    effect: "Meteor",
    colors: ["#ff3b30"],
    brightness: 85,
    speed: 3,
    direction: "Clockwise",
    scope: "All",
  },
  {
    id: "runway-default",
    label: "Runway",
    description: "Sharp fan-by-fan sweep across all fans.",
    effect: "Runway",
    colors: ["#5ec7ff"],
    brightness: 85,
    speed: 3,
    direction: "Clockwise",
    scope: "All",
  },
];

export function getLightingEffectDefinition(effect: string): LightingEffectDefinition {
  const option = effectOptions.find((item) => item.value === effect) ?? {
    value: effect,
    label: effect,
  };

  const paletteMode: LightingPaletteMode = noPaletteEffects.has(effect)
    ? "none"
    : multiPaletteEffects.has(effect)
      ? "multi"
      : "single";

  return {
    value: option.value,
    label: option.label,
    paletteMode,
    supportsSpeed: !noSpeedEffects.has(effect),
    supportsDirection: directionalEffects.has(effect),
    supportsScope: scopedEffects.has(effect),
    pumpOnly: pumpOnlyEffects.has(effect),
    note: pumpOnlyEffects.has(effect)
      ? "This effect currently maps only to pump-capable devices in the web workbench."
      : undefined,
  };
}

export function findLightingPreset(presetId: string) {
  return lightingPresetCatalog.find((preset) => preset.id === presetId) ?? null;
}
