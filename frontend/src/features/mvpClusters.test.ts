import { describe, expect, it } from "vitest";
import { buildAvailableClusters, buildPairedClusters } from "./mvpClusters";
import type { DeviceView } from "../types/api";

function device(overrides: Partial<DeviceView> = {}): DeviceView {
  return {
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
      fan_count: 3,
      per_fan_control: true,
      mb_sync_support: false,
      rgb_zone_count: 3,
    },
    state: {
      fan_rpms: [900, 910, 920],
      coolant_temp: null,
      streaming_active: false,
    },
    ...overrides,
  };
}

describe("buildPairedClusters", () => {
  it("marks a cluster offline when all devices report online false", () => {
    const clusters = buildPairedClusters([
      device({
        online: false,
        health: {
          level: "offline",
          summary: "Offline",
        },
        state: {
          fan_rpms: null,
          coolant_temp: null,
          streaming_active: false,
        },
      }),
    ]);

    expect(clusters).toHaveLength(1);
    expect(clusters[0]?.status).toBe("offline");
  });

  it("marks a cluster offline when health is offline even if online stayed stale", () => {
    const clusters = buildPairedClusters([
      device({
        online: true,
        health: {
          level: "offline",
          summary: "Wireless device not seen",
        },
      }),
    ]);

    expect(clusters).toHaveLength(1);
    expect(clusters[0]?.status).toBe("offline");
  });

  it("excludes offline available clusters from the pairable list", () => {
    const clusters = buildAvailableClusters([
      device({
        online: false,
        wireless: {
          transport: "wireless",
          channel: 8,
          group_id: "desk-cluster",
          group_label: "Desk cluster",
          binding_state: "available",
          master_mac: null,
        },
        health: {
          level: "offline",
          summary: "Wireless device not seen",
        },
        state: {
          fan_rpms: null,
          coolant_temp: null,
          streaming_active: false,
        },
      }),
    ]);

    expect(clusters).toEqual([]);
  });
});
