<template>
  <div class="ribbon">
    <button
      v-for="b in topButtons"
      :key="b.name"
      class="rb-btn"
      :class="{ 'is-active': b.active }"
      :title="b.title"
      @click="b.action"
    >
      <Icon :name="b.icon" :size="17" />
      <span class="rb-label">{{ b.label }}</span>
    </button>
    <div class="rb-spacer" />
    <button class="rb-btn" :title="t('rb.settingsTitle')" @click="ui.settingsOpen = true">
      <Icon name="settings" :size="17" />
      <span class="rb-label">{{ t('rb.settings') }}</span>
    </button>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import Icon from "./Icon.vue";
import { useUiStore } from "../../stores/ui";
import { useSettingsStore } from "../../stores/settings";
import { useVaultStore } from "../../stores/vault";
import { t } from "../../i18n";

const ui = useUiStore();
const settings = useSettingsStore();
const vault = useVaultStore();

interface RibbonBtn {
  name: string;
  icon: string;
  label: string;
  title: string;
  action: () => void;
  active: boolean;
}

/** 图谱/日历为全屏独立视图：左侧栏不显示，点面板按钮时退回编辑器 */
const fullView = computed(() => ui.view === "graph" || ui.view === "calendar");

function showLeft(tab: "notes" | "jots" | "tags") {
  if (fullView.value) ui.view = "editor";
  ui.leftVisible = true;
  ui.leftTab = tab;
}

async function createJot() {
  const now = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  const name = `小计-${now.getFullYear()}${pad(now.getMonth() + 1)}${pad(now.getDate())}-${pad(now.getHours())}${pad(now.getMinutes())}`;
  const dir = (settings.data.daily_dir || "jots").replace(/^\/+$|\/+$/g, "");
  await vault.newNote(dir || null, name, "");
  ui.view = "jot";
}

const topButtons = computed<RibbonBtn[]>(() => [
  { name: "notes", icon: "file-text", label: t("rb.notes"), title: t("rb.notesTitle"), action: () => showLeft("notes"), active: !fullView.value && ui.leftVisible && ui.leftTab === "notes" },
  { name: "jots", icon: "pencil", label: t("rb.jot"), title: t("rb.jotListTitle"), action: () => showLeft("jots"), active: !fullView.value && ui.leftVisible && ui.leftTab === "jots" },
  { name: "tags", icon: "tag", label: t("rb.tags"), title: t("rb.tagsTitle"), action: () => showLeft("tags"), active: !fullView.value && ui.leftVisible && ui.leftTab === "tags" },
  { name: "graph", icon: "share-2", label: t("rb.graph"), title: t("rb.graphTitle"), action: () => { ui.view = "graph"; }, active: ui.view === "graph" },
  { name: "calendar", icon: "calendar-days", label: t("rb.calendar"), title: t("rb.calendarTitle"), action: () => { ui.view = "calendar"; }, active: ui.view === "calendar" },
  {
    name: "theme",
    icon: settings.data.theme === "dark" ? "sun" : "moon",
    label: settings.data.theme === "dark" ? t("rb.light") : t("rb.dark"),
    title: settings.data.theme === "dark" ? t("rb.lightTitle") : t("rb.darkTitle"),
    action: () => settings.toggleTheme(),
    active: false,
  },
]);
</script>

<style scoped>
.ribbon { width: var(--ribbon-width); background: var(--background-secondary); border-right: 1px solid var(--background-modifier-border); display: flex; flex-direction: column; align-items: stretch; padding: 8px 0; gap: 2px; flex-shrink: 0; overflow-y: auto; }
.rb-btn { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 3px; padding: 7px 2px 6px; border-radius: var(--radius-s); color: var(--text-muted); transition: background var(--anim-fast), color var(--anim-fast); }
.rb-btn:hover { background: var(--background-modifier-hover); color: var(--text-normal); }
.rb-btn.is-active { background: var(--background-modifier-active-hover); color: var(--text-normal); }
.rb-label { font-size: 11px; line-height: 1; letter-spacing: 0.02em; }
.rb-spacer { flex: 1; min-height: 12px; }
</style>
