import { fireEvent, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ProfilesPage } from "./ProfilesPage";
import { renderAtRoute } from "../test/render";
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

describe("ProfilesPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    listDevicesMock.mockResolvedValue([device()]);
    listProfilesMock.mockResolvedValue([profile()]);
  });

  it("renders stored profiles from the backend", async () => {
    renderAtRoute(<ProfilesPage />, {
      initialPath: "/profiles",
      routePath: "/profiles",
    });

    await waitFor(() => expect(screen.getByText("Stored profiles")).toBeInTheDocument());

    expect(screen.getByText("Night Mode")).toBeInTheDocument();
    expect(screen.getByText("Dim everything")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Apply" })).toBeInTheDocument();
  });

  it("creates a profile from the form", async () => {
    const user = userEvent.setup();
    createProfileMock.mockResolvedValue(profile({ id: "focus-mode", name: "Focus Mode" }));

    renderAtRoute(<ProfilesPage />, {
      initialPath: "/profiles",
      routePath: "/profiles",
    });

    await screen.findByRole("heading", { name: "Create profile" });

    await user.clear(screen.getByPlaceholderText("night-mode"));
    await user.type(screen.getByPlaceholderText("night-mode"), "focus-mode");
    await user.clear(screen.getByPlaceholderText("Night Mode"));
    await user.type(screen.getByPlaceholderText("Night Mode"), "Focus Mode");
    await user.clear(screen.getByPlaceholderText("Dim lighting and reduce fan speed"));
    await user.type(screen.getByPlaceholderText("Dim lighting and reduce fan speed"), "Work profile");

    fireEvent.change(screen.getByLabelText("Profile lighting color picker"), {
      target: { value: "#445566" },
    });
    fireEvent.change(screen.getByRole("slider", { name: /Brightness/i }), {
      target: { value: "35" },
    });

    await user.click(screen.getByRole("button", { name: "Create profile" }));

    await waitFor(() =>
      expect(createProfileMock).toHaveBeenCalledWith(
        expect.objectContaining({
          id: "focus-mode",
          name: "Focus Mode",
          description: "Work profile",
        }),
      ),
    );

    expect(screen.getByText("Profile action completed.")).toBeInTheDocument();
    expect(screen.getByText("Focus Mode")).toBeInTheDocument();
  });

  it("deletes a stored profile", async () => {
    const user = userEvent.setup();
    deleteProfileMock.mockResolvedValue({ deleted: true, id: "night-mode" });

    renderAtRoute(<ProfilesPage />, {
      initialPath: "/profiles",
      routePath: "/profiles",
    });

    await waitFor(() => expect(screen.getByText("Night Mode")).toBeInTheDocument());

    await user.click(screen.getByRole("button", { name: "Delete" }));

    await waitFor(() => expect(deleteProfileMock).toHaveBeenCalledWith("night-mode"));
    expect(screen.queryByText("Night Mode")).not.toBeInTheDocument();
  });

  it("applies a profile and shows skipped devices", async () => {
    const user = userEvent.setup();
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

    renderAtRoute(<ProfilesPage />, {
      initialPath: "/profiles",
      routePath: "/profiles",
    });

    await waitFor(() => expect(screen.getByRole("button", { name: "Apply" })).toBeInTheDocument());

    await user.click(screen.getByRole("button", { name: "Apply" }));

    await waitFor(() => expect(applyProfileMock).toHaveBeenCalledWith("night-mode"));

    expect(screen.getByText("Last apply result")).toBeInTheDocument();
    expect(screen.getByText("wireless:fan")).toBeInTheDocument();
    expect(screen.getByText("device has no RGB capability")).toBeInTheDocument();
  });

  it("validates explicit device targets before create", async () => {
    const user = userEvent.setup();

    renderAtRoute(<ProfilesPage />, {
      initialPath: "/profiles",
      routePath: "/profiles",
    });

    await screen.findByRole("heading", { name: "Create profile" });

    await user.clear(screen.getByPlaceholderText("night-mode"));
    await user.type(screen.getByPlaceholderText("night-mode"), "device-only");
    await user.clear(screen.getByPlaceholderText("Night Mode"));
    await user.type(screen.getByPlaceholderText("Night Mode"), "Device Only");
    await user.selectOptions(screen.getByRole("combobox", { name: "Targets" }), "devices");
    await user.click(screen.getByRole("button", { name: "Create profile" }));

    expect(createProfileMock).not.toHaveBeenCalled();
    expect(await screen.findByText("Select at least one device when using explicit device targets")).toBeInTheDocument();
  });
});
