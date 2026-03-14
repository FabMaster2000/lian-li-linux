import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { apiClient } from "./api";
import type { BackendEventEnvelope } from "../types/api";

type WebSocketListener = (event: Event | MessageEvent | CloseEvent) => void;

class MockWebSocket {
  static instances: MockWebSocket[] = [];

  readonly url: string;
  readonly close = vi.fn();
  private listeners = new Map<string, WebSocketListener[]>();

  constructor(url: string) {
    this.url = url;
    MockWebSocket.instances.push(this);
  }

  addEventListener(type: string, listener: WebSocketListener) {
    const listeners = this.listeners.get(type) ?? [];
    listeners.push(listener);
    this.listeners.set(type, listeners);
  }

  emit(type: string, event: Event | MessageEvent | CloseEvent) {
    for (const listener of this.listeners.get(type) ?? []) {
      listener(event);
    }
  }

  static reset() {
    MockWebSocket.instances = [];
  }
}

describe("apiClient", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    MockWebSocket.reset();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("serializes query params and parses json responses", async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      new Response(JSON.stringify({ status: "ok" }), {
        status: 200,
        headers: { "content-type": "application/json" },
      }),
    );
    vi.stubGlobal("fetch", fetchMock);

    const response = await apiClient.get<{ status: string }>("/health", {
      query: {
        device: "wireless:one",
        zone: [0, 1],
        verbose: true,
        ignored: null,
      },
    });

    expect(response).toEqual({ status: "ok" });
    expect(fetchMock).toHaveBeenCalledWith(
      "http://localhost:3000/api/health?device=wireless%3Aone&zone=0&zone=1&verbose=true",
      expect.objectContaining({
        method: "GET",
      }),
    );
  });

  it("sends json bodies for write requests", async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      new Response(JSON.stringify({ ok: true }), {
        status: 200,
        headers: { "content-type": "application/json" },
      }),
    );
    vi.stubGlobal("fetch", fetchMock);

    await apiClient.post("/devices/wireless:one/fans/manual", { percent: 42 });

    expect(fetchMock).toHaveBeenCalledWith(
      "http://localhost:3000/api/devices/wireless:one/fans/manual",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({ percent: 42 }),
        headers: expect.any(Headers),
      }),
    );

    const requestHeaders = fetchMock.mock.calls[0]?.[1]?.headers as Headers;
    expect(requestHeaders.get("content-type")).toBe("application/json");
  });

  it("maps structured API errors to ApiClientError", async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      new Response(
        JSON.stringify({
          error: {
            code: "UNAUTHORIZED",
            message: "unauthorized: invalid bearer token",
            details: { source: "auth" },
          },
        }),
        {
          status: 401,
          headers: { "content-type": "application/json" },
        },
      ),
    );
    vi.stubGlobal("fetch", fetchMock);

    await expect(apiClient.get("/runtime")).rejects.toMatchObject({
      name: "ApiClientError",
      status: 401,
      code: "UNAUTHORIZED",
      message: "unauthorized: invalid bearer token",
      details: { source: "auth" },
      method: "GET",
      url: "http://localhost:3000/api/runtime",
    });
  });

  it("maps network failures to a synthetic ApiClientError", async () => {
    const fetchMock = vi.fn().mockRejectedValue(new Error("socket hang up"));
    vi.stubGlobal("fetch", fetchMock);

    await expect(apiClient.get("/devices")).rejects.toMatchObject({
      name: "ApiClientError",
      status: 0,
      code: "NETWORK_ERROR",
      message: "GET http://localhost:3000/api/devices could not be completed",
      method: "GET",
      url: "http://localhost:3000/api/devices",
    });
  });

  it("parses websocket events and surfaces parse failures", () => {
    vi.stubGlobal("WebSocket", MockWebSocket as unknown as typeof WebSocket);

    const onMessage = vi.fn();
    const onOpen = vi.fn();
    const onClose = vi.fn();
    const onError = vi.fn();
    const onParseError = vi.fn();

    const socket = apiClient.connectEvents({
      onMessage,
      onOpen,
      onClose,
      onError,
      onParseError,
    });

    const instance = MockWebSocket.instances[0];
    expect(socket).toBe(instance);
    expect(instance?.url).toBe("ws://localhost:3000/api/ws");

    instance?.emit("open", new Event("open"));
    instance?.emit(
      "message",
      new MessageEvent("message", {
        data: JSON.stringify({
          type: "lighting.changed",
          timestamp: "2026-03-14T10:00:00Z",
          source: "ws",
          device_id: "wireless:one",
          data: { zone: 0 },
        } satisfies BackendEventEnvelope),
      }),
    );
    instance?.emit(
      "message",
      new MessageEvent("message", {
        data: "{invalid-json",
      }),
    );
    instance?.emit("message", new MessageEvent("message", { data: 42 }));
    instance?.emit("error", new Event("error"));
    instance?.emit("close", new CloseEvent("close"));

    expect(onOpen).toHaveBeenCalledTimes(1);
    expect(onMessage).toHaveBeenCalledWith(
      expect.objectContaining({
        type: "lighting.changed",
        device_id: "wireless:one",
      }),
    );
    expect(onParseError).toHaveBeenCalledTimes(1);
    expect(onError).toHaveBeenCalledTimes(1);
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
