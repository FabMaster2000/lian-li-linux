import { ColorField } from "../components/ColorField";
import { EffectSelect } from "../components/EffectSelect";
import { PageIntro } from "../components/PageIntro";
import { SliderField } from "../components/SliderField";
import { lightingEffectOptions } from "../features/lighting";
import { useDocumentTitle } from "../hooks/useDocumentTitle";
import { useProfilesWorkbenchData } from "../hooks/useProfilesWorkbenchData";

function formatTargets(mode: string, deviceIds: string[]) {
  return mode === "all" ? "all devices" : `${deviceIds.length} selected device(s)`;
}

function formatTimestamp(timestamp: string) {
  return new Intl.DateTimeFormat(undefined, {
    year: "numeric",
    month: "short",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(timestamp));
}

export function ProfilesPage() {
  useDocumentTitle("Profiles - Lian Li Control Surface");
  const {
    devices,
    profiles,
    draft,
    setDraft,
    loading,
    submitting,
    deletingProfileId,
    applyingProfileId,
    error,
    success,
    applyResult,
    refresh,
    createDraftProfile,
    removeProfile,
    runProfile,
  } = useProfilesWorkbenchData();

  return (
    <main className="page-shell">
      <PageIntro
        eyebrow="profiles"
        title="Profile library"
        description="Create reusable lighting and cooling presets, store them in the backend profile library, and apply them against the live device inventory."
        aside={
          <div className="dashboard-aside">
            <dl className="runtime-grid">
              <div>
                <dt>Profiles</dt>
                <dd>{loading ? "loading" : profiles.length}</dd>
              </div>
              <div>
                <dt>Target devices</dt>
                <dd>{loading ? "loading" : devices.length}</dd>
              </div>
              <div>
                <dt>Mode</dt>
                <dd>{submitting ? "creating" : applyingProfileId ? "applying" : "ready"}</dd>
              </div>
            </dl>

            <button className="refresh-button" onClick={() => void refresh()} type="button">
              Refresh profiles
            </button>
          </div>
        }
      />

      {error ? (
        <section className="error-banner" role="alert">
          <strong>Profile action failed.</strong>
          <span>{error}</span>
        </section>
      ) : null}

      {success ? (
        <section className="success-banner" role="status">
          <strong>Profile action completed.</strong>
          <span>{success}</span>
        </section>
      ) : null}

      <section className="profiles-grid">
        <article className="profile-form-card">
          <div className="content-panel__header">
            <h2>Create profile</h2>
            <p>Define targets plus lighting and fan behavior for a reusable preset.</p>
          </div>

          <div className="form-grid form-grid--profile">
            <label className="field-group">
              <span className="field-group__label">Profile id</span>
              <input
                className="field-input"
                onChange={(event) =>
                  setDraft((current) => ({
                    ...current,
                    id: event.target.value,
                  }))
                }
                placeholder="night-mode"
                value={draft.id}
              />
            </label>

            <label className="field-group">
              <span className="field-group__label">Name</span>
              <input
                className="field-input"
                onChange={(event) =>
                  setDraft((current) => ({
                    ...current,
                    name: event.target.value,
                  }))
                }
                placeholder="Night Mode"
                value={draft.name}
              />
            </label>

            <label className="field-group">
              <span className="field-group__label">Targets</span>
              <select
                className="field-input"
                onChange={(event) =>
                  setDraft((current) => ({
                    ...current,
                    targetMode: event.target.value === "devices" ? "devices" : "all",
                    selectedDeviceIds:
                      event.target.value === "devices" ? current.selectedDeviceIds : [],
                  }))
                }
                value={draft.targetMode}
              >
                <option value="all">All devices</option>
                <option value="devices">Selected devices</option>
              </select>
            </label>
          </div>

          <label className="field-group profile-form__row">
            <span className="field-group__label">Description</span>
            <textarea
              className="field-input profile-textarea"
              onChange={(event) =>
                setDraft((current) => ({
                  ...current,
                  description: event.target.value,
                }))
              }
              placeholder="Dim lighting and reduce fan speed"
              value={draft.description}
            />
          </label>

          {draft.targetMode === "devices" ? (
            <div className="profile-form__row">
              <div className="content-panel__header">
                <h2>Target devices</h2>
                <p>Select at least one device when the profile should not apply globally.</p>
              </div>

              {devices.length > 0 ? (
                <div className="checkbox-grid">
                  {devices.map((device) => {
                    const checked = draft.selectedDeviceIds.includes(device.id);
                    return (
                      <label key={device.id} className="checkbox-card">
                        <input
                          checked={checked}
                          onChange={(event) =>
                            setDraft((current) => ({
                              ...current,
                              selectedDeviceIds: event.target.checked
                                ? [...current.selectedDeviceIds, device.id]
                                : current.selectedDeviceIds.filter((id) => id !== device.id),
                            }))
                          }
                          type="checkbox"
                        />
                        <span>
                          <strong>{device.name}</strong>
                          <small>{device.id}</small>
                        </span>
                      </label>
                    );
                  })}
                </div>
              ) : (
                <div className="empty-state">
                  <h3>No target devices available</h3>
                  <p>The backend has not reported any RGB- or fan-capable devices yet.</p>
                </div>
              )}
            </div>
          ) : null}

          <div className="profile-form__row">
            <div className="toggle-row">
              <label className="checkbox-card checkbox-card--inline">
                <input
                  checked={draft.lightingEnabled}
                  onChange={(event) =>
                    setDraft((current) => ({
                      ...current,
                      lightingEnabled: event.target.checked,
                    }))
                  }
                  type="checkbox"
                />
                <span>
                  <strong>Lighting</strong>
                  <small>Store color, effect, and brightness</small>
                </span>
              </label>
              <label className="checkbox-card checkbox-card--inline">
                <input
                  checked={draft.fanEnabled}
                  onChange={(event) =>
                    setDraft((current) => ({
                      ...current,
                      fanEnabled: event.target.checked,
                    }))
                  }
                  type="checkbox"
                />
                <span>
                  <strong>Fans</strong>
                  <small>Store a manual all-slot percentage</small>
                </span>
              </label>
            </div>
          </div>

          {draft.lightingEnabled ? (
            <div className="profile-form__row">
              <div className="content-panel__header">
                <h2>Lighting preset</h2>
                <p>Minimal lighting settings required for the first profile UI iteration.</p>
              </div>

              <div className="form-grid form-grid--profile">
                <EffectSelect
                  onChange={(lightingEffect) =>
                    setDraft((current) => ({
                      ...current,
                      lightingEffect,
                    }))
                  }
                  options={lightingEffectOptions}
                  value={draft.lightingEffect}
                />

                <ColorField
                  label="Color"
                  onChange={(lightingColor) =>
                    setDraft((current) => ({
                      ...current,
                      lightingColor,
                    }))
                  }
                  pickerAriaLabel="Profile lighting color picker"
                  value={draft.lightingColor}
                />

                <SliderField
                  label="Brightness"
                  onChange={(lightingBrightness) =>
                    setDraft((current) => ({
                      ...current,
                      lightingBrightness,
                    }))
                  }
                  value={draft.lightingBrightness}
                />
              </div>
            </div>
          ) : null}

          {draft.fanEnabled ? (
            <div className="profile-form__row">
              <div className="content-panel__header">
                <h2>Fan preset</h2>
                <p>The first profile release stores a single manual percentage.</p>
              </div>

              <div className="form-grid form-grid--profile">
                <label className="field-group">
                  <span className="field-group__label">Mode</span>
                  <select className="field-input" value={draft.fanMode}>
                    <option value="manual">Manual</option>
                  </select>
                </label>

                <SliderField
                  className="profile-form__slider"
                  label="Fan percent"
                  onChange={(fanPercent) =>
                    setDraft((current) => ({
                      ...current,
                      fanPercent,
                    }))
                  }
                  value={draft.fanPercent}
                />
              </div>
            </div>
          ) : null}

          <div className="device-actions">
            <button
              className="refresh-button"
              disabled={submitting}
              onClick={() => void createDraftProfile()}
              type="button"
            >
              {submitting ? "Creating..." : "Create profile"}
            </button>
          </div>
        </article>

        <article className="profile-list-card">
          <div className="content-panel__header">
            <h2>Stored profiles</h2>
            <p>Apply or delete the profiles that are currently stored in the backend.</p>
          </div>

          {profiles.length > 0 ? (
            <div className="profile-card-grid">
              {profiles.map((profile) => (
                <article key={profile.id} className="profile-card">
                  <div className="lighting-zone-card__header">
                    <div>
                      <p className="device-card__eyebrow">{profile.id}</p>
                      <h3>{profile.name}</h3>
                    </div>
                    <span className="capability-chip">
                      {formatTargets(profile.targets.mode, profile.targets.device_ids)}
                    </span>
                  </div>

                  {profile.description ? <p className="profile-card__body">{profile.description}</p> : null}

                  <div className="chip-row">
                    {profile.lighting?.enabled ? (
                      <span className="capability-chip">
                        lighting {profile.lighting.effect ?? "preset"}
                      </span>
                    ) : null}
                    {profile.fans?.enabled ? (
                      <span className="capability-chip">
                        fans {profile.fans.percent ?? "n/a"}%
                      </span>
                    ) : null}
                  </div>

                  <dl className="detail-list detail-list--compact">
                    <div>
                      <dt>Updated</dt>
                      <dd>{formatTimestamp(profile.metadata.updated_at)}</dd>
                    </div>
                    <div>
                      <dt>Targets</dt>
                      <dd>{formatTargets(profile.targets.mode, profile.targets.device_ids)}</dd>
                    </div>
                  </dl>

                  <div className="device-actions">
                    <button
                      className="refresh-button"
                      disabled={applyingProfileId === profile.id}
                      onClick={() => void runProfile(profile.id)}
                      type="button"
                    >
                      {applyingProfileId === profile.id ? "Applying..." : "Apply"}
                    </button>
                    <button
                      className="button-link"
                      disabled={deletingProfileId === profile.id}
                      onClick={() => void removeProfile(profile.id)}
                      type="button"
                    >
                      {deletingProfileId === profile.id ? "Deleting..." : "Delete"}
                    </button>
                  </div>
                </article>
              ))}
            </div>
          ) : (
            <div className="empty-state">
              <h3>No profiles stored</h3>
              <p>Create the first profile in the form on the left.</p>
            </div>
          )}
        </article>
      </section>

      {applyResult ? (
        <section className="detail-section">
          <div className="panel-stack__header">
            <div>
              <p className="panel-stack__eyebrow">apply result</p>
              <h2>Last apply result</h2>
            </div>
            <p>The backend returns applied device IDs and any skipped targets after each apply run.</p>
          </div>

          <div className="profiles-grid">
            <article className="profile-result-card">
              <div className="content-panel__header">
                <h2>{applyResult.profile_name}</h2>
                <p>{applyResult.transaction_mode}</p>
              </div>
              <dl className="detail-list">
                <div>
                  <dt>Lighting applied</dt>
                  <dd>{applyResult.applied_lighting_device_ids.join(", ") || "none"}</dd>
                </div>
                <div>
                  <dt>Fans applied</dt>
                  <dd>{applyResult.applied_fan_device_ids.join(", ") || "none"}</dd>
                </div>
                <div>
                  <dt>Rollback supported</dt>
                  <dd>{applyResult.rollback_supported ? "yes" : "no"}</dd>
                </div>
              </dl>
            </article>

            <article className="profile-result-card">
              <div className="content-panel__header">
                <h2>Skipped devices</h2>
                <p>Devices the backend skipped during the apply call.</p>
              </div>
              {applyResult.skipped_devices.length > 0 ? (
                <div className="profile-skip-list">
                  {applyResult.skipped_devices.map((skip) => (
                    <article key={`${skip.device_id}-${skip.section}`} className="profile-skip-card">
                      <strong>{skip.device_id}</strong>
                      <span>{skip.section}</span>
                      <p>{skip.reason}</p>
                    </article>
                  ))}
                </div>
              ) : (
                <div className="empty-state">
                  <h3>No skipped devices</h3>
                  <p>The backend applied this profile without partial skips.</p>
                </div>
              )}
            </article>
          </div>
        </section>
      ) : null}
    </main>
  );
}
