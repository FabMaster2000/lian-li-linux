import { useEffect } from "react";
import { useSearchParams } from "react-router-dom";
import { ActionBar } from "../components/feedback/ActionBar";
import { EmptyState } from "../components/feedback/EmptyState";
import { InlineNotice } from "../components/feedback/InlineNotice";
import { SliderField } from "../components/forms/SliderField";
import { PageIntro } from "../components/PageIntro";
import { Panel } from "../components/ui/Panel";
import { StatusBadge } from "../components/ui/StatusBadge";
import { useDocumentTitle } from "../hooks/useDocumentTitle";
import { useMvpFansPageData } from "../hooks/useMvpFansPageData";
import { summarizeFanRpm } from "../features/mvpClusters";

export function FansPage() {
  useDocumentTitle("Fans - Lian Li Control Surface");
  const [searchParams, setSearchParams] = useSearchParams();
  const requestedClusterId = searchParams.get("cluster");
  const requestedDeviceId = searchParams.get("device");
  const {
    clusters,
    selectedClusterId,
    setSelectedClusterId,
    selectedCluster,
    fanState,
    loading,
    refreshing,
    stateLoading,
    stateRefreshing,
    applying,
    error,
    success,
    dirty,
    draft,
    refresh,
    applyChanges,
    resetDraft,
    setMode,
    setManualPercent,
    setCurveSource,
    updateCurvePoint,
    addCurvePoint,
    removeCurvePoint,
  } = useMvpFansPageData(requestedClusterId, requestedDeviceId);

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
  const visibleRpms =
    selectedCluster && fanState?.device_id === selectedCluster.primaryDeviceId
      ? fanState.rpms
      : selectedCluster?.primaryDevice.state.fan_rpms ?? null;

  return (
    <main className="page-shell">
      <PageIntro
        eyebrow="fans"
        title="Fans"
        description="Cluster auswählen und entweder einen festen Prozentwert oder eine temperaturbasierte Lüfterkurve setzen."
        aside={
          <button className="refresh-button" onClick={() => void refresh()} type="button">
            {refreshing || stateRefreshing ? "Aktualisiere..." : "Aktualisieren"}
          </button>
        }
      />

      {error ? (
        <InlineNotice tone="error" title="Lüfterseite konnte nicht geladen werden">
          {error}
        </InlineNotice>
      ) : null}

      {success ? (
        <InlineNotice tone="success" title="Lüftereinstellungen gespeichert">
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
          title="Fans lädt"
          message="Die gekoppelten Cluster und Lüfterwerte werden geladen."
        />
      ) : clusters.length === 0 ? (
        <EmptyState
          title="Keine gekoppelten Geräte"
          message="Es sind keine gekoppelten Wireless-Cluster für Fans verfügbar."
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
              title="Live-Status"
              description="Aktueller Zustand des ausgewählten Clusters."
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
                    <dt>Aktuelle Drehzahl</dt>
                    <dd>{summarizeFanRpm(visibleRpms)}</dd>
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
            eyebrow="steuerung"
            title="Lüftersteuerung"
            description="Fester Prozentwert oder kompakte Temperaturkurve."
          >
            <div className="form-grid">
              <label className="field-group">
                <span className="field-group__label">Modus</span>
                <select
                  className="field-input"
                  disabled={!selectedCluster || applying}
                  onChange={(event) =>
                    setMode(event.target.value as "manual" | "curve")
                  }
                  value={draft.mode}
                >
                  <option value="manual">Fester Prozentwert</option>
                  <option value="curve">Lüfterkurve</option>
                </select>
              </label>
            </div>

            {draft.mode === "manual" ? (
              <SliderField
                disabled={!selectedCluster || applyDisabled}
                label="Fester Prozentwert"
                onChange={setManualPercent}
                value={draft.manualPercent}
              />
            ) : (
              <>
                <div className="form-grid">
                  <label className="field-group">
                    <span className="field-group__label">Temperaturquelle</span>
                    <select
                      className="field-input"
                      disabled={!selectedCluster || applyDisabled}
                      onChange={(event) =>
                        setCurveSource(event.target.value as "cpu" | "gpu")
                      }
                      value={draft.curveSource}
                    >
                      <option value="cpu">CPU</option>
                      <option value="gpu">GPU</option>
                    </select>
                  </label>
                </div>

                <div className="fan-curve-points">
                  <div className="fan-curve-points__header">
                    <strong>Kurvenpunkte</strong>
                    <button className="button-link" onClick={addCurvePoint} type="button">
                      Punkt hinzufügen
                    </button>
                  </div>

                  {draft.points.map((point, index) => (
                    <div className="fan-curve-point-row" key={`${index}-${point.temperature_celsius}`}>
                      <label className="field-group">
                        <span className="field-group__label">Temperatur</span>
                        <input
                          aria-label={`Curve point ${index + 1} temperature`}
                          className="field-input"
                          disabled={!selectedCluster || applyDisabled}
                          onChange={(event) =>
                            updateCurvePoint(
                              index,
                              "temperature_celsius",
                              Number(event.target.value),
                            )
                          }
                          step="1"
                          type="number"
                          value={point.temperature_celsius}
                        />
                      </label>
                      <label className="field-group">
                        <span className="field-group__label">Prozent</span>
                        <input
                          aria-label={`Curve point ${index + 1} percent`}
                          className="field-input"
                          disabled={!selectedCluster || applyDisabled}
                          onChange={(event) =>
                            updateCurvePoint(index, "percent", Number(event.target.value))
                          }
                          step="1"
                          type="number"
                          value={point.percent}
                        />
                      </label>
                      <button
                        className="button-link"
                        disabled={!selectedCluster || draft.points.length <= 2 || applyDisabled}
                        onClick={() => removeCurvePoint(index)}
                        type="button"
                      >
                        Entfernen
                      </button>
                    </div>
                  ))}
                </div>
              </>
            )}
          </Panel>

          <ActionBar
            summary={
              dirty
                ? "Es gibt nicht gespeicherte Lüfteränderungen."
                : "Keine ausstehenden Lüfteränderungen."
            }
          >
            <button
              className="button-link button-link--primary"
              disabled={applyDisabled}
              onClick={() => void applyChanges()}
              type="button"
            >
              {applying ? "Speichere..." : "Übernehmen"}
            </button>
            <button
              className="button-link"
              disabled={applyDisabled || clusters.length === 0}
              onClick={() => void applyChanges(true)}
              type="button"
            >
              Auf alle Cluster übertragen
            </button>
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
