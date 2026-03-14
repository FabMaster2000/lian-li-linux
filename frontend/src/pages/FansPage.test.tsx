import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { FansPage } from "./FansPage";
import { renderAtRoute } from "../test/render";
import type { DeviceView, FanStateResponse } from "../types/api";
import { listDevices } from "../services/devices";
import { getFanState, setManualFanSpeed } from "../services/fans";

vi.mock("../services/devices", () => ({
  listDevices: vi.fn(),
}));

vi.mock("../services/fans", () => ({
  getFanState: vi.fn(),
  setManualFanSpeed: vi.fn(),
}));

const listDevicesMock = vi.mocked(listDevices);
const getFanStateMock = vi.mocked(getFanState);
const setManualFanSpeedMock = vi.mocked(setManualFanSpeed);

function fanDevice(id: string, name: string): DeviceView {
  return {
    id,
    name,
    family: "SlInf",
    online: true,
    capabilities: {
      has_fan: true,
      has_rgb: false,
      has_lcd: false,
      has_pump: false,
      fan_count: 4,
      per_fan_control: false,
      mb_sync_support: false,
      rgb_zone_count: 0,
    },
    state: {
      fan_rpms: [900, 910, 920, 930],
      coolant_temp: null,
      streaming_active: false,
    },
  };
}

function fanState(deviceId: string, slots: FanStateResponse["slots"]): FanStateResponse {
  return {
    device_id: deviceId,
    update_interval_ms: 1000,
    rpms: [900, 910, 920, 930],
    slots,
  };
}

describe("FansPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders device and slot data from the backend", async () => {
    listDevicesMock.mockResolvedValue([fanDevice("wireless:fan-one", "Fan Controller")]);
    getFanStateMock.mockResolvedValue(
      fanState("wireless:fan-one", [
        { slot: 1, mode: "manual", percent: 60, pwm: 153, curve: null },
        { slot: 2, mode: "manual", percent: 60, pwm: 153, curve: null },
      ]),
    );

    renderAtRoute(<FansPage />, {
      initialPath: "/fans?device=wireless%3Afan-one",
      routePath: "/fans",
    });

    await waitFor(() => {
      expect(screen.getByRole("combobox", { name: "Device" })).toHaveValue("wireless:fan-one");
      expect(screen.getByRole("slider")).toHaveValue("60");
    });

    expect(screen.getByText("Cooling console")).toBeInTheDocument();
    expect(screen.getByText("Reported fan slots")).toBeInTheDocument();
    expect(screen.getByText("Slot 1")).toBeInTheDocument();
    expect(screen.getAllByText("900 / 910 / 920 / 930").length).toBeGreaterThan(0);
  });

  it("applies a manual fan speed and shows a success state", async () => {
    const user = userEvent.setup();
    listDevicesMock.mockResolvedValue([fanDevice("wireless:fan-one", "Fan Controller")]);
    getFanStateMock.mockResolvedValue(
      fanState("wireless:fan-one", [
        { slot: 1, mode: "manual", percent: 40, pwm: 102, curve: null },
        { slot: 2, mode: "manual", percent: 40, pwm: 102, curve: null },
      ]),
    );
    setManualFanSpeedMock.mockResolvedValue(
      fanState("wireless:fan-one", [
        { slot: 1, mode: "manual", percent: 70, pwm: 178, curve: null },
        { slot: 2, mode: "manual", percent: 70, pwm: 178, curve: null },
      ]),
    );

    renderAtRoute(<FansPage />, {
      initialPath: "/fans?device=wireless%3Afan-one",
      routePath: "/fans",
    });

    await waitFor(() => expect(screen.getByRole("slider")).toHaveValue("40"));

    fireEvent.change(screen.getByRole("slider"), {
      target: { value: "70" },
    });
    await user.click(screen.getByRole("button", { name: "Apply fan speed" }));

    await waitFor(() =>
      expect(setManualFanSpeedMock).toHaveBeenCalledWith("wireless:fan-one", {
        percent: 70,
      }),
    );

    expect(screen.getByText("Fan speed updated.")).toBeInTheDocument();
    expect(screen.getByRole("slider")).toHaveValue("70");
  });

  it("shows backend errors for failed apply requests", async () => {
    const user = userEvent.setup();
    listDevicesMock.mockResolvedValue([fanDevice("wireless:fan-one", "Fan Controller")]);
    getFanStateMock.mockResolvedValue(
      fanState("wireless:fan-one", [
        { slot: 1, mode: "manual", percent: 40, pwm: 102, curve: null },
      ]),
    );
    setManualFanSpeedMock.mockRejectedValue(new Error("manual fan set failed"));

    renderAtRoute(<FansPage />, {
      initialPath: "/fans?device=wireless%3Afan-one",
      routePath: "/fans",
    });

    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Apply fan speed" })).toBeEnabled(),
    );

    await user.click(screen.getByRole("button", { name: "Apply fan speed" }));

    expect(await screen.findByText("Fan action failed.")).toBeInTheDocument();
    expect(screen.getByText("manual fan set failed")).toBeInTheDocument();
  });

  it("keeps previous rpm telemetry when the update response omits it", async () => {
    const user = userEvent.setup();
    listDevicesMock.mockResolvedValue([fanDevice("wireless:fan-one", "Fan Controller")]);
    getFanStateMock.mockResolvedValue(
      fanState("wireless:fan-one", [
        { slot: 1, mode: "manual", percent: 40, pwm: 102, curve: null },
        { slot: 2, mode: "manual", percent: 40, pwm: 102, curve: null },
      ]),
    );
    setManualFanSpeedMock.mockResolvedValue({
      device_id: "wireless:fan-one",
      update_interval_ms: 1000,
      rpms: null,
      slots: [
        { slot: 1, mode: "manual", percent: 31, pwm: 79, curve: null },
        { slot: 2, mode: "manual", percent: 31, pwm: 79, curve: null },
      ],
    });

    renderAtRoute(<FansPage />, {
      initialPath: "/fans?device=wireless%3Afan-one",
      routePath: "/fans",
    });

    await waitFor(() => expect(screen.getByRole("slider")).toHaveValue("40"));

    fireEvent.change(screen.getByRole("slider"), {
      target: { value: "31" },
    });
    await user.click(screen.getByRole("button", { name: "Apply fan speed" }));

    await waitFor(() => expect(screen.getByText("Fan speed updated.")).toBeInTheDocument());

    const liveFanStateCard = screen
      .getByRole("heading", { name: "Live fan state" })
      .closest("article");

    expect(liveFanStateCard).not.toBeNull();

    const liveFanState = within(liveFanStateCard as HTMLElement);
    expect(liveFanState.getByText("900 / 910 / 920 / 930")).toBeInTheDocument();
    expect(liveFanState.queryByText("n/a")).not.toBeInTheDocument();
  });

  it("shows a mixed current value when slot percentages differ", async () => {
    listDevicesMock.mockResolvedValue([fanDevice("wireless:fan-one", "Fan Controller")]);
    getFanStateMock.mockResolvedValue(
      fanState("wireless:fan-one", [
        { slot: 1, mode: "manual", percent: 30, pwm: 76, curve: null },
        { slot: 2, mode: "manual", percent: 60, pwm: 153, curve: null },
      ]),
    );

    renderAtRoute(<FansPage />, {
      initialPath: "/fans?device=wireless%3Afan-one",
      routePath: "/fans",
    });

    await waitFor(() => expect(screen.getByText("Current value")).toBeInTheDocument());

    expect(screen.getAllByText("mixed").length).toBeGreaterThan(0);
    expect(screen.getByRole("slider")).toHaveValue("45");
  });

  it("requires an explicit device selection on the generic route", async () => {
    const user = userEvent.setup();
    listDevicesMock.mockResolvedValue([fanDevice("wireless:fan-one", "Fan Controller")]);
    getFanStateMock.mockResolvedValue(
      fanState("wireless:fan-one", [
        { slot: 1, mode: "manual", percent: 60, pwm: 153, curve: null },
      ]),
    );

    renderAtRoute(<FansPage />, {
      initialPath: "/fans",
      routePath: "/fans",
    });

    await waitFor(() =>
      expect(screen.getByRole("combobox", { name: "Device" })).toHaveValue(""),
    );

    expect(screen.getByText("Select a fan-capable device to start applying a manual speed.")).toBeInTheDocument();
    expect(getFanStateMock).not.toHaveBeenCalled();

    await user.selectOptions(screen.getByRole("combobox", { name: "Device" }), "wireless:fan-one");

    await waitFor(() => expect(screen.getByRole("slider")).toHaveValue("60"));
  });

  it("shows an offline warning and disables fan writes for offline devices", async () => {
    listDevicesMock.mockResolvedValue([
      {
        ...fanDevice("wireless:fan-one", "Fan Controller"),
        online: false,
      },
    ]);
    getFanStateMock.mockResolvedValue(
      fanState("wireless:fan-one", [
        { slot: 1, mode: "manual", percent: 60, pwm: 153, curve: null },
      ]),
    );

    renderAtRoute(<FansPage />, {
      initialPath: "/fans?device=wireless%3Afan-one",
      routePath: "/fans",
    });

    await waitFor(() =>
      expect(screen.getByRole("combobox", { name: "Device" })).toHaveValue("wireless:fan-one"),
    );

    expect(screen.getByText("Device offline.")).toBeInTheDocument();
    expect(screen.getByRole("slider")).toBeDisabled();
    expect(screen.getByRole("button", { name: "Apply fan speed" })).toBeDisabled();
  });
});
