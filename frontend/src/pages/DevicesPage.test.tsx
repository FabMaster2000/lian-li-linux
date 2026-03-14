import { screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { DevicesPage } from "./DevicesPage";
import { renderAtRoute } from "../test/render";
import type { DeviceView } from "../types/api";
import { listDevices } from "../services/devices";

vi.mock("../services/devices", () => ({
  listDevices: vi.fn(),
}));

const listDevicesMock = vi.mocked(listDevices);

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

describe("DevicesPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("lists devices and links to explicit detail routes", async () => {
    listDevicesMock.mockResolvedValue([device()]);

    renderAtRoute(<DevicesPage />, {
      initialPath: "/devices",
      routePath: "/devices",
    });

    await waitFor(() => expect(screen.getByText("Available devices")).toBeInTheDocument());

    expect(screen.getByText("Controller One")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Details" })).toHaveAttribute(
      "href",
      "/devices/wireless%3Aone",
    );
    expect(screen.queryByRole("link", { name: "Lighting" })).not.toBeInTheDocument();
    expect(screen.queryByRole("link", { name: "Fans" })).not.toBeInTheDocument();
  });

  it("shows an error banner when the device list cannot be loaded", async () => {
    listDevicesMock.mockRejectedValue(new Error("device inventory unavailable"));

    renderAtRoute(<DevicesPage />, {
      initialPath: "/devices",
      routePath: "/devices",
    });

    await waitFor(() =>
      expect(screen.getByText("Device selection load failed.")).toBeInTheDocument(),
    );

    expect(screen.getByText("device inventory unavailable")).toBeInTheDocument();
  });
});
