import { Link, useParams } from "react-router-dom";
import { ActionBar } from "../components/feedback/ActionBar";
import { EmptyState } from "../components/feedback/EmptyState";
import { PageIntro } from "../components/PageIntro";
import { Panel } from "../components/ui/Panel";
import { StatTile } from "../components/ui/StatTile";
import { profilesForDevice } from "../features/deviceInventory";
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

function summarizeLighting(deviceHasRgb: boolean, zoneCount: number, firstZoneLabel: string | null) {
  if (!deviceHasRgb) {
    return "No RGB capability";
  }

  if (zoneCount === 0) {
    return "No stored lighting state";
  }

  return `${zoneCount} configured zone(s), starting with ${firstZoneLabel ?? "zone 0"}`;
}

function summarizeFan(deviceHasFan: boolean, slotCount: number, modeSummary: string) {
  if (!deviceHasFan) {
    return "No fan capability";
  }

  if (slotCount === 0) {
    return "No stored fan slots";
  }

  return `${slotCount} slot(s), mode ${modeSummary}`;
}

export function DeviceDetailPage() {
  const { deviceId: routeDeviceId = "unknown-device" } = useParams();
  const {
    deviceId,
    device,
    lightingState,
    fanState,
    profiles,
    loading,
    refreshing,
    error,
    lightingError,
    fanError,
    profileError,
    refresh,
  } = useDeviceDetailData(routeDeviceId);

  useDocumentTitle(
    device ? `Device Detail - ${device.display_name}` : `Device Detail - ${deviceId}`,
  );

  const title = device?.display_name ?? deviceId;
  const description = device
    ? `Family ${device.family}. This workspace combines identity, controller context, capability coverage, and the current lighting and fan state for the selected device.`
    : "The frontend is loading the device workspace and all currently available backend-backed summaries.";
  const assignedProfiles = device ? profilesForDevice(profiles, device.id) : [];
  const firstLightingZone = lightingState?.zones[0] ?? null;
  const fanModes = fanState ? [...new Set(fanState.slots.map((slot) => slot.mode))] : [];
  const fanModeSummary = fanModes.length === 0 ? "n/a" : fanModes.length === 1 ? fanModes[0]! : "mixed";

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
                <dt>Controller</dt>
                <dd>{device?.controller.label ?? "pending"}</dd>
              </div>
              <div>
                <dt>Workspace</dt>
                <dd>{refreshing ? "refreshing" : loading ? "loading" : "ready"}</dd>
              </div>
            </dl>

            <div className="device-actions">
              <button className="refresh-button" onClick={() => void refresh()} type="button">
                {refreshing ? "Refreshing..." : "Refresh detail"}
              </button>
              <Link className="button-link" to="/devices">
                Back to devices
              </Link>
            </div>
          </div>
        }
      />

      <section className="summary-strip">
        <StatTile
          detail="Backend-reported online state plus inventory health summary."
          label="Health"
          tone={device?.health.level === "healthy" ? "success" : "warning"}
          value={device ? device.health.level : loading ? "loading" : "unknown"}
        />
        <StatTile
          detail="The current controller or topology parent for this device."
          label="Controller"
          value={device?.controller.label ?? "pending"}
        />
        <StatTile
          detail="Stored lighting summary for the selected workspace."
          label="Lighting"
          value={summarizeLighting(!!device?.capabilities.has_rgb, lightingState?.zones.length ?? 0, firstLightingZone?.effect ?? null)}
        />
        <StatTile
          detail="Stored fan summary for the selected workspace."
          label="Fans"
          value={summarizeFan(!!device?.capabilities.has_fan, fanState?.slots.length ?? 0, fanModeSummary)}
        />
      </section>

      {error ? (
        <section className="error-banner" role="alert">
          <strong>Device detail load failed.</strong>
          <span>{error}</span>
        </section>
      ) : null}

      {profileError ? (
        <section className="warning-banner" role="status">
          <strong>Profiles unavailable.</strong>
          <span>{profileError}</span>
        </section>
      ) : null}

      {loading && !device ? (
        <EmptyState
          message="The frontend is requesting the current device, lighting, fan, and profile snapshot."
          title="Loading device workspace"
        />
      ) : null}

      {device ? (
        <>
          <section className="page-main-grid">
            <Panel
              className="page-main-grid__primary"
              description="Stable identity plus persisted presentation metadata from the backend inventory model."
              eyebrow="identity"
              title="Identity"
            >
              <dl className="detail-list">
                <div>
                  <dt>Display name</dt>
                  <dd>{device.display_name}</dd>
                </div>
                <div>
                  <dt>Daemon name</dt>
                  <dd>{device.name}</dd>
                </div>
                <div>
                  <dt>Family</dt>
                  <dd>{device.family}</dd>
                </div>
                <div>
                  <dt>Stable UI order</dt>
                  <dd>{device.ui_order}</dd>
                </div>
                <div>
                  <dt>Physical role</dt>
                  <dd>{device.physical_role}</dd>
                </div>
                <div>
                  <dt>Device id</dt>
                  <dd>{device.id}</dd>
                </div>
              </dl>
            </Panel>

            <Panel
              className="page-main-grid__secondary"
              description="Controller and wireless metadata explain how this device fits into the current topology."
              eyebrow="topology"
              title="Controller and wireless context"
            >
              <dl className="detail-list">
                <div>
                  <dt>Controller label</dt>
                  <dd>{device.controller.label}</dd>
                </div>
                <div>
                  <dt>Controller kind</dt>
                  <dd>{device.controller.kind.replace(/_/g, " ")}</dd>
                </div>
                <div>
                  <dt>Wireless transport</dt>
                  <dd>{device.wireless?.transport ?? "not wireless"}</dd>
                </div>
                <div>
                  <dt>Wireless channel</dt>
                  <dd>{device.wireless?.channel ?? "n/a"}</dd>
                </div>
                <div>
                  <dt>Group label</dt>
                  <dd>{device.wireless?.group_label ?? "n/a"}</dd>
                </div>
              </dl>
            </Panel>
          </section>

          <section className="page-main-grid">
            <Panel
              className="page-main-grid__primary"
              description="The device model the backend currently exposes for discovery and capability-aware workbenches."
              eyebrow="capabilities"
              title="Capabilities"
            >
              <div className="chip-row">
                <span className="capability-chip">{device.capability_summary}</span>
                {device.capabilities.has_rgb ? (
                  <span className="capability-chip">{rgbZoneLabel(device.capabilities.rgb_zone_count)} RGB</span>
                ) : null}
                {device.capabilities.has_fan ? (
                  <span className="capability-chip">{formatNumber(device.capabilities.fan_count)} fan slots</span>
                ) : null}
                {device.capabilities.has_lcd ? <span className="capability-chip">LCD</span> : null}
                {device.capabilities.has_pump ? <span className="capability-chip">Pump</span> : null}
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
            </Panel>

            <Panel
              className="page-main-grid__secondary"
              description="Live status and telemetry stay visible here without sending you into a separate diagnostics workspace."
              eyebrow="status"
              title="Status summary"
            >
              <dl className="detail-list">
                <div>
                  <dt>Health</dt>
                  <dd>{device.health.summary}</dd>
                </div>
                <div>
                  <dt>Current mode</dt>
                  <dd>{device.current_mode_summary}</dd>
                </div>
                <div>
                  <dt>Fan RPM telemetry</dt>
                  <dd>{device.state.fan_rpms && device.state.fan_rpms.length > 0 ? device.state.fan_rpms.join(" / ") : "n/a"}</dd>
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
            </Panel>
          </section>

          <section className="page-main-grid">
            <Panel
              className="page-main-grid__primary"
              description="Current stored desired-state lighting data from the backend lighting endpoint."
              eyebrow="lighting"
              title="Current lighting summary"
            >
              {!device.capabilities.has_rgb ? (
                <EmptyState message="This device does not expose RGB control." title="No lighting capability" />
              ) : lightingError ? (
                <article className="error-banner" role="alert">
                  <strong>Lighting data unavailable.</strong>
                  <span>{lightingError}</span>
                </article>
              ) : lightingState && lightingState.zones.length > 0 ? (
                <div className="lighting-zone-grid">
                  {lightingState.zones.map((zone) => (
                    <article key={zone.zone} className="lighting-zone-card">
                      <div className="lighting-zone-card__header">
                        <div>
                          <p className="device-card__eyebrow">Zone {zone.zone}</p>
                          <h3>{zone.effect}</h3>
                        </div>
                        <span className="capability-chip">{zone.brightness_percent}% brightness</span>
                      </div>
                      <div className="chip-row">
                        {zone.colors.map((color) => (
                          <span key={color} className="color-chip">
                            <span aria-hidden="true" className="color-chip__swatch" style={{ backgroundColor: color }} />
                            {color}
                          </span>
                        ))}
                      </div>
                    </article>
                  ))}
                </div>
              ) : (
                <EmptyState message="The backend returned no lighting zones for this device." title="No lighting state reported" />
              )}
            </Panel>

            <Panel
              className="page-main-grid__secondary"
              description="Config-backed slot state plus telemetry RPMs, when the backend exposes them."
              eyebrow="fans"
              title="Current fan summary"
            >
              {!device.capabilities.has_fan ? (
                <EmptyState message="This device does not expose manual fan control." title="No fan capability" />
              ) : fanError ? (
                <article className="error-banner" role="alert">
                  <strong>Fan data unavailable.</strong>
                  <span>{fanError}</span>
                </article>
              ) : fanState ? (
                <dl className="detail-list">
                  <div>
                    <dt>Update interval</dt>
                    <dd>{fanState.update_interval_ms} ms</dd>
                  </div>
                  <div>
                    <dt>RPM telemetry</dt>
                    <dd>{fanState.rpms && fanState.rpms.length > 0 ? fanState.rpms.join(" / ") : "n/a"}</dd>
                  </div>
                  <div>
                    <dt>Mode summary</dt>
                    <dd>{fanModeSummary}</dd>
                  </div>
                  <div>
                    <dt>Configured slots</dt>
                    <dd>{fanState.slots.length}</dd>
                  </div>
                </dl>
              ) : (
                <EmptyState message="The backend returned no fan slots for this device." title="No fan state reported" />
              )}
            </Panel>
          </section>

          <Panel
            description="Profiles listed here already target this device directly or apply to all devices."
            eyebrow="profiles"
            title="Assigned profiles"
          >
            {assignedProfiles.length > 0 ? (
              <div className="inventory-device-grid">
                {assignedProfiles.map((profile) => (
                  <article key={profile.id} className="controller-summary-card">
                    <div className="controller-summary-card__header">
                      <div>
                        <p className="device-card__eyebrow">{profile.targets.mode}</p>
                        <h3>{profile.name}</h3>
                      </div>
                      <span className="capability-chip">{profile.id}</span>
                    </div>
                    <p>{profile.description ?? "No profile description provided."}</p>
                  </article>
                ))}
              </div>
            ) : (
              <EmptyState
                message="No stored profile currently targets this device specifically or globally."
                title="No assigned profiles"
              />
            )}
          </Panel>

          <ActionBar summary="Jump directly from the device workspace into the relevant control surfaces.">
            <Link className="button-link button-link--primary" to={`/lighting?device=${encodeURIComponent(device.id)}`}>
              Open lighting
            </Link>
            <Link className="button-link" to={`/fans?device=${encodeURIComponent(device.id)}`}>
              Open fans
            </Link>
          </ActionBar>
        </>
      ) : null}
    </main>
  );
}
