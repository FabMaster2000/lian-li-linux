import { render, screen } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { AppLayout } from "./AppLayout";
import { useBackendEvents } from "./BackendEventsProvider";
import { useSystemStatus } from "./SystemStatusProvider";
import { routerFuture } from "../test/render";

vi.mock("./BackendEventsProvider", () => ({
  useBackendEvents: vi.fn(),
}));

vi.mock("./SystemStatusProvider", () => ({
  useSystemStatus: vi.fn(),
}));

vi.mock("../components/GlobalSystemNotices", () => ({
  GlobalSystemNotices: () => <div>system notices</div>,
}));

const useBackendEventsMock = vi.mocked(useBackendEvents);
const useSystemStatusMock = vi.mocked(useSystemStatus);

describe("AppLayout", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useBackendEventsMock.mockReturnValue({
      connectionState: "connected",
      subscribe: vi.fn(() => () => undefined),
    });
    useSystemStatusMock.mockReturnValue({
      apiReachable: true,
      apiError: null,
      daemonStatus: {
        reachable: true,
        socket_path: "/tmp/lianli-daemon.sock",
        error: null,
      },
      loading: false,
      refreshing: false,
      refresh: vi.fn(),
    });
  });

  it("renders the redesigned app shell, active route, and top-bar runtime badges", () => {
    render(
      <MemoryRouter future={routerFuture} initialEntries={["/fans"]}>
        <Routes>
          <Route element={<AppLayout />}>
            <Route index element={<div>dashboard content</div>} />
            <Route path="/rgb" element={<div>rgb content</div>} />
            <Route path="/rgb-effects" element={<div>rgb effects content</div>} />
            <Route path="/fans" element={<div>fans content</div>} />
            <Route path="/wireless-sync" element={<div>wireless content</div>} />
          </Route>
        </Routes>
      </MemoryRouter>,
    );

    expect(screen.getByText("Control Suite")).toBeInTheDocument();
    expect(screen.getByText("system notices")).toBeInTheDocument();
    expect(screen.getByText("fans content")).toBeInTheDocument();
    expect(screen.getByText("api reachable")).toBeInTheDocument();
    expect(screen.getByText("daemon reachable")).toBeInTheDocument();
    expect(screen.getByText("ws connected")).toBeInTheDocument();

    const fansLink = screen
      .getAllByRole("link")
      .find((link) => link.getAttribute("href") === "/fans");
    const dashboardLink = screen
      .getAllByRole("link")
      .find((link) => link.getAttribute("href") === "/");

    expect(fansLink).toBeDefined();
    expect(dashboardLink).toBeDefined();
    expect(fansLink).toHaveClass("app-nav__link--active");
    expect(dashboardLink).toHaveClass("app-nav__link");
    expect(dashboardLink).not.toHaveClass("app-nav__link--active");
    const rgbLink = screen
      .getAllByRole("link")
      .find((link) => link.getAttribute("href") === "/rgb");
    const wirelessLink = screen
      .getAllByRole("link")
      .find((link) => link.getAttribute("href") === "/wireless-sync");

    expect(rgbLink).toBeDefined();
    expect(wirelessLink).toBeDefined();

    const devicesLink = screen
      .getAllByRole("link")
      .find((link) => link.getAttribute("href") === "/devices");

    expect(devicesLink).toBeDefined();
  });
});
