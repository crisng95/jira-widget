import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// Tauri chay webview tro toi dev server nay; port co dinh de tauri.conf.json bam theo.
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 5183,
    strictPort: true,
  },
  build: {
    // Webview cua Tauri tren macOS la WKWebView -> ban Safari cua he dieu hanh.
    target: "safari15",
    // Vite 8 minify bang oxc; "esbuild" doi cai them goi rieng.
    minify: "oxc",
    sourcemap: false,
    rollupOptions: {
      // Hai trang: panel (index) va cua so cai dat (settings).
      input: {
        index: "index.html",
        settings: "settings.html",
      },
    },
  },
});
