import { defineStore } from "pinia";
import { api } from "../ipc/tauri";
import type { FsEntry, NoteIndex, OpenVaultResult, VaultEntry } from "../types";
import { useNotesIndexStore } from "./notesIndex";
import { useEditorStore } from "./editor";
import { useUiStore } from "./ui";
import { t } from "../i18n";

export const useVaultStore = defineStore("vault", {  state: () => ({
    opened: false,
    root: "",
    vaultList: [] as VaultEntry[],
    /** 目录缓存：rel path -> children（文件树按需加载） */
    dirCache: {} as Record<string, FsEntry[]>,
    /** 已展开的目录 */
    expanded: {} as Record<string, boolean>,
  }),
  actions: {
    async loadVaultList() {
      this.vaultList = await api.listVaults();
    },
    /** 后端已打开 vault（如右键文件打开）时，前端同步状态 */
    applyOpened(root: string, notes: NoteIndex[]) {
      this.root = root;
      this.opened = true;
      useNotesIndexStore().setNotes(notes);
      this.dirCache = {};
      this.expanded = {};
      this.loadDir("");
      this.loadVaultList();
    },
    async open(path: string) {
      const res: OpenVaultResult = await api.openVault(path);
      this.root = res.root;
      this.opened = true;
      const notes = useNotesIndexStore();
      notes.setNotes(res.notes);
      this.dirCache = {};
      this.expanded = {};
      await this.loadDir("");
      await this.loadVaultList();
    },
    async create(path: string) {
      const res = await api.createVault(path);
      this.root = res.root;
      this.opened = true;
      useNotesIndexStore().setNotes(res.notes as NoteIndex[]);
      this.dirCache = {};
      this.expanded = {};
      await this.loadDir("");
      await this.loadVaultList();
    },
    async close() {
      await api.closeVault();
      this.opened = false;
      this.root = "";
      this.dirCache = {};
      this.expanded = {};
      useEditorStore().reset();
      useUiStore().reset();
    },
    async forget(path: string) {
      await api.forgetVault(path);
      await this.loadVaultList();
    },
    /** 加载某目录一层（带缓存） */
    async loadDir(rel: string): Promise<FsEntry[]> {
      if (this.dirCache[rel]) return this.dirCache[rel];
      const children = await api.listDir(rel || null);
      this.dirCache = { ...this.dirCache, [rel]: children };
      return children;
    },
    async refreshDir(rel: string) {
      const children = await api.listDir(rel || null);
      this.dirCache = { ...this.dirCache, [rel]: children };
    },
    toggleExpand(rel: string) {
      this.expanded = { ...this.expanded, [rel]: !this.expanded[rel] };
      if (this.expanded[rel]) this.loadDir(rel);
    },
    /** 新建笔记（支持模板内容），并打开 */
    async newNote(folder: string | null, title: string, content = "") {
      const rel = await api.createNote(folder, title, content || "");
      await this.refreshParents(rel);
      await useEditorStore().openNote(rel);
      return rel;
    },
    async refreshParents(rel: string) {
      // 刷新该文件所在目录及祖先链的缓存
      const parts = rel.split("/");
      for (let i = parts.length - 1; i >= 0; i--) {
        const dir = parts.slice(0, i).join("/");
        if (this.dirCache[dir] !== undefined) {
          await this.refreshDir(dir);
        }
      }
    },
    /** 新建画布（.canvas JSON 文件）并打开画布视图 */
    async newCanvas(folder: string | null, name = t("cv.newCanvasName")) {
      const prefix = folder ? `${folder}/` : "";
      let rel = `${prefix}${name}.canvas`;
      let i = 2;
      while (await api.pathExists(rel)) {
        rel = `${prefix}${name} ${i++}.canvas`;
      }
      await api.writeTextFile(rel, '{"nodes":[],"edges":[]}');
      await this.refreshParents(rel);
      const ui = useUiStore();
      ui.canvasPath = rel;
      ui.view = "canvas";
      return rel;
    },
  },
});
