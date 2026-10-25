import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { fileURLToPath, URL } from "node:url";
export default defineConfig({
  resolve: { alias: { "@": fileURLToPath(new URL("./src", import.meta.url)) } },
  plugins: [react(), tailwindcss()],
  optimizeDeps: { include: ["react-day-picker/locale", "@daypicker/persian", "date-fns-jalali"] },
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
