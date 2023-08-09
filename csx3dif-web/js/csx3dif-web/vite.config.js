import { defineConfig } from 'vite'
import wasm from "vite-plugin-wasm";
import topLevelAwait from "vite-plugin-top-level-await";
import crossOriginIsolation from 'vite-plugin-cross-origin-isolation'

export default defineConfig({
  plugins: [
    wasm(),
    topLevelAwait(),
    crossOriginIsolation()
  ],
  server: {
    fs: {
      allow: ["../../pkg", "src"]
    }
  }
});