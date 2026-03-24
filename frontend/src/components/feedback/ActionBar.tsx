import type { ReactNode } from "react";

type ActionBarProps = {
  summary: string;
  children: ReactNode;
};

export function ActionBar({ summary, children }: ActionBarProps) {
  return (
    <div className="action-bar">
      <p>{summary}</p>
      <div className="action-bar__controls">{children}</div>
    </div>
  );
}
