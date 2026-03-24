import { renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useDeviceInventoryData } from "./useDeviceInventoryData";
import type { DeviceView, ProfileDocument } from "../types/api";
import { listDevices } from "../services/devices";
import { listProfiles } from "../services/profiles";

vi.mock("../services/devices", () => ({
  listDevices: vi.fn(),
  updateDevicePresentation: vi.fn(),
}));

vi.mock("../services/profiles", () => ({
  listProfiles: vi.fn(),
  applyProfile: vi.fn(),
}));

vi.mock("../services/lighting", () => ({
  setLightingColor: vi.fn(),
}));

vi.mock("./useBackendEventSubscription", () => ({
  useBackendEventSubscription: vi.fn(),
}));

const listDevicesMock = vi.mocked(listDevices);
const listProfilesMock = vi.mocked(listProfiles);

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
      coolant_temp: null,
      streaming_active: false,
    },
    ...overrides,
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

describe("useDeviceInventoryData", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("loads device inventory and matching profiles", async () => {
    listDevicesMock.mockResolvedValue([device()]);
    listProfilesMock.mockResolvedValue([profile()]);

    const { result } = renderHook(() => useDeviceInventoryData());

    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(result.current.devices).toHaveLength(1);
    expect(result.current.profiles).toHaveLength(1);
    expect(result.current.error).toBeNull();
    expect(result.current.profileError).toBeNull();
  });

  it("keeps inventory visible when only profile loading fails", async () => {
    listDevicesMock.mockResolvedValue([device()]);
    listProfilesMock.mockRejectedValue(new Error("profiles unavailable"));

    const { result } = renderHook(() => useDeviceInventoryData());

    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(result.current.devices).toHaveLength(1);
    expect(result.current.error).toBeNull();
    expect(result.current.profileError).toBe("profiles unavailable");
    expect(result.current.profiles).toEqual([]);
  });
});


