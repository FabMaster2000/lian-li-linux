import { screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { DeviceCard } from "./devices/DeviceCard";
import { renderAtRoute } from "../test/render";
import type { DeviceView } from "../types/api";

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

describe("DeviceCard", () => {
  it("renders status, topology context, telemetry, and actions", () => {
    renderAtRoute(
      <DeviceCard
        actions={[
          { label: "Details", to: "/devices/wireless%3Aone", tone: "primary" },
          { label: "Lighting", to: "/lighting?device=wireless%3Aone" },
        ]}
        device={device()}
      />,
    );

    expect(screen.getByText("healthy")).toHaveClass("status-badge--online");
    expect(screen.getByText("Wireless mesh")).toBeInTheDocument();
    expect(screen.getByText("900 / 910 / 920 / 930 RPM")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Details" })).toHaveAttribute(
      "href",
      "/devices/wireless%3Aone",
    );
  });
});


