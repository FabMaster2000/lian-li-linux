import { render, screen } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { AppLayout } from "./AppLayout";
import { useBackendEvents } from "./BackendEventsProvider";
import { routerFuture } from "../test/render";

vi.mock("./BackendEventsProvider", () => ({
  useBackendEvents: vi.fn(),
}));

vi.mock("../components/GlobalSystemNotices", () => ({
  GlobalSystemNotices: () => <div>system notices</div>,
}));

const useBackendEventsMock = vi.mocked(useBackendEvents);

describe("AppLayout", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useBackendEventsMock.mockReturnValue({
      connectionState: "connected",
      subscribe: vi.fn(() => () => undefined),
    });
  });

  it("renders the navigation shell, active route, and live-event status", () => {
    render(
      <MemoryRouter future={routerFuture} initialEntries={["/fans"]}>
        <Routes>
          <Route element={<AppLayout />}>
            <Route index element={<div>dashboard content</div>} />
            <Route path="/devices" element={<div>devices content</div>} />
            <Route path="/lighting" element={<div>lighting content</div>} />
            <Route path="/fans" element={<div>fans content</div>} />
            <Route path="/profiles" element={<div>profiles content</div>} />
            <Route path="/settings" element={<div>settings content</div>} />
          </Route>
        </Routes>
      </MemoryRouter>,
    );

    expect(screen.getByText("Live events: connected")).toBeInTheDocument();
    expect(screen.getByText("system notices")).toBeInTheDocument();
    expect(screen.getByText("fans content")).toBeInTheDocument();

    const fansLink = screen.getByText("Fans").closest("a");
    const dashboardLink = screen.getByText("Dashboard").closest("a");

    expect(fansLink).toHaveClass("app-nav__link--active");
    expect(dashboardLink).toHaveClass("app-nav__link");
    expect(dashboardLink).not.toHaveClass("app-nav__link--active");
  });
});
