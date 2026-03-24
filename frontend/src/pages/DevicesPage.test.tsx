import { screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { DevicesPage } from "./DevicesPage";
import { renderAtRoute } from "../test/render";
import type { DeviceView, ProfileDocument } from "../types/api";
import { useDeviceInventoryData } from "../hooks/useDeviceInventoryData";

vi.mock("../hooks/useDeviceInventoryData", () => ({
  useDeviceInventoryData: vi.fn(),
}));

const useDeviceInventoryDataMock = vi.mocked(useDeviceInventoryData);

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

describe("DevicesPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders a simplified inventory view with device and fan information", () => {
    useDeviceInventoryDataMock.mockReturnValue({
      devices: [device()],
      profiles: [profile()],
      loading: false,
      refreshing: false,
      error: null,
      profileError: null,
      actionError: null,
      actionSuccess: null,
      savingDeviceId: null,
      colorDeviceId: null,
      profileDeviceId: null,
      refresh: vi.fn(),
      savePresentation: vi.fn(),
      setStaticColorForDevice: vi.fn(),
      applyProfileToDevice: vi.fn(),
    });

    renderAtRoute(<DevicesPage />, {
      initialPath: "/devices",
      routePath: "/devices",
    });

    expect(screen.getByText("Devices and fan status")).toBeInTheDocument();
    expect(screen.getByText("Device filters")).toBeInTheDocument();
    expect(screen.getByText("Connected devices")).toBeInTheDocument();
    expect(screen.getByText("Controller One")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Details" })).toHaveAttribute(
      "href",
      "/devices/wireless%3Aone",
    );
    expect(screen.getByRole("link", { name: "Lighting" })).toHaveAttribute(
      "href",
      "/lighting?device=wireless%3Aone",
    );
    expect(screen.getByRole("link", { name: "Fans" })).toHaveAttribute(
      "href",
      "/fans?device=wireless%3Aone",
    );
    expect(screen.getByText("4 fans cluster | id one")).toBeInTheDocument();
    expect(screen.getByText("4 fans")).toBeInTheDocument();
    expect(screen.getByText("1 zone")).toBeInTheDocument();
    expect(screen.getByText("900 / 910 / 920 / 930")).toBeInTheDocument();
  });

  it("shows an error banner when the inventory cannot be loaded", () => {
    useDeviceInventoryDataMock.mockReturnValue({
      devices: [],
      profiles: [],
      loading: false,
      refreshing: false,
      error: "device inventory unavailable",
      profileError: null,
      actionError: null,
      actionSuccess: null,
      savingDeviceId: null,
      colorDeviceId: null,
      profileDeviceId: null,
      refresh: vi.fn(),
      savePresentation: vi.fn(),
      setStaticColorForDevice: vi.fn(),
      applyProfileToDevice: vi.fn(),
    });

    renderAtRoute(<DevicesPage />, {
      initialPath: "/devices",
      routePath: "/devices",
    });

    expect(screen.getByText("Device inventory load failed.")).toBeInTheDocument();
    expect(screen.getByText("device inventory unavailable")).toBeInTheDocument();
  });

  it("keeps the inventory focused even when profile data is unavailable", () => {
    useDeviceInventoryDataMock.mockReturnValue({
      devices: [device()],
      profiles: [],
      loading: false,
      refreshing: false,
      error: null,
      profileError: "profiles unavailable",
      actionError: null,
      actionSuccess: null,
      savingDeviceId: null,
      colorDeviceId: null,
      profileDeviceId: null,
      refresh: vi.fn(),
      savePresentation: vi.fn(),
      setStaticColorForDevice: vi.fn(),
      applyProfileToDevice: vi.fn(),
    });

    renderAtRoute(<DevicesPage />, {
      initialPath: "/devices",
      routePath: "/devices",
    });

    expect(screen.getByText("Controller One")).toBeInTheDocument();
    expect(screen.queryByText("Profiles unavailable.")).not.toBeInTheDocument();
  });

  it("keeps unpaired wireless devices out of the main inventory list", () => {
    useDeviceInventoryDataMock.mockReturnValue({
      devices: [
        device(),
        device({
          id: "wireless:ready",
          display_name: "Ready cluster",
          name: "Ready cluster",
          wireless: {
            transport: "wireless",
            channel: 8,
            group_id: "wireless:ready",
            group_label: "Ready cluster",
            binding_state: "available",
            master_mac: null,
          },
        }),
      ],
      profiles: [],
      loading: false,
      refreshing: false,
      error: null,
      profileError: null,
      actionError: null,
      actionSuccess: null,
      savingDeviceId: null,
      colorDeviceId: null,
      profileDeviceId: null,
      refresh: vi.fn(),
      savePresentation: vi.fn(),
      setStaticColorForDevice: vi.fn(),
      applyProfileToDevice: vi.fn(),
    });

    renderAtRoute(<DevicesPage />, {
      initialPath: "/devices",
      routePath: "/devices",
    });

    expect(screen.getByText("Controller One")).toBeInTheDocument();
    expect(screen.queryByText("Ready cluster")).not.toBeInTheDocument();
  });
});


