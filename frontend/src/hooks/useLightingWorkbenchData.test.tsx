import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useLightingWorkbenchData } from "./useLightingWorkbenchData";
import type { BackendEventEnvelope, DeviceView, LightingStateResponse } from "../types/api";
import { listDevices } from "../services/devices";
import { getLightingState, setLightingEffect } from "../services/lighting";

vi.mock("../services/devices", () => ({
  listDevices: vi.fn(),
}));

vi.mock("../services/lighting", () => ({
  getLightingState: vi.fn(),
  setLightingEffect: vi.fn(),
}));

let latestBackendListener: ((event: BackendEventEnvelope) => void) | null = null;

vi.mock("./useBackendEventSubscription", () => ({
  useBackendEventSubscription: vi.fn((listener: (event: BackendEventEnvelope) => void) => {
    latestBackendListener = listener;
  }),
}));

const listDevicesMock = vi.mocked(listDevices);
const getLightingStateMock = vi.mocked(getLightingState);
const setLightingEffectMock = vi.mocked(setLightingEffect);

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

function lightingState(
  deviceId: string,
  zones: LightingStateResponse["zones"],
): LightingStateResponse {
  return {
    device_id: deviceId,
    zones,
  };
}

describe("useLightingWorkbenchData", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    latestBackendListener = null;
  });

  it("does not auto-select a device without an explicit request", async () => {
    listDevicesMock.mockResolvedValue([
      device({
        id: "fan-only",
        capabilities: { ...device({}).capabilities, has_rgb: false },
      }),
      device({ id: "rgb-primary", name: "RGB Primary" }),
    ]);
    getLightingStateMock.mockResolvedValue(
      lightingState("rgb-primary", [
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
      ]),
    );

    const { result } = renderHook(() => useLightingWorkbenchData(null));

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
      expect(result.current.selectedDeviceId).toBe("");
    });

    expect(result.current.devices).toHaveLength(1);
    expect(result.current.lightingState).toBeNull();
    expect(getLightingStateMock).not.toHaveBeenCalled();
  });

  it("loads lighting for an explicitly selected device without reloading the device list", async () => {
    listDevicesMock.mockResolvedValue([
      device({ id: "rgb-primary", name: "RGB Primary" }),
      device({ id: "rgb-secondary", name: "RGB Secondary" }),
    ]);
    getLightingStateMock
      .mockResolvedValueOnce(
        lightingState("rgb-primary", [
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
      )
      .mockResolvedValueOnce(
        lightingState("rgb-secondary", [
          {
            zone: 0,
            effect: "Breathing",
            colors: ["#abcdef"],
            speed: 1,
            brightness_percent: 40,
            direction: "Clockwise",
            scope: "All",
          },
        ]),
      );

    const { result } = renderHook(() => useLightingWorkbenchData("rgb-primary"));

    await waitFor(() => expect(result.current.selectedDeviceId).toBe("rgb-primary"));

    await act(async () => {
      result.current.setSelectedDeviceId("rgb-secondary");
    });

    await waitFor(() => {
      expect(result.current.selectedDeviceId).toBe("rgb-secondary");
      expect(result.current.form.effect).toBe("Breathing");
    });

    expect(listDevicesMock).toHaveBeenCalledTimes(1);
    expect(getLightingStateMock).toHaveBeenNthCalledWith(1, "rgb-primary");
    expect(getLightingStateMock).toHaveBeenNthCalledWith(2, "rgb-secondary");
  });

  it("applies lighting changes and updates state from the backend response", async () => {
    listDevicesMock.mockResolvedValue([device({ id: "rgb-primary", name: "RGB Primary" })]);
    getLightingStateMock.mockResolvedValue(
      lightingState("rgb-primary", [
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
    setLightingEffectMock.mockResolvedValue(
      lightingState("rgb-primary", [
        {
          zone: 0,
          effect: "Rainbow",
          colors: ["#abcdef"],
          speed: 3,
          brightness_percent: 20,
          direction: "Clockwise",
          scope: "All",
        },
      ]),
    );

    const { result } = renderHook(() => useLightingWorkbenchData("rgb-primary"));
    await waitFor(() => expect(result.current.selectedDeviceId).toBe("rgb-primary"));

    act(() => {
      result.current.setForm((current) => ({
        ...current,
        effect: "Rainbow",
        color: "#abcdef",
        brightness: 20,
      }));
    });

    await act(async () => {
      await result.current.applyChanges();
    });

    expect(setLightingEffectMock).toHaveBeenCalledWith("rgb-primary", {
      zone: 0,
      effect: "Rainbow",
      brightness: 20,
      color: { hex: "#abcdef" },
    });
    expect(result.current.success).toBe("Lighting state applied");
    expect(result.current.form.effect).toBe("Rainbow");
    expect(result.current.form.color).toBe("#abcdef");
    expect(result.current.form.brightness).toBe(20);
  });

  it("surfaces lighting-state loading errors", async () => {
    listDevicesMock.mockResolvedValue([device({ id: "rgb-primary", name: "RGB Primary" })]);
    getLightingStateMock.mockRejectedValue(new Error("backend unavailable"));

    const { result } = renderHook(() => useLightingWorkbenchData("rgb-primary"));

    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(result.current.error).toBe("backend unavailable");
    expect(result.current.lightingState).toBeNull();
  });

  it("refreshes readonly lighting data without overwriting edited form values", async () => {
    listDevicesMock.mockResolvedValue([device({ id: "rgb-primary", name: "RGB Primary" })]);
    getLightingStateMock
      .mockResolvedValueOnce(
        lightingState("rgb-primary", [
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
      )
      .mockResolvedValueOnce(
        lightingState("rgb-primary", [
          {
            zone: 0,
            effect: "Rainbow",
            colors: ["#445566"],
            speed: 4,
            brightness_percent: 25,
            direction: "Up",
            scope: "Inner",
          },
        ]),
      );

    const { result } = renderHook(() => useLightingWorkbenchData("rgb-primary"));
    await waitFor(() => expect(result.current.selectedDeviceId).toBe("rgb-primary"));

    act(() => {
      result.current.setForm((current) => ({
        ...current,
        effect: "Breathing",
        color: "#abcdef",
        brightness: 60,
      }));
    });

    await act(async () => {
      await result.current.refresh();
    });

    expect(result.current.form).toEqual({
      zone: 0,
      effect: "Breathing",
      color: "#abcdef",
      brightness: 60,
    });
    expect(result.current.activeZone).toMatchObject({
      effect: "Rainbow",
      colors: ["#445566"],
      brightness_percent: 25,
    });
    expect(result.current.stateLoading).toBe(false);
    expect(result.current.stateRefreshing).toBe(false);
    expect(getLightingStateMock).toHaveBeenCalledTimes(2);
  });

  it("reacts to lighting events with a readonly refresh for the selected device", async () => {
    listDevicesMock.mockResolvedValue([device({ id: "rgb-primary", name: "RGB Primary" })]);
    getLightingStateMock
      .mockResolvedValueOnce(
        lightingState("rgb-primary", [
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
      )
      .mockResolvedValueOnce(
        lightingState("rgb-primary", [
          {
            zone: 0,
            effect: "Rainbow",
            colors: ["#445566"],
            speed: 4,
            brightness_percent: 25,
            direction: "Up",
            scope: "Inner",
          },
        ]),
      );

    const { result } = renderHook(() => useLightingWorkbenchData("rgb-primary"));
    await waitFor(() => expect(result.current.selectedDeviceId).toBe("rgb-primary"));

    act(() => {
      result.current.setForm((current) => ({
        ...current,
        effect: "Breathing",
        color: "#abcdef",
        brightness: 60,
      }));
    });

    act(() => {
      emitBackendEvent({
        type: "lighting.changed",
        device_id: "rgb-primary",
      });
    });

    await waitFor(() =>
      expect(result.current.activeZone).toMatchObject({
        effect: "Rainbow",
        colors: ["#445566"],
        brightness_percent: 25,
      }),
    );

    expect(result.current.form).toEqual({
      zone: 0,
      effect: "Breathing",
      color: "#abcdef",
      brightness: 60,
    });
  });
});
