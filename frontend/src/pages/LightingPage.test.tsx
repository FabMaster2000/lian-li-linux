import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { LightingPage } from "./LightingPage";
import { renderAtRoute } from "../test/render";
import type { DeviceView, LightingStateResponse } from "../types/api";
import { listDevices } from "../services/devices";
import { getLightingState, setLightingEffect } from "../services/lighting";

vi.mock("../services/devices", () => ({
  listDevices: vi.fn(),
}));

vi.mock("../services/lighting", () => ({
  getLightingState: vi.fn(),
  setLightingEffect: vi.fn(),
}));

const listDevicesMock = vi.mocked(listDevices);
const getLightingStateMock = vi.mocked(getLightingState);
const setLightingEffectMock = vi.mocked(setLightingEffect);

function rgbDevice(id: string, name: string): DeviceView {
  return {
    id,
    name,
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
  };
}

function lightingState(
  deviceId: string,
  zones: LightingStateResponse["zones"] = [
    {
      zone: 0,
      effect: "Static",
      colors: ["#112233"],
      speed: 2,
      brightness_percent: 75,
      direction: "Clockwise",
      scope: "All",
    },
    {
      zone: 1,
      effect: "Rainbow",
      colors: ["#445566"],
      speed: 4,
      brightness_percent: 25,
      direction: "Up",
      scope: "Inner",
    },
  ],
): LightingStateResponse {
  return {
    device_id: deviceId,
    zones,
  };
}

describe("LightingPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders device and zone data from the backend", async () => {
    listDevicesMock.mockResolvedValue([rgbDevice("wireless:one", "Controller One")]);
    getLightingStateMock.mockResolvedValue(lightingState("wireless:one"));

    renderAtRoute(<LightingPage />, {
      initialPath: "/lighting?device=wireless%3Aone",
      routePath: "/lighting",
    });

    expect(screen.getByText("Lighting workbench")).toBeInTheDocument();

    await waitFor(() => {
      expect(screen.getByRole("combobox", { name: "Device" })).toHaveValue("wireless:one");
      expect(screen.getByRole("combobox", { name: "Effect" })).toBeEnabled();
    });

    expect(screen.getByRole("combobox", { name: "Zone" })).toHaveValue("0");
    expect(screen.getByRole("combobox", { name: "Effect" })).toHaveValue("Static");
    expect(await screen.findByLabelText("Lighting color picker")).toHaveValue("#112233");
    expect(screen.getByText("Other zones")).toBeInTheDocument();
    expect(screen.queryByText(/^Colors$/)).not.toBeInTheDocument();
  });

  it("updates the form when a different zone is selected", async () => {
    const user = userEvent.setup();
    listDevicesMock.mockResolvedValue([rgbDevice("wireless:one", "Controller One")]);
    getLightingStateMock.mockResolvedValue(lightingState("wireless:one"));

    renderAtRoute(<LightingPage />, {
      initialPath: "/lighting?device=wireless%3Aone",
      routePath: "/lighting",
    });

    await waitFor(() => {
      expect(screen.getByRole("combobox", { name: "Zone" })).toBeEnabled();
      expect(screen.getByRole("combobox", { name: "Zone" })).toHaveValue("0");
    });

    await user.selectOptions(screen.getByRole("combobox", { name: "Zone" }), "1");

    expect(screen.getByRole("combobox", { name: "Effect" })).toHaveValue("Rainbow");
    expect(screen.getByLabelText("Lighting color picker")).toHaveValue("#445566");
    expect(screen.getByRole("slider")).toHaveValue("25");
  });

  it("applies lighting changes and shows a success state", async () => {
    const user = userEvent.setup();
    listDevicesMock.mockResolvedValue([rgbDevice("wireless:one", "Controller One")]);
    getLightingStateMock.mockResolvedValue(lightingState("wireless:one"));
    setLightingEffectMock.mockResolvedValue({
      device_id: "wireless:one",
      zones: [
        {
          zone: 0,
          effect: "Breathing",
          colors: ["#abcdef"],
          speed: 2,
          brightness_percent: 60,
          direction: "Clockwise",
          scope: "All",
        },
      ],
    });

    renderAtRoute(<LightingPage />, {
      initialPath: "/lighting?device=wireless%3Aone",
      routePath: "/lighting",
    });

    await waitFor(() => {
      expect(screen.getByRole("combobox", { name: "Effect" })).toBeEnabled();
      expect(screen.getByRole("combobox", { name: "Effect" })).toHaveValue("Static");
    });

    await user.selectOptions(screen.getByRole("combobox", { name: "Effect" }), "Breathing");
    expect(screen.getByRole("combobox", { name: "Effect" })).toHaveValue("Breathing");
    fireEvent.change(screen.getByLabelText("Lighting color picker"), {
      target: { value: "#abcdef" },
    });
    fireEvent.change(screen.getByRole("slider"), {
      target: { value: "60" },
    });
    await user.click(screen.getByRole("button", { name: "Apply lighting" }));

    await waitFor(() =>
      expect(setLightingEffectMock).toHaveBeenCalledWith("wireless:one", {
        zone: 0,
        effect: "Breathing",
        brightness: 60,
        color: { hex: "#abcdef" },
      }),
    );

    expect(screen.getByText("Lighting updated.")).toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "Effect" })).toHaveValue("Breathing");
  });

  it("shows backend errors for failed apply requests", async () => {
    const user = userEvent.setup();
    listDevicesMock.mockResolvedValue([rgbDevice("wireless:one", "Controller One")]);
    getLightingStateMock.mockResolvedValue(lightingState("wireless:one"));
    setLightingEffectMock.mockRejectedValue(new Error("bad request: unsupported effect"));

    renderAtRoute(<LightingPage />, {
      initialPath: "/lighting?device=wireless%3Aone",
      routePath: "/lighting",
    });

    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Apply lighting" })).toBeEnabled(),
    );

    await user.click(screen.getByRole("button", { name: "Apply lighting" }));

    expect(await screen.findByText("Lighting action failed.")).toBeInTheDocument();
    expect(screen.getByText("bad request: unsupported effect")).toBeInTheDocument();
  });

  it("hides the zone overview when the backend only reports one zone", async () => {
    listDevicesMock.mockResolvedValue([rgbDevice("wireless:one", "Controller One")]);
    getLightingStateMock.mockResolvedValue(
      lightingState("wireless:one", [
        {
          zone: 0,
          effect: "Static",
          colors: ["#112233"],
          speed: 2,
          brightness_percent: 75,
          direction: "Clockwise",
          scope: "All",
        },
      ]),
    );

    renderAtRoute(<LightingPage />, {
      initialPath: "/lighting?device=wireless%3Aone",
      routePath: "/lighting",
    });

    await waitFor(() =>
      expect(screen.getByRole("combobox", { name: "Effect" })).toHaveValue("Static"),
    );

    expect(screen.queryByText("Other zones")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Use this zone" })).not.toBeInTheDocument();
  });

  it("refreshes readonly fields in the background without overwriting the form", async () => {
    const user = userEvent.setup();
    listDevicesMock.mockResolvedValue([rgbDevice("wireless:one", "Controller One")]);
    getLightingStateMock
      .mockResolvedValueOnce(lightingState("wireless:one"))
      .mockImplementationOnce(
        () =>
          new Promise<LightingStateResponse>((resolve) => {
            setTimeout(() => {
              resolve(
                lightingState("wireless:one", [
                  {
                    zone: 0,
                    effect: "Static",
                    colors: ["#778899"],
                    speed: 2,
                    brightness_percent: 10,
                    direction: "Clockwise",
                    scope: "All",
                  },
                  {
                    zone: 1,
                    effect: "Rainbow",
                    colors: ["#445566"],
                    speed: 4,
                    brightness_percent: 25,
                    direction: "Up",
                    scope: "Inner",
                  },
                ]),
              );
            }, 20);
          }),
      );

    renderAtRoute(<LightingPage />, {
      initialPath: "/lighting?device=wireless%3Aone",
      routePath: "/lighting",
    });

    await waitFor(() =>
      expect(screen.getByRole("combobox", { name: "Effect" })).toHaveValue("Static"),
    );

    await user.selectOptions(screen.getByRole("combobox", { name: "Effect" }), "Breathing");
    fireEvent.change(screen.getByLabelText("Lighting color picker"), {
      target: { value: "#abcdef" },
    });
    fireEvent.change(screen.getByRole("slider"), {
      target: { value: "60" },
    });

    await user.click(screen.getByRole("button", { name: "Reload lighting" }));

    expect(screen.getByRole("button", { name: "Refreshing..." })).toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "Effect" })).toBeEnabled();
    expect(screen.getByRole("combobox", { name: "Effect" })).toHaveValue("Breathing");
    expect(screen.getByLabelText("Lighting color picker")).toHaveValue("#abcdef");
    expect(screen.getByRole("slider")).toHaveValue("60");

    expect((await screen.findAllByText("#778899")).length).toBeGreaterThan(0);
    expect(screen.getByRole("combobox", { name: "Effect" })).toHaveValue("Breathing");
    expect(screen.getByLabelText("Lighting color picker")).toHaveValue("#abcdef");
    expect(screen.getByRole("slider")).toHaveValue("60");
  });

  it("shows only readonly backend fields in the live state card", async () => {
    listDevicesMock.mockResolvedValue([rgbDevice("wireless:one", "Controller One")]);
    getLightingStateMock.mockResolvedValue(lightingState("wireless:one"));

    renderAtRoute(<LightingPage />, {
      initialPath: "/lighting?device=wireless%3Aone",
      routePath: "/lighting",
    });

    await waitFor(() =>
      expect(screen.getByRole("heading", { name: "Live zone state" })).toBeInTheDocument(),
    );

    const liveStateCard = screen
      .getByRole("heading", { name: "Live zone state" })
      .closest("article");

    expect(liveStateCard).not.toBeNull();

    const liveState = within(liveStateCard as HTMLElement);
    expect(liveState.getByText("Speed")).toBeInTheDocument();
    expect(liveState.getByText("Direction")).toBeInTheDocument();
    expect(liveState.getByText("Scope")).toBeInTheDocument();
    expect(liveState.queryByText("Colors")).not.toBeInTheDocument();
    expect(liveState.queryByText("Brightness")).not.toBeInTheDocument();
  });

  it("requires an explicit device selection on the generic route", async () => {
    const user = userEvent.setup();
    listDevicesMock.mockResolvedValue([rgbDevice("wireless:one", "Controller One")]);
    getLightingStateMock.mockResolvedValue(lightingState("wireless:one"));

    renderAtRoute(<LightingPage />, {
      initialPath: "/lighting",
      routePath: "/lighting",
    });

    await waitFor(() =>
      expect(screen.getByRole("combobox", { name: "Device" })).toHaveValue(""),
    );

    expect(screen.getByText("Select an RGB-capable device to start editing.")).toBeInTheDocument();
    expect(getLightingStateMock).not.toHaveBeenCalled();

    await user.selectOptions(screen.getByRole("combobox", { name: "Device" }), "wireless:one");

    await waitFor(() =>
      expect(screen.getByRole("combobox", { name: "Effect" })).toHaveValue("Static"),
    );
  });

  it("shows an offline warning and disables lighting edits for offline devices", async () => {
    listDevicesMock.mockResolvedValue([
      {
        ...rgbDevice("wireless:one", "Controller One"),
        online: false,
      },
    ]);
    getLightingStateMock.mockResolvedValue(lightingState("wireless:one"));

    renderAtRoute(<LightingPage />, {
      initialPath: "/lighting?device=wireless%3Aone",
      routePath: "/lighting",
    });

    await waitFor(() =>
      expect(screen.getByRole("combobox", { name: "Device" })).toHaveValue("wireless:one"),
    );

    expect(screen.getByText("Device offline.")).toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "Effect" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Apply lighting" })).toBeDisabled();
  });
});
