import { screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { FansPage } from "./FansPage";
import { renderAtRoute } from "../test/render";
import { useMvpFansPageData } from "../hooks/useMvpFansPageData";
import type { MvpCluster } from "../features/mvpClusters";

vi.mock("../hooks/useMvpFansPageData", () => ({
  useMvpFansPageData: vi.fn(),
}));

const useMvpFansPageDataMock = vi.mocked(useMvpFansPageData);

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

describe("FansPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders the reduced MVP fan controls", () => {
    useMvpFansPageDataMock.mockReturnValue({
      clusters: [cluster()],
      selectedClusterId: "desk-cluster",
      setSelectedClusterId: vi.fn(),
      selectedCluster: cluster(),
      fanState: {
        device_id: "wireless:one",
        update_interval_ms: 1000,
        rpms: [900, 910, 920, 930],
        slots: [],
        active_mode: "manual",
      },
      loading: false,
      refreshing: false,
      stateLoading: false,
      stateRefreshing: false,
      applying: false,
      error: null,
      success: null,
      dirty: true,
      draft: {
        mode: "curve",
        manualPercent: 55,
        curveSource: "cpu",
        points: [
          { temperature_celsius: 30, percent: 30 },
          { temperature_celsius: 50, percent: 55 },
          { temperature_celsius: 70, percent: 80 },
        ],
      },
      refresh: vi.fn(),
      applyChanges: vi.fn(),
      resetDraft: vi.fn(),
      setMode: vi.fn(),
      setManualPercent: vi.fn(),
      setCurveSource: vi.fn(),
      updateCurvePoint: vi.fn(),
      addCurvePoint: vi.fn(),
      removeCurvePoint: vi.fn(),
    });

    renderAtRoute(<FansPage />, {
      initialPath: "/fans?cluster=desk-cluster",
      routePath: "/fans",
    });

    expect(screen.getByText("Fans")).toBeInTheDocument();
    expect(screen.getByText("Cluster auswählen")).toBeInTheDocument();
    expect(screen.getByText("Live-Status")).toBeInTheDocument();
    expect(screen.getByText("Lüftersteuerung")).toBeInTheDocument();
    expect(screen.getByText("900 / 910 / 920 / 930")).toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "Temperaturquelle" })).toHaveValue("cpu");
    expect(screen.getByText("Kurvenpunkte")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Punkt hinzufügen" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Übernehmen" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Zurücksetzen" })).toBeInTheDocument();
    expect(screen.queryByText("Curve library")).not.toBeInTheDocument();
    expect(screen.queryByText("Target selection")).not.toBeInTheDocument();
  });

  it("renders the no-data state when no paired clusters are available", () => {
    useMvpFansPageDataMock.mockReturnValue({
      clusters: [],
      selectedClusterId: "",
      setSelectedClusterId: vi.fn(),
      selectedCluster: null,
      fanState: null,
      loading: false,
      refreshing: false,
      stateLoading: false,
      stateRefreshing: false,
      applying: false,
      error: null,
      success: null,
      dirty: false,
      draft: {
        mode: "manual",
        manualPercent: 50,
        curveSource: "cpu",
        points: [],
      },
      refresh: vi.fn(),
      applyChanges: vi.fn(),
      resetDraft: vi.fn(),
      setMode: vi.fn(),
      setManualPercent: vi.fn(),
      setCurveSource: vi.fn(),
      updateCurvePoint: vi.fn(),
      addCurvePoint: vi.fn(),
      removeCurvePoint: vi.fn(),
    });

    renderAtRoute(<FansPage />, {
      initialPath: "/fans",
      routePath: "/fans",
    });

    expect(screen.getByText("Keine gekoppelten Geräte")).toBeInTheDocument();
  });
});
