import { PageIntro } from "../components/PageIntro";
import { Panel } from "../components/ui/Panel";
import { useDocumentTitle } from "../hooks/useDocumentTitle";

export function SettingsPage() {
  useDocumentTitle("Settings - Lian Li Control Surface");

  return (
    <main className="page-shell">
      <PageIntro
        eyebrow="settings"
        title="Settings"
        description="Kleine Systemseite fuer allgemeine Hinweise und Wege in die aktiven Hardware-Werkzeuge."
      />

      <Panel
        eyebrow="allgemein"
        title="Allgemeine Einstellungen"
        description="Kleine Systemseite mit Hinweisen zu den aktiven Werkzeugen."
      >
        <p>
          Die RGB-Steuerung ist auf den Meteor-Workflow reduziert. Verwende die RGB-Effekte-Seite,
          um den Meteor-Effekt ueber mehrere Luefter und Cluster zu steuern.
        </p>
      </Panel>
    </main>
  );
}
