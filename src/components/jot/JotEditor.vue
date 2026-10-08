<template>
  <div class="jot-editor">
    <div class="jot-header">
      <Icon name="pencil" :size="13" class="jot-icon" />
      <span class="jot-title">{{ title }}</span>
      <span v-if="editor.isDirty" class="jot-dirty" />
      <div class="jot-spacer" />
      <button class="jot-btn" :title="t('ed.tag')" @pointerdown.stop @click="tagOpen = !tagOpen">
        <Icon name="tag" :size="12" />
        {{ t("ed.tagTxt") }}
      </button>
      <button class="jot-btn" :title="t('c.close')" @click="closeJot">
        <Icon name="x" :size="12" />
        {{ t("c.close") }}
      </button>
    </div>
    <!-- 快速加标签 -->
    <DropdownMenu :open="tagOpen" :anchor="tagBtn" align="right" :title="t('ed.tagTitle')" @close="tagOpen = false">
      <div class="jot-tag-box" @pointerdown.stop>
        <input
          v-model="tagInput"
          type="text"
          class="jot-tag-input"
          :placeholder="t('ed.tagPh')"
          @keydown.enter.prevent="addTag"
          @keydown.esc="tagOpen = false"
        />
        <div v-if="suggestions.length" class="jot-tag-sug">
          <button v-for="tg in suggestions" :key="tg" class="jot-tag-chip" @click="quickAdd(tg)">#{{ tg }}</button>
        </div>
      </div>
    </DropdownMenu>

    <!-- 纯文本编辑区 -->
    <textarea
      ref="textareaRef"
      v-model="content"
      class="jot-textarea"
      :placeholder="t('jot.placeholder')"
      @input="onInput"
      @paste="onPaste"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from "vue";
import Icon from "../common/Icon.vue";
import DropdownMenu from "../common/DropdownMenu.vue";
import { useEditorStore } from "../../stores/editor";
import { useUiStore } from "../../stores/ui";
import { useNotesIndexStore } from "../../stores/notesIndex";
import { api } from "../../ipc/tauri";
import { addTagToContent } from "../../lib/tags";
import { t } from "../../i18n";

const editor = useEditorStore();
const ui = useUiStore();
const indexStore = useNotesIndexStore();
const textareaRef = ref<HTMLTextAreaElement>();
const content = ref("");
let saveTimer: ReturnType<typeof setTimeout> | null = null;
let syncing = false;

// 标签
const tagOpen = ref(false);
const tagBtn = ref<HTMLElement>();
const tagInput = ref("");
const suggestions = computed(() => indexStore.tags.slice(0, 10).map((x) => x.tag));

const title = computed(() =>
  editor.activePath
    ? editor.activePath.split("/").pop()?.replace(/\.md$/i, "") || t("rb.jot")
    : t("rb.jot"),
);

onMounted(() => {
  content.value = editor.content;
  nextTick(() => textareaRef.value?.focus());
});

watch(
  () => editor.openToken,
  () => { content.value = editor.content; },
);

function onInput() {
  if (syncing) return;
  syncing = true;
  editor.setContent(content.value);
  syncing = false;
}

watch(
  () => editor.content,
  (c) => {
    if (!syncing && c !== content.value) content.value = c;
  },
);

/** 粘贴图片 → 存 assets → 插入 ![](<路径>)（<> 包裹以兼容带空格文件名） */
async function onPaste(e: ClipboardEvent) {
  const items = e.clipboardData?.items;
  if (!items) return;
  for (const item of Array.from(items)) {
    if (item.type.startsWith("image/")) {
      e.preventDefault();
      const file = item.getAsFile();
      if (!file) continue;
      const ext = (file.name.split(".").pop() || "png").toLowerCase();
      try {
        const buf = await file.arrayBuffer();
        const rel = await api.saveImage(Array.from(new Uint8Array(buf)), ext);
        insertAtCursor(`![](<${rel}>)\n`);
      } catch { /* 忽略 */ }
      return;
    }
  }
}

function insertAtCursor(text: string) {
  const ta = textareaRef.value;
  if (!ta) return;
  const start = ta.selectionStart;
  const end = ta.selectionEnd;
  content.value = content.value.slice(0, start) + text + content.value.slice(end);
  onInput();
  nextTick(() => {
    ta.selectionStart = ta.selectionEnd = start + text.length;
    ta.focus();
  });
}

function addTag() {
  const tg = tagInput.value.trim().replace(/^#/, "");
  if (!tg) return;
  quickAdd(tg);
}
function quickAdd(tg: string) {
  editor.setContent(addTagToContent(editor.content, tg));
  tagOpen.value = false;
  tagInput.value = "";
}

function closeJot() {
  if (saveTimer) clearTimeout(saveTimer);
  editor.save();
  ui.view = "editor";
}
</script>

<style scoped>
.jot-editor {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--background-primary);
  overflow: hidden;
}
.jot-header {
  height: 36px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 10px 0 16px;
  background: var(--background-secondary);
  border-bottom: 1px solid var(--background-modifier-border);
}
.jot-icon { color: var(--text-accent); }
.jot-title {
  font-weight: 600;
  font-size: 14px;
  color: var(--text-normal);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.jot-dirty {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--interactive-accent);
  flex-shrink: 0;
}
.jot-spacer { flex: 1; }
.jot-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  height: 26px;
  padding: 0 8px;
  border-radius: var(--radius-s);
  color: var(--text-muted);
  font-size: 12px;
  transition: background var(--anim-fast);
}
.jot-btn:hover { background: var(--background-modifier-hover); color: var(--text-normal); }
.jot-textarea {
  flex: 1;
  resize: none;
  border: none;
  outline: none;
  background: var(--background-primary);
  color: var(--text-normal);
  font-family: var(--font-text);
  font-size: 16px;
  line-height: 1.8;
  padding: 16px 20px;
  overflow-y: auto;
}
.jot-textarea::placeholder { color: var(--text-faint); }
.jot-tag-box { padding: 6px; width: 240px; }
.jot-tag-input { width: 100%; }
.jot-tag-sug {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  margin-top: 8px;
}
.jot-tag-chip {
  background: var(--tag-background);
  color: var(--tag-color);
  border-radius: var(--radius-m);
  padding: 2px 8px;
  font-size: var(--font-ui-smaller);
}
.jot-tag-chip:hover { background: var(--tag-background-hover); }
</style>
