import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useLightingWorkbenchData } from "./useLightingWorkbenchData";
import type { BackendEventEnvelope, DeviceView, LightingApplyResponse, LightingStateResponse } from "../types/api";
import { listDevices } from "../services/devices";
import { applyLightingWorkbench, getLightingState } from "../services/lighting";

vi.mock("../services/devices", () => ({
  listDevices: vi.fn(),
}));

vi.mock("../services/lighting", () => ({
  getLightingState: vi.fn(),
  applyLightingWorkbench: vi.fn(),
}));

let latestBackendListener: ((event: BackendEventEnvelope) => void) | null = null;

vi.mock("./useBackendEventSubscription", () => ({
  useBackendEventSubscription: vi.fn((listener: (event: BackendEventEnvelope) => void) => {
    latestBackendListener = listener;
  }),
}));

const listDevicesMock = vi.mocked(listDevices);
const getLightingStateMock = vi.mocked(getLightingState);
const applyLightingWorkbenchMock = vi.mocked(applyLightingWorkbench);

function emitBackendEvent(event: Partial<BackendEventEnvelope> & Pick<BackendEventEnvelope, "type">) {
  latestBackendListener?.({
    timestamp: "2026-03-15T12:00:00Z",
    source: "ws",
    device_id: null,
    data: {},
    ...event,
  });
}

function rgbDevice(id: string, name: string, overrides: Partial<DeviceView> = {}): DeviceView {
  return {
    id,
    name,
    display_name: name,
    ui_order: 10,
    physical_role: "Wireless cluster",
    capability_summary: "2 RGB zone(s) | 4 fan slot(s)",
    current_mode_summary: "Lighting ready | Cooling telemetry live",
    controller: {
      id: "wireless:mesh",
      label: "Wireless mesh",
      kind: "wireless_mesh",
    },
    wireless: {
      transport: "wireless",
      channel: 8,
      group_id: id,
      group_label: name,
    binding_state: "connected",
    master_mac: "3b:59:87:e5:66:e4",
    },
    health: {
      level: "healthy",
      summary: "Device inventory healthy",
    },
    family: "SlInf",
    online: true,
    capabilities: {
      has_fan: true,
      has_rgb: true,
      has_lcd: false,
      has_pump: false,
      fan_count: 4,
      per_fan_control: false,
      mb_sync_support: false,
      rgb_zone_count: 2,
    },
    state: {
      fan_rpms: [900, 910, 920, 930],
      coolant_temp: null,
      streaming_active: false,
    },
    ...overrides,
  };
}

function lightingState(
  deviceId: string,
  effect = "Static",
  color = "#112233",
  brightness = 75,
): LightingStateResponse {
  return {
    device_id: deviceId,
    zones: [
      {
        zone: 0,
        effect,
        colors: [color],
        speed: 2,
        brightness_percent: brightness,
        direction: "Clockwise",
        scope: "All",
        smoothness_ms: 0,
      },
      {
        zone: 1,
        effect: "Rainbow",
        colors: ["#445566"],
        speed: 4,
        brightness_percent: 25,
        direction: "Up",
        scope: "Inner",
        smoothness_ms: 0,
      },
    ],
  };
}

function applyResponse(deviceId: string, skipped = 0): LightingApplyResponse {
  return {
    target_mode: "selected",
    zone_mode: "active",
    requested_device_ids: [deviceId],
    applied_devices: [
      {
        device_id: deviceId,
        zones: lightingState(deviceId, "Wave", "#abcdef", 60).zones,
      },
    ],
    skipped_devices:
      skipped > 0
        ? [
            {
              device_id: "rgb-secondary",
              reason: "selected effect is limited to pump-capable devices",
            },
          ]
        : [],
    sync_selected: true,
  };
}

describe("useLightingWorkbenchData", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    latestBackendListener = null;
    window.localStorage.clear();
  });

  it("loads the requested primary device and seeds the draft from backend state", async () => {
    listDevicesMock.mockResolvedValue([rgbDevice("rgb-primary", "RGB Primary")]);
    getLightingStateMock.mockResolvedValue(lightingState("rgb-primary"));

    const { result } = renderHook(() => useLightingWorkbenchData("rgb-primary"));

    await waitFor(() => expect(result.current.selectedDeviceId).toBe("rgb-primary"));

    expect(result.current.draft.effect).toBe("Static");
    expect(result.current.draft.colors).toEqual(["#112233"]);
    expect(result.current.targetMode).toBe("single");
    expect(result.current.previewSummary).toContain("Static");
  });

  it("builds a selected-device multi-apply request and records partial success", async () => {
    listDevicesMock.mockResolvedValue([
      rgbDevice("rgb-primary", "RGB Primary"),
      rgbDevice("rgb-secondary", "RGB Secondary", {
        capabilities: {
          ...rgbDevice("tmp", "tmp").capabilities,
          has_pump: false,
        },
      }),
    ]);
    getLightingStateMock.mockResolvedValue(lightingState("rgb-primary"));
    applyLightingWorkbenchMock.mockResolvedValue(applyResponse("rgb-primary", 1));

    const { result } = renderHook(() => useLightingWorkbenchData("rgb-primary"));
    await waitFor(() => expect(result.current.selectedDeviceId).toBe("rgb-primary"));

    act(() => {
      result.current.setTargetMode("selected");
    });

    await waitFor(() => expect(result.current.selectedDeviceIds).toContain("rgb-primary"));

    act(() => {
      result.current.toggleSelectedDevice("rgb-secondary");
      result.current.setSyncSelected(true);
      result.current.setDraft((current) => ({
        ...current,
        effect: "Meteor",
        colors: ["#abcdef"],
        brightness: 60,
        speed: 3,
      }));
    });

    await act(async () => {
      await result.current.applyChanges();
    });

    expect(applyLightingWorkbenchMock).toHaveBeenCalledWith({
      target_mode: "selected",
      device_id: "rgb-primary",
      device_ids: expect.arrayContaining(["rgb-primary", "rgb-secondary"]),
      zone_mode: "active",
      zone: 0,
      sync_selected: true,
      effect: "Meteor",
      brightness: 60,
      speed: 3,
      colors: [{ hex: "#abcdef" }],
      direction: "Clockwise",
      scope: null,
    });
    expect(result.current.success).toContain("skipped");
    expect(result.current.lastApplySummary?.skipped_devices).toHaveLength(1);
    expect(result.current.draft.effect).toBe("Wave");
    expect(result.current.preserveMultiTargetScope).toBe(true);
    expect(result.current.notices.some((notice) => notice.id === "preserve-target-scope")).toBe(
      false,
    );
  });

  it("refreshes live lighting state without overwriting a dirty draft", async () => {
    listDevicesMock.mockResolvedValue([rgbDevice("rgb-primary", "RGB Primary")]);
    getLightingStateMock
      .mockResolvedValueOnce(lightingState("rgb-primary", "Static", "#112233", 75))
      .mockResolvedValueOnce(lightingState("rgb-primary", "Rainbow", "#778899", 10));

    const { result } = renderHook(() => useLightingWorkbenchData("rgb-primary"));
    await waitFor(() => expect(result.current.selectedDeviceId).toBe("rgb-primary"));

    act(() => {
      result.current.setDraft((current) => ({
        ...current,
        effect: "Breathing",
        colors: ["#abcdef"],
        brightness: 60,
      }));
    });

    await act(async () => {
      await result.current.refresh();
    });

    expect(result.current.draft.effect).toBe("Breathing");
    expect(result.current.draft.colors).toEqual(["#abcdef"]);
    expect(result.current.activeZone?.effect).toBe("Rainbow");
    expect(result.current.activeZone?.colors).toEqual(["#778899"]);
  });

  it("saves a browser-local custom preset from the current draft", async () => {
    listDevicesMock.mockResolvedValue([rgbDevice("rgb-primary", "RGB Primary")]);
    getLightingStateMock.mockResolvedValue(lightingState("rgb-primary"));

    const { result } = renderHook(() => useLightingWorkbenchData("rgb-primary"));
    await waitFor(() => expect(result.current.selectedDeviceId).toBe("rgb-primary"));

    act(() => {
      result.current.setPresetName("Desk evening");
      result.current.setDraft((current) => ({
        ...current,
        effect: "Breathing",
        colors: ["#ffaa66"],
        brightness: 55,
      }));
    });

    act(() => {
      result.current.saveCurrentPreset();
    });

    expect(result.current.customPresets[0]?.label).toBe("Desk evening");
    expect(window.localStorage.getItem("lighting-workbench-presets-v1")).toContain("Desk evening");
  });

  it("reacts to lighting backend events with a readonly refresh", async () => {
    listDevicesMock.mockResolvedValue([rgbDevice("rgb-primary", "RGB Primary")]);
    getLightingStateMock
      .mockResolvedValueOnce(lightingState("rgb-primary", "Static", "#112233", 75))
      .mockResolvedValueOnce(lightingState("rgb-primary", "Wave", "#334455", 40));

    const { result } = renderHook(() => useLightingWorkbenchData("rgb-primary"));
    await waitFor(() => expect(result.current.selectedDeviceId).toBe("rgb-primary"));

    act(() => {
      emitBackendEvent({
        type: "lighting.changed",
        device_id: "rgb-primary",
      });
    });

    await waitFor(() => expect(result.current.activeZone?.effect).toBe("Wave"));
    expect(result.current.activeZone?.colors).toEqual(["#334455"]);
  });
});



