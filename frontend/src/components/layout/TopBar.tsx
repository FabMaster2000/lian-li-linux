import { StatusBadge } from "../ui/StatusBadge";

type TopBarProps = {
  sectionLabel: string;
  sectionDescription: string;
  daemonReachable: boolean | null;
  apiReachable: boolean;
  connectionState: string;
};

export function TopBar({
  sectionLabel,
  sectionDescription,
  daemonReachable,
  apiReachable,
  connectionState,
}: TopBarProps) {
  return (
    <header className="top-bar">
      <div className="top-bar__copy">
        <p className="top-bar__eyebrow">application</p>
        <h2>{sectionLabel}</h2>
        <p>{sectionDescription}</p>
      </div>

      <div className="top-bar__actions">
        <StatusBadge tone={apiReachable ? "ready" : "warning"}>
          api {apiReachable ? "reachable" : "attention"}
        </StatusBadge>
        <StatusBadge tone={daemonReachable === false ? "warning" : "ready"}>
          daemon {daemonReachable === false ? "offline" : "reachable"}
        </StatusBadge>
        <StatusBadge tone={connectionState === "connected" ? "info" : "warning"}>
          ws {connectionState}
        </StatusBadge>
      </div>
    </header>
  );
}
