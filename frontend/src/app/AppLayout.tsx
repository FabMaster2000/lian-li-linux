import { NavLink, Outlet } from "react-router-dom";
import { useBackendEvents } from "./BackendEventsProvider";
import { GlobalSystemNotices } from "../components/GlobalSystemNotices";
import { primaryNavigation } from "./navigation";

export function AppLayout() {
  const { connectionState } = useBackendEvents();

  return (
    <div className="app-frame">
      <aside className="app-sidebar">
        <div className="brand-panel">
          <p className="brand-panel__eyebrow">Lian Li Linux</p>
          <h1>Control Surface</h1>
          <p>
            A web operator panel for lighting, cooling, profiles, and daemon
            visibility.
          </p>
          <small className="brand-panel__status">Live events: {connectionState}</small>
        </div>

        <nav className="app-nav" aria-label="Primary">
          {primaryNavigation.map((item) => (
            <NavLink
              key={item.to}
              className={({ isActive }) =>
                isActive ? "app-nav__link app-nav__link--active" : "app-nav__link"
              }
              end={item.to === "/"}
              to={item.to}
            >
              <span className="app-nav__eyebrow">{item.eyebrow}</span>
              <span className="app-nav__label">{item.label}</span>
            </NavLink>
          ))}
        </nav>
      </aside>

      <div className="app-main">
        <GlobalSystemNotices />
        <Outlet />
      </div>
    </div>
  );
}
