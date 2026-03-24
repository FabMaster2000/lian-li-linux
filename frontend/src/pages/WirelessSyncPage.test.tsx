import { screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { WirelessSyncPage } from "./WirelessSyncPage";
import { renderAtRoute } from "../test/render";
import { useMvpWirelessSyncData } from "../hooks/useMvpWirelessSyncData";
import type { MvpCluster } from "../features/mvpClusters";

vi.mock("../hooks/useMvpWirelessSyncData", () => ({
  useMvpWirelessSyncData: vi.fn(),
}));

const useMvpWirelessSyncDataMock = vi.mocked(useMvpWirelessSyncData);

function cluster(id: string, status: "healthy" | "offline" = "healthy"): MvpCluster {
  return {
    id,
    label: id,
    deviceIds: [`wireless:${id}`],
    primaryDeviceId: `wireless:${id}`,
    fanCount: 4,
    fanType: "SlInf",
    status,
    devices: [],
    primaryDevice: {
      id: `wireless:${id}`,
      name: `wireless:${id}`,
      display_name: id,
      family: "SlInf",
      online: status === "healthy",
      ui_order: 10,
      physical_role: "Wireless cluster",
      capability_summary: "RGB | Fan",
      current_mode_summary: "Ready",
      controller: {
        id: "wireless:mesh",
        label: "Desk mesh",
        kind: "wireless_mesh",
      },
      wireless: {
        transport: "wireless",
        channel: 8,
        group_id: id,
        group_label: id,
        binding_state: "connected",
        master_mac: "aa:bb",
      },
      health: {
        level: "healthy",
        summary: "Healthy",
      },
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
    },
  };
}

describe("WirelessSyncPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders the two-tab MVP wireless sync view", () => {
    useMvpWirelessSyncDataMock.mockReturnValue({
      pairedClusters: [cluster("desk-cluster")],
      availableClusters: [cluster("new-cluster")],
      daemonStatus: {
        reachable: true,
        socket_path: "/tmp/lianli.sock",
        error: null,
      },
      loading: false,
      refreshing: false,
      error: null,
      lastUpdated: "2026-03-17T10:00:00Z",
      actionError: null,
      actionSuccess: null,
      searching: false,
      connectingClusterId: null,
      disconnectingClusterId: null,
      refresh: vi.fn(),
      searchForDevices: vi.fn(),
      connectCluster: vi.fn(),
      disconnectCluster: vi.fn(),
    });

    renderAtRoute(<WirelessSyncPage />, {
      initialPath: "/wireless-sync",
      routePath: "/wireless-sync",
    });

    expect(screen.getByText("Wireless-Sync")).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "Gerät koppeln" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "Gekoppelte Geräte" })).toBeInTheDocument();
    expect(screen.getByText("new-cluster")).toBeInTheDocument();
    expect(screen.getByText("SlInf")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Gerät koppeln" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Geräte suchen" })).toBeInTheDocument();
  });

  it("shows the reduced empty state for missing pairable devices", () => {
    useMvpWirelessSyncDataMock.mockReturnValue({
      pairedClusters: [],
      availableClusters: [],
      daemonStatus: {
        reachable: true,
        socket_path: "/tmp/lianli.sock",
        error: null,
      },
      loading: false,
      refreshing: false,
      error: null,
      lastUpdated: null,
      actionError: null,
      actionSuccess: null,
      searching: false,
      connectingClusterId: null,
      disconnectingClusterId: null,
      refresh: vi.fn(),
      searchForDevices: vi.fn(),
      connectCluster: vi.fn(),
      disconnectCluster: vi.fn(),
    });

    renderAtRoute(<WirelessSyncPage />, {
      initialPath: "/wireless-sync",
      routePath: "/wireless-sync",
    });

    expect(screen.getByText("Keine koppelbaren Geräte gefunden")).toBeInTheDocument();
  });
});
