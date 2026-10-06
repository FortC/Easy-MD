import { defineStore } from "pinia";
import { api } from "../ipc/tauri";
import { useEditorStore } from "./editor";
import { t, tf, locale } from "../i18n";

/** AI 面板状态：选中提问 / 整篇总结，结果可插入或替换选区 */
export const useAiStore = defineStore("ai", {
  state: () => ({
    open: false,
    /** 选中的上下文（空 = 整篇笔记） */
    selection: "",
    prompt: "",
    result: "",
    running: false,
    error: "",
  }),
  getters: {
    contextLabel: (s) =>
      s.selection ? tf("ai.selected", { n: s.selection.length }) : t("ai.whole"),
  },
  actions: {
    openWith(selection = "") {
      this.selection = selection;
      this.prompt = "";
      this.result = "";
      this.error = "";
      this.open = true;
    },
    close() {
      this.open = false;
    },
    async run(kind: "ask" | "explain" | "summarize" | "translate" | "polish" | "continue") {
      const editor = useEditorStore();
      const content = this.selection || editor.content;
      if (!content.trim()) {
        this.error = t("ai.noContent");
        return;
      }
      const ctx = this.selection
        ? content
        : content.slice(0, 30000);
      const sys = locale.value === "en"
        ? "You are the AI assistant inside EasyMD, a note-taking app. Answer in English unless the note content is in another language. Output plain text or Markdown only."
        : "你是 EasyMD 笔记软件里的 AI 助手。回答使用简体中文（除非笔记内容是其它语言），输出纯文本或 Markdown，不要输出无关内容。";
      const q = this.selection
        ? `${t("ai.pSelCtx")}\n\n<<<\n${ctx}\n>>>\n\n`
        : `${t("ai.pNoteCtx")}\n\n<<<\n${ctx}\n>>>\n\n`;
      const map: Record<typeof kind, string> = {
        ask: `${q}${t("ai.pAsk")}${this.prompt || t("ai.pAskDef")}`,
        explain: `${q}${t("ai.pExplain")}`,
        summarize: `${q}${t("ai.pSum")}`,
        translate: `${q}${t("ai.pTr")}`,
        polish: `${q}${t("ai.pPolish")}`,
        continue: `${q}${t("ai.pCont")}`,
      };
      this.running = true;
      this.error = "";
      this.result = "";
      try {
        this.result = await api.aiChat(map[kind], sys);
      } catch (e) {
        this.error = String(e);
      } finally {
        this.running = false;
      }
    },
    /** 结果插入到光标处 */
    insertResult() {
      if (!this.result) return;
      window.dispatchEvent(new CustomEvent("emd-insert-text", { detail: "\n" + this.result + "\n" }));
      this.close();
    },
    /** 结果替换选中文本 */
    replaceSelection() {
      if (!this.result) return;
      window.dispatchEvent(new CustomEvent("emd-replace-selection", { detail: this.result }));
      this.close();
    },
    async copyResult() {
      if (!this.result) return;
      try {
        await navigator.clipboard.writeText(this.result);
      } catch {
        /* 剪贴板不可用时忽略 */
      }
    },
  },
});
