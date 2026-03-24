import { useMemo, useState } from "react";
import { EmptyState } from "../components/feedback/EmptyState";
import { InventoryDeviceCard } from "../components/devices/InventoryDeviceCard";
import { PageIntro } from "../components/PageIntro";
import { Panel } from "../components/ui/Panel";
import { StatTile } from "../components/ui/StatTile";
import { isInventoryVisibleDevice } from "../features/wirelessSync";
import { useDeviceInventoryData } from "../hooks/useDeviceInventoryData";
import { useDocumentTitle } from "../hooks/useDocumentTitle";
import type { DeviceView } from "../types/api";

function filterDevices(devices: DeviceView[], searchTerm: string, familyFilter: string) {
  const normalizedSearch = searchTerm.trim().toLowerCase();

  return devices.filter((device) => {
    const matchesFamily = familyFilter === "all" || device.family === familyFilter;
    const matchesSearch =
      normalizedSearch.length === 0 ||
      [
        device.display_name,
        device.name,
        device.id,
        device.family,
        device.controller.label,
        device.physical_role,
        device.wireless?.group_label ?? "",
      ]
        .join(" ")
        .toLowerCase()
        .includes(normalizedSearch);

    return matchesFamily && matchesSearch;
  });
}

export function DevicesPage() {
  useDocumentTitle("Devices - Lian Li Control Surface");
  const { devices, loading, refreshing, error, refresh } = useDeviceInventoryData();
  const [searchTerm, setSearchTerm] = useState("");
  const [familyFilter, setFamilyFilter] = useState("all");
  const visibleDevices = useMemo(
    () => devices.filter(isInventoryVisibleDevice),
    [devices],
  );

  const familyOptions = useMemo(
    () =>
      [...new Set(visibleDevices.map((device) => device.family))].sort((left, right) =>
        left.localeCompare(right),
      ),
    [visibleDevices],
  );
  const filteredDevices = useMemo(
    () => filterDevices(visibleDevices, searchTerm, familyFilter),
    [familyFilter, searchTerm, visibleDevices],
  );
  const onlineCount = visibleDevices.filter((device) => device.online).length;
  const fanCapableCount = visibleDevices.filter((device) => device.capabilities.has_fan).length;
  const wirelessCount = visibleDevices.filter((device) => device.wireless !== null).length;

  return (
    <main className="page-shell">
      <PageIntro
        eyebrow="devices"
        title="Devices and fan status"
        description="This page stays focused on what is connected, how each device is grouped, and what the current fan telemetry looks like."
        aside={
          <div className="dashboard-aside">
            <dl className="runtime-grid">
              <div>
                <dt>Devices</dt>
                <dd>{loading ? "loading" : visibleDevices.length}</dd>
              </div>
              <div>
                <dt>Online</dt>
                <dd>{loading ? "loading" : onlineCount}</dd>
              </div>
              <div>
                <dt>Inventory</dt>
                <dd>{refreshing ? "refreshing" : error ? "attention" : "ready"}</dd>
              </div>
            </dl>

            <button className="refresh-button" onClick={() => void refresh()} type="button">
              {refreshing ? "Refreshing..." : "Refresh devices"}
            </button>
          </div>
        }
      />

      <section className="summary-strip">
        <StatTile
          detail="All devices currently reported by the backend."
          label="Devices"
          tone="accent"
          value={loading ? "loading" : String(visibleDevices.length)}
        />
        <StatTile
          detail="Devices that currently report online."
          label="Online"
          tone={onlineCount === visibleDevices.length ? "success" : "warning"}
          value={loading ? "loading" : String(onlineCount)}
        />
        <StatTile
          detail="Devices that expose fan control or fan telemetry."
          label="Fan capable"
          value={loading ? "loading" : String(fanCapableCount)}
        />
        <StatTile
          detail="Devices currently reported through the wireless path."
          label="Wireless"
          value={loading ? "loading" : String(wirelessCount)}
        />
      </section>

      {error ? (
        <section className="error-banner" role="alert">
          <strong>Device inventory load failed.</strong>
          <span>{error}</span>
        </section>
      ) : null}

      <section className="page-main-grid">
        <Panel
          className="page-main-grid__primary"
          description="Narrow the list when you need to find a specific cluster or controller, without adding more control UI to this page."
          eyebrow="filters"
          title="Device filters"
        >
          <div className="inventory-controls">
            <label className="field-group">
              <span className="field-group__label">Search</span>
              <input
                className="field-input"
                onChange={(event) => setSearchTerm(event.target.value)}
                placeholder="Search by name, id, family, controller, or wireless group"
                value={searchTerm}
              />
            </label>

            <label className="field-group">
              <span className="field-group__label">Family</span>
              <select
                className="field-input"
                onChange={(event) => setFamilyFilter(event.target.value)}
                value={familyFilter}
              >
                <option value="all">All families</option>
                {familyOptions.map((family) => (
                  <option key={family} value={family}>
                    {family}
                  </option>
                ))}
              </select>
            </label>
          </div>
        </Panel>

        <Panel
          className="page-main-grid__secondary"
          description="Lighting and fan changes now live on their own focused pages, so this inventory stays readable."
          eyebrow="scope"
          title="What stays here"
        >
          <ul className="content-list">
            <li>Identity, family, controller, and wireless group context.</li>
            <li>Fan count, RGB zone count, and reported RPM telemetry.</li>
            <li>Quick links into the focused lighting and fan pages.</li>
          </ul>
        </Panel>
      </section>

      <section className="panel-stack">
        <div className="panel-stack__header">
          <div>
            <p className="panel-stack__eyebrow">inventory</p>
            <h2>Connected devices</h2>
          </div>
          <p>Each card keeps only device information, grouping context, and fan-related status.</p>
        </div>

        {loading ? (
          <EmptyState
            message="The frontend is requesting the current inventory snapshot."
            title="Loading connected devices"
          />
        ) : filteredDevices.length > 0 ? (
          <div className="inventory-device-grid">
            {filteredDevices.map((device) => (
              <InventoryDeviceCard device={device} key={device.id} />
            ))}
          </div>
        ) : (
          <EmptyState
            message="Adjust the current search or family filter to reveal matching devices."
            title="No devices match the current filters"
          />
        )}
      </section>
    </main>
  );
}
