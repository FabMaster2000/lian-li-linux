import { useCallback, useEffect, useState } from "react";
import { DeviceOverviewCard } from "../components/DeviceOverviewCard";
import { PageIntro } from "../components/PageIntro";
import { useDocumentTitle } from "../hooks/useDocumentTitle";
import { listDevices } from "../services/devices";
import type { DeviceView } from "../types/api";

function toErrorMessage(error: unknown, fallback: string) {
  return error instanceof Error ? error.message : fallback;
}

export function DevicesPage() {
  useDocumentTitle("Devices - Lian Li Control Surface");
  const [devices, setDevices] = useState<DeviceView[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);

    try {
      setDevices(await listDevices());
    } catch (err) {
      setError(toErrorMessage(err, "Device inventory could not be loaded"));
      setDevices([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return (
    <main className="page-shell">
      <PageIntro
        eyebrow="devices"
        title="Choose a device"
        description="Select a device explicitly before opening the detail view. This avoids jumping straight into whichever simulated or physical controller happens to be listed first."
        aside={
          <div className="dashboard-aside">
            <dl className="runtime-grid">
              <div>
                <dt>Devices</dt>
                <dd>{loading ? "loading" : devices.length}</dd>
              </div>
              <div>
                <dt>Status</dt>
                <dd>{error ? "attention" : loading ? "refreshing" : "ready"}</dd>
              </div>
              <div>
                <dt>Entry mode</dt>
                <dd>manual selection</dd>
              </div>
            </dl>

            <button className="refresh-button" onClick={() => void refresh()} type="button">
              Refresh devices
            </button>
          </div>
        }
      />

      {error ? (
        <section className="error-banner" role="alert">
          <strong>Device selection load failed.</strong>
          <span>{error}</span>
        </section>
      ) : null}

      {loading ? (
        <section className="device-grid">
          <article className="empty-state">
            <h3>Loading device inventory</h3>
            <p>The frontend is requesting the current backend device list.</p>
          </article>
        </section>
      ) : devices.length > 0 ? (
        <section className="panel-stack">
          <div className="panel-stack__header">
            <div>
              <p className="panel-stack__eyebrow">devices</p>
              <h2>Available devices</h2>
            </div>
            <p>Open a detail view only after you have explicitly chosen the controller you want.</p>
          </div>

          <div className="device-grid">
            {devices.map((device) => (
              <DeviceOverviewCard
                key={device.id}
                device={device}
                detailsTo={`/devices/${encodeURIComponent(device.id)}`}
                fansTo={null}
                lightingTo={null}
              />
            ))}
          </div>
        </section>
      ) : (
        <section className="device-grid">
          <article className="empty-state">
            <h3>No devices reported</h3>
            <p>The backend is reachable, but there are no devices to open yet.</p>
          </article>
        </section>
      )}
    </main>
  );
}
