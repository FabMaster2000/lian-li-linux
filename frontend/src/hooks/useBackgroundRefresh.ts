import { useEffect, useRef } from "react";

export const LIVE_STATUS_REFRESH_INTERVAL_MS = 5000;

export function useBackgroundRefresh(
  refresh: () => Promise<unknown> | void,
  intervalMs = LIVE_STATUS_REFRESH_INTERVAL_MS,
  enabled = true,
) {
  const refreshRef = useRef(refresh);
  const inFlightRef = useRef(false);

  useEffect(() => {
    refreshRef.current = refresh;
  }, [refresh]);

  useEffect(() => {
    if (!enabled) {
      return;
    }

    const timer = window.setInterval(() => {
      if (inFlightRef.current) {
        return;
      }

      inFlightRef.current = true;
      Promise.resolve(refreshRef.current()).finally(() => {
        inFlightRef.current = false;
      });
    }, intervalMs);

    return () => {
      window.clearInterval(timer);
    };
  }, [enabled, intervalMs]);
}
