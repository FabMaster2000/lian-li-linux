import { DeviceOverviewCard } from "../components/DeviceOverviewCard";
import { PageIntro } from "../components/PageIntro";
import { PlaceholderPanel } from "../components/PlaceholderPanel";
import { SectionCard } from "../components/SectionCard";
import { plannedSections } from "../features/sections";
import { useDashboardData } from "../hooks/useDashboardData";
import { useDocumentTitle } from "../hooks/useDocumentTitle";

function formatTimestamp(timestamp: string | null) {
  if (!timestamp) {
    return "not loaded yet";
  }

  return new Intl.DateTimeFormat(undefined, {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  }).format(new Date(timestamp));
}

export function DashboardPage() {
  useDocumentTitle("Dashboard - Lian Li Control Surface");
  const { devices, daemonStatus, runtime, loading, refreshing, error, lastUpdated, refresh } =
    useDashboardData();

  const rgbCapableCount = devices.filter((device) => device.capabilities.has_rgb).length;
  const fanCapableCount = devices.filter((device) => device.capabilities.has_fan).length;

  return (
    <main className="page-shell">
      <PageIntro
        eyebrow="dashboard"
        title="Fleet dashboard and control entry points"
        description="The dashboard is now wired to the backend. It loads the current device inventory, daemon reachability, and runtime paths, then exposes section entry points where you explicitly choose the device before drilling in."
        aside={
          <div className="dashboard-aside">
            <dl className="runtime-grid">
              <div>
                <dt>Daemon</dt>
                <dd>{daemonStatus?.reachable ? "reachable" : "offline"}</dd>
              </div>
              <div>
                <dt>Socket</dt>
                <dd>{runtime?.daemon.socket_path ?? "pending"}</dd>
              </div>
              <div>
                <dt>Updated</dt>
                <dd>
                  {loading ? "loading" : refreshing ? "live sync" : formatTimestamp(lastUpdated)}
                </dd>
              </div>
            </dl>

            <button className="refresh-button" onClick={() => void refresh()} type="button">
              {refreshing ? "Refreshing..." : "Refresh dashboard"}
            </button>
          </div>
        }
      />

      <section className="dashboard-overview">
        <article className="metric-card">
          <p className="metric-card__label">Devices</p>
          <strong>{loading ? "..." : devices.length}</strong>
          <span>Detected controllers and wireless targets</span>
        </article>
        <article className="metric-card">
          <p className="metric-card__label">RGB capable</p>
          <strong>{loading ? "..." : rgbCapableCount}</strong>
          <span>Ready for color, effects, and brightness changes</span>
        </article>
        <article className="metric-card">
          <p className="metric-card__label">Fan capable</p>
          <strong>{loading ? "..." : fanCapableCount}</strong>
          <span>Supports manual speed or future curve workflows</span>
        </article>
        <article className="metric-card">
          <p className="metric-card__label">Daemon status</p>
          <strong>{daemonStatus?.reachable ? "online" : "offline"}</strong>
          <span>{daemonStatus?.error ?? "IPC path responding"}</span>
        </article>
      </section>

      {error ? (
        <section className="error-banner" role="alert">
          <strong>Dashboard load failed.</strong>
          <span>{error}</span>
        </section>
      ) : null}

      <section className="panel-stack">
        <div className="panel-stack__header">
          <div>
            <p className="panel-stack__eyebrow">devices</p>
            <h2>Live device inventory</h2>
          </div>
          <p>
            Name, type, status, capabilities, and section entry points are surfaced here
            from the backend device API.
          </p>
        </div>

        {loading ? (
          <div className="device-grid">
            <article className="empty-state">
              <h3>Loading device inventory</h3>
              <p>The dashboard is requesting the current backend snapshot.</p>
            </article>
          </div>
        ) : devices.length > 0 ? (
          <div className="device-grid">
            {devices.map((device) => (
              <DeviceOverviewCard key={device.id} device={device} />
            ))}
          </div>
        ) : (
          <div className="device-grid">
            <article className="empty-state">
              <h3>No devices reported</h3>
              <p>
                The backend responded, but there are no simulated or physical
                devices in the current snapshot.
              </p>
            </article>
          </div>
        )}
      </section>

      <div className="page-grid">
        <PlaceholderPanel
          title="Backend runtime"
          description="These paths come from /api/runtime and help confirm which backend instance the frontend is talking to."
          items={[
            `Backend host: ${runtime?.backend.host ?? "pending"}`,
            `Backend port: ${runtime?.backend.port ?? 0}`,
            `Profile store: ${runtime?.backend.profile_store_path ?? "pending"}`,
          ]}
        />
        <PlaceholderPanel
          title="Daemon runtime"
          description="Useful for support and diagnostics before the richer settings page is built."
          items={[
            `Socket: ${runtime?.daemon.socket_path ?? "pending"}`,
            `Config: ${runtime?.daemon.config_path ?? "pending"}`,
            `XDG runtime: ${runtime?.daemon.xdg_runtime_dir ?? "pending"}`,
          ]}
        />
        <PlaceholderPanel
          title="Immediate next routes"
          description="The dashboard exposes the section entry points while keeping device selection explicit on the next screen."
          items={[
            "Open the device picker before loading detail",
            "Choose a device inside lighting before editing",
            "Choose a device inside fans before applying speed",
          ]}
        />
      </div>

      <section className="panel-stack">
        <div className="panel-stack__header">
          <div>
            <p className="panel-stack__eyebrow">navigation</p>
            <h2>Control areas</h2>
          </div>
          <p>
            The global route map remains available here for direct navigation.
          </p>
        </div>
      </section>

      <section className="section-grid">
        {plannedSections.map((section) => (
          <SectionCard key={section.id} section={section} />
        ))}
      </section>
    </main>
  );
}
