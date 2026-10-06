import { defineStore } from "pinia";
import type { LeftTab, MainView, RightTab } from "../types";

export const useUiStore = defineStore("ui", {
  state: () => ({
    view: "editor" as MainView,
    leftVisible: true,
    leftTab: "notes" as LeftTab,
    rightVisible: true,
    rightTab: "outline" as RightTab,
    searchOpen: false,
    searchMode: "file" as "file" | "content",
    settingsOpen: false,
    /** 新建笔记对话框 */
    newNoteOpen: false,
    newNoteFolder: null as string | null,
    /** 当前打开的 canvas 文件（相对路径） */
    canvasPath: "",
    /** 当前打开的图谱："" = 全库引用图谱（自动），否则 .graph 相对路径 */
    graphPath: "",
  }),
  actions: {
    reset() {
      this.view = "editor";
      this.leftTab = "notes";
      this.rightTab = "outline";
      this.searchOpen = false;
      this.settingsOpen = false;
      this.newNoteOpen = false;
      this.canvasPath = "";
      this.graphPath = "";
    },
    openSearch(mode: "file" | "content") {
      this.searchMode = mode;
      this.searchOpen = true;
    },
    openNewNote(folder: string | null = null) {
      this.newNoteFolder = folder;
      this.newNoteOpen = true;
    },
  },
});
