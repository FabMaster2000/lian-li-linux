import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useFansWorkbenchData } from "./useFansWorkbenchData";
import type { BackendEventEnvelope, DeviceView, FanStateResponse } from "../types/api";
import { listDevices } from "../services/devices";
import { getFanState, setManualFanSpeed } from "../services/fans";

vi.mock("../services/devices", () => ({
  listDevices: vi.fn(),
}));

vi.mock("../services/fans", () => ({
  getFanState: vi.fn(),
  setManualFanSpeed: vi.fn(),
}));

let latestBackendListener: ((event: BackendEventEnvelope) => void) | null = null;

vi.mock("./useBackendEventSubscription", () => ({
  useBackendEventSubscription: vi.fn((listener: (event: BackendEventEnvelope) => void) => {
    latestBackendListener = listener;
  }),
}));

const listDevicesMock = vi.mocked(listDevices);
const getFanStateMock = vi.mocked(getFanState);
const setManualFanSpeedMock = vi.mocked(setManualFanSpeed);

function emitBackendEvent(event: Partial<BackendEventEnvelope> & Pick<BackendEventEnvelope, "type">) {
  latestBackendListener?.({
    timestamp: "2026-03-14T10:00:00Z",
    source: "ws",
    device_id: null,
    data: {},
    ...event,
  });
}

function device(overrides: Partial<DeviceView>): DeviceView {
  return {
    id: "wireless:test",
    name: "Test Device",
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
      fan_rpms: [1000, 1010, 1020, 1030],
      coolant_temp: null,
      streaming_active: false,
    },
    ...overrides,
  };
}

function fanState(deviceId: string, slots: FanStateResponse["slots"]): FanStateResponse {
  return {
    device_id: deviceId,
    update_interval_ms: 1000,
    rpms: [1000, 1010, 1020, 1030],
    slots,
  };
}

describe("useFansWorkbenchData", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    latestBackendListener = null;
  });

  it("does not auto-select a device without an explicit request", async () => {
    listDevicesMock.mockResolvedValue([
      device({
        id: "rgb-only",
        capabilities: { ...device({}).capabilities, has_fan: false },
      }),
      device({ id: "fan-primary", name: "Fan Primary" }),
    ]);
    getFanStateMock.mockResolvedValue(
      fanState("fan-primary", [
        { slot: 1, mode: "manual", percent: 60, pwm: 153, curve: null },
        { slot: 2, mode: "manual", percent: 60, pwm: 153, curve: null },
      ]),
    );

    const { result } = renderHook(() => useFansWorkbenchData(null));

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
      expect(result.current.selectedDeviceId).toBe("");
    });

    expect(result.current.devices).toHaveLength(1);
    expect(result.current.fanState).toBeNull();
    expect(getFanStateMock).not.toHaveBeenCalled();
  });

  it("refreshes readonly fan state without overwriting the slider draft", async () => {
    listDevicesMock.mockResolvedValue([device({ id: "fan-primary", name: "Fan Primary" })]);
    getFanStateMock
      .mockResolvedValueOnce(
        fanState("fan-primary", [
          { slot: 1, mode: "manual", percent: 40, pwm: 102, curve: null },
          { slot: 2, mode: "manual", percent: 40, pwm: 102, curve: null },
        ]),
      )
      .mockResolvedValueOnce(
        fanState("fan-primary", [
          { slot: 1, mode: "manual", percent: 75, pwm: 191, curve: null },
          { slot: 2, mode: "manual", percent: 75, pwm: 191, curve: null },
        ]),
      );

    const { result } = renderHook(() => useFansWorkbenchData("fan-primary"));
    await waitFor(() => expect(result.current.selectedDeviceId).toBe("fan-primary"));

    act(() => {
      result.current.setFormPercent(55);
    });

    await act(async () => {
      await result.current.refresh();
    });

    expect(result.current.formPercent).toBe(55);
    expect(result.current.fanState?.slots[0]?.percent).toBe(75);
    expect(result.current.stateRefreshing).toBe(false);
  });

  it("applies manual fan changes and updates state from the backend response", async () => {
    listDevicesMock.mockResolvedValue([device({ id: "fan-primary", name: "Fan Primary" })]);
    getFanStateMock.mockResolvedValue(
      fanState("fan-primary", [
        { slot: 1, mode: "manual", percent: 40, pwm: 102, curve: null },
        { slot: 2, mode: "manual", percent: 40, pwm: 102, curve: null },
      ]),
    );
    setManualFanSpeedMock.mockResolvedValue(
      fanState("fan-primary", [
        { slot: 1, mode: "manual", percent: 65, pwm: 166, curve: null },
        { slot: 2, mode: "manual", percent: 65, pwm: 166, curve: null },
      ]),
    );

    const { result } = renderHook(() => useFansWorkbenchData("fan-primary"));
    await waitFor(() => expect(result.current.selectedDeviceId).toBe("fan-primary"));

    act(() => {
      result.current.setFormPercent(65);
    });

    await act(async () => {
      await result.current.applyChanges();
    });

    expect(setManualFanSpeedMock).toHaveBeenCalledWith("fan-primary", {
      percent: 65,
    });
    expect(result.current.success).toBe("Manual fan speed applied");
    expect(result.current.formPercent).toBe(65);
    expect(result.current.fanState?.slots[0]?.percent).toBe(65);
  });

  it("preserves last known rpm telemetry when the manual update response omits it", async () => {
    listDevicesMock.mockResolvedValue([device({ id: "fan-primary", name: "Fan Primary" })]);
    getFanStateMock.mockResolvedValue(
      fanState("fan-primary", [
        { slot: 1, mode: "manual", percent: 40, pwm: 102, curve: null },
        { slot: 2, mode: "manual", percent: 40, pwm: 102, curve: null },
      ]),
    );
    setManualFanSpeedMock.mockResolvedValue({
      device_id: "fan-primary",
      update_interval_ms: 1000,
      rpms: null,
      slots: [
        { slot: 1, mode: "manual", percent: 31, pwm: 79, curve: null },
        { slot: 2, mode: "manual", percent: 31, pwm: 79, curve: null },
      ],
    });

    const { result } = renderHook(() => useFansWorkbenchData("fan-primary"));
    await waitFor(() => expect(result.current.selectedDeviceId).toBe("fan-primary"));

    act(() => {
      result.current.setFormPercent(31);
    });

    await act(async () => {
      await result.current.applyChanges();
    });

    expect(result.current.fanState?.rpms).toEqual([1000, 1010, 1020, 1030]);
    expect(result.current.fanState?.slots[0]?.percent).toBe(31);
    expect(result.current.formPercent).toBe(31);
  });

  it("surfaces fan-state loading errors", async () => {
    listDevicesMock.mockResolvedValue([device({ id: "fan-primary", name: "Fan Primary" })]);
    getFanStateMock.mockRejectedValue(new Error("fan backend unavailable"));

    const { result } = renderHook(() => useFansWorkbenchData("fan-primary"));

    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(result.current.error).toBe("fan backend unavailable");
    expect(result.current.fanState).toBeNull();
  });

  it("reacts to fan events with a readonly refresh for the selected device", async () => {
    listDevicesMock.mockResolvedValue([device({ id: "fan-primary", name: "Fan Primary" })]);
    getFanStateMock
      .mockResolvedValueOnce(
        fanState("fan-primary", [
          { slot: 1, mode: "manual", percent: 40, pwm: 102, curve: null },
          { slot: 2, mode: "manual", percent: 40, pwm: 102, curve: null },
        ]),
      )
      .mockResolvedValueOnce(
        fanState("fan-primary", [
          { slot: 1, mode: "manual", percent: 75, pwm: 191, curve: null },
          { slot: 2, mode: "manual", percent: 75, pwm: 191, curve: null },
        ]),
      );

    const { result } = renderHook(() => useFansWorkbenchData("fan-primary"));
    await waitFor(() => expect(result.current.selectedDeviceId).toBe("fan-primary"));

    act(() => {
      result.current.setFormPercent(55);
    });

    act(() => {
      emitBackendEvent({
        type: "fan.changed",
        device_id: "fan-primary",
      });
    });

    await waitFor(() => expect(result.current.fanState?.slots[0]?.percent).toBe(75));

    expect(result.current.formPercent).toBe(55);
  });
});
