import { screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { DashboardPage } from "./DashboardPage";
import { renderAtRoute } from "../test/render";
import type { DaemonStatusResponse, DeviceView, RuntimeResponse } from "../types/api";
import { useDashboardData } from "../hooks/useDashboardData";

vi.mock("../hooks/useDashboardData", () => ({
  useDashboardData: vi.fn(),
}));

const useDashboardDataMock = vi.mocked(useDashboardData);

function dashboardDevice(overrides: Partial<DeviceView> = {}): DeviceView {
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
      coolant_temp: null,
      streaming_active: false,
    },
    ...overrides,
  };
}

function runtime(): RuntimeResponse {
  return {
    backend: {
      host: "127.0.0.1",
      port: 9000,
      log_level: "info",
      config_path: "/tmp/backend.json",
      profile_store_path: "/tmp/profiles.json",
      auth: {
        enabled: false,
        mode: "none",
        reload_requires_restart: true,
        basic_username_configured: false,
        basic_password_configured: false,
        token_configured: false,
        proxy_header: null,
      },
    },
    daemon: {
      socket_path: "/tmp/lianli-daemon.sock",
      config_path: "/tmp/config.json",
      xdg_runtime_dir: "/tmp",
      xdg_config_home: "/tmp",
    },
  };
}

function daemonStatus(): DaemonStatusResponse {
  return {
    reachable: true,
    socket_path: "/tmp/lianli-daemon.sock",
    error: null,
  };
}

describe("DashboardPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders device cards and metrics from dashboard data", () => {
    useDashboardDataMock.mockReturnValue({
      devices: [dashboardDevice()],
      daemonStatus: daemonStatus(),
      runtime: runtime(),
      loading: false,
      refreshing: false,
      error: null,
      lastUpdated: "2026-03-14T10:00:00Z",
      refresh: vi.fn(),
    });

    renderAtRoute(<DashboardPage />);

    expect(screen.getByText("Fleet dashboard and control entry points")).toBeInTheDocument();
    expect(screen.getByText("Controller One")).toBeInTheDocument();
    expect(screen.getByText("Detected controllers and wireless targets")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Details" })).toHaveAttribute(
      "href",
      "/devices",
    );
    expect(screen.getByRole("link", { name: "Lighting" })).toHaveAttribute("href", "/lighting");
    expect(screen.getByRole("link", { name: "Fans" })).toHaveAttribute("href", "/fans");
  });

  it("renders an error banner when dashboard loading fails", () => {
    useDashboardDataMock.mockReturnValue({
      devices: [],
      daemonStatus: null,
      runtime: null,
      loading: false,
      refreshing: false,
      error: "daemon unavailable",
      lastUpdated: null,
      refresh: vi.fn(),
    });

    renderAtRoute(<DashboardPage />);

    expect(screen.getByText("Dashboard load failed.")).toBeInTheDocument();
    expect(screen.getByText("daemon unavailable")).toBeInTheDocument();
  });
});
