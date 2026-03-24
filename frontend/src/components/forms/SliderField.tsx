import type { ReactNode } from "react";

type SliderFieldProps = {
  label: string;
  value: number;
  onChange: (value: number) => void;
  min?: number;
  max?: number;
  disabled?: boolean;
  className?: string;
  suffix?: string;
  children?: ReactNode;
};

export function SliderField({
  label,
  value,
  onChange,
  min = 0,
  max = 100,
  disabled = false,
  className,
  suffix = "%",
  children,
}: SliderFieldProps) {
  return (
    <label className={className ? `field-group ${className}` : "field-group"}>
      <span className="field-group__label">
        {label}
        <strong className="slider-value">
          {value}
          {suffix}
        </strong>
      </span>
      <input
        className="slider-input"
        disabled={disabled}
        max={max}
        min={min}
        onChange={(event) => onChange(Number(event.target.value))}
        type="range"
        value={value}
      />
      {children}
    </label>
  );
}
