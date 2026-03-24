import type { ReactNode } from "react";

type AppShellProps = {
  sidebar: ReactNode;
  topBar: ReactNode;
  children: ReactNode;
};

export function AppShell({ sidebar, topBar, children }: AppShellProps) {
  return (
    <div className="app-shell">
      <aside className="app-shell__sidebar">{sidebar}</aside>
      <div className="app-shell__main">
        {topBar}
        <div className="app-shell__content">{children}</div>
      </div>
    </div>
  );
}
