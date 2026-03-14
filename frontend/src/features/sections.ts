import type { SectionDescriptor } from "../types/section";

export const plannedSections: SectionDescriptor[] = [
  {
    id: "dashboard",
    title: "Fleet Dashboard",
    description:
      "The landing view for device status, availability, and quick entry into lighting or fan controls.",
    status: "next",
    accent: "copper",
    path: "/",
  },
  {
    id: "device-detail",
    title: "Device Detail",
    description:
      "Per-device state, capabilities, and rich drill-down data for wireless and wired controllers.",
    status: "planned",
    accent: "teal",
    path: "/devices",
  },
  {
    id: "lighting",
    title: "Lighting Workbench",
    description:
      "Color, brightness, and effect editing against the backend's lighting endpoints and event stream.",
    status: "planned",
    accent: "ice",
    path: "/lighting",
  },
  {
    id: "fans",
    title: "Fan Console",
    description:
      "Manual control, telemetry snapshots, and later profile-aware fan orchestration.",
    status: "planned",
    accent: "copper",
    path: "/fans",
  },
  {
    id: "profiles",
    title: "Profile Library",
    description:
      "Profile CRUD and apply flows on top of the backend profile storage that already exists.",
    status: "planned",
    accent: "teal",
    path: "/profiles",
  },
  {
    id: "settings",
    title: "System Settings",
    description:
      "Backend runtime info, daemon status, and system-level diagnostics for operations and support.",
    status: "planned",
    accent: "ice",
    path: "/settings",
  },
];
