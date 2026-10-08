import { defineStore } from "pinia";
import { api } from "../ipc/tauri";
import type { Backlink, NoteIndex, TagCount, VaultChangedPayload } from "../types";

/** refreshAssets 在途请求（模块级，避免 Promise 被 reactive 代理） */
let assetsLoading: Promise<void> | null = null;

/** 索引镜像：后端推送增量更新，这里只做缓存视图 */
export const useNotesIndexStore = defineStore("notesIndex", {
  state: () => ({
    notes: [] as NoteIndex[],
    tags: [] as TagCount[],
    /** vault 内全部资源文件（vault 相对路径），供图片按文件名全库解析 */
    assets: [] as string[],
    /** 资源表版本：刷新后 +1，驱动预览重渲染 */
    assetVersion: 0,
  }),
  getters: {
    noteCount: (s) => s.notes.length,
    /** 供 [[ 自动补全：标题 + 别名 */
    linkSuggestions: (s) => {
      const out: { label: string; path: string }[] = [];
      for (const n of s.notes) {
        out.push({ label: n.title, path: n.path });
        for (const a of n.aliases) out.push({ label: a, path: n.path });
      }
      return out;
    },
    byPath: (s) => {
      const m: Record<string, NoteIndex> = {};
      for (const n of s.notes) m[n.path] = n;
      return m;
    },
  },
  actions: {
    setNotes(notes: NoteIndex[]) {
      this.notes = notes;
      this.refreshTags();
    },
    applyVaultChanged(payload: VaultChangedPayload) {
      const map: Record<string, NoteIndex> = {};
      for (const n of this.notes) map[n.path] = n;
      for (const n of payload.updated) map[n.path] = n;
      for (const r of payload.removed) delete map[r];
      this.notes = Object.values(map).sort((a, b) => a.path.localeCompare(b.path));
      this.refreshTags();
      // 资源文件增量合并（图片粘贴/拖入/删除后立即可解析）
      if (payload.assets_added.length || payload.assets_removed.length) {
        const set = new Set(this.assets);
        for (const a of payload.assets_added) set.add(a);
        for (const r of payload.assets_removed) set.delete(r);
        this.assets = Array.from(set);
        this.assetVersion++;
      }
    },
    /** 全量加载 vault 资源文件列表（开库时调用；并发时复用在途请求） */
    async refreshAssets() {
      if (assetsLoading) return assetsLoading;
      const job = (async () => {
        try {
          const files = await api.listAllFilesWithMtime();
          this.assets = files
            .filter((f) => f.kind === "image" || f.kind === "other")
            .map((f) => f.path);
          this.assetVersion++;
        } catch {
          /* vault 未打开等情况静默 */
        } finally {
          assetsLoading = null;
        }
      })();
      assetsLoading = job;
      return job;
    },
    async refreshTags() {
      try {
        this.tags = await api.getTags();
      } catch {
        this.tags = [];
      }
    },
    async rebuild() {
      const notes = await api.rebuildIndex();
      this.notes = notes;
      this.refreshTags();
      return notes;
    },
    async backlinksOf(path: string): Promise<Backlink[]> {
      return api.getBacklinks(path);
    },
  },
});
