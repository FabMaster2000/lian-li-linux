import { useEffect } from "react";
import { useSearchParams } from "react-router-dom";
import { ActionBar } from "../components/feedback/ActionBar";
import { EmptyState } from "../components/feedback/EmptyState";
import { InlineNotice } from "../components/feedback/InlineNotice";
import { ColorField } from "../components/forms/ColorField";
import { SliderField } from "../components/forms/SliderField";
import { PageIntro } from "../components/PageIntro";
import { Panel } from "../components/ui/Panel";
import { StatusBadge } from "../components/ui/StatusBadge";
import { useDocumentTitle } from "../hooks/useDocumentTitle";
import { useMvpRgbPageData } from "../hooks/useMvpRgbPageData";
import type { RgbEffectChoice } from "../hooks/useMvpRgbPageData";

export function LightingPage() {
  useDocumentTitle("RGB - Lian Li Control Surface");
  const [searchParams, setSearchParams] = useSearchParams();
  const requestedClusterId = searchParams.get("cluster");
  const requestedDeviceId = searchParams.get("device");
  const {
    clusters,
    selectedClusterId,
    setSelectedClusterId,
    selectedCluster,
    lightingState,
    loading,
    refreshing,
    stateLoading,
    stateRefreshing,
    applying,
    error,
    success,
    effect,
    setEffect,
    color,
    setColor,
    speed,
    setSpeed,
    dirty,
    rgbSummary,
    refresh,
    applyChanges,
    resetDraft,
  } = useMvpRgbPageData(requestedClusterId, requestedDeviceId);

  useEffect(() => {
    setSearchParams(
      (current) => {
        const next = new URLSearchParams(current);
        if (selectedClusterId) {
          next.set("cluster", selectedClusterId);
        } else {
          next.delete("cluster");
        }
        next.delete("device");
        return next;
      },
      { replace: true },
    );
  }, [selectedClusterId, setSearchParams]);

  const applyDisabled =
    !selectedCluster ||
    selectedCluster.status === "offline" ||
    applying ||
    stateLoading;
  const visibleLightingState =
    selectedCluster && lightingState?.device_id === selectedCluster.primaryDeviceId
      ? lightingState
      : null;

  return (
    <main className="page-shell">
      <PageIntro
        eyebrow="rgb"
        title="RGB"
        description="Cluster auswählen, Effekt und Farbe setzen."
        aside={
          <button className="refresh-button" onClick={() => void refresh()} type="button">
            {refreshing || stateRefreshing ? "Aktualisiere..." : "Aktualisieren"}
          </button>
        }
      />

      {error ? (
        <InlineNotice tone="error" title="RGB-Seite konnte nicht geladen werden">
          {error}
        </InlineNotice>
      ) : null}

      {success ? (
        <InlineNotice tone="success" title="RGB-Einstellung gespeichert">
          {success}
        </InlineNotice>
      ) : null}

      {selectedCluster?.status === "offline" ? (
        <InlineNotice tone="warning" title="Gerät offline">
          Der ausgewählte Cluster ist aktuell offline.
        </InlineNotice>
      ) : null}

      {loading ? (
        <EmptyState
          title="RGB lädt"
          message="Die gekoppelten Cluster und RGB-Werte werden geladen."
        />
      ) : clusters.length === 0 ? (
        <EmptyState
          title="Keine gekoppelten Geräte"
          message="Es sind keine gekoppelten Wireless-Cluster für RGB verfügbar."
        />
      ) : (
        <>
          <section className="page-main-grid">
            <Panel
              className="page-main-grid__primary"
              eyebrow="cluster"
              title="Cluster auswählen"
              description="Nur aktuell gekoppelte Wireless-Cluster sind auswählbar."
            >
              <label className="field-group">
                <span className="field-group__label">Cluster</span>
                <select
                  className="field-input"
                  onChange={(event) => setSelectedClusterId(event.target.value)}
                  value={selectedClusterId}
                >
                  {clusters.map((cluster) => (
                    <option key={cluster.id} value={cluster.id}>
                      {cluster.id}
                    </option>
                  ))}
                </select>
              </label>
            </Panel>

            <Panel
              className="page-main-grid__secondary"
              eyebrow="live"
              title="Aktueller RGB-Status"
              description="Der aktuell gemeldete RGB-Zustand des ausgewählten Clusters."
            >
              {selectedCluster ? (
                <dl className="detail-list detail-list--compact">
                  <div>
                    <dt>Status</dt>
                    <dd>
                      <StatusBadge tone={selectedCluster.status === "healthy" ? "online" : "offline"}>
                        {selectedCluster.status}
                      </StatusBadge>
                    </dd>
                  </div>
                  <div>
                    <dt>RGB-Einstellung</dt>
                    <dd>{visibleLightingState ? rgbSummary : "n/a"}</dd>
                  </div>
                  <div>
                    <dt>Zonen</dt>
                    <dd>{visibleLightingState?.zones.length ?? 0}</dd>
                  </div>
                </dl>
              ) : (
                <EmptyState
                  title="Kein Cluster ausgewählt"
                  message="Bitte zuerst ein Cluster auswählen."
                />
              )}
            </Panel>
          </section>

          <Panel
            eyebrow="einstellung"
            title="RGB-Einstellung"
            description="Effekt und Farbe für den ausgewählten Cluster."
          >
            <label className="field-group">
              <span className="field-group__label">Effekt</span>
              <select
                className="field-input"
                disabled={!selectedCluster || applyDisabled}
                onChange={(event) => setEffect(event.target.value as RgbEffectChoice)}
                value={effect}
              >
                <option value="Static">Statisch</option>
                <option value="Meteor">Meteor</option>
              </select>
            </label>
            <ColorField
              disabled={!selectedCluster || applyDisabled}
              label="Farbe"
              onChange={setColor}
              value={color}
            />
            {effect === "Meteor" ? (
              <SliderField
                disabled={!selectedCluster || applyDisabled}
                label="Geschwindigkeit"
                min={0}
                max={20}
                onChange={setSpeed}
                suffix=""
                value={speed}
              />
            ) : null}
          </Panel>

          <ActionBar
            summary={
              dirty
                ? "Es gibt nicht gespeicherte RGB-Änderungen."
                : "Keine ausstehenden RGB-Änderungen."
            }
          >
            <button
              className="button-link button-link--primary"
              disabled={applyDisabled}
              onClick={() => void applyChanges(false)}
              type="button"
            >
              {applying ? "Speichere..." : "Übernehmen"}
            </button>
            {effect !== "Meteor" ? (
              <button
                className="button-link"
                disabled={applyDisabled || clusters.length === 0}
                onClick={() => void applyChanges(true)}
                type="button"
              >
                Auf alle Cluster übertragen
              </button>
            ) : null}
            <button
              className="button-link"
              disabled={!dirty || applying}
              onClick={resetDraft}
              type="button"
            >
              Zurücksetzen
            </button>
          </ActionBar>
        </>
      )}
    </main>
  );
}
