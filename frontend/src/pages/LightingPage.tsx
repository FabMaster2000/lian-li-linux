import { useEffect } from "react";
import { Link, useSearchParams } from "react-router-dom";
import { ColorField } from "../components/ColorField";
import { EffectSelect } from "../components/EffectSelect";
import { PageIntro } from "../components/PageIntro";
import { SliderField } from "../components/SliderField";
import { lightingEffectOptions } from "../features/lighting";
import { useLightingWorkbenchData } from "../hooks/useLightingWorkbenchData";
import { useDocumentTitle } from "../hooks/useDocumentTitle";

export function LightingPage() {
  useDocumentTitle("Lighting - Lian Li Control Surface");
  const [searchParams, setSearchParams] = useSearchParams();
  const requestedDeviceId = searchParams.get("device");
  const {
    devices,
    selectedDeviceId,
    setSelectedDeviceId,
    lightingState,
    activeZone,
    form,
    setForm,
    loading,
    stateLoading,
    stateRefreshing,
    submitting,
    error,
    success,
    refresh,
    applyChanges,
  } = useLightingWorkbenchData(requestedDeviceId);
  const selectedDevice = devices.find((device) => device.id === selectedDeviceId) ?? null;
  const selectedDeviceOffline = selectedDevice !== null && !selectedDevice.online;
  const zones = lightingState?.zones ?? [];
  const hasMultipleZones = zones.length > 1;

  useEffect(() => {
    if (!selectedDeviceId) {
      return;
    }

    if (requestedDeviceId === selectedDeviceId) {
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
        eyebrow="lighting"
        title="Lighting workbench"
        description="Adjust effect, color, and brightness for the selected RGB-capable device. Only readonly backend state refreshes automatically while editable fields stay stable."
        aside={
          <div className="dashboard-aside">
            <dl className="runtime-grid">
              <div>
                <dt>Device</dt>
                <dd>{selectedDevice ? selectedDevice.name : loading ? "loading" : "none"}</dd>
              </div>
              <div>
                <dt>Selected zone</dt>
                <dd>{activeZone ? `Zone ${activeZone.zone}` : "pending"}</dd>
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
                        : activeZone
                          ? "synced"
                          : "idle"}
                </dd>
              </div>
            </dl>

            <div className="device-actions">
              <button className="refresh-button" onClick={() => void refresh()} type="button">
                {stateRefreshing ? "Refreshing..." : "Reload lighting"}
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
          <strong>Lighting action failed.</strong>
          <span>{error}</span>
        </section>
      ) : null}

      {selectedDeviceOffline ? (
        <section className="warning-banner" role="status">
          <strong>Device offline.</strong>
          <span>Lighting changes are disabled until the selected device is online again.</span>
        </section>
      ) : null}

      {success ? (
        <section className="success-banner" role="status">
          <strong>Lighting updated.</strong>
          <span>{success}</span>
        </section>
      ) : null}

      <section className="lighting-workbench">
        <article className="lighting-form-card">
          <div className="content-panel__header">
            <h2>Controls</h2>
            <p>Choose a device, select a zone, set color and effect, then apply.</p>
          </div>

          <div className="form-grid">
            <label className="field-group">
              <span className="field-group__label">Device</span>
              <select
                className="field-input"
                disabled={loading || devices.length === 0}
                onChange={(event) => setSelectedDeviceId(event.target.value)}
                value={selectedDeviceId}
              >
                {devices.length === 0 ? <option value="">No RGB devices</option> : null}
                {devices.length > 0 ? <option value="">Choose a device</option> : null}
                {devices.map((device) => (
                  <option key={device.id} value={device.id}>
                    {device.name} ({device.family})
                  </option>
                ))}
              </select>
            </label>

            <label className="field-group">
              <span className="field-group__label">Zone</span>
              <select
                className="field-input"
                disabled={!lightingState || stateLoading}
                onChange={(event) => {
                  const nextZoneNumber = Number(event.target.value);
                  const nextZone =
                    lightingState?.zones.find((zone) => zone.zone === nextZoneNumber) ?? null;

                  setForm((current) => ({
                    ...current,
                    zone: nextZoneNumber,
                    effect: nextZone?.effect ?? current.effect,
                    color: nextZone?.colors[0] ?? current.color,
                    brightness: nextZone?.brightness_percent ?? current.brightness,
                  }));
                }}
                value={form.zone}
              >
                {(lightingState?.zones.length ? lightingState.zones : [{ zone: 0 }]).map((zone) => (
                  <option key={zone.zone} value={zone.zone}>
                    Zone {zone.zone}
                  </option>
                ))}
              </select>
            </label>

            <EffectSelect
              disabled={!selectedDeviceId || selectedDeviceOffline || stateLoading || submitting}
              onChange={(effect) =>
                setForm((current) => ({
                  ...current,
                  effect,
                }))
              }
              options={lightingEffectOptions}
              value={form.effect}
            />
          </div>

          <div className="lighting-form__row">
            <ColorField
              disabled={!selectedDeviceId || selectedDeviceOffline || stateLoading || submitting}
              label="Color picker"
              onChange={(color) =>
                setForm((current) => ({
                  ...current,
                  color,
                }))
              }
              pickerAriaLabel="Lighting color picker"
              value={form.color}
            />
          </div>

          <SliderField
            disabled={!selectedDeviceId || selectedDeviceOffline || stateLoading || submitting}
            label="Brightness"
            onChange={(brightness) =>
              setForm((current) => ({
                ...current,
                brightness,
              }))
            }
            value={form.brightness}
          />

          <div className="device-actions">
            <button
              className="refresh-button"
              disabled={!selectedDeviceId || selectedDeviceOffline || stateLoading || submitting}
              onClick={() => void applyChanges()}
              type="button"
            >
              {submitting ? "Applying..." : "Apply lighting"}
            </button>
          </div>
        </article>

        <article className="lighting-preview-card">
          <div className="content-panel__header">
            <h2>Live zone state</h2>
            <p>Readonly backend values for the currently selected zone.</p>
          </div>

          {stateLoading && !activeZone ? (
            <div className="empty-state">
              <h3>Loading lighting state</h3>
              <p>The selected device is being refreshed from the backend.</p>
            </div>
          ) : activeZone ? (
            <>
              {stateRefreshing ? (
                <p className="panel-stack__eyebrow">Readonly fields are refreshing in the background.</p>
              ) : null}
              <div
                className="lighting-preview-swatch"
                style={{
                  background:
                    activeZone.colors.length > 1
                      ? `linear-gradient(135deg, ${activeZone.colors.join(", ")})`
                      : activeZone.colors[0] ?? form.color,
                }}
              />
              {activeZone.colors.length > 1 ? (
                <div className="chip-row">
                  {activeZone.colors.map((color) => (
                    <span key={color} className="color-chip">
                      <span
                        aria-hidden="true"
                        className="color-chip__swatch"
                        style={{ backgroundColor: color }}
                      />
                      {color}
                    </span>
                  ))}
                </div>
              ) : null}
              <dl className="detail-list">
                <div>
                  <dt>Speed</dt>
                  <dd>{activeZone.speed}</dd>
                </div>
                <div>
                  <dt>Direction</dt>
                  <dd>{activeZone.direction}</dd>
                </div>
                <div>
                  <dt>Scope</dt>
                  <dd>{activeZone.scope}</dd>
                </div>
              </dl>
            </>
          ) : (
            <div className="empty-state">
              <h3>No lighting state</h3>
              <p>Select an RGB-capable device to start editing.</p>
            </div>
          )}
        </article>
      </section>

      {zones.length === 0 ? (
        <section className="lighting-zones-panel">
          <div className="empty-state">
            <h3>No zones returned</h3>
            <p>The backend has not reported stored lighting zones for the selected device yet.</p>
          </div>
        </section>
      ) : null}

      {hasMultipleZones ? (
        <section className="lighting-zones-panel">
          <div className="panel-stack__header">
            <div>
              <p className="panel-stack__eyebrow">zones</p>
              <h2>Other zones</h2>
            </div>
            <p>Switch between backend-reported zones without losing your current draft.</p>
          </div>

          <div className="lighting-zone-grid">
            {zones.map((zone) => (
              <article
                key={zone.zone}
                className={
                  zone.zone === form.zone
                    ? "lighting-zone-card lighting-zone-card--active"
                    : "lighting-zone-card"
                }
              >
                <div className="lighting-zone-card__header">
                  <div>
                    <p className="device-card__eyebrow">Zone {zone.zone}</p>
                    <h3>{zone.effect}</h3>
                  </div>
                </div>
                <div className="chip-row">
                  {zone.colors.map((color) => (
                    <span key={color} className="color-chip">
                      <span
                        aria-hidden="true"
                        className="color-chip__swatch"
                        style={{ backgroundColor: color }}
                      />
                      {color}
                    </span>
                  ))}
                </div>
                <button
                  className="button-link"
                  onClick={() =>
                    setForm({
                      zone: zone.zone,
                      effect: zone.effect,
                      color: zone.colors[0] ?? "#ffffff",
                      brightness: zone.brightness_percent,
                    })
                  }
                  type="button"
                >
                  Use this zone
                </button>
              </article>
            ))}
          </div>
        </section>
      ) : null}
    </main>
  );
}
