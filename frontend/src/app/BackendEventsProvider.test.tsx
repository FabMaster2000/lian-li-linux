import { act, render, screen } from "@testing-library/react";
import { useEffect, useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  BackendEventsProvider,
  useBackendEvents,
} from "./BackendEventsProvider";
import { connectBackendEvents } from "../services/system";
import type { BackendEventEnvelope } from "../types/api";

vi.mock("../services/system", () => ({
  connectBackendEvents: vi.fn(),
}));

const connectBackendEventsMock = vi.mocked(connectBackendEvents);

type HandlerSet = Parameters<typeof connectBackendEvents>[0];

function Probe() {
  const { connectionState, subscribe } = useBackendEvents();
  const [lastEventType, setLastEventType] = useState("none");

  useEffect(() => subscribe((event) => setLastEventType(event.type)), [subscribe]);

  return (
    <>
      <span>{connectionState}</span>
      <span>{lastEventType}</span>
    </>
  );
}

describe("BackendEventsProvider", () => {
  const handlers: HandlerSet[] = [];
  const closeFns: ReturnType<typeof vi.fn>[] = [];

  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();
    handlers.length = 0;
    closeFns.length = 0;

    connectBackendEventsMock.mockImplementation((nextHandlers) => {
      handlers.push(nextHandlers);
      const close = vi.fn();
      closeFns.push(close);
      return { close } as unknown as WebSocket;
    });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("connects automatically and dispatches backend events to subscribers", () => {
    render(
      <BackendEventsProvider>
        <Probe />
      </BackendEventsProvider>,
    );

    expect(connectBackendEventsMock).toHaveBeenCalledTimes(1);
    expect(screen.getByText("connecting")).toBeInTheDocument();

    act(() => {
      handlers[0]?.onOpen?.();
    });

    expect(screen.getByText("connected")).toBeInTheDocument();

    act(() => {
      handlers[0]?.onMessage({
        type: "lighting.changed",
        timestamp: "2026-03-14T10:00:00Z",
        source: "ws",
        device_id: "wireless:one",
        data: {},
      } satisfies BackendEventEnvelope);
    });

    expect(screen.getByText("lighting.changed")).toBeInTheDocument();
  });

  it("reconnects after the websocket closes unexpectedly", () => {
    render(
      <BackendEventsProvider>
        <Probe />
      </BackendEventsProvider>,
    );

    act(() => {
      handlers[0]?.onOpen?.();
      handlers[0]?.onClose?.({} as CloseEvent);
    });

    expect(screen.getByText("reconnecting")).toBeInTheDocument();

    act(() => {
      vi.advanceTimersByTime(1000);
    });

    expect(connectBackendEventsMock).toHaveBeenCalledTimes(2);
  });
});
