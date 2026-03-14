import type { ReactNode } from "react";

type PageIntroProps = {
  eyebrow: string;
  title: string;
  description: string;
  aside?: ReactNode;
};

export function PageIntro({
  eyebrow,
  title,
  description,
  aside,
}: PageIntroProps) {
  return (
    <section className="hero-panel">
      <div className="hero-copy">
        <p className="hero-kicker">{eyebrow}</p>
        <h1>{title}</h1>
        <p className="hero-body">{description}</p>
      </div>
      {aside ? <div className="hero-aside">{aside}</div> : null}
    </section>
  );
}
