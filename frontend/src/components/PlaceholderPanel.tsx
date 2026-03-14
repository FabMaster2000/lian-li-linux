type PlaceholderPanelProps = {
  title: string;
  description: string;
  items: string[];
};

export function PlaceholderPanel({
  title,
  description,
  items,
}: PlaceholderPanelProps) {
  return (
    <section className="content-panel">
      <div className="content-panel__header">
        <h2>{title}</h2>
        <p>{description}</p>
      </div>
      <ul className="content-list">
        {items.map((item) => (
          <li key={item}>{item}</li>
        ))}
      </ul>
    </section>
  );
}
