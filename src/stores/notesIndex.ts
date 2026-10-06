import { defineStore } from "pinia";
import { api } from "../ipc/tauri";
import type { Backlink, NoteIndex, TagCount, VaultChangedPayload } from "../types";

/** 索引镜像：后端推送增量更新，这里只做缓存视图 */
export const useNotesIndexStore = defineStore("notesIndex", {
  state: () => ({
    notes: [] as NoteIndex[],
    tags: [] as TagCount[],
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
