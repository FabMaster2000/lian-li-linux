import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useDashboardData } from "./useDashboardData";
import type {
  BackendEventEnvelope,
  DaemonStatusResponse,
  DeviceView,
  RuntimeResponse,
} from "../types/api";
import { listDevices } from "../services/devices";
import { getDaemonStatus, getRuntime } from "../services/system";

vi.mock("../services/devices", () => ({
  listDevices: vi.fn(),
}));

vi.mock("../services/system", () => ({
  getDaemonStatus: vi.fn(),
  getRuntime: vi.fn(),
}));

let latestBackendListener: ((event: BackendEventEnvelope) => void) | null = null;

vi.mock("./useBackendEventSubscription", () => ({
  useBackendEventSubscription: vi.fn((listener: (event: BackendEventEnvelope) => void) => {
    latestBackendListener = listener;
  }),
}));

const listDevicesMock = vi.mocked(listDevices);
const getDaemonStatusMock = vi.mocked(getDaemonStatus);
const getRuntimeMock = vi.mocked(getRuntime);

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
      coolant_temp: null,
      streaming_active: false,
    },
    ...overrides,
  };
}

function daemonStatus(): DaemonStatusResponse {
  return {
    reachable: true,
    socket_path: "/tmp/lianli-daemon.sock",
    error: null,
  };
}

function runtime(overrides: Partial<RuntimeResponse> = {}): RuntimeResponse {
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
    ...overrides,
  };
}

describe("useDashboardData", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    latestBackendListener = null;
  });

  it("loads the combined dashboard snapshot", async () => {
    listDevicesMock.mockResolvedValue([device()]);
    getDaemonStatusMock.mockResolvedValue(daemonStatus());
    getRuntimeMock.mockResolvedValue(runtime());

    const { result } = renderHook(() => useDashboardData());

    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(result.current.devices).toHaveLength(1);
    expect(result.current.daemonStatus?.reachable).toBe(true);
    expect(result.current.runtime?.backend.log_level).toBe("info");
    expect(result.current.error).toBeNull();
  });

  it("refreshes in the background when a device event arrives", async () => {
    listDevicesMock
      .mockResolvedValueOnce([device({ name: "Controller One" })])
      .mockResolvedValueOnce([device({ name: "Controller Two" })]);
    getDaemonStatusMock.mockResolvedValue(daemonStatus());
    getRuntimeMock.mockResolvedValue(runtime());

    const { result } = renderHook(() => useDashboardData());
    await waitFor(() => expect(result.current.devices[0]?.name).toBe("Controller One"));

    act(() => {
      emitBackendEvent({
        type: "fan.changed",
        device_id: "wireless:one",
      });
    });

    await waitFor(() => expect(result.current.devices[0]?.name).toBe("Controller Two"));
    expect(listDevicesMock).toHaveBeenCalledTimes(2);
  });

  it("surfaces combined-snapshot load failures", async () => {
    listDevicesMock.mockResolvedValue([device()]);
    getDaemonStatusMock.mockResolvedValue(daemonStatus());
    getRuntimeMock.mockRejectedValue(new Error("runtime unavailable"));

    const { result } = renderHook(() => useDashboardData());

    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(result.current.error).toBe("runtime unavailable");
    expect(result.current.runtime).toBeNull();
  });
});


