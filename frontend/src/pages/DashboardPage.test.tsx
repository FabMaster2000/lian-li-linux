import { screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { DashboardPage } from "./DashboardPage";
import { renderAtRoute } from "../test/render";
import { useMvpDashboardData } from "../hooks/useMvpDashboardData";
import type { MvpCluster } from "../features/mvpClusters";

vi.mock("../hooks/useMvpDashboardData", () => ({
  useMvpDashboardData: vi.fn(),
}));

const useMvpDashboardDataMock = vi.mocked(useMvpDashboardData);

function cluster(overrides: Partial<MvpCluster> = {}): MvpCluster {
  return {
    id: "desk-cluster",
    label: "Desk cluster",
    deviceIds: ["wireless:one"],
    primaryDeviceId: "wireless:one",
    fanCount: 4,
    fanType: "SlInf",
    status: "healthy",
    devices: [],
    primaryDevice: {
      id: "wireless:one",
      name: "wireless:one",
      display_name: "Desk cluster",
      family: "SlInf",
      online: true,
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
        group_id: "desk-cluster",
        group_label: "Desk cluster",
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
    ...overrides,
  };
}

describe("DashboardPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders only the paired-cluster MVP card with fans, rgb, and disconnect actions", () => {
    useMvpDashboardDataMock.mockReturnValue({
      clusters: [cluster()],
      fanStates: {
        "desk-cluster": {
          device_id: "wireless:one",
          update_interval_ms: 1000,
          rpms: [900, 910, 920, 930],
          slots: [],
          active_mode: "manual",
        },
      },
      lightingStates: {
        "desk-cluster": {
          device_id: "wireless:one",
          zones: [
            {
              zone: 0,
              effect: "Static",
              colors: ["#112233"],
              speed: 2,
              brightness_percent: 100,
              direction: "Clockwise",
              scope: "All",
              smoothness_ms: 0,
            },
          ],
        },
      },
      loading: false,
      refreshing: false,
      error: null,
      lastUpdated: "2026-03-17T10:00:00Z",
      actionError: null,
      actionSuccess: null,
      disconnectingClusterId: null,
      refresh: vi.fn(),
      disconnectCluster: vi.fn(),
    });

    renderAtRoute(<DashboardPage />);

    expect(screen.getByText("Dashboard")).toBeInTheDocument();
    expect(screen.getByText("desk-cluster")).toBeInTheDocument();
    expect(screen.getByText("4")).toBeInTheDocument();
    expect(screen.getAllByText("healthy").length).toBeGreaterThan(0);
    expect(screen.getByText("900 / 910 / 920 / 930")).toBeInTheDocument();
    expect(screen.getByText("Static #112233")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Fans" })).toHaveAttribute(
      "href",
      "/fans?cluster=desk-cluster",
    );
    expect(screen.getByRole("link", { name: "RGB" })).toHaveAttribute(
      "href",
      "/rgb?cluster=desk-cluster",
    );
    expect(screen.getByRole("button", { name: "Entkoppeln" })).toBeInTheDocument();
  });

  it("renders the reduced empty state when no paired clusters exist", () => {
    useMvpDashboardDataMock.mockReturnValue({
      clusters: [],
      fanStates: {},
      lightingStates: {},
      loading: false,
      refreshing: false,
      error: null,
      lastUpdated: null,
      actionError: null,
      actionSuccess: null,
      disconnectingClusterId: null,
      refresh: vi.fn(),
      disconnectCluster: vi.fn(),
    });

    renderAtRoute(<DashboardPage />);

    expect(screen.getByText("Keine gekoppelten Geräte")).toBeInTheDocument();
  });
});
