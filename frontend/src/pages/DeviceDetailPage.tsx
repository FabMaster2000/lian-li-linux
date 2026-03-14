import { Link, useParams } from "react-router-dom";
import { PageIntro } from "../components/PageIntro";
import { useDeviceDetailData } from "../hooks/useDeviceDetailData";
import { useDocumentTitle } from "../hooks/useDocumentTitle";

function formatNumber(value: number | null | undefined, suffix = "") {
  return typeof value === "number" ? `${value}${suffix}` : "n/a";
}

function formatBoolean(value: boolean | null | undefined, label: string) {
  return value ? label : "no";
}

function rgbZoneLabel(count: number | null) {
  if (typeof count !== "number") {
    return "n/a";
  }

  return `${count} zone${count === 1 ? "" : "s"}`;
}

export function DeviceDetailPage() {
  const { deviceId: routeDeviceId = "unknown-device" } = useParams();
  const {
    deviceId,
    device,
    lightingState,
    fanState,
    loading,
    refreshing,
    error,
    lightingError,
    fanError,
    refresh,
  } = useDeviceDetailData(routeDeviceId);

  useDocumentTitle(
    device ? `Device Detail - ${device.name}` : `Device Detail - ${deviceId}`,
  );

  const title = device?.name ?? deviceId;
  const description = device
    ? `Family ${device.family}. This page shows the current device snapshot, capability profile, stored lighting state, and fan configuration reported by the backend.`
    : "The frontend is loading the device snapshot and the current lighting and fan state from the backend.";

  return (
    <main className="page-shell">
      <PageIntro
        eyebrow="device detail"
        title={title}
        description={description}
        aside={
          <div className="dashboard-aside">
            <dl className="runtime-grid">
              <div>
                <dt>Device id</dt>
                <dd>{deviceId}</dd>
              </div>
              <div>
                <dt>Status</dt>
                <dd>{device?.online ? "online" : loading ? "loading" : "offline"}</dd>
              </div>
              <div>
                <dt>Family</dt>
                <dd>{device?.family ?? "pending"}</dd>
              </div>
              <div>
                <dt>Live sync</dt>
                <dd>{refreshing ? "refreshing" : "idle"}</dd>
              </div>
            </dl>

            <div className="device-actions">
              <button className="refresh-button" onClick={() => void refresh()} type="button">
                {refreshing ? "Refreshing..." : "Refresh detail"}
              </button>
              <Link className="button-link" to="/">
                Back to dashboard
              </Link>
            </div>
          </div>
        }
      />

      {error ? (
        <section className="error-banner" role="alert">
          <strong>Device detail load failed.</strong>
          <span>{error}</span>
        </section>
      ) : null}

      {device && !device.online ? (
        <section className="warning-banner" role="status">
          <strong>Device offline.</strong>
          <span>The last reported snapshot may be stale until the controller comes back online.</span>
        </section>
      ) : null}

      {loading && !device ? (
        <section className="detail-section">
          <article className="empty-state">
            <h3>Loading device detail</h3>
            <p>The frontend is requesting the current device, lighting, and fan snapshot.</p>
          </article>
        </section>
      ) : null}

      {device ? (
        <>
          <section className="detail-overview">
            <article className="metric-card">
              <p className="metric-card__label">Online</p>
              <strong>{device.online ? "yes" : "no"}</strong>
              <span>Current reachability from the backend device inventory</span>
            </article>
            <article className="metric-card">
              <p className="metric-card__label">Fan slots</p>
              <strong>{formatNumber(device.capabilities.fan_count)}</strong>
              <span>{device.capabilities.has_fan ? "Cooling control available" : "No fan capability"}</span>
            </article>
            <article className="metric-card">
              <p className="metric-card__label">RGB zones</p>
              <strong>{formatNumber(device.capabilities.rgb_zone_count)}</strong>
              <span>{device.capabilities.has_rgb ? "Lighting control available" : "No RGB capability"}</span>
            </article>
            <article className="metric-card">
              <p className="metric-card__label">Telemetry</p>
              <strong>
                {device.state.fan_rpms && device.state.fan_rpms.length > 0 ? "live" : "partial"}
              </strong>
              <span>{device.state.streaming_active ? "Streaming active" : "No active media stream"}</span>
            </article>
          </section>

          <div className="page-grid">
            <section className="content-panel">
              <div className="content-panel__header">
                <h2>General information</h2>
                <p>Core identity and runtime state for this device.</p>
              </div>
              <dl className="detail-list">
                <div>
                  <dt>Name</dt>
                  <dd>{device.name}</dd>
                </div>
                <div>
                  <dt>Family</dt>
                  <dd>{device.family}</dd>
                </div>
                <div>
                  <dt>Device id</dt>
                  <dd>{device.id}</dd>
                </div>
                <div>
                  <dt>Online</dt>
                  <dd>{device.online ? "yes" : "no"}</dd>
                </div>
              </dl>
            </section>

            <section className="content-panel">
              <div className="content-panel__header">
                <h2>Capabilities</h2>
                <p>The device model the backend currently exposes.</p>
              </div>
              <div className="chip-row">
                {device.capabilities.has_rgb ? (
                  <span className="capability-chip">{rgbZoneLabel(device.capabilities.rgb_zone_count)} RGB</span>
                ) : null}
                {device.capabilities.has_fan ? (
                  <span className="capability-chip">
                    {formatNumber(device.capabilities.fan_count)} fan slots
                  </span>
                ) : null}
                {device.capabilities.has_lcd ? <span className="capability-chip">LCD</span> : null}
                {device.capabilities.has_pump ? <span className="capability-chip">Pump</span> : null}
                {device.capabilities.mb_sync_support ? (
                  <span className="capability-chip">MB sync</span>
                ) : null}
                {device.capabilities.per_fan_control ? (
                  <span className="capability-chip">Per-fan control</span>
                ) : null}
              </div>
              <dl className="detail-list">
                <div>
                  <dt>Per-fan control</dt>
                  <dd>{formatBoolean(device.capabilities.per_fan_control, "supported")}</dd>
                </div>
                <div>
                  <dt>Motherboard sync</dt>
                  <dd>{formatBoolean(device.capabilities.mb_sync_support, "supported")}</dd>
                </div>
                <div>
                  <dt>RGB capability</dt>
                  <dd>{device.capabilities.has_rgb ? "yes" : "no"}</dd>
                </div>
                <div>
                  <dt>Fan capability</dt>
                  <dd>{device.capabilities.has_fan ? "yes" : "no"}</dd>
                </div>
              </dl>
            </section>

            <section className="content-panel">
              <div className="content-panel__header">
                <h2>Current state</h2>
                <p>Live telemetry and stream-related state from the backend snapshot.</p>
              </div>
              <dl className="detail-list">
                <div>
                  <dt>Fan RPMs</dt>
                  <dd>
                    {device.state.fan_rpms && device.state.fan_rpms.length > 0
                      ? device.state.fan_rpms.join(" / ")
                      : "n/a"}
                  </dd>
                </div>
                <div>
                  <dt>Coolant temperature</dt>
                  <dd>{formatNumber(device.state.coolant_temp, " C")}</dd>
                </div>
                <div>
                  <dt>Streaming active</dt>
                  <dd>{device.state.streaming_active ? "yes" : "no"}</dd>
                </div>
              </dl>
            </section>
          </div>

          <section className="detail-section">
            <div className="panel-stack__header">
              <div>
                <p className="panel-stack__eyebrow">lighting</p>
                <h2>Current lighting state</h2>
              </div>
              <p>Stored desired-state lighting data from the backend lighting endpoint.</p>
            </div>

            {!device.capabilities.has_rgb ? (
              <article className="empty-state">
                <h3>No lighting capability</h3>
                <p>This device does not currently expose RGB control.</p>
              </article>
            ) : lightingError ? (
              <article className="error-banner" role="alert">
                <strong>Lighting data unavailable.</strong>
                <span>{lightingError}</span>
              </article>
            ) : lightingState ? (
              <div className="lighting-zone-grid">
                {lightingState.zones.map((zone) => (
                  <article key={zone.zone} className="lighting-zone-card">
                    <div className="lighting-zone-card__header">
                      <div>
                        <p className="device-card__eyebrow">Zone {zone.zone}</p>
                        <h3>{zone.effect}</h3>
                      </div>
                      <span className="capability-chip">
                        {zone.brightness_percent}% brightness
                      </span>
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
                    <dl className="detail-list detail-list--compact">
                      <div>
                        <dt>Speed</dt>
                        <dd>{zone.speed}</dd>
                      </div>
                      <div>
                        <dt>Direction</dt>
                        <dd>{zone.direction}</dd>
                      </div>
                      <div>
                        <dt>Scope</dt>
                        <dd>{zone.scope}</dd>
                      </div>
                    </dl>
                  </article>
                ))}
              </div>
            ) : (
              <article className="empty-state">
                <h3>No lighting state reported</h3>
                <p>The backend returned no lighting zones for this device.</p>
              </article>
            )}
          </section>

          <section className="detail-section">
            <div className="panel-stack__header">
              <div>
                <p className="panel-stack__eyebrow">fans</p>
                <h2>Current fan state</h2>
              </div>
              <p>Config-backed slot state plus telemetry RPMs, when the backend exposes them.</p>
            </div>

            {!device.capabilities.has_fan ? (
              <article className="empty-state">
                <h3>No fan capability</h3>
                <p>This device does not currently expose manual fan control.</p>
              </article>
            ) : fanError ? (
              <article className="error-banner" role="alert">
                <strong>Fan data unavailable.</strong>
                <span>{fanError}</span>
              </article>
            ) : fanState ? (
              <div className="fan-slot-grid">
                <article className="content-panel fan-summary-card">
                  <div className="content-panel__header">
                    <h2>Fan summary</h2>
                    <p>High-level values for the current fan configuration.</p>
                  </div>
                  <dl className="detail-list">
                    <div>
                      <dt>Update interval</dt>
                      <dd>{fanState.update_interval_ms} ms</dd>
                    </div>
                    <div>
                      <dt>RPM telemetry</dt>
                      <dd>
                        {fanState.rpms && fanState.rpms.length > 0
                          ? fanState.rpms.join(" / ")
                          : "n/a"}
                      </dd>
                    </div>
                  </dl>
                </article>

                {fanState.slots.map((slot) => (
                  <article key={slot.slot} className="fan-slot-card">
                    <div className="lighting-zone-card__header">
                      <div>
                        <p className="device-card__eyebrow">Slot {slot.slot}</p>
                        <h3>{slot.mode}</h3>
                      </div>
                      {slot.percent !== null ? (
                        <span className="capability-chip">{slot.percent}%</span>
                      ) : null}
                    </div>
                    <dl className="detail-list detail-list--compact">
                      <div>
                        <dt>PWM</dt>
                        <dd>{formatNumber(slot.pwm)}</dd>
                      </div>
                      <div>
                        <dt>Curve</dt>
                        <dd>{slot.curve ?? "n/a"}</dd>
                      </div>
                    </dl>
                  </article>
                ))}
              </div>
            ) : (
              <article className="empty-state">
                <h3>No fan state reported</h3>
                <p>The backend returned no slot state for this device.</p>
              </article>
            )}
          </section>
        </>
      ) : null}
    </main>
  );
}
