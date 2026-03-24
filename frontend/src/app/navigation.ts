export type NavigationStatus = "ready" | "planned" | "next";

export type NavigationItem = {
  label: string;
  to: string;
  eyebrow: string;
  description: string;
  status: NavigationStatus;
  matchPrefixes?: string[];
};

export const primaryNavigation: NavigationItem[] = [
  {
    label: "Dashboard",
    to: "/",
    eyebrow: "overview",
    description: "Paired clusters at a glance with quick access to fans, lighting, and disconnect.",
    status: "ready",
  },
  {
    label: "Devices",
    to: "/devices",
    eyebrow: "inventory",
    description: "Full device inventory with search, filtering, and access to device workspaces.",
    status: "ready",
    matchPrefixes: ["/devices"],
  },
  {
    label: "Lighting",
    to: "/rgb",
    eyebrow: "rgb",
    description: "Color, effect, and brightness control per cluster or across all paired devices.",
    status: "ready",
  },
  {
    label: "Fans",
    to: "/fans",
    eyebrow: "cooling",
    description: "Manual speed percentage or temperature curve per cluster.",
    status: "ready",
  },
  {
    label: "Wireless Sync",
    to: "/wireless-sync",
    eyebrow: "wireless",
    description: "Pair and manage wireless clusters in a two-tab workflow.",
    status: "ready",
  },
  {
    label: "Profiles",
    to: "/profiles",
    eyebrow: "presets",
    description: "Create, manage, and apply lighting and fan presets.",
    status: "ready",
  },
  {
    label: "Diagnostics",
    to: "/diagnostics",
    eyebrow: "health",
    description: "System status, daemon connectivity, and event stream health.",
    status: "ready",
  },
  {
    label: "Settings",
    to: "/settings",
    eyebrow: "settings",
    description: "System configuration and links to active hardware tools.",
    status: "ready",
  },
  {
    label: "Help",
    to: "/help",
    eyebrow: "docs",
    description: "Browse technical and functional project documentation.",
    status: "ready",
  },
];

export function navigationItemForPath(pathname: string) {
  return (
    primaryNavigation.find((item) => item.to === pathname) ??
    primaryNavigation.find((item) =>
      item.matchPrefixes?.some((prefix) => pathname.startsWith(prefix)),
    ) ??
    primaryNavigation[0]
  );
}
