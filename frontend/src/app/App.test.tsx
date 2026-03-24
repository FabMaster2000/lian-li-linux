import type { ReactNode } from "react";
import { render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";

vi.mock("./BackendEventsProvider", () => ({
  BackendEventsProvider: ({ children }: { children: ReactNode }) => children,
  useBackendEvents: () => ({
    connectionState: "connected",
    subscribe: vi.fn(() => () => undefined),
  }),
}));

vi.mock("./SystemStatusProvider", () => ({
  SystemStatusProvider: ({ children }: { children: ReactNode }) => children,
  useSystemStatus: () => ({
    apiReachable: true,
    apiError: null,
    daemonStatus: {
      reachable: true,
      socket_path: "/tmp/lianli.sock",
      error: null,
    },
  }),
}));

vi.mock("../components/GlobalSystemNotices", () => ({
  GlobalSystemNotices: () => null,
}));

vi.mock("../pages/DashboardPage", () => ({
  DashboardPage: () => <div>dashboard page</div>,
}));

vi.mock("../pages/FansPage", () => ({
  FansPage: () => <div>fans page</div>,
}));

vi.mock("../pages/LightingPage", () => ({
  LightingPage: () => <div>rgb page</div>,
}));

vi.mock("../pages/WirelessSyncPage", () => ({
  WirelessSyncPage: () => <div>wireless page</div>,
}));

describe("App", () => {
  beforeEach(() => {
    window.history.replaceState({}, "", "/");
  });

  it("redirects legacy /lighting routes to /rgb", async () => {
    window.history.replaceState({}, "", "/lighting?cluster=desk-cluster");

    render(<App />);

    await waitFor(() => {
      expect(screen.getByText("rgb page")).toBeInTheDocument();
    });
    expect(window.location.pathname).toBe("/rgb");
    expect(window.location.search).toBe("?cluster=desk-cluster");
  });

  it("redirects legacy /rgb-effects route to /rgb", async () => {
    window.history.replaceState({}, "", "/rgb-effects?cluster=desk-cluster");

    render(<App />);

    await waitFor(() => {
      expect(screen.getByText("rgb page")).toBeInTheDocument();
    });
    expect(window.location.pathname).toBe("/rgb");
  });
});
