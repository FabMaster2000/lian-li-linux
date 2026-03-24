import type { SectionDescriptor } from "../types/section";

export const plannedSections: SectionDescriptor[] = [
  {
    id: "dashboard",
    title: "Fleet Dashboard",
    description:
      "The landing view for device status, availability, and quick entry into the current control surfaces.",
    status: "ready",
    accent: "copper",
    path: "/",
  },
  {
    id: "devices",
    title: "Devices",
    description:
      "Device, controller, wireless, and fan information in one lighter inventory view.",
    status: "ready",
    accent: "teal",
    path: "/devices",
  },
  {
    id: "lighting",
    title: "Lighting Workbench",
    description:
      "Color and effect editing against the backend lighting endpoints.",
    status: "ready",
    accent: "ice",
    path: "/lighting",
  },
  {
    id: "fans",
    title: "Fan Console",
    description:
      "Manual control, telemetry snapshots, and later profile-aware fan orchestration.",
    status: "ready",
    accent: "copper",
    path: "/fans",
  },
  {
    id: "wireless",
    title: "Wireless Sync",
    description:
      "A slim pairing view for discovering wireless devices and managing future bind flows.",
    status: "next",
    accent: "teal",
    path: "/wireless-sync",
  },
  {
    id: "profiles",
    title: "Profile Library",
    description:
      "Profile CRUD and apply flows on top of the backend profile storage that already exists.",
    status: "ready",
    accent: "teal",
    path: "/profiles",
  },
  {
    id: "docs",
    title: "Help / Docs",
    description:
      "An in-app entry point into the technical and functional documentation split.",
    status: "next",
    accent: "teal",
    path: "/help",
  },
];
