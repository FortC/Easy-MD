import { defineStore } from "pinia";
import { api } from "../ipc/tauri";
import type { EditMode } from "../types";

/** 当前打开的笔记与编辑模式 */
export const useEditorStore = defineStore("editor", {
  state: () => ({
    activePath: "" as string,
    content: "",
    /** 已保存到磁盘的内容（判断脏状态） */
    savedContent: "",
    mode: "split" as EditMode,
    saving: false,
    /** 每次打开笔记递增，供编辑器组件感知"切换文件" */
    openToken: 0,
    /** 保存后跳转目标：打开后定位到某行/锚点 */
    pendingJump: null as { line?: number; anchor?: string } | null,
    _saveTimer: null as ReturnType<typeof setTimeout> | null,
  }),
  getters: {
    isOpen: (s) => s.activePath !== "",
    isDirty: (s) => s.content !== s.savedContent,
    wordCount: (s) => {
      const text = s.content
        .replace(/[#*`~\[\]()!>-]/g, " ")
        .replace(/\s+/g, " ")
        .trim();
      if (!text) return 0;
      // 中英混排：CJK 字符按字计数，其它按词
      const cjk = (text.match(/[\u4e00-\u9fff\u3400-\u4dbf]/g) || []).length;
      const words = text
        .replace(/[\u4e00-\u9fff\u3400-\u4dbf]/g, " ")
        .split(/\s+/)
        .filter(Boolean).length;
      return cjk + words;
    },
  },
  actions: {
    async openNote(path: string, jump: { line?: number; anchor?: string } | null = null) {
      this.activePath = path;
      this.content = await api.readTextFile(path);
      this.savedContent = this.content;
      this.pendingJump = jump;
      this.openToken++;
    },
    setContent(c: string) {
      this.content = c;
      this.scheduleSave();
    },
    scheduleSave() {
      if (this._saveTimer) clearTimeout(this._saveTimer);
      this._saveTimer = setTimeout(() => this.save(), 400);
    },
    async save() {
      if (!this.activePath || this.content === this.savedContent) return;
      this.saving = true;
      try {
        await api.writeTextFile(this.activePath, this.content);
        this.savedContent = this.content;
      } finally {
        this.saving = false;
      }
    },
    cycleMode() {
      const order: EditMode[] = ["source", "split", "preview"];
      const next = order[(order.indexOf(this.mode) + 1) % order.length];
      this.setMode(next);
    },
    setMode(m: EditMode) {
      this.mode = m;
    },
    reset() {
      if (this._saveTimer) clearTimeout(this._saveTimer);
      this.activePath = "";
      this.content = "";
      this.savedContent = "";
      this.pendingJump = null;
      this.openToken++;
    },
    /** 笔记被外部（重命名/删除）后同步路径 */
    renameSelf(newPath: string) {
      if (this.activePath) this.activePath = newPath;
    },
  },
});
