import { useEffect } from "react";
import { useBackendEvents } from "../app/BackendEventsProvider";
import type { BackendEventEnvelope } from "../types/api";

export function useBackendEventSubscription(
  listener: (event: BackendEventEnvelope) => void,
) {
  const { subscribe } = useBackendEvents();

  useEffect(() => subscribe(listener), [listener, subscribe]);
}
