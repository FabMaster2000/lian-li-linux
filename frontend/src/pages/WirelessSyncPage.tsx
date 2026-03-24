import { useState } from "react";
import { EmptyState } from "../components/feedback/EmptyState";
import { InlineNotice } from "../components/feedback/InlineNotice";
import { PageIntro } from "../components/PageIntro";
import { Tabs } from "../components/ui/Tabs";
import { Card } from "../components/ui/Card";
import { useDocumentTitle } from "../hooks/useDocumentTitle";
import { useMvpWirelessSyncData } from "../hooks/useMvpWirelessSyncData";

const tabItems = [
  { id: "pair", label: "Gerät koppeln" },
  { id: "paired", label: "Gekoppelte Geräte" },
] as const;

export function WirelessSyncPage() {
  useDocumentTitle("Wireless Sync - Lian Li Control Surface");
  const [activeTab, setActiveTab] = useState<(typeof tabItems)[number]["id"]>("pair");
  const {
    pairedClusters,
    availableClusters,
    daemonStatus,
    loading,
    refreshing,
    error,
    actionError,
    actionSuccess,
    searching,
    connectingClusterId,
    disconnectingClusterId,
    refresh,
    searchForDevices,
    connectCluster,
    disconnectCluster,
  } = useMvpWirelessSyncData();

  const visibleClusters = activeTab === "pair" ? availableClusters : pairedClusters;

  return (
    <main className="page-shell">
      <PageIntro
        eyebrow="wireless sync"
        title="Wireless-Sync"
        description="Geräte koppeln oder bereits gekoppelte Cluster wieder entkoppeln."
        aside={
          <div className="device-actions">
            <button className="refresh-button" onClick={() => void refresh()} type="button">
              {refreshing ? "Aktualisiere..." : "Aktualisieren"}
            </button>
            <button className="button-link" onClick={() => void searchForDevices()} type="button">
              {searching ? "Suche..." : "Geräte suchen"}
            </button>
          </div>
        }
      />

      {daemonStatus?.reachable === false ? (
        <InlineNotice tone="warning" title="Daemon offline">
          {daemonStatus.error ?? "Wireless Sync ist aktuell nicht erreichbar."}
        </InlineNotice>
      ) : null}

      {error ? (
        <InlineNotice tone="error" title="Wireless-Sync konnte nicht geladen werden">
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

      <Tabs
        label="Wireless Sync Tabs"
        items={[...tabItems]}
        onChange={(value) => setActiveTab(value as "pair" | "paired")}
        value={activeTab}
      />

      {loading ? (
        <EmptyState
          title="Wireless-Sync lädt"
          message="Koppelbare und gekoppelte Cluster werden geladen."
        />
      ) : visibleClusters.length === 0 ? (
        <EmptyState
          title={
            activeTab === "pair"
              ? "Keine koppelbaren Geräte gefunden"
              : "Keine gekoppelten Geräte"
          }
          message={
            activeTab === "pair"
              ? "Aktuell wurden keine koppelbaren Wireless-Cluster gefunden."
              : "Aktuell sind keine Wireless-Cluster gekoppelt."
          }
        />
      ) : (
        <section className="device-grid">
          {visibleClusters.map((cluster) => (
            <Card className="device-card" key={cluster.id}>
              <div className="device-card__header">
                <div>
                  <p className="device-card__eyebrow">ID</p>
                  <h3>{cluster.id}</h3>
                </div>
              </div>

              <dl className="detail-list detail-list--compact">
                <div>
                  <dt>Anzahl Lüfter</dt>
                  <dd>{cluster.fanCount ?? "n/a"}</dd>
                </div>
                <div>
                  <dt>Lüfter-Typ</dt>
                  <dd>{cluster.fanType}</dd>
                </div>
              </dl>

              <div className="device-card__actions">
                {activeTab === "pair" ? (
                  <button
                    className="button-link button-link--primary"
                    disabled={
                      daemonStatus?.reachable === false ||
                      connectingClusterId === cluster.id
                    }
                    onClick={() => void connectCluster(cluster.id)}
                    type="button"
                  >
                    {connectingClusterId === cluster.id ? "Koppele..." : "Gerät koppeln"}
                  </button>
                ) : (
                  <button
                    className="button-link"
                    disabled={
                      daemonStatus?.reachable === false ||
                      disconnectingClusterId === cluster.id
                    }
                    onClick={() => void disconnectCluster(cluster.id)}
                    type="button"
                  >
                    {disconnectingClusterId === cluster.id ? "Entkoppele..." : "Gerät entkoppeln"}
                  </button>
                )}
              </div>
            </Card>
          ))}
        </section>
      )}
    </main>
  );
}
