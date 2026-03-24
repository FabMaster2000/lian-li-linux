import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { StatusBadge } from "./ui/StatusBadge";

describe("StatusBadge", () => {
  it("renders its label with the requested tone", () => {
    render(<StatusBadge tone="online">online</StatusBadge>);

    expect(screen.getByText("online")).toHaveClass("status-badge", "status-badge--online");
  });
});
