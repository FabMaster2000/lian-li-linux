import { Link, useSearchParams } from "react-router-dom";
import { InlineNotice } from "../components/feedback/InlineNotice";
import { PageHeader } from "../components/layout/PageHeader";
import { Panel } from "../components/ui/Panel";
import { StatTile } from "../components/ui/StatTile";
import { useSystemStatus } from "../app/SystemStatusProvider";
import { useBackendEvents } from "../app/BackendEventsProvider";
import { useDocumentTitle } from "../hooks/useDocumentTitle";

export function DiagnosticsPage() {
  useDocumentTitle("Diagnostics - Lian Li Control Surface");
  const { apiReachable, daemonStatus, refreshing } = useSystemStatus();
  const { connectionState } = useBackendEvents();
  const [searchParams] = useSearchParams();
  const selectedDeviceId = searchParams.get("device");

  return (
    <main className="page-shell">
      <PageHeader
        actions={
          <div className="page-header__button-group">
            <Link className="button-link button-link--primary" to="/">
              Back to dashboard
            </Link>
            <Link className="button-link" to="/settings">
              Runtime settings
            </Link>
          </div>
        }
        description="Surface runtime health, daemon reachability, event-stream status, and the currently focused device context in a dedicated diagnostics route before the richer operational feature phase expands it."
        eyebrow="diagnostics"
        title="Diagnostics surface"
      />

      <section className="summary-strip">
        <StatTile detail="Frontend-to-backend HTTP availability." label="API" tone={apiReachable ? "success" : "warning"} value={apiReachable ? "reachable" : "attention"} />
        <StatTile detail="Backend-to-daemon socket health." label="Daemon" tone={daemonStatus?.reachable === false ? "warning" : "success"} value={daemonStatus?.reachable === false ? "offline" : "reachable"} />
        <StatTile detail="Websocket live-event stream state." label="Events" tone={connectionState === "connected" ? "accent" : "warning"} value={connectionState} />
      </section>

      <InlineNotice title="Diagnostics is now a first-class route." tone="info">
        {selectedDeviceId
          ? `Focused device: ${selectedDeviceId}. Detailed per-device diagnostics land in a later phase, but the route context is already carried through from the inventory.`
          : "The detailed diagnostics, logs, recovery actions, and system information panels are part of a later phase, but the navigation and page anatomy are in place now."}
      </InlineNotice>

      <section className="page-main-grid">
        <Panel
          className="page-main-grid__primary"
          description="These are the currently available runtime signals already exposed by the MVP stack."
          eyebrow="available now"
          title="Current operational visibility"
        >
          <dl className="detail-list">
            <div>
              <dt>API status</dt>
              <dd>{apiReachable ? "reachable" : "unreachable"}</dd>
            </div>
            <div>
              <dt>Daemon status</dt>
              <dd>{daemonStatus?.reachable === false ? "offline" : "reachable"}</dd>
            </div>
            <div>
              <dt>Socket path</dt>
              <dd>{daemonStatus?.socket_path ?? "pending"}</dd>
            </div>
            <div>
              <dt>Refresh mode</dt>
              <dd>{refreshing ? "refreshing" : "idle"}</dd>
            </div>
            <div>
              <dt>Focused device</dt>
              <dd>{selectedDeviceId ?? "none"}</dd>
            </div>
          </dl>
        </Panel>

        <Panel
          className="page-main-grid__secondary"
          description="Later phases expand this route into a richer support and recovery surface."
          eyebrow="next expansions"
          title="Planned diagnostics areas"
        >
          <ul className="content-list">
            <li>Structured log access and export</li>
            <li>Recent warnings and recent errors</li>
            <li>WebSocket and config-storage diagnostics</li>
            <li>Recovery actions and support bundle export</li>
          </ul>
        </Panel>
      </section>
    </main>
  );
}
