import { Link } from "react-router-dom";
import { StatusBadge } from "./ui/StatusBadge";
import type { SectionDescriptor } from "../types/section";

type SectionCardProps = {
  section: SectionDescriptor;
};

export function SectionCard({ section }: SectionCardProps) {
  return (
    <Link className="section-card" data-accent={section.accent} to={section.path}>
      <div className="section-card__header">
        <p className="section-card__eyebrow">{section.id}</p>
        <StatusBadge tone={section.status}>{section.status}</StatusBadge>
      </div>
      <h2>{section.title}</h2>
      <p>{section.description}</p>
    </Link>
  );
}
