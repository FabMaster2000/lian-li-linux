type StatTileProps = {
  label: string;
  value: string;
  detail: string;
  tone?: "default" | "accent" | "success" | "warning";
};

export function StatTile({ label, value, detail, tone = "default" }: StatTileProps) {
  return (
    <article className={`stat-tile stat-tile--${tone}`}>
      <p className="stat-tile__label">{label}</p>
      <strong>{value}</strong>
      <span>{detail}</span>
    </article>
  );
}
