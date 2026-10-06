<template>
  <div class="workspace">
    <TitleBar />
    <div class="ws-middle">
      <Ribbon />

      <!-- 左侧栏（图谱/日历为全屏独立视图，不带左侧栏） -->
      <transition name="emd-side-l">
        <div v-if="ui.leftVisible && !fullView" class="ws-left" :style="{ width: leftW + 'px' }">
          <transition name="emd-pane" mode="out-in">
            <FileExplorer v-if="ui.leftTab === 'notes'" />
            <JotPanel v-else-if="ui.leftTab === 'jots'" />
            <TagPanel v-else-if="ui.leftTab === 'tags'" />
          </transition>
          <div class="ws-resizer" @mousedown="startDrag('left', $event)" />
        </div>
      </transition>

      <!-- 主区域 -->
      <div class="ws-main">
        <transition name="emd-view" mode="out-in">
          <div :key="ui.view" class="ws-main-inner">
            <EditorPane v-if="ui.view === 'editor'" />
            <GraphView v-else-if="ui.view === 'graph'" />
            <CanvasEditor v-else-if="ui.view === 'canvas'" />
            <CalendarView v-else-if="ui.view === 'calendar'" />
            <JotEditor v-else-if="ui.view === 'jot'" />
          </div>
        </transition>

        <!-- 右侧栏收起后的展开手柄 -->
        <button
          v-if="!ui.rightVisible"
          class="ws-right-expand"
          :title="t('ws.expand')"
          @click="ui.rightVisible = true"
        >
          <Icon name="chevron-right" :size="13" />
          <span class="ws-expand-text">{{ t("ws.panel") }}</span>
        </button>
      </div>

      <!-- 右侧栏（可折叠） -->
      <transition name="emd-side-r">
        <div v-if="ui.rightVisible && ui.view === 'editor'" class="ws-right" :style="{ width: rightW + 'px' }">
          <div class="ws-right-tabs">
            <button
              v-for="t in rightTabs"
              :key="t.key"
              class="rt-btn"
              :class="{ 'is-active': ui.rightTab === t.key }"
              @click="ui.rightTab = t.key"
            >
              {{ t.label }}
            </button>
            <button class="rt-collapse" :title="t('ws.collapse')" @click="ui.rightVisible = false">
              <Icon name="x" :size="13" />
            </button>
          </div>
          <div class="ws-right-body">
            <transition name="emd-pane" mode="out-in">
              <OutlinePanel v-if="ui.rightTab === 'outline'" />
              <BacklinksPanel v-else-if="ui.rightTab === 'backlinks'" />
              <PropertiesPanel v-else-if="ui.rightTab === 'props'" />
            </transition>
          </div>
          <div class="ws-resizer ws-resizer-left" @mousedown="startDrag('right', $event)" />
        </div>
      </transition>
    </div>
    <StatusBar />

    <SearchPalette />
    <SettingsDialog />
    <NewNoteDialog />
    <AiDialog />
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import TitleBar from "../components/common/TitleBar.vue";
import Ribbon from "../components/common/Ribbon.vue";
import StatusBar from "../components/common/StatusBar.vue";
import NewNoteDialog from "../components/common/NewNoteDialog.vue";
import Icon from "../components/common/Icon.vue";
import AiDialog from "../components/ai/AiDialog.vue";
import FileExplorer from "../components/sidebar/FileExplorer.vue";
import TagPanel from "../components/sidebar/TagPanel.vue";
import EditorPane from "../components/editor/EditorPane.vue";
import OutlinePanel from "../components/panels/OutlinePanel.vue";
import BacklinksPanel from "../components/panels/BacklinksPanel.vue";
import PropertiesPanel from "../components/panels/PropertiesPanel.vue";
import SearchPalette from "../components/search/SearchPalette.vue";
import SettingsDialog from "../components/settings/SettingsDialog.vue";
import GraphView from "../components/graph/GraphView.vue";
import CanvasEditor from "../components/canvas/CanvasEditor.vue";
import CalendarView from "../components/calendar/CalendarView.vue";
import JotPanel from "../components/sidebar/JotPanel.vue";
import JotEditor from "../components/jot/JotEditor.vue";
import { useUiStore } from "../stores/ui";
import { t } from "../i18n";

const ui = useUiStore();
const leftW = ref(268);
const rightW = ref(300);

/** 图谱/日历：全屏独立视图，不显示左侧栏 */
const fullView = computed(() => ui.view === "graph" || ui.view === "calendar");

// computed：切换语言时标签同步更新
const rightTabs = computed(() => [
  { key: "outline" as const, label: t("ws.outline") },
  { key: "backlinks" as const, label: t("ws.backlinks") },
  { key: "props" as const, label: t("ws.props") },
]);

function startDrag(which: "left" | "right", ev: MouseEvent) {
  ev.preventDefault();
  const startX = ev.clientX;
  const startW = which === "left" ? leftW.value : rightW.value;
  const onMove = (e: MouseEvent) => {
    if (which === "left") {
      leftW.value = Math.min(460, Math.max(220, startW + e.clientX - startX));
    } else {
      rightW.value = Math.min(520, Math.max(220, startW - (e.clientX - startX)));
    }
  };
  const onUp = () => {
    window.removeEventListener("mousemove", onMove);
    window.removeEventListener("mouseup", onUp);
    document.body.classList.remove("is-col-resizing");
  };
  document.body.classList.add("is-col-resizing");
  window.addEventListener("mousemove", onMove);
  window.addEventListener("mouseup", onUp);
}
</script>

<style scoped>
.workspace {
  height: 100%;
  width: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: var(--background-primary);
}
.ws-middle {
  flex: 1;
  display: flex;
  min-height: 0;
  width: 100%;
}
.ws-left {
  position: relative;
  background: var(--background-secondary);
  border-right: 1px solid var(--background-modifier-border);
  flex-shrink: 0;
  display: flex;
  min-width: 0;
}
.ws-left-placeholder {
  flex: 1;
  overflow: hidden;
}
.ws-main {
  flex: 1 1 auto;
  min-width: 0;
  min-height: 0;
  display: flex;
  position: relative;
  width: 100%;
}
.ws-main-inner {
  flex: 1 1 auto;
  min-width: 0;
  min-height: 0;
  display: flex;
  width: 100%;
}
/* 主区子视图一律撑满并允许收缩 */
.ws-main-inner > * {
  flex: 1 1 auto;
  min-width: 0;
  width: 100%;
}
.ws-right {
  position: relative;
  background: var(--background-secondary);
  border-left: 1px solid var(--background-modifier-border);
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  min-width: 0;
}
.ws-right-tabs {
  display: flex;
  align-items: center;
  border-bottom: 1px solid var(--background-modifier-border);
  padding: 0 4px;
  flex-shrink: 0;
}
.rt-btn {
  padding: 8px 12px;
  color: var(--text-muted);
  font-size: var(--font-ui-size);
}
.rt-btn:hover {
  color: var(--text-normal);
}
.rt-btn.is-active {
  color: var(--text-normal);
  box-shadow: inset 0 -2px 0 var(--interactive-accent);
}
.rt-collapse {
  margin-left: auto;
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-s);
  color: var(--text-faint);
}
.rt-collapse:hover {
  background: var(--background-modifier-hover);
  color: var(--text-normal);
}
.ws-right-body {
  flex: 1;
  min-height: 0;
  overflow: hidden;
}
.ws-resizer {
  position: absolute;
  top: 0;
  right: -3px;
  width: 6px;
  height: 100%;
  cursor: col-resize;
  z-index: 20;
}
.ws-resizer-left {
  right: auto;
  left: -3px;
}

/* 右侧栏收起后的展开手柄（位于头部工具栏下方） */
.ws-right-expand {
  position: absolute;
  top: 56px;
  right: 0;
  display: flex;
  align-items: center;
  gap: 3px;
  padding: 7px 5px;
  background: var(--background-secondary);
  border: 1px solid var(--background-modifier-border);
  border-right: none;
  border-radius: var(--radius-m) 0 0 var(--radius-m);
  color: var(--text-muted);
  z-index: 15;
  transition: background var(--anim-fast), color var(--anim-fast);
}
.ws-right-expand:hover {
  background: var(--background-modifier-hover);
  color: var(--text-normal);
}
.ws-expand-text {
  writing-mode: vertical-rl;
  font-size: 11px;
  letter-spacing: 0.15em;
  line-height: 1;
}
</style>
