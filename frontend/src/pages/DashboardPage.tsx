import { Link } from "react-router-dom";
import { EmptyState } from "../components/feedback/EmptyState";
import { InlineNotice } from "../components/feedback/InlineNotice";
import { PageIntro } from "../components/PageIntro";
import { Card } from "../components/ui/Card";
import { StatusBadge } from "../components/ui/StatusBadge";
import { useDocumentTitle } from "../hooks/useDocumentTitle";
import { useMvpDashboardData } from "../hooks/useMvpDashboardData";
import {
  summarizeFanRpm,
  summarizeLightingState,
} from "../features/mvpClusters";

export function DashboardPage() {
  useDocumentTitle("Dashboard - Lian Li Control Surface");
  const {
    clusters,
    fanStates,
    lightingStates,
    loading,
    refreshing,
    error,
    actionError,
    actionSuccess,
    disconnectingClusterId,
    refresh,
    disconnectCluster,
  } = useMvpDashboardData();

  return (
    <main className="page-shell">
      <PageIntro
        eyebrow="dashboard"
        title="Dashboard"
        description="Nur aktuell gekoppelte Wireless-Cluster werden hier angezeigt."
        aside={
          <button className="refresh-button" onClick={() => void refresh()} type="button">
            {refreshing ? "Aktualisiere..." : "Aktualisieren"}
          </button>
        }
      />

      {error ? (
        <InlineNotice tone="error" title="Dashboard konnte nicht geladen werden">
          {error}
        </InlineNotice>
      ) : null}

      {actionError ? (
        <InlineNotice tone="warning" title="Aktion fehlgeschlagen">
          {actionError}
        </InlineNotice>
      ) : null}

      {actionSuccess ? (
        <InlineNotice tone="success" title="Aktion erfolgreich">
          {actionSuccess}
        </InlineNotice>
      ) : null}

      {loading ? (
        <EmptyState
          title="Dashboard lädt"
          message="Die aktuell gekoppelten Cluster werden geladen."
        />
      ) : clusters.length === 0 ? (
        <EmptyState
          title="Keine gekoppelten Geräte"
          message="Derzeit sind keine Wireless-Cluster gekoppelt."
        />
      ) : (
        <section className="device-grid">
          {clusters.map((cluster) => (
            <Card className="device-card" key={cluster.id}>
              <div className="device-card__header">
                <div>
                  <p className="device-card__eyebrow">ID</p>
                  <h3>{cluster.id}</h3>
                </div>
                <StatusBadge tone={cluster.status === "healthy" ? "online" : "offline"}>
                  {cluster.status}
                </StatusBadge>
              </div>

              <dl className="detail-list detail-list--compact">
                <div>
                  <dt>Anzahl Lüfter</dt>
                  <dd>{cluster.fanCount ?? "n/a"}</dd>
                </div>
                <div>
                  <dt>Status</dt>
                  <dd>{cluster.status}</dd>
                </div>
                <div>
                  <dt>Aktuelle Drehzahl</dt>
                  <dd>
                    {summarizeFanRpm(
                      fanStates[cluster.id]?.rpms ?? cluster.primaryDevice.state.fan_rpms,
                    )}
                  </dd>
                </div>
                <div>
                  <dt>Aktuelle RGB-Einstellung</dt>
                  <dd>{summarizeLightingState(lightingStates[cluster.id])}</dd>
                </div>
              </dl>

              <div className="device-card__actions">
                <Link className="button-link" to={`/fans?cluster=${encodeURIComponent(cluster.id)}`}>
                  Fans
                </Link>
                <Link className="button-link" to={`/rgb?cluster=${encodeURIComponent(cluster.id)}`}>
                  RGB
                </Link>
                <button
                  className="button-link"
                  disabled={disconnectingClusterId === cluster.id}
                  onClick={() => void disconnectCluster(cluster.id)}
                  type="button"
                >
                  {disconnectingClusterId === cluster.id ? "Entkoppele..." : "Entkoppeln"}
                </button>
              </div>
            </Card>
          ))}
        </section>
      )}
    </main>
  );
}
