import { useState } from "react";
import { Link } from "react-router-dom";
import { PageHeader } from "../components/layout/PageHeader";
import { Panel } from "../components/ui/Panel";
import { StatTile } from "../components/ui/StatTile";
import { Tabs } from "../components/ui/Tabs";
import { useDocumentTitle } from "../hooks/useDocumentTitle";

const helpTabs = [
  { id: "technical", label: "Technical docs" },
  { id: "functional", label: "Functional docs" },
  { id: "routes", label: "Route map" },
];

export function HelpDocsPage() {
  useDocumentTitle("Help / Docs - Lian Li Control Surface");
  const [activeTab, setActiveTab] = useState("technical");

  const tabContent =
    activeTab === "technical"
      ? {
          title: "Technical documentation",
          description: "Architecture, backend, frontend, deployment, testing, and development references.",
          items: [
            "Start with docs/technical/README.md",
            "Use the system, backend, frontend, and daemon-integration foundations",
            "Check the source-of-truth matrix before duplicating ownership",
          ],
        }
      : activeTab === "functional"
        ? {
            title: "Functional documentation",
            description: "User guides, feature behavior, and future-facing workflow specifications.",
            items: [
              "Start with docs/functional/README.md",
              "Current guides cover dashboard, devices, lighting, fans, and profiles",
              "Future workbenches are documented incrementally as their phases land",
            ],
          }
      : {
            title: "Current route map",
            description: "The redesigned application shell now focuses on the routes that are active right now.",
            items: [
              "Live routes: dashboard, devices, lighting, fans, profiles, wireless sync, help",
              "Removed from the primary surface for now: settings, diagnostics, LCD / media",
              "Device detail remains nested under the Devices workflow",
            ],
          };

  return (
    <main className="page-shell">
      <PageHeader
        actions={
          <div className="page-header__button-group">
            <Link className="button-link button-link--primary" to="/">
              Open dashboard
            </Link>
            <Link className="button-link" to="/devices">
              Open devices
            </Link>
          </div>
        }
        description="Provide a dedicated in-app help and documentation entry point that mirrors the new technical and functional documentation structure."
        eyebrow="help / docs"
        title="Documentation entry points"
      />

      <section className="summary-strip">
        <StatTile detail="Top-level documentation split is established." label="Docs structure" tone="success" value="separated" />
        <StatTile detail="The app shell now reserves a dedicated help route." label="Navigation" tone="accent" value="first-class" />
        <StatTile detail="Use the repository docs for deeper implementation detail." label="Depth" value="repo docs" />
      </section>

      <Panel
        description="Switch between the main documentation audiences and the current route model."
        eyebrow="navigation help"
        title="Documentation map"
      >
        <Tabs items={helpTabs} label="Documentation categories" onChange={setActiveTab} value={activeTab} />
        <div className="docs-map">
          <h3>{tabContent.title}</h3>
          <p>{tabContent.description}</p>
          <ul className="content-list">
            {tabContent.items.map((item) => (
              <li key={item}>{item}</li>
            ))}
          </ul>
        </div>
      </Panel>
    </main>
  );
}
