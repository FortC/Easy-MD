<template>
  <div ref="wrapEl" class="se-wrap">
    <div ref="host" class="cm-host" />
    <!-- 选中文本时浮动的 AI 入口 -->
    <button
      v-show="aiBtn.visible"
      class="se-ai-btn"
      :style="{ left: aiBtn.x + 'px', top: aiBtn.y + 'px' }"
      :title="t('se.askAi')"
      @mousedown.prevent
      @click="askAi"
    >
      AI
    </button>
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, reactive, ref, watch } from "vue";
import { EditorState, RangeSetBuilder } from "@codemirror/state";
import {
  EditorView,
  Decoration,
  type DecorationSet,
  ViewPlugin,
  keymap,
} from "@codemirror/view";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { markdown } from "@codemirror/lang-markdown";
import { autocompletion, type CompletionContext, type CompletionResult } from "@codemirror/autocomplete";
import { syntaxHighlighting, defaultHighlightStyle } from "@codemirror/language";
import { api } from "../../ipc/tauri";
import { useEditorStore } from "../../stores/editor";
import { useNotesIndexStore } from "../../stores/notesIndex";
import { useAiStore } from "../../stores/ai";
import { t, tf } from "../../i18n";

const host = ref<HTMLElement>();
const wrapEl = ref<HTMLElement>();
const editor = useEditorStore();
const indexStore = useNotesIndexStore();
const aiStore = useAiStore();
let view: EditorView | null = null;
let syncing = false; // 区分"编辑器输入"与"外部写入"

// ---------- 选中浮动 AI 按钮 ----------
const aiBtn = reactive({ visible: false, x: 0, y: 0, text: "" });

function updateAiButton() {
  const v = view;
  if (!v || !wrapEl.value) return;
  const sel = v.state.selection.main;
  const text = v.state.sliceDoc(sel.from, sel.to);
  if (sel.empty || !text.trim()) {
    aiBtn.visible = false;
    return;
  }
  aiBtn.text = text;
  const coords = v.coordsAtPos(sel.head);
  const rect = wrapEl.value.getBoundingClientRect();
  if (!coords) {
    aiBtn.visible = false;
    return;
  }
  aiBtn.x = Math.min(Math.max(coords.left - rect.left, 8), (rect.width || 400) - 60);
  aiBtn.y = Math.max(coords.top - rect.top - 32, 6);
  aiBtn.visible = true;
}

function askAi() {
  aiBtn.visible = false;
  aiStore.openWith(aiBtn.text);
}

// ---------- 格式动作（由头部工具栏通过事件触发） ----------
function runAction(key: string) {
  const v = view;
  if (!v) return;
  const map: Record<string, () => void> = {
    h1: () => toggleHeading(v, 1),
    h2: () => toggleHeading(v, 2),
    h3: () => toggleHeading(v, 3),
    bold: () => wrapSel(v, "**"),
    italic: () => wrapSel(v, "*"),
    strike: () => wrapSel(v, "~~"),
    icode: () => wrapSel(v, "`"),
    mark: () => wrapSel(v, "==", "==", t("se.phMark")),
    link: () => insertLink(v, false),
    wikilink: () => wrapSel(v, "[[", "]]", t("se.phNote")),
    image: () => insertLink(v, true),
    ul: () => toggleLinePrefix(v, "- "),
    ol: () => toggleLinePrefix(v, "1. "),
    task: () => toggleLinePrefix(v, "- [ ] "),
    quote: () => toggleLinePrefix(v, "> "),
    table: () => insertBlock(v, t("se.table")),
    hr: () => insertBlock(v, "---"),
    taskdone: () => toggleTaskDone(v),
    indent: () => indentLines(v, 1),
    outdent: () => indentLines(v, -1),
    clearfmt: () => clearFormat(v),
  };
  map[key]?.();
  v.focus();
}

function onFormatEvent(e: Event) {
  runAction((e as CustomEvent<string>).detail);
}

function onInsertText(e: Event) {
  const v = view;
  if (!v) return;
  const text = (e as CustomEvent<string>).detail;
  const sel = v.state.selection.main;
  v.dispatch({
    changes: { from: sel.from, to: sel.to, insert: text },
    selection: { anchor: sel.from + text.length },
    effects: EditorView.scrollIntoView(sel.from + text.length),
  });
  v.focus();
}

/** AI 结果替换当前选区 */
function onReplaceSelection(e: Event) {
  const v = view;
  if (!v) return;
  const text = (e as CustomEvent<string>).detail;
  const sel = v.state.selection.main;
  const from = sel.empty ? sel.from : sel.from;
  const to = sel.empty ? sel.from : sel.to;
  v.dispatch({
    changes: { from, to, insert: text },
    selection: { anchor: from + text.length },
    effects: EditorView.scrollIntoView(from + text.length),
  });
  v.focus();
}

/** 任务行勾选切换：- [ ] ↔ - [x] */
function toggleTaskDone(v: EditorView) {
  const { state } = v;
  const sel = state.selection.main;
  const first = state.doc.lineAt(sel.from).number;
  const last = state.doc.lineAt(sel.to).number;
  const changes: { from: number; to?: number; insert: string }[] = [];
  const re = /^(\s*(?:[-*+]|\d+\.)\s+\[)( |x|X)(\])/;
  for (let n = first; n <= last; n++) {
    const line = state.doc.line(n);
    const m = re.exec(line.text);
    if (m) {
      const from = line.from + m[1].length;
      changes.push({ from, to: from + 1, insert: m[2] === " " ? "x" : " " });
    }
  }
  if (changes.length === 0) {
    // 没有任务标记则把当前行变成已勾选任务
    changes.push({ from: state.doc.line(first).from, insert: "- [x] " });
  }
  v.dispatch({ changes });
}

/** 缩进 / 减少 缩进（2 空格） */
function indentLines(v: EditorView, dir: 1 | -1) {
  const { state } = v;
  const sel = state.selection.main;
  const first = state.doc.lineAt(sel.from).number;
  const last = state.doc.lineAt(sel.to).number;
  const changes: { from: number; to?: number; insert: string }[] = [];
  for (let n = first; n <= last; n++) {
    const line = state.doc.line(n);
    if (dir === 1) {
      changes.push({ from: line.from, insert: "  " });
    } else if (line.text.startsWith("  ")) {
      changes.push({ from: line.from, to: line.from + 2, insert: "" });
    } else if (line.text.startsWith(" ") || line.text.startsWith("\t")) {
      changes.push({ from: line.from, to: line.from + 1, insert: "" });
    }
  }
  v.dispatch({ changes });
}

/** 清除选区内常见 markdown 标记 */
function clearFormat(v: EditorView) {
  const { state } = v;
  const sel = state.selection.main;
  let text = state.sliceDoc(sel.from, sel.to);
  text = text
    .replace(/\*\*([^*]+)\*\*/g, "$1")
    .replace(/\*([^*]+)\*/g, "$1")
    .replace(/~~([^~]+)~~/g, "$1")
    .replace(/==([^=]+)==/g, "$1")
    .replace(/`([^`]+)`/g, "$1")
    .replace(/^#{1,6}\s+/gm, "")
    .replace(/^>\s?/gm, "")
    .replace(/^\s*[-*+]\s+(\[[ xX]\]\s+)?/gm, "")
    .replace(/^\s*\d+\.\s+/gm, "");
  v.dispatch({
    changes: { from: sel.from, to: sel.to, insert: text },
    selection: { anchor: sel.from + text.length },
  });
}

/** 选区包裹/解包（再次点击取消） */
function wrapSel(v: EditorView, before: string, after = before, placeholder = t("se.phText")) {
  const { state } = v;
  const sel = state.selection.main;
  const selected = state.sliceDoc(sel.from, sel.to);
  const hasEmpty = selected.length === 0;
  const text = hasEmpty ? placeholder : selected;

  const beforeCtx = state.sliceDoc(Math.max(0, sel.from - before.length), sel.from);
  const afterCtx = state.sliceDoc(sel.to, Math.min(state.doc.length, sel.to + after.length));
  if (beforeCtx === before && afterCtx === after && !hasEmpty) {
    // 解包
    v.dispatch({
      changes: [
        { from: sel.from - before.length, to: sel.from, insert: "" },
        { from: sel.to, to: sel.to + after.length, insert: "" },
      ],
      selection: { anchor: sel.from - before.length },
    });
    return;
  }
  v.dispatch({
    changes: { from: sel.from, to: sel.to, insert: before + text + after },
    selection: {
      anchor: sel.from + before.length,
      head: sel.from + before.length + text.length,
    },
  });
}

/** 链接/图片：插入后选中占位文字，直接输入替换 */
function insertLink(v: EditorView, isImage: boolean) {
  const { state } = v;
  const sel = state.selection.main;
  const selected = state.sliceDoc(sel.from, sel.to);
  const alt = selected || (isImage ? t("se.phAlt") : t("se.phLink"));
  const url = isImage ? t("se.phImg") : "https://";
  const insert = `${isImage ? "!" : ""}[${alt}](${url})`;
  v.dispatch({
    changes: { from: sel.from, to: sel.to, insert },
    selection: { anchor: sel.from + (isImage ? 2 : 1), head: sel.from + (isImage ? 2 : 1) + alt.length },
  });
}

/** 行前缀切换：全部已带则去掉，否则加上 */
function toggleLinePrefix(v: EditorView, prefix: string) {
  const { state } = v;
  const sel = state.selection.main;
  const first = state.doc.lineAt(sel.from).number;
  const last = state.doc.lineAt(sel.to).number;
  const changes: { from: number; to?: number; insert: string }[] = [];
  let allHave = true;
  for (let n = first; n <= last; n++) {
    if (!state.doc.line(n).text.startsWith(prefix)) allHave = false;
  }
  for (let n = first; n <= last; n++) {
    const line = state.doc.line(n);
    if (allHave) {
      changes.push({ from: line.from, to: line.from + prefix.length, insert: "" });
    } else if (!line.text.startsWith(prefix)) {
      changes.push({ from: line.from, insert: prefix });
    }
  }
  v.dispatch({ changes });
}

/** 标题切换：其它级别替换为该级别，相同则取消 */
function toggleHeading(v: EditorView, level: number) {
  const prefix = "#".repeat(level) + " ";
  const { state } = v;
  const sel = state.selection.main;
  const first = state.doc.lineAt(sel.from).number;
  const last = state.doc.lineAt(sel.to).number;
  const changes: { from: number; to?: number; insert: string }[] = [];
  const re = /^#{1,6} /;
  for (let n = first; n <= last; n++) {
    const line = state.doc.line(n);
    const m = re.exec(line.text);
    if (m && m[0] === prefix) {
      changes.push({ from: line.from, to: line.from + prefix.length, insert: "" });
    } else if (m) {
      changes.push({ from: line.from, to: line.from + m[0].length, insert: prefix });
    } else {
      changes.push({ from: line.from, insert: prefix });
    }
  }
  v.dispatch({ changes });
}

/** 块级插入（表格/分隔线）：保证前后空行 */
function insertBlock(v: EditorView, text: string) {
  const { state } = v;
  const sel = state.selection.main;
  const atLineStart = state.doc.lineAt(sel.from).from === sel.from;
  const prefix = atLineStart ? "" : "\n\n";
  v.dispatch({
    changes: { from: sel.from, to: sel.to, insert: prefix + text + "\n" },
    selection: { anchor: sel.from + prefix.length + text.length + 1 },
  });
}

// ---- [[ ]] / #tag / ==高亮== 高亮 ----
const reMark = /!?\[\[[^\[\]\n]+\]\]|#[\p{L}\p{N}_][\p{L}\p{N}_/-]*|==[^=\n]+==/gu;

function buildDeco(v: EditorView): DecorationSet {
  const b = new RangeSetBuilder<Decoration>();
  for (const { from, to } of v.visibleRanges) {
    const text = v.state.doc.sliceString(from, to);
    let m: RegExpExecArray | null;
    reMark.lastIndex = 0;
    while ((m = reMark.exec(text))) {
      const s = from + m.index;
      const t = m[0];
      let deco: Decoration;
      if (t.startsWith("![[")) deco = Decoration.mark({ class: "cm-embed-mark" });
      else if (t.startsWith("[[")) deco = Decoration.mark({ class: "cm-wikilink" });
      else if (t.startsWith("==")) deco = Decoration.mark({ class: "cm-mark" });
      else deco = Decoration.mark({ class: "cm-mdtag" });
      b.add(s, s + t.length, deco);
    }
  }
  return b.finish();
}

const markPlugin = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;
    constructor(v: EditorView) {
      this.decorations = buildDeco(v);
    }
    update(u: { docChanged: boolean; viewportChanged: boolean; view: EditorView }) {
      if (u.docChanged || u.viewportChanged) this.decorations = buildDeco(u.view);
    }
  },
  { decorations: (v) => v.decorations },
);

// ---- 补全：[[ 笔记名 / # 标签 ----
function noteCompletion(ctx: CompletionContext) {
  const before = ctx.state.doc.sliceString(Math.max(0, ctx.pos - 2), ctx.pos);
  if (before !== "[[") return null;
  return {
    from: ctx.pos,
    options: indexStore.linkSuggestions.map((s) => ({
      label: s.label,
      apply: s.label + "]]",
      detail: s.path,
      type: "text",
    })),
    validFor: /^[^\[\]]*$/,
  };
}

function tagCompletion(ctx: CompletionContext) {
  const word = ctx.matchBefore(/#[\p{L}\p{N}_/-]+/u);
  if (!word) return null;
  if (word.from === word.to && !ctx.explicit) return null;
  return {
    from: word.from,
    options: indexStore.tags.map((t) => ({
      label: "#" + t.tag,
      detail: tf("se.nNotes", { n: t.count }),
      type: "text",
    })),
    validFor: /^#[\p{L}\p{N}_/-]*$/u,
  };
}

// ---- 粘贴图片 → 附件目录 ----
const pasteHandler = EditorView.domEventHandlers({
  paste(event, v) {
    const items = event.clipboardData?.items;
    if (!items) return false;
    for (const item of Array.from(items)) {
      if (item.type.startsWith("image/")) {
        const file = item.getAsFile();
        if (!file) continue;
        event.preventDefault();
        const ext = (file.name.split(".").pop() || "png").toLowerCase();
        file.arrayBuffer().then(async (buf) => {
          const rel = await api.saveImage(Array.from(new Uint8Array(buf)), ext);
          v.dispatch({
            changes: {
              from: v.state.selection.main.head,
              // <> 包裹：文件名带空格（Pasted image ….png）时裸目标不是合法链接语法
              insert: `![](<${rel}>)`,
            },
          });
        });
        return true;
      }
    }
    return false;
  },
});

function makeState(): EditorState {
  return EditorState.create({
    doc: editor.content,
    extensions: [
      history(),
      keymap.of([
        // Ctrl+B 加粗 / Ctrl+I 斜体（Obsidian 习惯）
        { key: "Mod-b", run: () => (view ? (wrapSel(view, "**"), true) : false) },
        { key: "Mod-i", run: () => (view ? (wrapSel(view, "*"), true) : false) },
        ...defaultKeymap,
        ...historyKeymap,
      ]),
      markdown(),
      syntaxHighlighting(defaultHighlightStyle),
      markPlugin,
      autocompletion({
        override: [
          (ctx: CompletionContext): CompletionResult | null =>
            noteCompletion(ctx) ?? tagCompletion(ctx),
        ],
      }),
      pasteHandler,
      EditorView.lineWrapping,
      EditorView.theme({
        "&": { height: "100%", fontSize: "var(--font-text-size)" },
        ".cm-scroller": {
          fontFamily: "var(--font-text)",
          lineHeight: "1.6",
          padding: "12px 8px",
        },
        ".cm-content": { caretColor: "var(--interactive-accent)" },
        "&.cm-focused": { outline: "none" },
      }),
      EditorView.updateListener.of((u) => {
        if (u.docChanged && !syncing) {
          syncing = true;
          editor.setContent(u.state.doc.toString());
          syncing = false;
        }
        if (u.selectionSet || u.docChanged) updateAiButton();
      }),
    ],
  });
}

/** 定位到某行（0 基）并滚动到可视区顶部 */
function gotoLine(line: number) {
  if (!view) return;
  const l = view.state.doc.line(Math.min(Math.max(1, line + 1), view.state.doc.lines));
  view.dispatch({
    selection: { anchor: l.from },
    effects: EditorView.scrollIntoView(l.from, { y: "start" }),
  });
  view.focus();
}

onMounted(() => {
  if (!host.value) return;
  view = new EditorView({ parent: host.value, state: makeState() });

  // 容器尺寸变化（拖窗口/拖分屏分隔条）后强制重新测量，避免留白
  const ro = new ResizeObserver(() => view?.requestMeasure());
  ro.observe(host.value);
  onUnmounted(() => ro.disconnect());

  // 大纲/搜索跳转到某行
  const onGoto = (e: Event) => gotoLine((e as CustomEvent<number>).detail);
  window.addEventListener("emd-goto-line", onGoto);
  window.addEventListener("emd-format", onFormatEvent);
  window.addEventListener("emd-insert-text", onInsertText);
  window.addEventListener("emd-replace-selection", onReplaceSelection);
  onUnmounted(() => {
    window.removeEventListener("emd-goto-line", onGoto);
    window.removeEventListener("emd-format", onFormatEvent);
    window.removeEventListener("emd-insert-text", onInsertText);
    window.removeEventListener("emd-replace-selection", onReplaceSelection);
  });
});

onUnmounted(() => {
  view?.destroy();
  view = null;
});

// 切换文件 → 重建文档
watch(
  () => editor.openToken,
  () => {
    view?.setState(makeState());
    // 反链等行号跳转：纯源码模式没有预览组件兜底，这里直接定位；
    // 有锚点时留给预览组件处理
    const jump = editor.pendingJump;
    if (jump?.line !== undefined) {
      editor.pendingJump = jump.anchor ? { anchor: jump.anchor } : null;
      gotoLine(jump.line);
    }
  },
);

// 外部写入（属性面板 / 模板插入）→ 同步到编辑器
watch(
  () => editor.content,
  (c) => {
    if (!view || syncing) return;
    if (view.state.doc.toString() !== c) {
      view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: c } });
    }
  },
);
</script>

<style scoped>
.se-wrap {
  position: relative;
  height: 100%;
  width: 100%;
  min-height: 0;
}
.se-ai-btn {
  position: absolute;
  z-index: 30;
  height: 22px;
  padding: 0 9px;
  border-radius: var(--radius-m);
  background: var(--interactive-accent);
  color: var(--text-on-accent);
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.03em;
  box-shadow: var(--shadow-l1);
}
.se-ai-btn:hover {
  background: var(--interactive-accent-hover);
}
</style>

<style>
.cm-host {
  height: 100%;
  width: 100%;
  min-height: 0;
  overflow: hidden;
}
.cm-host .cm-editor {
  height: 100%;
}
.cm-host .cm-scroller {
  overflow-y: auto;
  background: var(--background-primary);
  color: var(--text-normal);
}
.cm-host .cm-gutters {
  display: none;
}
.cm-host .cm-activeLine {
  background: var(--background-primary-alt);
}
/* wikilink / embed / tag 着色（Obsidian 风格） */
.cm-wikilink {
  color: var(--link-color);
}
.cm-embed-mark {
  color: var(--text-accent);
}
.cm-mdtag {
  color: var(--tag-color);
  background: var(--tag-background);
  border-radius: var(--radius-s);
  padding: 0 2px;
}
.cm-mark {
  background: rgba(255, 208, 0, 0.28);
  border-radius: var(--radius-s);
  padding: 0 2px;
}
.cm-host .cm-selectionBackground,
.cm-host .cm-content ::selection {
  background: var(--selection-background) !important;
}
.cm-host .cm-tooltip-autocomplete > ul {
  font-family: var(--font-ui);
  max-height: 240px;
}
</style>
