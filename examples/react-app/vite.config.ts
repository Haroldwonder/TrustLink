import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  base: "/TrustLink/", // GitHub Pages base path
  define: {
    global: "globalThis",
  },
  build: {
    chunkSizeWarningLimit: 1000,
    rollupOptions: {
      output: {
        manualChunks: {
          stellar: ["@stellar/stellar-sdk"],
          freighter: ["@stellar/freighter-api"],
          react: ["react", "react-dom"],
        },
      },
    },
  },
});
