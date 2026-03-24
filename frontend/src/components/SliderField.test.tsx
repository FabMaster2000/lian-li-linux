import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { SliderField } from "./forms/SliderField";

describe("SliderField", () => {
  it("shows the current value and forwards slider updates", () => {
    const onChange = vi.fn();

    render(<SliderField label="Brightness" onChange={onChange} value={25} />);

    expect(screen.getByText("25%")).toBeInTheDocument();

    fireEvent.change(screen.getByRole("slider"), {
      target: { value: "60" },
    });

    expect(onChange).toHaveBeenCalledWith(60);
  });
});
