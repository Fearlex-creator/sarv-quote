import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: {
    port: 1420,
    strictPort: true,
    // QA runs a stable frontend while native builds write generated HTML.
    watch: process.env.VITE_QA_URL
      ? null
      : { ignored: (path) => /\/(src-tauri|dist)\//.test(path) },
  },
  envPrefix: ["VITE_", "TAURI_"],
  clearScreen: false,
});
