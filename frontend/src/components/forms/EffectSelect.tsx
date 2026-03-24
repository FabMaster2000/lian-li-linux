type EffectOption = {
  value: string;
  label: string;
};

type EffectSelectProps = {
  label?: string;
  value: string;
  options: EffectOption[];
  onChange: (value: string) => void;
  disabled?: boolean;
};

export function EffectSelect({
  label = "Effect",
  value,
  options,
  onChange,
  disabled = false,
}: EffectSelectProps) {
  return (
    <label className="field-group">
      <span className="field-group__label">{label}</span>
      <select
        className="field-input"
        disabled={disabled}
        onChange={(event) => onChange(event.target.value)}
        value={value}
      >
        {options.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
    </label>
  );
}
