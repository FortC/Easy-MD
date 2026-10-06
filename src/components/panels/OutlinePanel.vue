<template>
  <div class="outline-panel">
    <div class="emd-panel-header"><span>{{ t("ol.title") }}</span></div>
    <div class="ol-list">
      <div
        v-for="(h, i) in headings"
        :key="i"
        class="emd-tree-item"
        :style="{ paddingLeft: 6 + (h.level - 1) * 12 + 'px' }"
        @click="gotoLine(h.line)"
      >
        <span class="ol-text">{{ h.text || t("ol.emptyHeading") }}</span>
      </div>
      <div v-if="headings.length === 0" class="ol-empty">{{ t("ol.empty") }}</div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useEditorStore } from "../../stores/editor";
import { parseHeadings } from "../../lib/markdown/renderer";
import { t } from "../../i18n";

const editor = useEditorStore();
const headings = computed(() =>
  editor.activePath ? parseHeadings(editor.content) : [],
);

function gotoLine(line: number) {
  if (editor.mode === "preview") {
    // 阅读模式无源码行可跳，切换到分屏后跳
    editor.setMode("split");
    setTimeout(() => window.dispatchEvent(new CustomEvent("emd-goto-line", { detail: line })), 80);
  } else {
    window.dispatchEvent(new CustomEvent("emd-goto-line", { detail: line }));
  }
}
</script>

<style scoped>
.outline-panel {
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.ol-list {
  flex: 1;
  overflow-y: auto;
  padding: 2px 6px;
}
.ol-text {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: var(--font-ui-size);
}
.ol-empty {
  color: var(--text-faint);
  text-align: center;
  padding: 16px 8px;
}
</style>
