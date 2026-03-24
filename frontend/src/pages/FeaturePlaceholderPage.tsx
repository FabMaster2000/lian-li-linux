import { Link } from "react-router-dom";
import { ActionBar } from "../components/feedback/ActionBar";
import { InlineNotice } from "../components/feedback/InlineNotice";
import { PageHeader } from "../components/layout/PageHeader";
import { Panel } from "../components/ui/Panel";
import { StatTile } from "../components/ui/StatTile";

type FeaturePlaceholderPageProps = {
  eyebrow: string;
  title: string;
  description: string;
  primaryTitle: string;
  primaryDescription: string;
  primaryItems: string[];
  secondaryTitle: string;
  secondaryDescription: string;
  secondaryItems: string[];
};

export function FeaturePlaceholderPage({
  eyebrow,
  title,
  description,
  primaryTitle,
  primaryDescription,
  primaryItems,
  secondaryTitle,
  secondaryDescription,
  secondaryItems,
}: FeaturePlaceholderPageProps) {
  return (
    <main className="page-shell">
      <PageHeader
        actions={
          <div className="page-header__button-group">
            <Link className="button-link button-link--primary" to="/devices">
              Open device inventory
            </Link>
            <Link className="button-link" to="/help">
              Read planning context
            </Link>
          </div>
        }
        description={description}
        eyebrow={eyebrow}
        title={title}
      />

      <section className="summary-strip">
        <StatTile detail="Visible in the new top-level navigation." label="Navigation" tone="accent" value="live" />
        <StatTile detail="Workflow details land in their dedicated later phase." label="Delivery state" tone="warning" value="planned" />
        <StatTile detail="This page reserves the route, layout, and support context." label="Purpose" value="foundation" />
      </section>

      <InlineNotice title="This surface is intentionally staged." tone="info">
        Phase 14 establishes the navigation target and page anatomy now. Feature-complete workflows for
        this area are delivered in their dedicated later phase.
      </InlineNotice>

      <section className="page-main-grid">
        <Panel
          className="page-main-grid__primary"
          description={primaryDescription}
          eyebrow="primary work area"
          title={primaryTitle}
        >
          <ul className="content-list">
            {primaryItems.map((item) => (
              <li key={item}>{item}</li>
            ))}
          </ul>
        </Panel>

        <Panel
          className="page-main-grid__secondary"
          description={secondaryDescription}
          eyebrow="secondary information"
          title={secondaryTitle}
        >
          <ul className="content-list">
            {secondaryItems.map((item) => (
              <li key={item}>{item}</li>
            ))}
          </ul>
        </Panel>
      </section>

      <ActionBar summary="This placeholder route already follows the standard page anatomy for future feature work.">
        <Link className="button-link" to="/help">
          Review documentation map
        </Link>
      </ActionBar>
    </main>
  );
}
