export type NavigationItem = {
  label: string;
  to: string;
  eyebrow: string;
};

export const primaryNavigation: NavigationItem[] = [
  {
    label: "Dashboard",
    to: "/",
    eyebrow: "overview",
  },
  {
    label: "Device Detail",
    to: "/devices",
    eyebrow: "device",
  },
  {
    label: "Lighting",
    to: "/lighting",
    eyebrow: "rgb",
  },
  {
    label: "Fans",
    to: "/fans",
    eyebrow: "cooling",
  },
  {
    label: "Profiles",
    to: "/profiles",
    eyebrow: "presets",
  },
  {
    label: "Settings",
    to: "/settings",
    eyebrow: "system",
  },
];
