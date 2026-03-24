import { screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { DeviceDetailPage } from "./DeviceDetailPage";
import { renderAtRoute } from "../test/render";
import type {
  DeviceView,
  FanStateResponse,
  LightingStateResponse,
  ProfileDocument,
} from "../types/api";
import { useDeviceDetailData } from "../hooks/useDeviceDetailData";

vi.mock("../hooks/useDeviceDetailData", () => ({
  useDeviceDetailData: vi.fn(),
}));

const useDeviceDetailDataMock = vi.mocked(useDeviceDetailData);

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

function lightingState(): LightingStateResponse {
  return {
    device_id: "wireless:one",
    zones: [
      {
        zone: 0,
        effect: "Static",
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

function fanState(): FanStateResponse {
  return {
    device_id: "wireless:one",
    update_interval_ms: 1000,
    rpms: [900, 910, 920, 930],
    slots: [
      { slot: 1, mode: "manual", percent: 50, pwm: 128, curve: null },
      { slot: 2, mode: "manual", percent: 50, pwm: 128, curve: null },
      { slot: 3, mode: "manual", percent: 50, pwm: 128, curve: null },
      { slot: 4, mode: "manual", percent: 50, pwm: 128, curve: null },
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

describe("DeviceDetailPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders topology, capability, profile, lighting, and fan sections", () => {
    useDeviceDetailDataMock.mockReturnValue({
      deviceId: "wireless:one",
      device: device(),
      lightingState: lightingState(),
      fanState: fanState(),
      profiles: [profile()],
      loading: false,
      refreshing: false,
      error: null,
      lightingError: null,
      fanError: null,
      profileError: null,
      refresh: vi.fn(),
    });

    renderAtRoute(<DeviceDetailPage />, {
      initialPath: "/devices/wireless%3Aone",
      routePath: "/devices/:deviceId",
    });

    expect(screen.getByText("Identity")).toBeInTheDocument();
    expect(screen.getByText("Controller and wireless context")).toBeInTheDocument();
    expect(screen.getByText("Current lighting summary")).toBeInTheDocument();
    expect(screen.getByText("Current fan summary")).toBeInTheDocument();
    expect(screen.getByText("Assigned profiles")).toBeInTheDocument();
    expect(screen.getByText("Night Mode")).toBeInTheDocument();
    expect(screen.getByText("#112233")).toBeInTheDocument();
    expect(screen.getByText("1000 ms")).toBeInTheDocument();
  });

  it("renders the page-level error state", () => {
    useDeviceDetailDataMock.mockReturnValue({
      deviceId: "wireless:one",
      device: null,
      lightingState: null,
      fanState: null,
      profiles: [],
      loading: false,
      refreshing: false,
      error: "not found: unknown device id",
      lightingError: null,
      fanError: null,
      profileError: null,
      refresh: vi.fn(),
    });

    renderAtRoute(<DeviceDetailPage />, {
      initialPath: "/devices/wireless%3Aone",
      routePath: "/devices/:deviceId",
    });

    expect(screen.getByText("Device detail load failed.")).toBeInTheDocument();
    expect(screen.getByText("not found: unknown device id")).toBeInTheDocument();
  });

  it("surfaces profile loading warnings separately", () => {
    useDeviceDetailDataMock.mockReturnValue({
      deviceId: "wireless:one",
      device: device(),
      lightingState: lightingState(),
      fanState: fanState(),
      profiles: [],
      loading: false,
      refreshing: false,
      error: null,
      lightingError: null,
      fanError: null,
      profileError: "profile assignments unavailable",
      refresh: vi.fn(),
    });

    renderAtRoute(<DeviceDetailPage />, {
      initialPath: "/devices/wireless%3Aone",
      routePath: "/devices/:deviceId",
    });

    expect(screen.getByText("Profiles unavailable.")).toBeInTheDocument();
    expect(screen.getByText("profile assignments unavailable")).toBeInTheDocument();
  });
});


