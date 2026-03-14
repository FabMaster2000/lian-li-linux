import { PageIntro } from "../components/PageIntro";
import { PlaceholderPanel } from "../components/PlaceholderPanel";
import { useDocumentTitle } from "../hooks/useDocumentTitle";

export function SettingsPage() {
  useDocumentTitle("Settings - Lian Li Control Surface");

  return (
    <main className="page-shell">
      <PageIntro
        eyebrow="system"
        title="Runtime and system settings"
        description="This route is reserved for backend runtime info, daemon reachability, and operational diagnostics."
      />

      <PlaceholderPanel
        title="Planned controls"
        description="The route is intentionally broad enough for operational data and future backend toggles."
        items={[
          "Backend runtime panel",
          "Daemon reachability panel",
          "OpenRGB and config status",
          "Support diagnostics",
        ]}
      />
    </main>
  );
}
