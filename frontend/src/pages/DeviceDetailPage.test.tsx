import { screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { DeviceDetailPage } from "./DeviceDetailPage";
import { renderAtRoute } from "../test/render";
import type {
  DeviceView,
  FanStateResponse,
  LightingStateResponse,
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

describe("DeviceDetailPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders general, lighting, and fan sections from device detail data", () => {
    useDeviceDetailDataMock.mockReturnValue({
      deviceId: "wireless:one",
      device: device(),
      lightingState: lightingState(),
      fanState: fanState(),
      loading: false,
      refreshing: false,
      error: null,
      lightingError: null,
      fanError: null,
      refresh: vi.fn(),
    });

    renderAtRoute(<DeviceDetailPage />, {
      initialPath: "/devices/wireless%3Aone",
      routePath: "/devices/:deviceId",
    });

    expect(screen.getByText("General information")).toBeInTheDocument();
    expect(screen.getByText("Current lighting state")).toBeInTheDocument();
    expect(screen.getByText("Current fan state")).toBeInTheDocument();
    expect(screen.getByText("#112233")).toBeInTheDocument();
    expect(screen.getByText("1000 ms")).toBeInTheDocument();
  });

  it("renders the page-level error state", () => {
    useDeviceDetailDataMock.mockReturnValue({
      deviceId: "wireless:one",
      device: null,
      lightingState: null,
      fanState: null,
      loading: false,
      refreshing: false,
      error: "not found: unknown device id",
      lightingError: null,
      fanError: null,
      refresh: vi.fn(),
    });

    renderAtRoute(<DeviceDetailPage />, {
      initialPath: "/devices/wireless%3Aone",
      routePath: "/devices/:deviceId",
    });

    expect(screen.getByText("Device detail load failed.")).toBeInTheDocument();
    expect(screen.getByText("not found: unknown device id")).toBeInTheDocument();
  });

  it("shows an offline banner when the device is reported offline", () => {
    useDeviceDetailDataMock.mockReturnValue({
      deviceId: "wireless:one",
      device: device({ online: false }),
      lightingState: lightingState(),
      fanState: fanState(),
      loading: false,
      refreshing: false,
      error: null,
      lightingError: null,
      fanError: null,
      refresh: vi.fn(),
    });

    renderAtRoute(<DeviceDetailPage />, {
      initialPath: "/devices/wireless%3Aone",
      routePath: "/devices/:deviceId",
    });

    expect(screen.getByText("Device offline.")).toBeInTheDocument();
    expect(
      screen.getByText(
        "The last reported snapshot may be stale until the controller comes back online.",
      ),
    ).toBeInTheDocument();
  });
});
