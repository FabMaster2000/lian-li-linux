import { defineConfig, loadEnv } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, ".", "");
  const devPort = Number(env.FRONTEND_PORT ?? "9200");
  const previewPort = Number(env.FRONTEND_PREVIEW_PORT ?? "9201");
  const apiTarget = env.VITE_API_PROXY_TARGET ?? "http://127.0.0.1:9100";

  return {
    plugins: [react()],
    server: {
      host: "0.0.0.0",
      port: devPort,
      proxy: {
        "/api": {
          target: apiTarget,
          changeOrigin: true,
          ws: true,
        },
      },
    },
    preview: {
      host: "0.0.0.0",
      port: previewPort,
    },
  };
});
