<template>
  <div class="editor-pane">
    <!-- 头部：标题 + 全量格式工具 + 操作 -->
    <div class="ep-header">
      <div class="ep-title" :title="editor.activePath" @click="startRename">
        <Icon :name="editor.isDirty ? 'pencil' : 'file-text'" :size="13" class="ep-title-icon" />
        <input
          v-if="renaming"
          ref="renameInput"
          v-model="renameVal"
          class="ep-rename-input"
          @keydown.enter="confirmRename"
          @keydown.esc="cancelRename"
          @blur="confirmRename"
          @click.stop
        />
        <span v-else class="ep-title-text">{{ title }}</span>
        <span v-if="editor.isDirty && !renaming" class="ep-dirty-dot" :title="t('ed.unsaved')" />
        <Icon v-if="!renaming" name="pencil" :size="11" class="ep-rename-hint" :title="t('ed.renameHint')" />
      </div>

      <!-- 格式工具：全量内联，窄窗口内部横向滚动 -->
      <div v-if="editor.mode !== 'preview'" class="ep-format">
        <template v-for="b in formatBar" :key="b.key">
          <div v-if="b.key.startsWith('d:')" class="ep-f-sep" />
          <button v-else class="ep-f-btn" :title="b.title" @mousedown.prevent @click="runFormat(b.key)">
            {{ b.text }}
          </button>
        </template>
      </div>
      <div v-else class="ep-flex-spacer" />

      <div class="ep-actions">
        <!-- 引用笔记：搜索并插入 [[ ]] 双链（与引用块 > 区分） -->
        <button
          ref="citeBtn"
          class="ep-txt-btn ep-cite-btn"
          :class="{ 'is-open': citeOpen }"
          :title="t('ed.cite')"
          @pointerdown.stop
          @click="citeOpen = !citeOpen"
        >
          <Icon name="link" :size="12" />
          {{ t("ed.citeTxt") }}
        </button>
        <button
          ref="tplBtn"
          class="ep-txt-btn"
          :title="t('ed.tpl')"
          @pointerdown.stop
          @click="tplOpen = !tplOpen"
        >
          <Icon name="copy" :size="12" />
          {{ t("ed.tplTxt") }}
        </button>
        <button
          ref="tagBtn"
          class="ep-txt-btn"
          :title="t('ed.tag')"
          @pointerdown.stop
          @click="tagOpen = !tagOpen"
        >
          <Icon name="tag" :size="12" />
          {{ t("ed.tagTxt") }}
        </button>

        <div class="ep-seg">
          <button
            v-for="m in modes"
            :key="m.key"
            class="ep-seg-btn"
            :class="{ 'is-active': editor.mode === m.key }"
            :title="m.title"
            @click="editor.setMode(m.key)"
          >
            {{ m.label }}
          </button>
        </div>
        <button
          ref="moreBtn"
          class="ep-txt-btn"
          :class="{ 'is-open': menuOpen }"
          :title="t('ed.moreOps')"
          @pointerdown.stop
          @click="menuOpen = !menuOpen"
        >
          <Icon name="more-horizontal" :size="12" />
          {{ t("ed.moreTxt") }}
        </button>
      </div>

      <!-- 引用笔记下拉：搜索 + 列表，选中插入 [[笔记名]] -->
      <DropdownMenu :open="citeOpen" :anchor="citeBtn" align="right" :title="t('ed.cite')" @close="citeOpen = false">
        <div class="ep-cite-box" @pointerdown.stop>
          <input
            ref="citeInput"
            v-model="citeQuery"
            type="text"
            class="ep-cite-input"
            :placeholder="t('ed.citePh')"
            @keydown.down.prevent="moveCite(1)"
            @keydown.up.prevent="moveCite(-1)"
            @keydown.enter.prevent="insertCiteSelected"
            @keydown.esc="citeOpen = false"
          />
          <div class="ep-cite-list">
            <button
              v-for="(s, i) in citeHits"
              :key="s.path + '|' + s.label"
              class="ep-dd-slot-item ep-cite-item"
              :class="{ 'is-selected': i === citeSelected }"
              @mousemove="citeSelected = i"
              @click="insertCite(s)"
            >
              <Icon name="file-text" :size="14" />
              <span class="ep-cite-label">{{ s.label }}</span>
              <span class="ep-cite-path">{{ s.path }}</span>
            </button>
            <div v-if="citeHits.length === 0" class="ep-dd-tip">{{ t("ed.citeEmpty") }}</div>
          </div>
        </div>
      </DropdownMenu>

      <!-- 套用模板下拉 -->
      <DropdownMenu :open="tplOpen" :anchor="tplBtn" align="right" :title="t('ed.tplTitle')" @close="tplOpen = false">
        <div v-if="templates.length === 0" class="ep-dd-tip">
          {{ tf("ed.tplEmpty", { dir: settings.data.templates_dir || "templates" }) }}
        </div>
        <button
          v-for="tp in templates"
          :key="tp.path"
          class="ep-dd-slot-item"
          @click="applyTemplate(tp.path)"
        >
          <Icon name="file-text" :size="14" />
          <span>{{ tp.name }}</span>
        </button>
      </DropdownMenu>

      <!-- 快速加标签下拉 -->
      <DropdownMenu :open="tagOpen" :anchor="tagBtn" align="right" :title="t('ed.tagTitle')" @close="tagOpen = false">
        <div class="ep-tag-box" @pointerdown.stop>
          <input
            v-model="tagInput"
            type="text"
            class="ep-tag-input"
            :placeholder="t('ed.tagPh')"
            @keydown.enter.prevent="addTag"
            @keydown.esc="tagOpen = false"
          />
          <div v-if="indexStore.tags.length" class="ep-tag-sug">
            <button v-for="tg in tagSuggestions" :key="tg.tag" class="ep-tag-chip" @click="quickAddTag(tg.tag)">
              #{{ tg.tag }}
            </button>
          </div>
        </div>
      </DropdownMenu>

      <!-- 更多操作下拉 -->
      <DropdownMenu
        :open="menuOpen"
        :anchor="moreBtn"
        :items="menuItems"
        align="right"
        @select="onMenuSelect"
        @close="menuOpen = false"
      />
    </div>

    <!-- 主体 -->
    <div ref="bodyEl" class="ep-body" :class="`ep-${editor.mode}`">
      <div
        v-if="editor.mode !== 'preview'"
        class="ep-source"
        :style="editor.mode === 'split' ? { width: splitRatio + '%', flex: '0 0 auto' } : {}"
      >
        <SourceEditor v-if="editor.isOpen" />
        <div v-else class="ep-empty">
          <Icon name="file-text" :size="40" />
          <p>{{ t("ed.emptyHint") }}</p>
          <button class="emd-btn emd-btn-accent" @click="ui.openNewNote()">
            <Icon name="file-plus" :size="13" /> {{ t("ed.newNote") }}
          </button>
        </div>
      </div>
      <div
        v-if="editor.mode === 'split'"
        class="ep-splitter"
        :title="t('ed.splitter')"
        @pointerdown.stop="startSplitDrag"
        @dblclick="splitRatio = 50"
      />
      <div v-if="editor.mode !== 'source'" class="ep-preview">
        <PreviewView v-if="editor.isOpen" @jump="onJump" />
        <div v-else class="ep-empty"><Icon name="book" :size="40" /></div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import Icon from "../common/Icon.vue";
import DropdownMenu, { type DropItem } from "../common/DropdownMenu.vue";
import SourceEditor from "./SourceEditor.vue";
import PreviewView from "./PreviewView.vue";
import { useEditorStore } from "../../stores/editor";
import { useUiStore } from "../../stores/ui";
import { useSettingsStore } from "../../stores/settings";
import { useVaultStore } from "../../stores/vault";
import { useNotesIndexStore } from "../../stores/notesIndex";
import { api } from "../../ipc/tauri";
import { exportHtml, exportMarkdown, exportPdf } from "../../lib/export";
import { addTagToContent } from "../../lib/tags";
import { applyTemplateVars, listTemplates } from "../../lib/template";
import { useAiStore } from "../../stores/ai";
import { t, tf } from "../../i18n";

const editor = useEditorStore();
const ui = useUiStore();
const settings = useSettingsStore();
const vault = useVaultStore();
const indexStore = useNotesIndexStore();
const aiStore = useAiStore();
const menuOpen = ref(false);
const moreBtn = ref<HTMLElement>();
const bodyEl = ref<HTMLElement>();
const splitRatio = ref(50);

// ---- 模板下拉 ----
const tplOpen = ref(false);
const tplBtn = ref<HTMLElement>();
const templates = ref<{ name: string; path: string }[]>([]);
watch(tplOpen, async (v) => {
  if (v) templates.value = await listTemplates();
});
async function applyTemplate(path: string) {
  tplOpen.value = false;
  if (!editor.isOpen) return;
  try {
    const raw = await api.readTextFile(path);
    const text = applyTemplateVars(raw, title.value);
    window.dispatchEvent(new CustomEvent("emd-insert-text", { detail: text }));
  } catch (e) {
    alert(`${t("ed.readTplFail")}: ${e}`);
  }
}

// ---- 引用笔记（搜索插入 [[ ]] 双链） ----
const citeOpen = ref(false);
const citeBtn = ref<HTMLElement>();
const citeInput = ref<HTMLInputElement>();
const citeQuery = ref("");
const citeSelected = ref(0);
const citeHits = computed(() => {
  const q = citeQuery.value.trim().toLowerCase();
  const all = indexStore.linkSuggestions;
  const hits = q
    ? all.filter((s) => s.label.toLowerCase().includes(q) || s.path.toLowerCase().includes(q))
    : all;
  return hits.slice(0, 50);
});
watch(citeOpen, (v) => {
  if (v) {
    citeQuery.value = "";
    citeSelected.value = 0;
    nextTick(() => citeInput.value?.focus());
  }
});
function moveCite(d: number) {
  const n = citeHits.value.length;
  if (n === 0) return;
  citeSelected.value = (citeSelected.value + d + n) % n;
}
function insertCiteSelected() {
  const hit = citeHits.value[citeSelected.value];
  if (hit) insertCite(hit);
}
function insertCite(hit: { label: string }) {
  citeOpen.value = false;
  if (!editor.isOpen) return;
  window.dispatchEvent(new CustomEvent("emd-insert-text", { detail: `[[${hit.label}]]` }));
}

// ---- 快速加标签 ----
const tagOpen = ref(false);
const tagBtn = ref<HTMLElement>();
const tagInput = ref("");
const tagSuggestions = computed(() => indexStore.tags.slice(0, 12));
watch(tagOpen, (v) => {
  if (v) nextTick(() => {
    tagInput.value = "";
  });
});
function addTag() {
  const tg = tagInput.value.trim().replace(/^#/, "");
  if (!tg) return;
  quickAddTag(tg);
}
function quickAddTag(tag: string) {
  if (!editor.isOpen) return;
  editor.setContent(addTagToContent(editor.content, tag));
  tagOpen.value = false;
}

// ---- 标题重命名 ----
const renaming = ref(false);
const renameVal = ref("");
const renameInput = ref<HTMLInputElement>();
function startRename() {
  if (!editor.isOpen || renaming.value) return;
  renameVal.value = title.value;
  renaming.value = true;
  nextTick(() => {
    renameInput.value?.focus();
    renameInput.value?.select();
  });
}
function cancelRename() {
  renaming.value = false;
}
async function confirmRename() {
  if (!renaming.value) return;
  const name = renameVal.value.trim();
  renaming.value = false;
  if (!name || name === title.value) return;
  try {
    const newPath = await api.renamePath(editor.activePath, name);
    editor.renameSelf(newPath);
    await vault.refreshParents(newPath);
    await indexStore.rebuild();
  } catch (e) {
    alert(`${t("ed.renameFail")}: ${e}`);
  }
}

const title = computed(() =>
  editor.activePath
    ? editor.activePath.split("/").pop()?.replace(/\.md$/i, "") || t("ed.noteWord")
    : t("ed.notOpen"),
);

const modes = computed(() => [
  { key: "source" as const, label: t("ed.src"), title: t("ed.mSource") },
  { key: "split" as const, label: t("ed.split"), title: t("ed.mSplit") },
  { key: "preview" as const, label: t("ed.read"), title: t("ed.mPreview") },
]);

// ---------- 格式工具栏（全量内联，窄窗口横向滚动） ----------
const formatBar = computed(() => [
  { key: "h1", text: "H1", title: t("ed.fH1") },
  { key: "h2", text: "H2", title: t("ed.fH2") },
  { key: "h3", text: "H3", title: t("ed.fH3") },
  { key: "d:1", text: "", title: "" },
  { key: "bold", text: t("ed.fBold"), title: "Ctrl+B" },
  { key: "italic", text: t("ed.fItalic"), title: "Ctrl+I" },
  { key: "mark", text: t("ed.fMark"), title: "==" },
  { key: "icode", text: t("ed.fCode"), title: "`" },
  { key: "strike", text: t("ed.fStrike"), title: "~~" },
  { key: "d:2", text: "", title: "" },
  { key: "link", text: t("ed.fLink"), title: "" },
  { key: "wikilink", text: t("ed.fWiki"), title: "[[ ]]" },
  { key: "image", text: t("ed.fImage"), title: "" },
  { key: "d:3", text: "", title: "" },
  { key: "ul", text: t("ed.fList"), title: "" },
  { key: "ol", text: t("ed.fOl"), title: "" },
  { key: "task", text: t("ed.fTask"), title: "" },
  { key: "taskdone", text: t("ed.fTaskDone"), title: "" },
  { key: "quote", text: t("ed.fQuote"), title: "" },
  { key: "d:4", text: "", title: "" },
  { key: "indent", text: t("ed.fIndent"), title: "" },
  { key: "outdent", text: t("ed.fOutdent"), title: "" },
  { key: "table", text: t("ed.fTable"), title: "" },
  { key: "hr", text: t("ed.fHr"), title: "" },
  { key: "clearfmt", text: t("ed.fClear"), title: "" },
]);

function runFormat(key: string) {
  window.dispatchEvent(new CustomEvent("emd-format", { detail: key }));
}

const menuItems = computed<DropItem[]>(() => [
  { key: "aisum", label: t("ed.aiSum"), icon: "play" },
  { key: "aiask", label: t("ed.aiAsk"), icon: "play" },
  { key: "sep-ai", label: "", separator: true },
  { key: "md", label: t("ed.expMd"), icon: "download", hint: t("ed.keepWiki") },
  { key: "html", label: t("ed.expHtml"), icon: "download", hint: t("ed.singleFile") },
  { key: "pdf", label: t("ed.expPdf"), icon: "download", hint: t("ed.printHint") },
  { key: "sep-view", label: "", separator: true },
  { key: "graph", label: t("ed.openGraph"), icon: "share-2", hint: "Ctrl+G" },
  { key: "search", label: t("ed.searchAll"), icon: "search", hint: "Ctrl+Shift+F" },
]);

async function onMenuSelect(key: string) {
  menuOpen.value = false;
  if (key === "aisum") {
    if (!editor.isOpen) return;
    aiStore.openWith("");
    void aiStore.run("summarize");
    return;
  }
  if (key === "aiask") {
    aiStore.openWith("");
    return;
  }
  if (["md", "html", "pdf"].includes(key)) {
    if (editor.isDirty) await editor.save();
    try {
      if (key === "md") await exportMarkdown();
      else if (key === "html") await exportHtml();
      else await exportPdf();
    } catch (e) {
      alert(`${t("ed.exportFail")}: ${e}`);
    }
    return;
  }
  if (key === "graph") {
    if (editor.isDirty) editor.save();
    ui.view = "graph";
  } else if (key === "search") {
    ui.openSearch("content");
  }
}

/** 分屏分隔条拖拽 */
function startSplitDrag(e: PointerEvent) {
  e.preventDefault();
  const body = bodyEl.value;
  if (!body) return;
  const rect = body.getBoundingClientRect();
  const onMove = (ev: PointerEvent) => {
    const pct = ((ev.clientX - rect.left) / rect.width) * 100;
    splitRatio.value = Math.min(80, Math.max(20, pct));
  };
  const onUp = () => {
    window.removeEventListener("pointermove", onMove);
    window.removeEventListener("pointerup", onUp);
    document.body.classList.remove("is-col-resizing");
  };
  document.body.classList.add("is-col-resizing");
  window.addEventListener("pointermove", onMove);
  window.addEventListener("pointerup", onUp);
}

async function onJump(target: { path?: string; anchor?: string; line?: number }) {
  if (!target.path) return;
  await editor.openNote(target.path, {
    anchor: target.anchor,
    line: target.line,
  });
}

// ---- 图片粘贴：容器层统一处理（capture，任何模式/焦点位置都生效） ----
// 不放在 CodeMirror 里：保存图片的 await 期间组件可能因切模式/切笔记被销毁，
// 旧实现 dispatch 到已销毁的 view 会静默丢失插入内容（图片入库但语法没进笔记）。
function onBodyPaste(ev: ClipboardEvent) {
  if (!editor.isOpen) return;
  const items = ev.clipboardData?.items;
  if (!items) return;
  for (const item of Array.from(items)) {
    if (!item.type.startsWith("image/")) continue;
    const file = item.getAsFile();
    if (!file) continue;
    ev.preventDefault();
    ev.stopPropagation();
    const ext = (file.name.split(".").pop() || "png").toLowerCase();
    file.arrayBuffer()
      .then(async (buf) => {
        const rel = await api.saveImage(Array.from(new Uint8Array(buf)), ext);
        // 阅读模式下看不到源码，切到分屏让用户立即看到图片
        if (editor.mode === "preview") editor.setMode("split");
        // <> 包裹：文件名带空格（Pasted image ….png）时裸目标不是合法链接语法
        editor.insertText(`![](<${rel}>)`);
      })
      .catch((e) => console.error("粘贴图片失败：", e));
    return;
  }
}

onMounted(() => {
  bodyEl.value?.addEventListener("paste", onBodyPaste, true);
});
onUnmounted(() => {
  bodyEl.value?.removeEventListener("paste", onBodyPaste, true);
});
</script>

<style scoped>
.editor-pane {
  height: 100%;
  width: 100%;
  flex: 1 1 auto;
  min-width: 0;
  display: flex;
  flex-direction: column;
  background: var(--background-primary);
  overflow: hidden;
}

/* ---------- 头部（单行） ---------- */
.ep-header {
  height: 36px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 8px 0 14px;
  background: var(--background-secondary);
  border-bottom: 1px solid var(--background-modifier-border);
  overflow: hidden;
  flex-wrap: nowrap;
}
.ep-title {
  display: flex;
  align-items: center;
  gap: 7px;
  flex: 0 1 auto;
  min-width: 0;
  overflow: hidden;
  color: var(--text-muted);
  cursor: text;
  border-radius: var(--radius-s);
  padding: 3px 4px;
  margin-left: -4px;
  transition: background var(--anim-fast);
  white-space: nowrap;
}
.ep-title:hover {
  background: var(--background-modifier-hover);
}
.ep-rename-hint {
  opacity: 0;
  color: var(--text-faint);
  flex-shrink: 0;
}
.ep-title:hover .ep-rename-hint {
  opacity: 0.9;
}
.ep-rename-input {
  width: 120px;
  max-width: 100%;
  height: 22px;
  background: var(--background-primary);
  border: 1px solid var(--interactive-accent);
  border-radius: var(--radius-s);
  padding: 0 6px;
  color: var(--text-normal);
  font-weight: 600;
}
.ep-title-icon {
  color: var(--text-faint);
  flex-shrink: 0;
}
.ep-title-text {
  font-weight: 600;
  color: var(--text-normal);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.ep-dirty-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--interactive-accent);
  flex-shrink: 0;
}
.ep-flex-spacer {
  flex: 1 1 auto;
  min-width: 10px;
}

/* 格式按钮组：弹性中段，占满可用空间；窄窗口内部横向滚动 */
.ep-format {
  flex: 1 1 0;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 2px;
  flex-wrap: nowrap;
  padding: 0 4px 0 2px;
  border-left: 1px solid var(--background-modifier-border);
  margin-left: 6px;
  overflow-x: auto;
  overflow-y: hidden;
  scrollbar-width: none;
}
.ep-format::-webkit-scrollbar {
  display: none;
}
.ep-f-btn {
  height: 24px;
  padding: 0 7px;
  flex-shrink: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-s);
  color: var(--text-muted);
  font-size: 12px;
  white-space: nowrap;
  transition: background var(--anim-fast), color var(--anim-fast);
}
.ep-f-btn:hover {
  background: var(--background-modifier-hover);
  color: var(--text-normal);
}
.ep-f-sep {
  width: 1px;
  height: 14px;
  margin: 0 4px;
  background: var(--background-modifier-border);
  flex-shrink: 0;
}

/* 右侧操作按钮：图标+文字 */
.ep-actions {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-shrink: 0;
  padding-left: 4px;
}
.ep-txt-btn {
  height: 26px;
  padding: 0 8px;
  display: inline-flex;
  align-items: center;
  gap: 5px;
  border-radius: var(--radius-s);
  color: var(--text-muted);
  font-size: 12px;
  white-space: nowrap;
  transition: background var(--anim-fast), color var(--anim-fast);
}
.ep-txt-btn:hover,
.ep-txt-btn.is-open {
  background: var(--background-modifier-hover);
  color: var(--text-normal);
}
.ep-seg {
  display: flex;
  gap: 2px;
  background: var(--background-primary);
  border: 1px solid var(--background-modifier-border);
  border-radius: var(--radius-m);
  padding: 2px;
}
.ep-seg-btn {
  height: 22px;
  padding: 0 9px;
  display: inline-flex;
  align-items: center;
  border-radius: var(--radius-s);
  color: var(--text-muted);
  font-size: 12px;
  white-space: nowrap;
  transition: background var(--anim-fast), color var(--anim-fast);
}
.ep-seg-btn:hover {
  color: var(--text-normal);
}
.ep-seg-btn.is-active {
  background: var(--background-modifier-active-hover);
  color: var(--text-normal);
}

/* 下拉内嵌内容 */
.ep-dd-tip {
  color: var(--text-faint);
  font-size: var(--font-ui-smaller);
  padding: 8px 10px;
  max-width: 240px;
  line-height: 1.5;
}
.ep-dd-slot-item {
  display: flex;
  align-items: center;
  gap: 9px;
  width: 100%;
  padding: 6px 10px;
  border-radius: var(--radius-s);
  color: var(--text-normal);
  font-size: var(--font-ui-size);
  text-align: left;
}
.ep-dd-slot-item:hover {
  background: var(--background-modifier-hover);
}
.ep-tag-box {
  padding: 6px;
  width: 240px;
}
.ep-tag-input {
  width: 100%;
}
.ep-tag-sug {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  margin-top: 8px;
}
.ep-tag-chip {
  background: var(--tag-background);
  color: var(--tag-color);
  border-radius: var(--radius-m);
  padding: 2px 8px;
  font-size: var(--font-ui-smaller);
}
.ep-tag-chip:hover {
  background: var(--tag-background-hover);
}

/* 引用笔记下拉 */
.ep-cite-box {
  padding: 6px;
  width: 300px;
}
.ep-cite-input {
  width: 100%;
}
.ep-cite-list {
  margin-top: 4px;
  max-height: 280px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 1px;
}
.ep-cite-item.is-selected {
  background: var(--background-modifier-hover);
}
.ep-cite-label {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  text-align: left;
}
.ep-cite-path {
  flex-shrink: 0;
  max-width: 96px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text-faint);
  font-size: var(--font-ui-smaller);
}
.ep-cite-btn {
  color: var(--text-normal);
}
.ep-cite-btn:hover {
  color: var(--text-normal);
}

/* ---------- 主体 ---------- */
.ep-body {
  flex: 1 1 0;
  width: 100%;
  min-height: 0;
  display: flex;
}
.ep-source,
.ep-preview {
  flex: 1 1 0;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}
.ep-splitter {
  width: 7px;
  margin: 0 -3px;
  flex: 0 0 auto;
  cursor: col-resize;
  position: relative;
  z-index: 5;
}
.ep-splitter::after {
  content: "";
  position: absolute;
  left: 3px;
  top: 0;
  bottom: 0;
  width: 1px;
  background: var(--background-modifier-border);
  transition: background var(--anim-fast), width var(--anim-fast);
}
.ep-splitter:hover::after {
  background: var(--interactive-accent);
  width: 2px;
}
.ep-empty {
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 14px;
  color: var(--text-faint);
}
</style>
