import { screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { DeviceCard } from "./DeviceCard";
import { renderAtRoute } from "../test/render";
import type { DeviceView } from "../types/api";

function device(overrides: Partial<DeviceView> = {}): DeviceView {
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

describe("DeviceCard", () => {
  it("renders status, capabilities, telemetry, and actions", () => {
    renderAtRoute(
      <DeviceCard
        actions={[
          { label: "Details", to: "/devices/wireless%3Aone", tone: "primary" },
          { label: "Lighting", to: "/lighting?device=wireless%3Aone" },
        ]}
        device={device()}
      />,
    );

    expect(screen.getByText("online")).toHaveClass("status-badge--online");
    expect(screen.getByText("1 RGB zone")).toBeInTheDocument();
    expect(screen.getByText("900 / 910 / 920 / 930 RPM")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Details" })).toHaveAttribute(
      "href",
      "/devices/wireless%3Aone",
    );
  });
});
