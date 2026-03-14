import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { EffectSelect } from "./EffectSelect";

describe("EffectSelect", () => {
  it("renders effect options and forwards selection changes", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();

    render(
      <EffectSelect
        onChange={onChange}
        options={[
          { value: "Static", label: "Static" },
          { value: "Rainbow", label: "Rainbow" },
        ]}
        value="Static"
      />,
    );

    await user.selectOptions(screen.getByRole("combobox", { name: "Effect" }), "Rainbow");

    expect(onChange).toHaveBeenCalledWith("Rainbow");
  });
});
