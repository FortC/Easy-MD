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
      <span class="sb-item">{{ modeLabel }}</span>
      <span class="sb-item sb-ver" :title="`EasyMD v${appVersion} ${BUILD_ID}`">
        v{{ appVersion }} · {{ BUILD_ID }}
      </span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import Icon from "./Icon.vue";
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
