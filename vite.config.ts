import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

// Tauri 推荐配置：固定端口、失败即退出、不打印杂音
export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  envPrefix: ["VITE_", "TAURI_ENV_*"],
  build: {
    target: "chrome110",
    minify: "esbuild",
    sourcemap: false,
  },
});
