import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useProfilesWorkbenchData } from "./useProfilesWorkbenchData";
import type { DeviceView, ProfileApplyResponse, ProfileDocument } from "../types/api";
import { listDevices } from "../services/devices";
import {
  applyProfile,
  createProfile,
  deleteProfile,
  listProfiles,
} from "../services/profiles";

vi.mock("../services/devices", () => ({
  listDevices: vi.fn(),
}));

vi.mock("../services/profiles", () => ({
  listProfiles: vi.fn(),
  createProfile: vi.fn(),
  deleteProfile: vi.fn(),
  applyProfile: vi.fn(),
}));

const listDevicesMock = vi.mocked(listDevices);
const listProfilesMock = vi.mocked(listProfiles);
const createProfileMock = vi.mocked(createProfile);
const deleteProfileMock = vi.mocked(deleteProfile);
const applyProfileMock = vi.mocked(applyProfile);

function createDeferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });

  return { promise, resolve, reject };
}

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

function profile(overrides: Partial<ProfileDocument> = {}): ProfileDocument {
  return {
    id: "night-mode",
    name: "Night Mode",
    description: "Dim everything",
    targets: {
      mode: "all",
      device_ids: [],
    },
    lighting: {
      enabled: true,
      color: "#223366",
      effect: "Static",
      brightness_percent: 15,
    },
    fans: {
      enabled: true,
      mode: "manual",
      percent: 25,
    },
    metadata: {
      created_at: "2026-03-14T12:00:00Z",
      updated_at: "2026-03-14T12:00:00Z",
    },
    ...overrides,
  };
}

function applyResult(overrides: Partial<ProfileApplyResponse> = {}): ProfileApplyResponse {
  return {
    profile_id: "night-mode",
    profile_name: "Night Mode",
    transaction_mode: "single_config_write",
    rollback_supported: false,
    applied_lighting_device_ids: ["wireless:one"],
    applied_fan_device_ids: ["wireless:one"],
    skipped_devices: [],
    ...overrides,
  };
}

describe("useProfilesWorkbenchData", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("loads profiles and targetable devices", async () => {
    listProfilesMock.mockResolvedValue([profile()]);
    listDevicesMock.mockResolvedValue([device()]);

    const { result } = renderHook(() => useProfilesWorkbenchData());

    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(result.current.profiles).toHaveLength(1);
    expect(result.current.devices).toHaveLength(1);
  });

  it("creates a profile from the current draft", async () => {
    listProfilesMock.mockResolvedValue([]);
    listDevicesMock.mockResolvedValue([device()]);
    createProfileMock.mockResolvedValue(profile({ id: "focus-mode", name: "Focus Mode" }));

    const { result } = renderHook(() => useProfilesWorkbenchData());
    await waitFor(() => expect(result.current.loading).toBe(false));

    act(() => {
      result.current.setDraft((current) => ({
        ...current,
        id: "focus-mode",
        name: "Focus Mode",
        description: "Work profile",
        lightingEnabled: true,
        fanEnabled: false,
      }));
    });

    await act(async () => {
      await result.current.createDraftProfile();
    });

    expect(createProfileMock).toHaveBeenCalledWith(
      expect.objectContaining({
        id: "focus-mode",
        name: "Focus Mode",
      }),
    );
    expect(result.current.profiles[0]?.id).toBe("focus-mode");
    expect(result.current.success).toContain("Focus Mode");
  });

  it("does not add a profile before the create response resolves", async () => {
    listProfilesMock.mockResolvedValue([]);
    listDevicesMock.mockResolvedValue([device()]);
    const deferred = createDeferred<ProfileDocument>();
    createProfileMock.mockReturnValue(deferred.promise);

    const { result } = renderHook(() => useProfilesWorkbenchData());
    await waitFor(() => expect(result.current.loading).toBe(false));

    act(() => {
      result.current.setDraft((current) => ({
        ...current,
        id: "focus-mode",
        name: "Focus Mode",
      }));
    });

    act(() => {
      void result.current.createDraftProfile();
    });

    expect(result.current.submitting).toBe(true);
    expect(result.current.profiles).toHaveLength(0);

    await act(async () => {
      deferred.resolve(profile({ id: "focus-mode", name: "Focus Mode" }));
      await deferred.promise;
    });

    expect(result.current.profiles[0]?.id).toBe("focus-mode");
  });

  it("validates explicit device targets before creating a profile", async () => {
    listProfilesMock.mockResolvedValue([]);
    listDevicesMock.mockResolvedValue([device()]);

    const { result } = renderHook(() => useProfilesWorkbenchData());
    await waitFor(() => expect(result.current.loading).toBe(false));

    act(() => {
      result.current.setDraft((current) => ({
        ...current,
        id: "device-only",
        name: "Device Only",
        targetMode: "devices",
        selectedDeviceIds: [],
      }));
    });

    await act(async () => {
      await result.current.createDraftProfile();
    });

    expect(createProfileMock).not.toHaveBeenCalled();
    expect(result.current.error).toBe(
      "Select at least one device when using explicit device targets",
    );
  });

  it("deletes a profile and removes it from the local list", async () => {
    listProfilesMock.mockResolvedValue([profile()]);
    listDevicesMock.mockResolvedValue([device()]);
    deleteProfileMock.mockResolvedValue({ deleted: true, id: "night-mode" });

    const { result } = renderHook(() => useProfilesWorkbenchData());
    await waitFor(() => expect(result.current.loading).toBe(false));

    await act(async () => {
      await result.current.removeProfile("night-mode");
    });

    expect(deleteProfileMock).toHaveBeenCalledWith("night-mode");
    expect(result.current.profiles).toHaveLength(0);
  });

  it("does not remove a profile before the delete response resolves", async () => {
    listProfilesMock.mockResolvedValue([profile()]);
    listDevicesMock.mockResolvedValue([device()]);
    const deferred = createDeferred<{ deleted: boolean; id: string }>();
    deleteProfileMock.mockReturnValue(deferred.promise);

    const { result } = renderHook(() => useProfilesWorkbenchData());
    await waitFor(() => expect(result.current.loading).toBe(false));

    act(() => {
      void result.current.removeProfile("night-mode");
    });

    expect(result.current.deletingProfileId).toBe("night-mode");
    expect(result.current.profiles).toHaveLength(1);

    await act(async () => {
      deferred.resolve({ deleted: true, id: "night-mode" });
      await deferred.promise;
    });

    expect(result.current.profiles).toHaveLength(0);
  });

  it("applies a profile and stores the backend result", async () => {
    listProfilesMock.mockResolvedValue([profile()]);
    listDevicesMock.mockResolvedValue([device()]);
    applyProfileMock.mockResolvedValue(
      applyResult({
        skipped_devices: [
          {
            device_id: "wireless:fan",
            section: "lighting",
            reason: "device has no RGB capability",
          },
        ],
      }),
    );

    const { result } = renderHook(() => useProfilesWorkbenchData());
    await waitFor(() => expect(result.current.loading).toBe(false));

    await act(async () => {
      await result.current.runProfile("night-mode");
    });

    expect(applyProfileMock).toHaveBeenCalledWith("night-mode");
    expect(result.current.applyResult?.skipped_devices).toHaveLength(1);
    expect(result.current.success).toContain("Night Mode");
  });
});


