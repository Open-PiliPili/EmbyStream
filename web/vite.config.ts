import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { fileURLToPath, URL } from "node:url";
import fs from "node:fs";

const cargoToml = fs.readFileSync(
  fileURLToPath(new URL("../Cargo.toml", import.meta.url)),
  "utf8",
);
const cargoVersion = cargoToml.match(/^version = "([^"]+)"/m)?.[1] ?? "0.0.0";
const githubUrl = "https://github.com/PiliPili-Team/EmbyStream";
const changelogUrl = "https://github.com/PiliPili-Team/EmbyStream/releases";

export default defineConfig({
  plugins: [vue()],
  define: {
    __APP_VERSION__: JSON.stringify(cargoVersion),
    __APP_GITHUB_URL__: JSON.stringify(githubUrl),
    __APP_CHANGELOG_URL__: JSON.stringify(changelogUrl),
  },
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  server: {
    host: "127.0.0.1",
    port: 5173,
  },
  build: {
    outDir: "dist",
    sourcemap: true,
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (id.includes("node_modules/vue-i18n")) {
            return "vendor-i18n";
          }
          if (id.includes("node_modules/highlight.js")) {
            return "vendor-highlight";
          }
          if (
            id.includes("node_modules/vue") ||
            id.includes("node_modules/pinia") ||
            id.includes("node_modules/vue-router")
          ) {
            return "vendor-core";
          }
        },
      },
    },
  },
});
