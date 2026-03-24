type SectionHeaderProps = {
  eyebrow: string;
  title: string;
  description: string;
};

export function SectionHeader({ eyebrow, title, description }: SectionHeaderProps) {
  return (
    <header className="section-header">
      <div>
        <p className="section-header__eyebrow">{eyebrow}</p>
        <h2>{title}</h2>
      </div>
      <p>{description}</p>
    </header>
  );
}
