import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// The Android emulator loads the dev server from the desktop, so it
// listens where Tauri's CLI tells it (TAURI_DEV_HOST) during `android dev`.
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 5174 } : undefined,
  },
});
