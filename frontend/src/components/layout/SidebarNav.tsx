import { NavLink } from "react-router-dom";
import { StatusBadge } from "../ui/StatusBadge";
import type { NavigationItem } from "../../app/navigation";

type SidebarNavProps = {
  items: NavigationItem[];
  eventConnectionState: string;
};

export function SidebarNav({ items, eventConnectionState }: SidebarNavProps) {
  return (
    <div className="sidebar-nav">
      <section className="sidebar-nav__brand">
        <p className="sidebar-nav__brand-mark">Lian Li Linux</p>
        <h1>Control Suite</h1>
        <p>
          Browser-native lighting, cooling, and device control on top of the daemon-backed Linux
          stack.
        </p>
        <div className="sidebar-nav__brand-status">
          <StatusBadge tone="info">events {eventConnectionState}</StatusBadge>
        </div>
      </section>

      <nav aria-label="Primary" className="sidebar-nav__list">
        {items.map((item) => (
          <NavLink
            className={({ isActive }) =>
              isActive
                ? "sidebar-nav__link app-nav__link sidebar-nav__link--active app-nav__link--active"
                : "sidebar-nav__link app-nav__link"
            }
            end={item.to === "/"}
            key={item.to}
            to={item.to}
          >
            <span className="sidebar-nav__eyebrow app-nav__eyebrow">{item.eyebrow}</span>
            <span className="sidebar-nav__main">
              <span className="sidebar-nav__label app-nav__label">{item.label}</span>
              <StatusBadge tone={item.status}>{item.status}</StatusBadge>
            </span>
            <span className="sidebar-nav__description">{item.description}</span>
          </NavLink>
        ))}
      </nav>
    </div>
  );
}
