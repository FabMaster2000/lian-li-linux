import type { ReactNode } from "react";
import { Card } from "./Card";

type PanelProps = {
  title?: string;
  description?: string;
  eyebrow?: string;
  actions?: ReactNode;
  children: ReactNode;
  className?: string;
};

export function Panel({
  title,
  description,
  eyebrow,
  actions,
  children,
  className,
}: PanelProps) {
  return (
    <Card className={className ? `panel ${className}` : "panel"}>
      {title || description || eyebrow || actions ? (
        <div className="panel__header">
          <div>
            {eyebrow ? <p className="section-header__eyebrow">{eyebrow}</p> : null}
            {title ? <h2>{title}</h2> : null}
            {description ? <p>{description}</p> : null}
          </div>
          {actions ? <div className="panel__actions">{actions}</div> : null}
        </div>
      ) : null}
      {children}
    </Card>
  );
}
