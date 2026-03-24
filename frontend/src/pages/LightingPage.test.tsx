import { screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { LightingPage } from "./LightingPage";
import { renderAtRoute } from "../test/render";
import { useMvpRgbPageData } from "../hooks/useMvpRgbPageData";
import type { MvpCluster } from "../features/mvpClusters";

vi.mock("../hooks/useMvpRgbPageData", () => ({
  useMvpRgbPageData: vi.fn(),
}));

const useMvpRgbPageDataMock = vi.mocked(useMvpRgbPageData);

function cluster(): MvpCluster {
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
  };
}

describe("LightingPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders the reduced RGB MVP surface", () => {
    useMvpRgbPageDataMock.mockReturnValue({
      clusters: [cluster()],
      selectedClusterId: "desk-cluster",
      setSelectedClusterId: vi.fn(),
      selectedCluster: cluster(),
      lightingState: {
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
      loading: false,
      refreshing: false,
      stateLoading: false,
      stateRefreshing: false,
      applying: false,
      error: null,
      success: null,
      effect: "Meteor" as const,
      setEffect: vi.fn(),
      color: "#aabbcc",
      setColor: vi.fn(),
      routeDraft: [
        { key: "wireless:one::1", deviceId: "wireless:one", fanIndex: 1, label: "Desk cluster · Lüfter 1" },
        { key: "wireless:one::2", deviceId: "wireless:one", fanIndex: 2, label: "Desk cluster · Lüfter 2" },
      ],
      reorderRouteEntry: vi.fn(),
      dirty: true,
      rgbSummary: "Static #112233",
      refresh: vi.fn(),
      applyChanges: vi.fn(),
      resetDraft: vi.fn(),
    });

    renderAtRoute(<LightingPage />, {
      initialPath: "/rgb?cluster=desk-cluster",
      routePath: "/rgb",
    });

    expect(screen.getByText("RGB")).toBeInTheDocument();
    expect(screen.getByText("Aktueller RGB-Status")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "RGB-Einstellung" })).toBeInTheDocument();
    expect(screen.getByText("Static #112233")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Übernehmen" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Lüfterreihenfolge" })).toBeInTheDocument();
    expect(screen.getByText("Desk cluster · Lüfter 1")).toBeInTheDocument();
    expect(screen.getByText("Desk cluster · Lüfter 2")).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Auf alle Cluster übertragen" }),
    ).not.toBeInTheDocument();
    expect(screen.queryByText("Colors and effects")).not.toBeInTheDocument();
    expect(screen.queryByText("Quick presets")).not.toBeInTheDocument();
  });

  it("renders the no-data state when no paired RGB clusters are available", () => {
    useMvpRgbPageDataMock.mockReturnValue({
      clusters: [],
      selectedClusterId: "",
      setSelectedClusterId: vi.fn(),
      selectedCluster: null,
      lightingState: null,
      loading: false,
      refreshing: false,
      stateLoading: false,
      stateRefreshing: false,
      applying: false,
      error: null,
      success: null,
      effect: "Meteor" as const,
      setEffect: vi.fn(),
      color: "#ffffff",
      setColor: vi.fn(),
      routeDraft: [],
      reorderRouteEntry: vi.fn(),
      dirty: false,
      rgbSummary: "n/a",
      refresh: vi.fn(),
      applyChanges: vi.fn(),
      resetDraft: vi.fn(),
    });

    renderAtRoute(<LightingPage />, {
      initialPath: "/rgb",
      routePath: "/rgb",
    });

    expect(screen.getByText("Keine gekoppelten Geräte")).toBeInTheDocument();
  });
});
