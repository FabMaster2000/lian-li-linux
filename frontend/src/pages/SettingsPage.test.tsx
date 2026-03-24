import { screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { SettingsPage } from "./SettingsPage";
import { renderAtRoute } from "../test/render";

describe("SettingsPage", () => {
  it("renders a small system page", () => {
    renderAtRoute(<SettingsPage />, {
      initialPath: "/settings",
      routePath: "/settings",
    });

    expect(screen.getByText("Settings")).toBeInTheDocument();
    expect(screen.getByText("Allgemeine Einstellungen")).toBeInTheDocument();
    expect(document.title).toBe("Settings - Lian Li Control Surface");
  });
});
