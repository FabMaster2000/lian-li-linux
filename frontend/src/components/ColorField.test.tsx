import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ColorField } from "./forms/ColorField";

describe("ColorField", () => {
  it("renders the current color and forwards picker changes", () => {
    const onChange = vi.fn();

    render(
      <ColorField
        label="Color"
        onChange={onChange}
        pickerAriaLabel="Lighting color picker"
        value="#112233"
      />,
    );

    expect(screen.getByLabelText("Lighting color picker")).toHaveValue("#112233");

    fireEvent.change(screen.getByLabelText("Lighting color picker"), {
      target: { value: "#445566" },
    });

    expect(onChange).toHaveBeenCalledWith("#445566");
  });
});
