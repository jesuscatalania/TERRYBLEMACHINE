/// <reference types="vitest" />
import path from "node:path";
import process from "node:process";
import react from "@vitejs/plugin-react";
import { visualizer } from "rollup-plugin-visualizer";
import { defineConfig } from "vite";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig(async () => ({
  plugins: [
    react(),
    visualizer({
      filename: "dist/stats.html",
      template: "treemap",
      gzipSize: true,
      brotliSize: false,
      open: false,
    }),
  ],

  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },

  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },

  build: {
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (id.includes("node_modules")) {
            // Three.js + react-three ecosystem (Graphic3D module)
            if (/node_modules\/(three|@react-three\/(fiber|drei|postprocessing))/.test(id)) {
              return "vendor-three";
            }
            // Fabric.js (Graphic2D + Typography modules)
            if (/node_modules\/fabric\//.test(id)) return "vendor-fabric";
            // Monaco editor (WebsiteBuilder module)
            if (
              /node_modules\/@monaco-editor\//.test(id) ||
              /node_modules\/monaco-editor\//.test(id)
            ) {
              return "vendor-monaco";
            }
            // framer-motion (used by AnimatePresence, Toast, Tooltip — eager)
            if (/node_modules\/framer-motion/.test(id)) return "vendor-motion";
            // React core + DOM + scheduler + react-router. Goes into a single
            // vendor-react chunk so the shell critical path is `entry +
            // vendor-react` (~180KB combined) instead of `entry + vendor-misc`
            // (~1.2MB) which used to drag react-dom along with everything else.
            if (
              /node_modules\/(react|react-dom|scheduler|react-router|react-router-dom)\//.test(id)
            ) {
              return "vendor-react";
            }
            // Everything else (lucide-react, zustand, gif.js, jsPDF, etc.)
            return "vendor-misc";
          }
        },
      },
    },
  },

  test: {
    globals: true,
    environment: "jsdom",
    setupFiles: ["./src/test/setup.ts"],
    css: true,
    include: ["src/**/*.test.{ts,tsx}", "e2e/fixtures/**/*.test.ts"],
    coverage: {
      provider: "v8",
      reporter: ["text", "html", "lcov"],
      include: ["src/**/*.{ts,tsx}"],
      exclude: ["src/**/*.test.{ts,tsx}", "src/test/**", "src/main.tsx", "src/vite-env.d.ts"],
    },
  },
}));
