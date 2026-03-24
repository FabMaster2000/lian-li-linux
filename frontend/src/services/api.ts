import { frontendEnvironment } from "./environment";
import type { ApiErrorResponse, BackendEventEnvelope, JsonValue } from "../types/api";

export class ApiClientError extends Error {
  readonly status: number;
  readonly code: string;
  readonly details: Record<string, JsonValue>;
  readonly method: string;
  readonly url: string;
  readonly causeError?: unknown;

  constructor(params: {
    message: string;
    status: number;
    code: string;
    details?: Record<string, JsonValue>;
    method: string;
    url: string;
    causeError?: unknown;
  }) {
    super(params.message);
    this.name = "ApiClientError";
    this.status = params.status;
    this.code = params.code;
    this.details = params.details ?? {};
    this.method = params.method;
    this.url = params.url;
    this.causeError = params.causeError;
  }
}

type QueryValue = string | number | boolean | null | undefined;
type QueryParams = Record<string, QueryValue | QueryValue[]>;

type RequestOptions = Omit<RequestInit, "body" | "method"> & {
  query?: QueryParams;
};

type EventStreamHandlers<TData extends JsonValue = JsonValue> = {
  onMessage: (event: BackendEventEnvelope<TData>) => void;
  onOpen?: () => void;
  onClose?: (event: CloseEvent) => void;
  onError?: (event: Event) => void;
  onParseError?: (raw: string, error: unknown) => void;
};

function trimLeadingSlash(value: string) {
  return value.replace(/^\/+/, "");
}

function trimTrailingSlash(value: string) {
  return value.replace(/\/+$/, "");
}

function joinPath(base: string, path: string) {
  const normalizedBase = trimTrailingSlash(base);
  const normalizedPath = trimLeadingSlash(path);
  return `${normalizedBase}/${normalizedPath}`;
}

function resolveHttpUrl(path: string) {
  const joined = joinPath(frontendEnvironment.apiBaseUrl, path);
  if (/^https?:\/\//i.test(joined)) {
    return joined;
  }

  return new URL(joined, window.location.origin).toString();
}

function resolveWebSocketUrl(path: string) {
  if (/^wss?:\/\//i.test(path)) {
    return path;
  }

  if (/^https?:\/\//i.test(path)) {
    const url = new URL(path);
    url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
    return url.toString();
  }

  const url = new URL(path, window.location.origin);
  url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
  return url.toString();
}

function withQuery(url: string, query?: QueryParams) {
  if (!query) {
    return url;
  }

  const next = new URL(url);
  for (const [key, value] of Object.entries(query)) {
    if (Array.isArray(value)) {
      for (const item of value) {
        if (item !== undefined && item !== null) {
          next.searchParams.append(key, String(item));
        }
      }
      continue;
    }

    if (value !== undefined && value !== null) {
      next.searchParams.set(key, String(value));
    }
  }

  return next.toString();
}

async function parseResponse<TResponse>(response: Response) {
  if (response.status === 204) {
    return undefined as TResponse;
  }

  const contentType = response.headers.get("content-type") ?? "";
  if (contentType.includes("application/json")) {
    return (await response.json()) as TResponse;
  }

  return (await response.text()) as TResponse;
}

async function toApiClientError(response: Response, method: string, url: string) {
  let payload: ApiErrorResponse | null = null;

  try {
    payload = (await response.json()) as ApiErrorResponse;
  } catch {
    payload = null;
  }

  const message =
    payload?.error.message ??
    `${method.toUpperCase()} ${url} failed with ${response.status}`;

  return new ApiClientError({
    message,
    status: response.status,
    code: payload?.error.code ?? "HTTP_ERROR",
    details: payload?.error.details,
    method,
    url,
  });
}

async function request<TResponse, TBody = undefined>(
  method: string,
  path: string,
  body?: TBody,
  options: RequestOptions = {},
) {
  const url = withQuery(resolveHttpUrl(path), options.query);
  const headers = new Headers(options.headers);
  const init: RequestInit = {
    ...options,
    method,
    headers,
  };

  if (method === "GET" && init.cache === undefined) {
    init.cache = "no-store";
  }

  if (body !== undefined) {
    headers.set("content-type", "application/json");
    init.body = JSON.stringify(body);
  }

  try {
    const response = await fetch(url, init);
    if (!response.ok) {
      throw await toApiClientError(response, method, url);
    }

    return await parseResponse<TResponse>(response);
  } catch (error) {
    if (error instanceof ApiClientError) {
      throw error;
    }

    throw new ApiClientError({
      message: `${method.toUpperCase()} ${url} could not be completed`,
      status: 0,
      code: "NETWORK_ERROR",
      method,
      url,
      causeError: error,
    });
  }
}

export const apiClient = {
  get<TResponse>(path: string, options?: RequestOptions) {
    return request<TResponse>("GET", path, undefined, options);
  },

  post<TResponse, TBody>(path: string, body: TBody, options?: RequestOptions) {
    return request<TResponse, TBody>("POST", path, body, options);
  },

  put<TResponse, TBody>(path: string, body: TBody, options?: RequestOptions) {
    return request<TResponse, TBody>("PUT", path, body, options);
  },

  delete<TResponse>(path: string, options?: RequestOptions) {
    return request<TResponse>("DELETE", path, undefined, options);
  },

  connectEvents<TData extends JsonValue = JsonValue>(
    handlers: EventStreamHandlers<TData>,
    path = frontendEnvironment.websocketUrl,
  ) {
    const socket = new WebSocket(resolveWebSocketUrl(path));

    socket.addEventListener("open", () => handlers.onOpen?.());
    socket.addEventListener("close", (event) => handlers.onClose?.(event));
    socket.addEventListener("error", (event) => handlers.onError?.(event));
    socket.addEventListener("message", (message) => {
      if (typeof message.data !== "string") {
        return;
      }

      try {
        const event = JSON.parse(message.data) as BackendEventEnvelope<TData>;
        handlers.onMessage(event);
      } catch (error) {
        handlers.onParseError?.(message.data, error);
      }
    });

    return socket;
  },
};
