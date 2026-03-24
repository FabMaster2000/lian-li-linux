import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useFansWorkbenchData } from "./useFansWorkbenchData";
import type { BackendEventEnvelope, DeviceView, FanCurveDocument, FanStateResponse } from "../types/api";
import { listDevices } from "../services/devices";
import { getFanState, listFanCurves, previewFanTemperatureSource } from "../services/fans";

vi.mock("../services/devices", () => ({
  listDevices: vi.fn(),
}));

vi.mock("../services/fans", () => ({
  getFanState: vi.fn(),
  listFanCurves: vi.fn(),
  previewFanTemperatureSource: vi.fn(),
  createFanCurve: vi.fn(),
  updateFanCurve: vi.fn(),
  deleteFanCurve: vi.fn(),
  applyFanWorkbench: vi.fn(),
}));

vi.mock("./useBackendEventSubscription", () => ({
  useBackendEventSubscription: vi.fn((_listener: (event: BackendEventEnvelope) => void) => {}),
}));

const listDevicesMock = vi.mocked(listDevices);
const getFanStateMock = vi.mocked(getFanState);
const listFanCurvesMock = vi.mocked(listFanCurves);
const previewFanTemperatureSourceMock = vi.mocked(previewFanTemperatureSource);

function fanDevice(id: string, name: string, overrides: Partial<DeviceView> = {}): DeviceView {
  return {
    id,
    name,
    display_name: name,
    ui_order: 10,
    physical_role: "Wireless cluster",
    capability_summary: "4 fan slot(s)",
    current_mode_summary: "Cooling telemetry live",
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
      has_rgb: false,
      has_lcd: false,
      has_pump: false,
      fan_count: 4,
      per_fan_control: false,
      mb_sync_support: false,
      rgb_zone_count: 0,
    },
    state: {
      fan_rpms: [900, 910, 920, 930],
      coolant_temp: null,
      streaming_active: false,
    },
    ...overrides,
  };
}

function fanCurve(name: string, temperatureSource = "sensors temperature --source coolant"): FanCurveDocument {
  return {
    name,
    temperature_source: temperatureSource,
    points: [
      { temperature_celsius: 28, percent: 25 },
      { temperature_celsius: 40, percent: 45 },
      { temperature_celsius: 58, percent: 72 },
    ],
  };
}

function fanState(
  deviceId: string,
  mode: FanStateResponse["active_mode"] = "manual",
  curveName: string | null = null,
  percent = 40,
): FanStateResponse {
  const slotMode = mode === "curve" ? "curve" : mode === "mb_sync" ? "mb_sync" : "manual";
  return {
    device_id: deviceId,
    update_interval_ms: 1000,
    rpms: [900, 910, 920, 930],
    slots: [
      {
        slot: 1,
        mode: slotMode,
        percent: slotMode === "manual" ? percent : null,
        pwm: slotMode === "manual" ? 102 : null,
        curve: curveName,
      },
      {
        slot: 2,
        mode: slotMode,
        percent: slotMode === "manual" ? percent : null,
        pwm: slotMode === "manual" ? 102 : null,
        curve: curveName,
      },
    ],
    active_mode: mode,
    active_curve: curveName,
    temperature_source: curveName ? "sensors temperature --source coolant" : null,
    mb_sync_enabled: mode === "mb_sync",
    start_stop_supported: false,
    start_stop_enabled: false,
  };
}

describe("useFansWorkbenchData", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    previewFanTemperatureSourceMock.mockResolvedValue({
      source: "cpu",
      display_name: "CPU temperature",
      available: true,
      celsius: 58.2,
      components: [{ key: "cpu", label: "CPU", kind: "cpu", available: true, celsius: 58.2 }],
    });
  });

  it("does not auto-select a device without an explicit request", async () => {
    listDevicesMock.mockResolvedValue([
      fanDevice("rgb-only", "RGB Only", {
        capabilities: {
          ...fanDevice("tmp", "tmp").capabilities,
          has_fan: false,
        },
      }),
      fanDevice("fan-primary", "Fan Primary"),
    ]);
    listFanCurvesMock.mockResolvedValue([fanCurve("Balanced curve")]);

    const { result } = renderHook(() => useFansWorkbenchData(null));

    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(result.current.selectedDeviceId).toBe("");
    expect(result.current.devices).toHaveLength(1);
    expect(result.current.fanState).toBeNull();
    expect(getFanStateMock).not.toHaveBeenCalled();
  });

  it("loads the requested device and seeds curve mode from backend state", async () => {
    listDevicesMock.mockResolvedValue([fanDevice("fan-primary", "Fan Primary")]);
    listFanCurvesMock.mockResolvedValue([fanCurve("Balanced curve")]);
    getFanStateMock.mockResolvedValue(fanState("fan-primary", "curve", "Balanced curve"));

    const { result } = renderHook(() => useFansWorkbenchData("fan-primary"));

    await waitFor(() => expect(result.current.fanState?.active_mode).toBe("curve"));

    expect(result.current.selectedDeviceId).toBe("fan-primary");
    expect(result.current.curveDraft.name).toBe("Balanced curve");
    expect(result.current.selectedCurveName).toBe("Balanced curve");
    expect(result.current.curves).toHaveLength(1);
    await waitFor(() => expect(result.current.temperaturePreview?.display_name).toBe("CPU temperature"));
  });

  it("surfaces safety and validation notices for unsupported or invalid drafts", async () => {
    listDevicesMock.mockResolvedValue([fanDevice("fan-primary", "Fan Primary")]);
    listFanCurvesMock.mockResolvedValue([fanCurve("Balanced curve")]);
    getFanStateMock.mockResolvedValue(fanState("fan-primary"));

    const { result } = renderHook(() => useFansWorkbenchData("fan-primary"));
    await waitFor(() => expect(result.current.fanState?.device_id).toBe("fan-primary"));

    act(() => {
      result.current.setControlDraft((current) => ({
        ...current,
        mode: "start_stop",
        manualPercent: 90,
      }));
      result.current.newCurve();
      result.current.setCurveDraft((current) => ({
        ...current,
        temperature_source: "",
        points: [{ temperature_celsius: 40, percent: 55 }],
      }));
    });

    const noticeTitles = result.current.notices.map((notice) => notice.title);
    expect(noticeTitles).toContain("Start / stop is not available yet");
    expect(noticeTitles).toContain("Curve validation");
    expect(result.current.notices.some((notice) => notice.message.includes("at least two curve points"))).toBe(true);
    expect(result.current.notices.some((notice) => notice.message.includes("Temperature source is required"))).toBe(true);
  });
});



