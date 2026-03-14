import type { ReactNode } from "react";

export type StatusBadgeTone =
  | "online"
  | "offline"
  | "next"
  | "planned"
  | "ready"
  | "warning";

type StatusBadgeProps = {
  tone: StatusBadgeTone;
  children: ReactNode;
};

export function StatusBadge({ tone, children }: StatusBadgeProps) {
  return <span className={`status-badge status-badge--${tone}`}>{children}</span>;
}
