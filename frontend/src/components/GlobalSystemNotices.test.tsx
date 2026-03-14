import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { GlobalSystemNotices } from "./GlobalSystemNotices";
import { useBackendEvents } from "../app/BackendEventsProvider";
import { useSystemStatus } from "../app/SystemStatusProvider";

vi.mock("../app/BackendEventsProvider", () => ({
  useBackendEvents: vi.fn(),
}));

vi.mock("../app/SystemStatusProvider", () => ({
  useSystemStatus: vi.fn(),
}));

const useBackendEventsMock = vi.mocked(useBackendEvents);
const useSystemStatusMock = vi.mocked(useSystemStatus);

describe("GlobalSystemNotices", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("shows the api error as the primary global notice", () => {
    useSystemStatusMock.mockReturnValue({
      apiReachable: false,
      apiError: "backend unavailable",
      daemonStatus: null,
      loading: false,
      refreshing: false,
      refresh: vi.fn(),
    });
    useBackendEventsMock.mockReturnValue({
      connectionState: "disconnected",
      subscribe: vi.fn(),
    });

    render(<GlobalSystemNotices />);

    expect(screen.getByText("API unreachable.")).toBeInTheDocument();
    expect(screen.getByText("backend unavailable")).toBeInTheDocument();
    expect(screen.queryByText("Live event stream disconnected.")).not.toBeInTheDocument();
  });

  it("shows daemon and websocket notices when the api is still reachable", () => {
    useSystemStatusMock.mockReturnValue({
      apiReachable: true,
      apiError: null,
      daemonStatus: {
        reachable: false,
        socket_path: "/tmp/lianli-daemon.sock",
        error: "socket timeout",
      },
      loading: false,
      refreshing: false,
      refresh: vi.fn(),
    });
    useBackendEventsMock.mockReturnValue({
      connectionState: "reconnecting",
      subscribe: vi.fn(),
    });

    render(<GlobalSystemNotices />);

    expect(screen.getByText("Daemon unavailable.")).toBeInTheDocument();
    expect(screen.getByText("socket timeout")).toBeInTheDocument();
    expect(screen.getByText("Live event stream disconnected.")).toBeInTheDocument();
  });
});
