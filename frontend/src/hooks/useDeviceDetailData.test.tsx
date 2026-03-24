import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useDeviceDetailData } from "./useDeviceDetailData";
import type {
  BackendEventEnvelope,
  DeviceView,
  FanStateResponse,
  LightingStateResponse,
  ProfileDocument,
} from "../types/api";
import { getDevice } from "../services/devices";
import { getFanState } from "../services/fans";
import { getLightingState } from "../services/lighting";
import { listProfiles } from "../services/profiles";

vi.mock("../services/devices", () => ({
  getDevice: vi.fn(),
}));

vi.mock("../services/fans", () => ({
  getFanState: vi.fn(),
}));

vi.mock("../services/lighting", () => ({
  getLightingState: vi.fn(),
}));

vi.mock("../services/profiles", () => ({
  listProfiles: vi.fn(),
}));

let latestBackendListener: ((event: BackendEventEnvelope) => void) | null = null;

vi.mock("./useBackendEventSubscription", () => ({
  useBackendEventSubscription: vi.fn((listener: (event: BackendEventEnvelope) => void) => {
    latestBackendListener = listener;
  }),
}));

const getDeviceMock = vi.mocked(getDevice);
const getFanStateMock = vi.mocked(getFanState);
const getLightingStateMock = vi.mocked(getLightingState);
const listProfilesMock = vi.mocked(listProfiles);

function emitBackendEvent(event: Partial<BackendEventEnvelope> & Pick<BackendEventEnvelope, "type">) {
  latestBackendListener?.({
    timestamp: "2026-03-14T10:00:00Z",
    source: "ws",
    device_id: null,
    data: {},
    ...event,
  });
}

function device(overrides: Partial<DeviceView> = {}): DeviceView {
  return {
    id: "wireless:one",
    name: "Controller One",
    display_name: "Controller One",
    ui_order: 10,
    physical_role: "Wireless cluster",
    capability_summary: "1 RGB zone(s) | 4 fan slot(s)",
    current_mode_summary: "Lighting ready | Cooling telemetry live",
    controller: {
      id: "wireless:mesh",
      label: "Wireless mesh",
      kind: "wireless_mesh",
    },
    wireless: {
      transport: "wireless",
      channel: 8,
      group_id: "wireless:one",
      group_label: "Controller One",
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
      rgb_zone_count: 1,
    },
    state: {
      fan_rpms: [900, 910, 920, 930],
      coolant_temp: 31.5,
      streaming_active: false,
    },
    ...overrides,
  };
}

function lightingState(effect = "Static"): LightingStateResponse {
  return {
    device_id: "wireless:one",
    zones: [
      {
        zone: 0,
        effect,
        colors: ["#112233"],
        speed: 2,
        brightness_percent: 75,
        direction: "Clockwise",
        scope: "All",
        smoothness_ms: 0,
      },
    ],
  };
}

function fanState(percent = 50): FanStateResponse {
  return {
    device_id: "wireless:one",
    update_interval_ms: 1000,
    rpms: [900, 910, 920, 930],
    slots: [
      { slot: 1, mode: "manual", percent, pwm: 128, curve: null },
      { slot: 2, mode: "manual", percent, pwm: 128, curve: null },
      { slot: 3, mode: "manual", percent, pwm: 128, curve: null },
      { slot: 4, mode: "manual", percent, pwm: 128, curve: null },
    ],
  };
}

function profile(): ProfileDocument {
  return {
    id: "night-mode",
    name: "Night Mode",
    description: "Dim the desk fans",
    targets: {
      mode: "devices",
      device_ids: ["wireless:one"],
    },
    metadata: {
      created_at: "2026-03-14T10:00:00Z",
      updated_at: "2026-03-14T10:00:00Z",
    },
  };
}

describe("useDeviceDetailData", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    latestBackendListener = null;
    listProfilesMock.mockResolvedValue([profile()]);
  });

  it("loads device, lighting, fan, and profile data and decodes the route device id", async () => {
    getDeviceMock.mockResolvedValue(device());
    getLightingStateMock.mockResolvedValue(lightingState());
    getFanStateMock.mockResolvedValue(fanState());

    const { result } = renderHook(() => useDeviceDetailData("wireless%3Aone"));

    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(result.current.deviceId).toBe("wireless:one");
    expect(result.current.device?.name).toBe("Controller One");
    expect(result.current.lightingState?.zones[0]?.effect).toBe("Static");
    expect(result.current.fanState?.slots[0]?.percent).toBe(50);
    expect(result.current.profiles).toHaveLength(1);
    expect(result.current.error).toBeNull();
  });

  it("keeps partial device detail data when one secondary request fails", async () => {
    getDeviceMock.mockResolvedValue(device());
    getLightingStateMock.mockRejectedValue(new Error("lighting backend unavailable"));
    getFanStateMock.mockResolvedValue(fanState());

    const { result } = renderHook(() => useDeviceDetailData("wireless%3Aone"));

    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(result.current.device?.id).toBe("wireless:one");
    expect(result.current.error).toBeNull();
    expect(result.current.lightingError).toBe("lighting backend unavailable");
    expect(result.current.fanError).toBeNull();
    expect(result.current.profileError).toBeNull();
    expect(result.current.fanState?.slots[0]?.percent).toBe(50);
  });

  it("refreshes the snapshot in the background when a matching backend event arrives", async () => {
    getDeviceMock
      .mockResolvedValueOnce(device())
      .mockResolvedValueOnce(
        device({
          state: { fan_rpms: [950, 960, 970, 980], coolant_temp: 32, streaming_active: true },
        }),
      );
    getLightingStateMock
      .mockResolvedValueOnce(lightingState("Static"))
      .mockResolvedValueOnce(lightingState("Rainbow"));
    getFanStateMock.mockResolvedValueOnce(fanState(50)).mockResolvedValueOnce(fanState(65));
    listProfilesMock.mockResolvedValueOnce([profile()]).mockResolvedValueOnce([profile()]);

    const { result } = renderHook(() => useDeviceDetailData("wireless%3Aone"));
    await waitFor(() => expect(result.current.lightingState?.zones[0]?.effect).toBe("Static"));

    act(() => {
      emitBackendEvent({
        type: "config.changed",
        device_id: "wireless:one",
      });
    });

    await waitFor(() => expect(result.current.lightingState?.zones[0]?.effect).toBe("Rainbow"));
    expect(result.current.fanState?.slots[0]?.percent).toBe(65);
    expect(getDeviceMock).toHaveBeenCalledTimes(2);
    expect(listProfilesMock).toHaveBeenCalledTimes(2);
  });
});


