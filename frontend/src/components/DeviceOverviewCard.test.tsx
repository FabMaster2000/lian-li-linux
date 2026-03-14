import { screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { DeviceOverviewCard } from "./DeviceOverviewCard";
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

describe("DeviceOverviewCard", () => {
  it("uses generic section links by default", () => {
    renderAtRoute(<DeviceOverviewCard device={device()} />);

    expect(screen.getByRole("link", { name: "Details" })).toHaveAttribute("href", "/devices");
    expect(screen.getByRole("link", { name: "Lighting" })).toHaveAttribute("href", "/lighting");
    expect(screen.getByRole("link", { name: "Fans" })).toHaveAttribute("href", "/fans");
  });

  it("supports explicit detail links and hiding secondary actions", () => {
    renderAtRoute(
      <DeviceOverviewCard
        device={device()}
        detailsTo="/devices/wireless%3Aone"
        lightingTo={null}
        fansTo={null}
      />,
    );

    expect(screen.getByRole("link", { name: "Details" })).toHaveAttribute(
      "href",
      "/devices/wireless%3Aone",
    );
    expect(screen.queryByRole("link", { name: "Lighting" })).not.toBeInTheDocument();
    expect(screen.queryByRole("link", { name: "Fans" })).not.toBeInTheDocument();
  });
});
