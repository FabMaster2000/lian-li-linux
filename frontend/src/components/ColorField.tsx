type ColorFieldProps = {
  label: string;
  value: string;
  onChange: (value: string) => void;
  disabled?: boolean;
  pickerAriaLabel?: string;
  placeholder?: string;
};

export function ColorField({
  label,
  value,
  onChange,
  disabled = false,
  pickerAriaLabel = `${label} picker`,
  placeholder = "#ffffff",
}: ColorFieldProps) {
  return (
    <label className="field-group field-group--color">
      <span className="field-group__label">{label}</span>
      <div className="color-input-row">
        <input
          aria-label={pickerAriaLabel}
          className="color-input"
          disabled={disabled}
          onChange={(event) => onChange(event.target.value)}
          type="color"
          value={value}
        />
        <input className="field-input" placeholder={placeholder} readOnly value={value} />
      </div>
    </label>
  );
}
