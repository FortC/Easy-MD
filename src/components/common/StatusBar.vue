<template>
  <div class="statusbar">
    <div class="sb-left">
      <span v-if="sync.running" class="sb-item sb-sync">
        <Icon name="refresh-cw" :size="12" /> {{ sync.message || t("ai.thinking") }}
        <template v-if="sync.total > 0">（{{ sync.done }}/{{ sync.total }}）</template>
      </span>
      <span v-else-if="sync.lastError" class="sb-item sb-sync-err" :title="sync.lastError">
        <Icon name="alert-triangle" :size="12" /> {{ t("sb.syncFail") }}
      </span>
      <span v-if="editor.saving" class="sb-item">{{ t("sb.saving") }}</span>
      <span v-else-if="editor.isDirty" class="sb-item">{{ t("sb.unsaved") }}</span>
    </div>
    <div class="sb-right">
      <span v-if="backlinkCount > 0" class="sb-item">
        <Icon name="link" :size="12" /> {{ tf("sb.backlinks", { n: backlinkCount }) }}
      </span>
      <span class="sb-item">{{ tf("sb.words", { n: editor.wordCount }) }}</span>
      <button
        v-if="fileEncoding"
        ref="encBtn"
        class="sb-item sb-enc"
        title="文件编码，点击转换"
        @click="encOpen = !encOpen"
      >
        {{ fileEncoding }}
      </button>
      <DropdownMenu
        :open="encOpen"
        :anchor="encBtn"
        align="right"
        :items="encItems"
        @select="onEncSelect"
        @close="encOpen = false"
      />
      <span class="sb-item">{{ modeLabel }}</span>
      <span class="sb-item sb-ver" :title="`EasyMD v${appVersion} ${BUILD_ID}`">
        v{{ appVersion }} · {{ BUILD_ID }}
      </span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import Icon from "./Icon.vue";
import DropdownMenu, { type DropItem } from "./DropdownMenu.vue";
import { useEditorStore } from "../../stores/editor";
import { useNotesIndexStore } from "../../stores/notesIndex";
import { useSyncStore } from "../../stores/sync";
import { api } from "../../ipc/tauri";
import { BUILD_ID, APP_VERSION } from "../../build";
import { t, tf } from "../../i18n";

const editor = useEditorStore();
const indexStore = useNotesIndexStore();
const sync = useSyncStore();
const backlinkCount = ref(0);
// 运行时读 tauri.conf.json 的版本号（build.ts 的静态常量仅作兜底）
const appVersion = ref(APP_VERSION);

// ---- 文件编码：检测显示 + 一键转换 ----
const fileEncoding = ref("");
const encOpen = ref(false);
const encBtn = ref<HTMLElement>();
const encItems = computed<DropItem[]>(() => [
  { key: "to-utf8", label: t("sb.encToUtf8") },
  { key: "to-gb", label: t("sb.encToGb") },
]);

async function refreshEncoding() {
  if (!editor.activePath) {
    fileEncoding.value = "";
    return;
  }
  try {
    fileEncoding.value = await api.detectFileEncoding(editor.activePath);
  } catch {
    fileEncoding.value = "";
  }
}

watch(() => editor.activePath, refreshEncoding);

async function onEncSelect(key: string) {
  encOpen.value = false;
  if (!editor.activePath) return;
  const toGb = key === "to-gb";
  const target = toGb ? "GB18030" : "UTF-8";
  if (fileEncoding.value === target) return;
  if (!confirm(t(toGb ? "sb.encGbConfirm" : "sb.encUtf8Confirm"))) return;
  try {
    // 写入当前编辑器内容（未保存的改动一并落盘），再重新加载同步状态
    await api.writeTextFileAs(editor.activePath, editor.content, toGb ? "gb18030" : "utf-8");
    await editor.openNote(editor.activePath);
    await refreshEncoding();
  } catch (e) {
    alert(`${t("sb.encFail")}: ${e}`);
  }
}

const modeLabel = computed(
  () =>
    ({ source: t("sb.source"), preview: t("sb.preview"), split: t("sb.split") })[
      editor.mode
    ] || "",
);

onMounted(async () => {
  try {
    appVersion.value = await getVersion();
  } catch {
    /* 保持 build.ts 兜底值 */
  }
  // 反链数跟随当前笔记变化刷新
  const refresh = async () => {
    if (!editor.activePath) {
      backlinkCount.value = 0;
      return;
    }
    try {
      backlinkCount.value = (await indexStore.backlinksOf(editor.activePath)).length;
    } catch {
      backlinkCount.value = 0;
    }
  };
  void api; // 预留
  let lastPath = "";
  setInterval(() => {
    if (editor.activePath !== lastPath) {
      lastPath = editor.activePath;
      refresh();
    }
  }, 500);
  indexStore.$subscribe(() => refresh());
});
</script>

<style scoped>
.statusbar {
  height: var(--statusbar-height);
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: var(--background-secondary);
  border-top: 1px solid var(--background-modifier-border);
  padding: 0 10px;
  font-size: var(--font-ui-smaller);
  color: var(--text-muted);
  flex-shrink: 0;
  user-select: none;
}
.sb-left,
.sb-right {
  display: flex;
  gap: 12px;
  align-items: center;
}
.sb-enc {
  cursor: pointer;
  border-radius: var(--radius-s);
  padding: 1px 6px;
  transition: background var(--anim-fast), color var(--anim-fast);
}
.sb-enc:hover {
  background: var(--background-modifier-hover);
  color: var(--text-normal);
}
.sb-item {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}
.sb-sync {
  color: var(--text-accent);
}
.sb-sync-err {
  color: var(--text-error);
  cursor: help;
}
.sb-ver {
  color: var(--text-faint);
  font-size: 10px;
  user-select: text;
}
</style>
