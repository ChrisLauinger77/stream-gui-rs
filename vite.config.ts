import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  // Cargo output can exhaust native file watchers during desktop validation.
  server: { port: 1420, strictPort: true, watch: { ignored: ["**/src-tauri/**"] } },
  envPrefix: ["VITE_"],
  build: { target: ["es2022", "chrome105", "safari15"] },
});
