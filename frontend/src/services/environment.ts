export const frontendEnvironment = {
  apiBaseUrl: import.meta.env.VITE_API_BASE_URL ?? "/api",
  websocketUrl: import.meta.env.VITE_WS_URL ?? "/api/ws",
} as const;
