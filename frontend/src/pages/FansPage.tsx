import { useEffect } from "react";
import { Link, useSearchParams } from "react-router-dom";
import { PageIntro } from "../components/PageIntro";
import { SliderField } from "../components/SliderField";
import { useDocumentTitle } from "../hooks/useDocumentTitle";
import { useFansWorkbenchData } from "../hooks/useFansWorkbenchData";
import type { FanStateResponse } from "../types/api";

function summarizeMode(fanState: FanStateResponse | null) {
  if (!fanState || fanState.slots.length === 0) {
    return "n/a";
  }

  const modes = [...new Set(fanState.slots.map((slot) => slot.mode))];
  return modes.length === 1 ? modes[0] : "mixed";
}

function summarizePercent(fanState: FanStateResponse | null) {
  if (!fanState || fanState.slots.length === 0) {
    return "n/a";
  }

  const percents = fanState.slots
    .map((slot) => slot.percent)
    .filter((percent): percent is number => typeof percent === "number");

  if (percents.length === 0) {
    return "n/a";
  }

  return percents.every((percent) => percent === percents[0]) ? `${percents[0]}%` : "mixed";
}

function summarizeRpms(rpms: number[] | null) {
  return rpms && rpms.length > 0 ? rpms.join(" / ") : "n/a";
}

function slotRpm(fanState: FanStateResponse | null, slot: number) {
  return fanState?.rpms?.[slot - 1] ?? null;
}

export function FansPage() {
  useDocumentTitle("Fans - Lian Li Control Surface");
  const [searchParams, setSearchParams] = useSearchParams();
  const requestedDeviceId = searchParams.get("device");
  const {
    devices,
    selectedDeviceId,
    setSelectedDeviceId,
    fanState,
    formPercent,
    setFormPercent,
    loading,
    stateLoading,
    stateRefreshing,
    submitting,
    error,
    success,
    refresh,
    applyChanges,
  } = useFansWorkbenchData(requestedDeviceId);
  const selectedDevice = devices.find((device) => device.id === selectedDeviceId) ?? null;
  const selectedDeviceOffline = selectedDevice !== null && !selectedDevice.online;

  useEffect(() => {
    if (!selectedDeviceId || requestedDeviceId === selectedDeviceId) {
      return;
    }

    setSearchParams((current) => {
      const next = new URLSearchParams(current);
      next.set("device", selectedDeviceId);
      return next;
    }, { replace: true });
  }, [requestedDeviceId, selectedDeviceId, setSearchParams]);

  return (
    <main className="page-shell">
      <PageIntro
        eyebrow="fans"
        title="Cooling console"
        description="Set a fixed cooling percentage for the selected fan-capable device. Readonly fan state and telemetry can refresh without overwriting your draft value."
        aside={
          <div className="dashboard-aside">
            <dl className="runtime-grid">
              <div>
                <dt>Device</dt>
                <dd>{selectedDevice ? selectedDevice.name : loading ? "loading" : "none"}</dd>
              </div>
              <div>
                <dt>Current value</dt>
                <dd>{summarizePercent(fanState)}</dd>
              </div>
              <div>
                <dt>Live state</dt>
                <dd>
                  {submitting
                    ? "applying"
                    : stateLoading
                      ? "loading"
                      : stateRefreshing
                        ? "refreshing"
                        : fanState
                          ? "synced"
                          : "idle"}
                </dd>
              </div>
            </dl>

            <div className="device-actions">
              <button className="refresh-button" onClick={() => void refresh()} type="button">
                {stateRefreshing ? "Refreshing..." : "Reload fans"}
              </button>
              {selectedDeviceId ? (
                <Link className="button-link" to={`/devices/${encodeURIComponent(selectedDeviceId)}`}>
                  Device detail
                </Link>
              ) : null}
            </div>
          </div>
        }
      />

      {error ? (
        <section className="error-banner" role="alert">
          <strong>Fan action failed.</strong>
          <span>{error}</span>
        </section>
      ) : null}

      {selectedDeviceOffline ? (
        <section className="warning-banner" role="status">
          <strong>Device offline.</strong>
          <span>Fan changes are disabled until the selected device is online again.</span>
        </section>
      ) : null}

      {success ? (
        <section className="success-banner" role="status">
          <strong>Fan speed updated.</strong>
          <span>{success}</span>
        </section>
      ) : null}

      <section className="fans-workbench">
        <article className="fan-control-card">
          <div className="content-panel__header">
            <h2>Manual control</h2>
            <p>Applies the same fixed percentage to all reported fan slots of the selected device.</p>
          </div>

          <div className="form-grid form-grid--fan">
            <label className="field-group">
              <span className="field-group__label">Device</span>
              <select
                className="field-input"
                disabled={loading || devices.length === 0}
                onChange={(event) => setSelectedDeviceId(event.target.value)}
                value={selectedDeviceId}
              >
                {devices.length === 0 ? <option value="">No fan devices</option> : null}
                {devices.length > 0 ? <option value="">Choose a device</option> : null}
                {devices.map((device) => (
                  <option key={device.id} value={device.id}>
                    {device.name} ({device.family})
                  </option>
                ))}
              </select>
            </label>

            <div className="fan-inline-stat">
              <span className="field-group__label">Current mode</span>
              <strong>{summarizeMode(fanState)}</strong>
            </div>

            <div className="fan-inline-stat">
              <span className="field-group__label">RPM telemetry</span>
              <strong>{summarizeRpms(fanState?.rpms ?? null)}</strong>
            </div>
          </div>

          <SliderField
            disabled={!selectedDeviceId || selectedDeviceOffline || stateLoading || submitting}
            label="Manual percent"
            onChange={setFormPercent}
            value={formPercent}
          />

          <p className="fan-control-note">
            The slider is your editable draft. Reloading updates readonly state but does not reset an
            unfinished change.
          </p>

          <div className="device-actions">
            <button
              className="refresh-button"
              disabled={!selectedDeviceId || selectedDeviceOffline || stateLoading || submitting}
              onClick={() => void applyChanges()}
              type="button"
            >
              {submitting ? "Applying..." : "Apply fan speed"}
            </button>
          </div>
        </article>

        <article className="fan-preview-card">
          <div className="content-panel__header">
            <h2>Live fan state</h2>
            <p>Readonly backend slot state and telemetry for the current device.</p>
          </div>

          {stateLoading && !fanState ? (
            <div className="empty-state">
              <h3>Loading fan state</h3>
              <p>The selected device is being refreshed from the backend.</p>
            </div>
          ) : fanState ? (
            <>
              {stateRefreshing ? (
                <p className="panel-stack__eyebrow">Readonly fields are refreshing in the background.</p>
              ) : null}
              <dl className="detail-list">
                <div>
                  <dt>Mode</dt>
                  <dd>{summarizeMode(fanState)}</dd>
                </div>
                <div>
                  <dt>Configured value</dt>
                  <dd>{summarizePercent(fanState)}</dd>
                </div>
                <div>
                  <dt>Update interval</dt>
                  <dd>{fanState.update_interval_ms} ms</dd>
                </div>
                <div>
                  <dt>RPM telemetry</dt>
                  <dd>{summarizeRpms(fanState.rpms)}</dd>
                </div>
              </dl>
            </>
          ) : (
            <div className="empty-state">
              <h3>No fan state</h3>
              <p>Select a fan-capable device to start applying a manual speed.</p>
            </div>
          )}
        </article>
      </section>

      {fanState?.slots.length ? (
        <section className="detail-section">
          <div className="panel-stack__header">
            <div>
              <p className="panel-stack__eyebrow">slots</p>
              <h2>Reported fan slots</h2>
            </div>
            <p>Per-slot readonly values reported by the backend after the last successful refresh.</p>
          </div>

          <div className="fan-slot-grid">
            {fanState.slots.map((slot) => (
              <article key={slot.slot} className="fan-slot-card">
                <div className="lighting-zone-card__header">
                  <div>
                    <p className="device-card__eyebrow">Slot {slot.slot}</p>
                    <h3>{slot.mode}</h3>
                  </div>
                  {slot.percent !== null ? <span className="capability-chip">{slot.percent}%</span> : null}
                </div>
                <dl className="detail-list detail-list--compact">
                  <div>
                    <dt>PWM</dt>
                    <dd>{slot.pwm ?? "n/a"}</dd>
                  </div>
                  <div>
                    <dt>RPM</dt>
                    <dd>{slotRpm(fanState, slot.slot) ?? "n/a"}</dd>
                  </div>
                  <div>
                    <dt>Curve</dt>
                    <dd>{slot.curve ?? "n/a"}</dd>
                  </div>
                </dl>
              </article>
            ))}
          </div>
        </section>
      ) : null}
    </main>
  );
}
