import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useWirelessSyncData } from "./useWirelessSyncData";
import {
  connectWirelessDevice,
  disconnectWirelessDevice,
  listDevices,
  refreshWirelessDiscovery,
  updateDevicePresentation,
} from "../services/devices";
import { getDaemonStatus, getRuntime } from "../services/system";
import type { DeviceView, RuntimeResponse } from "../types/api";

vi.mock("../services/devices", () => ({
  connectWirelessDevice: vi.fn(),
  disconnectWirelessDevice: vi.fn(),
  listDevices: vi.fn(),
  refreshWirelessDiscovery: vi.fn(),
  updateDevicePresentation: vi.fn(),
}));

vi.mock("../services/system", () => ({
  getDaemonStatus: vi.fn(),
  getRuntime: vi.fn(),
}));

vi.mock("./useBackendEventSubscription", () => ({
  useBackendEventSubscription: vi.fn(),
}));

const connectWirelessDeviceMock = vi.mocked(connectWirelessDevice);
const listDevicesMock = vi.mocked(listDevices);
const disconnectWirelessDeviceMock = vi.mocked(disconnectWirelessDevice);
const refreshWirelessDiscoveryMock = vi.mocked(refreshWirelessDiscovery);
const updateDevicePresentationMock = vi.mocked(updateDevicePresentation);
const getDaemonStatusMock = vi.mocked(getDaemonStatus);
const getRuntimeMock = vi.mocked(getRuntime);

function runtime(): RuntimeResponse {
  return {
    backend: {
      host: "0.0.0.0",
      port: 9000,
      log_level: "debug",
      config_path: "/root/.config/lianli/backend.json",
      profile_store_path: "/data/profiles.json",
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
      socket_path: "/runtime/lianli-daemon.sock",
      config_path: "/data/config.json",
      xdg_runtime_dir: "/runtime",
      xdg_config_home: "/root/.config",
    },
  };
}

function wirelessDevice(
  id: string,
  groupId = id,
  overrides: Partial<DeviceView> = {},
): DeviceView {
  return {
    id,
    name: id,
    display_name: `Device ${id}`,
    ui_order: 10,
    physical_role: "Wireless cluster",
    capability_summary: "RGB | Fan",
    current_mode_summary: "Lighting ready",
    controller: {
      id: "wireless:mesh",
      label: "Wireless mesh",
      kind: "wireless_mesh",
    },
      wireless: {
        transport: "wireless",
        channel: 8,
        group_id: groupId,
        group_label: "Desk cluster",
        binding_state: "connected",
        master_mac: "3b:59:87:e5:66:e4",
      },
    health: {
      level: "healthy",
      summary: "Healthy",
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
      rgb_zone_count: 2,
    },
    state: {
      fan_rpms: [900, 910, 920, 930],
      coolant_temp: null,
      streaming_active: false,
    },
    ...overrides,
  };
}

function wiredDevice(): DeviceView {
  return {
    ...wirelessDevice("usb:controller"),
    id: "usb:controller",
    name: "usb:controller",
    display_name: "Wired Controller",
    wireless: null,
    controller: {
      id: "usb:hub",
      label: "USB hub",
      kind: "usb_controller",
    },
  };
}

describe("useWirelessSyncData", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    connectWirelessDeviceMock.mockResolvedValue({
      device_id: "wireless:test",
      connected: true,
    });
    getDaemonStatusMock.mockResolvedValue({
      reachable: true,
      socket_path: "/runtime/lianli-daemon.sock",
      error: null,
    });
    getRuntimeMock.mockResolvedValue(runtime());
    refreshWirelessDiscoveryMock.mockResolvedValue({
      refreshed: true,
      device_count: 2,
    });
  });

  it("loads wireless inventory with daemon and runtime context", async () => {
    listDevicesMock.mockResolvedValue([wirelessDevice("wireless:one"), wiredDevice()]);

    const { result } = renderHook(() => useWirelessSyncData());

    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(result.current.devices).toHaveLength(2);
    expect(result.current.wirelessDevices).toHaveLength(1);
    expect(result.current.connectedWirelessDevices).toHaveLength(1);
    expect(result.current.availableWirelessDevices).toHaveLength(0);
    expect(result.current.wirelessDevices[0]?.id).toBe("wireless:one");
    expect(result.current.daemonStatus?.reachable).toBe(true);
    expect(result.current.runtime?.daemon.socket_path).toBe("/runtime/lianli-daemon.sock");
    expect(result.current.error).toBeNull();
  });

  it("treats legacy wireless inventory without binding metadata as connected", async () => {
    listDevicesMock.mockResolvedValue([
      wirelessDevice("wireless:legacy", "wireless:legacy", {
        wireless: {
          transport: "wireless",
          channel: 8,
          group_id: "wireless:legacy",
          group_label: "Legacy cluster",
          binding_state: null,
          master_mac: null,
        },
      }),
    ]);

    const { result } = renderHook(() => useWirelessSyncData());

    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(result.current.connectedWirelessDevices).toHaveLength(1);
    expect(result.current.availableWirelessDevices).toHaveLength(0);
    expect(result.current.foreignWirelessDevices).toHaveLength(0);
  });

  it("saves shared controller and cluster labels for every device in a wireless group", async () => {
    const initialDevices = [
      wirelessDevice("wireless:one", "desk-group"),
      wirelessDevice("wireless:two", "desk-group", {
        display_name: "Device Two",
        ui_order: 20,
      }),
    ];
    const updatedDevices = [
      wirelessDevice("wireless:one", "desk-group", {
        controller: {
          id: "wireless:mesh",
          label: "Desk mesh",
          kind: "wireless_mesh",
        },
        wireless: {
          transport: "wireless",
          channel: 8,
          group_id: "desk-group",
          group_label: "Desk cluster",
          binding_state: "connected",
          master_mac: "3b:59:87:e5:66:e4",
        },
      }),
      wirelessDevice("wireless:two", "desk-group", {
        display_name: "Device Two",
        ui_order: 20,
        controller: {
          id: "wireless:mesh",
          label: "Desk mesh",
          kind: "wireless_mesh",
        },
        wireless: {
          transport: "wireless",
          channel: 8,
          group_id: "desk-group",
          group_label: "Desk cluster",
          binding_state: "connected",
          master_mac: "3b:59:87:e5:66:e4",
        },
      }),
    ];

    listDevicesMock.mockResolvedValueOnce(initialDevices).mockResolvedValue(updatedDevices);
    updateDevicePresentationMock.mockResolvedValue(updatedDevices[0]!);

    const { result } = renderHook(() => useWirelessSyncData());

    await waitFor(() => expect(result.current.loading).toBe(false));

    await act(async () => {
      await result.current.saveGroupPresentation("desk-group", {
        controllerLabel: "Desk mesh",
        clusterLabel: "Desk cluster",
      });
    });

    await waitFor(() => expect(result.current.actionSuccess).toContain("Saved wireless labels"));

    expect(updateDevicePresentationMock).toHaveBeenCalledTimes(2);
    expect(updateDevicePresentationMock).toHaveBeenNthCalledWith(1, "wireless:one", {
      display_name: "Device wireless:one",
      ui_order: 10,
      physical_role: "Wireless cluster",
      controller_label: "Desk mesh",
      cluster_label: "Desk cluster",
    });
    expect(updateDevicePresentationMock).toHaveBeenNthCalledWith(2, "wireless:two", {
      display_name: "Device Two",
      ui_order: 20,
      physical_role: "Wireless cluster",
      controller_label: "Desk mesh",
      cluster_label: "Desk cluster",
    });
    expect(result.current.wirelessDevices[0]?.controller.label).toBe("Desk mesh");
    expect(result.current.wirelessDevices[0]?.wireless?.group_label).toBe("Desk cluster");
  });

  it("disconnects every device in a wireless group through the backend action", async () => {
    const devices = [
      wirelessDevice("wireless:one", "desk-group"),
      wirelessDevice("wireless:two", "desk-group"),
      wirelessDevice("wireless:three", "other-group"),
    ];

    listDevicesMock.mockResolvedValueOnce(devices).mockResolvedValue([
      wirelessDevice("wireless:three", "other-group"),
    ]);
    disconnectWirelessDeviceMock.mockResolvedValue({
      device_id: "wireless:one",
      disconnected: true,
    });

    const { result } = renderHook(() => useWirelessSyncData());

    await waitFor(() => expect(result.current.loading).toBe(false));

    await act(async () => {
      await result.current.disconnectGroup("desk-group");
    });

    await waitFor(() =>
      expect(result.current.actionSuccess).toContain("Disconnected 2 wireless devices"),
    );

    expect(disconnectWirelessDeviceMock).toHaveBeenCalledTimes(2);
    expect(disconnectWirelessDeviceMock).toHaveBeenNthCalledWith(1, "wireless:one");
    expect(disconnectWirelessDeviceMock).toHaveBeenNthCalledWith(2, "wireless:two");
  });

  it("searches for devices through the dedicated discovery endpoint", async () => {
    listDevicesMock.mockResolvedValue([wirelessDevice("wireless:one")]);

    const { result } = renderHook(() => useWirelessSyncData());

    await waitFor(() => expect(result.current.loading).toBe(false));

    await act(async () => {
      await result.current.searchForDevices();
    });

    expect(refreshWirelessDiscoveryMock).toHaveBeenCalledTimes(1);
    expect(result.current.actionSuccess).toContain("Wireless scan completed");
  });

  it("connects an available wireless device through the backend action", async () => {
    const available = wirelessDevice("wireless:available", "wireless:available", {
      wireless: {
        transport: "wireless",
        channel: 8,
        group_id: "wireless:available",
        group_label: "Open cluster",
        binding_state: "available",
        master_mac: null,
      },
    });
    const connected = wirelessDevice("wireless:available", "wireless:available");

    listDevicesMock.mockResolvedValueOnce([available]).mockResolvedValue([connected]);

    const { result } = renderHook(() => useWirelessSyncData());

    await waitFor(() => expect(result.current.loading).toBe(false));

    await act(async () => {
      await result.current.connectDevice("wireless:available");
    });

    expect(connectWirelessDeviceMock).toHaveBeenCalledWith("wireless:available");
    expect(result.current.actionSuccess).toContain("Connected wireless:available");
  });
});
