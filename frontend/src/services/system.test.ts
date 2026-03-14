import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  connectBackendEvents,
  getDaemonStatus,
  getHealth,
  getRuntime,
  getVersion,
} from "./system";
import { apiClient } from "./api";

vi.mock("./api", () => ({
  apiClient: {
    get: vi.fn(),
    connectEvents: vi.fn(),
  },
}));

const apiClientMock = vi.mocked(apiClient);

describe("system service", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("routes health, version, runtime, and daemon status to the shared API client", async () => {
    apiClientMock.get.mockResolvedValue({ ok: true });

    await getHealth();
    await getVersion();
    await getRuntime();
    await getDaemonStatus();

    expect(apiClientMock.get).toHaveBeenNthCalledWith(1, "/health");
    expect(apiClientMock.get).toHaveBeenNthCalledWith(2, "/version");
    expect(apiClientMock.get).toHaveBeenNthCalledWith(3, "/runtime");
    expect(apiClientMock.get).toHaveBeenNthCalledWith(4, "/daemon/status");
  });

  it("routes backend event connections through the shared API client", () => {
    const handlers = {
      onMessage: vi.fn(),
      onOpen: vi.fn(),
      onClose: vi.fn(),
      onError: vi.fn(),
      onParseError: vi.fn(),
    };
    const socket = { close: vi.fn() } as unknown as WebSocket;
    apiClientMock.connectEvents.mockReturnValue(socket);

    const result = connectBackendEvents(handlers);

    expect(result).toBe(socket);
    expect(apiClientMock.connectEvents).toHaveBeenCalledWith(handlers);
  });
});
