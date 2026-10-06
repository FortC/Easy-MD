<template>
  <transition name="emd-fade">
    <div v-if="ui.searchOpen" class="emd-modal-bg" @click.self="close">
    <div class="emd-modal sp-modal">
      <div class="sp-tabs">
        <button
          class="sp-tab"
          :class="{ 'is-active': mode === 'file' }"
          @click="switchMode('file')"
        >
          {{ t("sp.file") }}
        </button>
        <button
          class="sp-tab"
          :class="{ 'is-active': mode === 'content' }"
          @click="switchMode('content')"
        >
          {{ t("sp.content") }}
        </button>
      </div>
      <input
        ref="inputRef"
        v-model="query"
        type="text"
        class="sp-input"
        :placeholder="mode === 'file' ? t('sp.filePh') : t('sp.contentPh')"
        @keydown.down.prevent="move(1)"
        @keydown.up.prevent="move(-1)"
        @keydown.enter="chooseSelected"
        @keydown.esc="close"
      />
      <div class="sp-results">
        <template v-if="mode === 'file'">
          <div
            v-for="(h, i) in fileHits"
            :key="h.path"
            class="sp-item"
            :class="{ 'is-selected': i === selected }"
            @mousemove="selected = i"
            @click="openFile(h.path)"
          >
            <Icon name="file-text" :size="14" />
            <span class="sp-title">{{ h.title }}</span>
            <span class="sp-sub">{{ h.path }}</span>
          </div>
        </template>
        <template v-else>
          <div
            v-for="(h, i) in contentHits"
            :key="h.path + h.line_no"
            class="sp-item sp-item-content"
            :class="{ 'is-selected': i === selected }"
            @mousemove="selected = i"
            @click="openContent(h)"
          >
            <div class="sp-line1">
              <Icon name="file-text" :size="14" />
              <span class="sp-title">{{ h.title }}</span>
              <span class="sp-ln">:{{ h.line_no }}</span>
            </div>
            <div class="sp-text">{{ h.text }}</div>
          </div>
        </template>
        <div v-if="searched && allHits.length === 0" class="sp-empty">
          {{ t("sp.none") }}
        </div>
        <div v-if="!searched" class="sp-empty">
          {{ mode === "file" ? t("sp.fileHint") : t("sp.contentHint") }}
        </div>
      </div>
    </div>
  </div>
  </transition>
</template>

<script setup lang="ts">
import { nextTick, onMounted, ref, watch } from "vue";
import Icon from "../common/Icon.vue";
import { useUiStore } from "../../stores/ui";
import { useEditorStore } from "../../stores/editor";
import { t } from "../../i18n";
import { api } from "../../ipc/tauri";

const ui = useUiStore();
const editor = useEditorStore();
const query = ref("");
const mode = ref<"file" | "content">("file");
const selected = ref(0);
const searched = ref(false);
const inputRef = ref<HTMLInputElement>();

const fileHits = ref<{ path: string; title: string; aliases: string[] }[]>([]);
const contentHits = ref<
  { path: string; title: string; line_no: number; text: string }[]
>([]);

const allHits = ref<{ path: string }[]>([]);

let timer: ReturnType<typeof setTimeout> | null = null;

watch(query, (q) => {
  if (timer) clearTimeout(timer);
  timer = setTimeout(() => doSearch(q), 150);
});

watch(mode, () => {
  query.value = "";
  searched.value = false;
  fileHits.value = [];
  contentHits.value = [];
});

async function doSearch(q: string) {
  selected.value = 0;
  if (!q.trim()) {
    searched.value = false;
    fileHits.value = [];
    contentHits.value = [];
    return;
  }
  if (mode.value === "file") {
    fileHits.value = await api.searchFiles(q);
    allHits.value = fileHits.value;
  } else {
    contentHits.value = await api.searchContent(q);
    allHits.value = contentHits.value;
  }
  searched.value = true;
}

function switchMode(m: "file" | "content") {
  mode.value = m;
  nextTick(() => inputRef.value?.focus());
}

function move(delta: number) {
  const n = allHits.value.length;
  if (!n) return;
  selected.value = (selected.value + delta + n) % n;
}

async function chooseSelected() {
  if (mode.value === "file") {
    const h = fileHits.value[selected.value];
    if (h) await openFile(h.path);
  } else {
    const h = contentHits.value[selected.value];
    if (h) await openContent(h);
  }
}

async function openFile(path: string) {
  close();
  await editor.openNote(path);
}

async function openContent(h: { path: string; line_no: number }) {
  close();
  await editor.openNote(h.path, { line: h.line_no - 1 });
}

function close() {
  ui.searchOpen = false;
}

onMounted(() => {
  mode.value = ui.searchMode;
  nextTick(() => inputRef.value?.focus());
});
</script>

<style scoped>
.sp-modal {
  width: 620px;
  margin-top: -15vh;
}
.sp-tabs {
  display: flex;
  gap: 4px;
  padding: 10px 12px 0;
}
.sp-tab {
  padding: 5px 12px;
  border-radius: var(--radius-s);
  color: var(--text-muted);
}
.sp-tab:hover {
  background: var(--background-modifier-hover);
}
.sp-tab.is-active {
  background: var(--background-modifier-active-hover);
  color: var(--text-normal);
}
.sp-input {
  margin: 10px 12px;
  width: calc(100% - 24px);
}
.sp-results {
  flex: 1;
  overflow-y: auto;
  padding: 4px 8px 10px;
  min-height: 120px;
  max-height: 50vh;
}
.sp-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 7px 10px;
  border-radius: var(--radius-s);
  cursor: pointer;
}
.sp-item.is-selected {
  background: var(--background-modifier-active-hover);
}
.sp-item-content {
  flex-direction: column;
  align-items: stretch;
  gap: 3px;
}
.sp-line1 {
  display: flex;
  align-items: center;
  gap: 8px;
}
.sp-title {
  color: var(--text-normal);
}
.sp-sub,
.sp-ln {
  color: var(--text-faint);
  font-size: var(--font-ui-smaller);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.sp-sub {
  flex: 1;
}
.sp-text {
  color: var(--text-muted);
  font-size: var(--font-ui-smaller);
  padding-left: 22px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.sp-empty {
  color: var(--text-faint);
  text-align: center;
  padding: 24px 8px;
}
</style>
