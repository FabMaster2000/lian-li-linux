import { useBackendEvents } from "../app/BackendEventsProvider";
import { useSystemStatus } from "../app/SystemStatusProvider";

export function GlobalSystemNotices() {
  const { connectionState } = useBackendEvents();
  const { apiReachable, apiError, daemonStatus } = useSystemStatus();

  if (!apiReachable) {
    return (
      <section className="banner-stack" aria-label="Global system notices">
        <article className="error-banner" role="alert">
          <strong>API unreachable.</strong>
          <span>{apiError ?? "The frontend cannot reach the backend HTTP API."}</span>
        </article>
      </section>
    );
  }

  const notices: Array<{ key: string; title: string; body: string }> = [];

  if (daemonStatus && !daemonStatus.reachable) {
    notices.push({
      key: "daemon",
      title: "Daemon unavailable.",
      body:
        daemonStatus.error ??
        "The backend is running, but the daemon socket is currently not reachable.",
    });
  }

  if (connectionState === "reconnecting" || connectionState === "disconnected") {
    notices.push({
      key: "websocket",
      title: "Live event stream disconnected.",
      body:
        connectionState === "reconnecting"
          ? "The frontend is trying to reconnect to the websocket event stream."
          : "Live updates are currently unavailable until the websocket reconnects.",
    });
  }

  if (notices.length === 0) {
    return null;
  }

  return (
    <section className="banner-stack" aria-label="Global system notices">
      {notices.map((notice) => (
        <article key={notice.key} className="warning-banner" role="status">
          <strong>{notice.title}</strong>
          <span>{notice.body}</span>
        </article>
      ))}
    </section>
  );
}
