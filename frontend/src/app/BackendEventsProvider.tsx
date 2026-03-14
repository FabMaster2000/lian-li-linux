import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import type { PropsWithChildren } from "react";
import { connectBackendEvents } from "../services/system";
import type { BackendEventEnvelope } from "../types/api";

export type BackendEventConnectionState =
  | "connecting"
  | "connected"
  | "reconnecting"
  | "disconnected";

type BackendEventListener = (event: BackendEventEnvelope) => void;

type BackendEventsContextValue = {
  connectionState: BackendEventConnectionState;
  subscribe: (listener: BackendEventListener) => () => void;
};

const noopUnsubscribe = () => undefined;

const BackendEventsContext = createContext<BackendEventsContextValue>({
  connectionState: "disconnected",
  subscribe: () => noopUnsubscribe,
});

const reconnectDelayMs = 1000;

export function BackendEventsProvider({ children }: PropsWithChildren) {
  const [connectionState, setConnectionState] =
    useState<BackendEventConnectionState>("connecting");
  const listenersRef = useRef(new Set<BackendEventListener>());
  const socketRef = useRef<WebSocket | null>(null);
  const reconnectTimerRef = useRef<number | null>(null);
  const disposedRef = useRef(false);

  const clearReconnectTimer = useCallback(() => {
    if (reconnectTimerRef.current !== null) {
      window.clearTimeout(reconnectTimerRef.current);
      reconnectTimerRef.current = null;
    }
  }, []);

  const subscribe = useCallback((listener: BackendEventListener) => {
    listenersRef.current.add(listener);

    return () => {
      listenersRef.current.delete(listener);
    };
  }, []);

  const connect = useCallback(() => {
    clearReconnectTimer();

    if (disposedRef.current) {
      return;
    }

    setConnectionState((current) =>
      current === "reconnecting" ? "reconnecting" : "connecting",
    );

    socketRef.current = connectBackendEvents({
      onOpen: () => {
        setConnectionState("connected");
      },
      onClose: () => {
        socketRef.current = null;

        if (disposedRef.current) {
          setConnectionState("disconnected");
          return;
        }

        setConnectionState("reconnecting");
        reconnectTimerRef.current = window.setTimeout(() => {
          reconnectTimerRef.current = null;
          connect();
        }, reconnectDelayMs);
      },
      onMessage: (event) => {
        for (const listener of listenersRef.current) {
          listener(event);
        }
      },
    });
  }, [clearReconnectTimer]);

  useEffect(() => {
    connect();

    return () => {
      disposedRef.current = true;
      clearReconnectTimer();
      socketRef.current?.close();
      socketRef.current = null;
      listenersRef.current.clear();
    };
  }, [clearReconnectTimer, connect]);

  const value = useMemo<BackendEventsContextValue>(
    () => ({
      connectionState,
      subscribe,
    }),
    [connectionState, subscribe],
  );

  return <BackendEventsContext.Provider value={value}>{children}</BackendEventsContext.Provider>;
}

export function useBackendEvents() {
  return useContext(BackendEventsContext);
}
