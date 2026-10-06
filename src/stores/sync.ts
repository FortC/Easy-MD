import { defineStore } from "pinia";
import { listen } from "@tauri-apps/api/event";
import { api } from "../ipc/tauri";
import { t, tf } from "../i18n";

/** 同步状态：进度事件来自 Rust 端 mcp_sync */
export const useSyncStore = defineStore("sync", {
  state: () => ({
    running: false,
    message: "",
    done: 0,
    total: 0,
    lastReport: "" ,
    lastError: "",
  }),
  actions: {
    async init() {
      await listen<{ message: string; done: number; total: number }>(
        "mcp-sync-progress",
        (e) => {
          this.running = true;
          this.message = e.payload.message;
          this.done = e.payload.done;
          this.total = e.payload.total;
        },
      );
      await listen<{
        uploaded: string[];
        downloaded: string[];
        errors: string[];
        remote_files: number;
        local_files: number;
      }>("mcp-sync-done", (e) => {
        this.running = false;
        const r = e.payload;
        this.lastReport = tf("sy.done", { u: r.uploaded.length, d: r.downloaded.length, e: r.errors.length, l: r.local_files, r: r.remote_files });
        if (r.errors.length > 0) {
          this.lastError = r.errors.slice(0, 5).join("\n");
        } else {
          this.lastError = "";
        }
      });
      await listen<string>("mcp-sync-error", (e) => {
        this.running = false;
        this.lastError = e.payload;
        this.lastReport = t("sy.fail");
      });
    },
    async syncNow() {
      if (this.running) return;
      this.running = true;
      this.message = t("sy.connecting");
      this.lastError = "";
      try {
        await api.mcpSync();
      } catch (e) {
        this.running = false;
        this.lastError = String(e);
        this.lastReport = t("sy.fail");
      }
    },
  },
});
