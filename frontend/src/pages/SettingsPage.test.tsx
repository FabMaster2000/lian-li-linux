import { screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { SettingsPage } from "./SettingsPage";
import { renderAtRoute } from "../test/render";

describe("SettingsPage", () => {
  it("renders the current placeholder route and updates the document title", () => {
    renderAtRoute(<SettingsPage />, {
      initialPath: "/settings",
      routePath: "/settings",
    });

    expect(screen.getByText("Runtime and system settings")).toBeInTheDocument();
    expect(screen.getByText("Planned controls")).toBeInTheDocument();
    expect(screen.getByText("Backend runtime panel")).toBeInTheDocument();
    expect(screen.getByText("Daemon reachability panel")).toBeInTheDocument();
    expect(document.title).toBe("Settings - Lian Li Control Surface");
  });
});
