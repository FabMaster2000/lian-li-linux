import { Outlet, useLocation } from "react-router-dom";
import { useBackendEvents } from "./BackendEventsProvider";
import { useSystemStatus } from "./SystemStatusProvider";
import { GlobalSystemNotices } from "../components/GlobalSystemNotices";
import { AppShell } from "../components/layout/AppShell";
import { SidebarNav } from "../components/layout/SidebarNav";
import { TopBar } from "../components/layout/TopBar";
import { navigationItemForPath, primaryNavigation } from "./navigation";

export function AppLayout() {
  const location = useLocation();
  const { connectionState } = useBackendEvents();
  const { apiReachable, daemonStatus } = useSystemStatus();
  const currentSection = navigationItemForPath(location.pathname);

  return (
    <AppShell
      sidebar={
        <SidebarNav eventConnectionState={connectionState} items={primaryNavigation} />
      }
      topBar={
        <TopBar
          apiReachable={apiReachable}
          connectionState={connectionState}
          daemonReachable={daemonStatus?.reachable ?? null}
          sectionDescription={currentSection.description}
          sectionLabel={currentSection.label}
        />
      }
    >
      <div className="app-main">
        <GlobalSystemNotices />
        <Outlet />
      </div>
    </AppShell>
  );
}
