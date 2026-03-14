import { render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  SystemStatusProvider,
  useSystemStatus,
} from "./SystemStatusProvider";
import { getDaemonStatus, getHealth } from "../services/system";

vi.mock("../services/system", () => ({
  getHealth: vi.fn(),
  getDaemonStatus: vi.fn(),
}));

vi.mock("../hooks/useBackendEventSubscription", () => ({
  useBackendEventSubscription: vi.fn(),
}));

const getHealthMock = vi.mocked(getHealth);
const getDaemonStatusMock = vi.mocked(getDaemonStatus);

function Probe() {
  const { apiReachable, apiError, daemonStatus } = useSystemStatus();

  return (
    <>
      <span>{apiReachable ? "api-up" : "api-down"}</span>
      <span>{daemonStatus?.reachable ? "daemon-up" : "daemon-down"}</span>
      <span>{apiError ?? "no-error"}</span>
    </>
  );
}

describe("SystemStatusProvider", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("loads health and daemon status on mount", async () => {
    getHealthMock.mockResolvedValue({ status: "ok" });
    getDaemonStatusMock.mockResolvedValue({
      reachable: true,
      socket_path: "/tmp/lianli-daemon.sock",
      error: null,
    });

    render(
      <SystemStatusProvider>
        <Probe />
      </SystemStatusProvider>,
    );

    await waitFor(() => expect(screen.getByText("api-up")).toBeInTheDocument());
    expect(screen.getByText("daemon-up")).toBeInTheDocument();
    expect(screen.getByText("no-error")).toBeInTheDocument();
  });

  it("marks the api as unreachable when the health request fails", async () => {
    getHealthMock.mockRejectedValue(new Error("network down"));
    getDaemonStatusMock.mockResolvedValue({
      reachable: false,
      socket_path: "/tmp/lianli-daemon.sock",
      error: "daemon not reachable",
    });

    render(
      <SystemStatusProvider>
        <Probe />
      </SystemStatusProvider>,
    );

    await waitFor(() => expect(screen.getByText("api-down")).toBeInTheDocument());
    expect(screen.getByText("network down")).toBeInTheDocument();
  });
});
