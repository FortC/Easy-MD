<template>
  <div class="jot-panel">
    <div class="emd-panel-header">
      <span>{{ t("rb.jot") }}</span>
      <button class="jot-new-btn" :title="t('jot.newJot')" @click="createJot">
        <Icon name="plus" :size="13" />
      </button>
    </div>
    <div class="jot-timeline">
      <div v-for="group in groupedJots" :key="group.date" class="jot-date-group">
        <div class="jot-date-line">
          <span class="jot-date-label">{{ group.date }}</span>
          <div class="jot-date-bar" />
        </div>
        <div
          v-for="jot in group.jots"
          :key="jot.path"
          class="jot-item"
          :class="{ 'is-active': editor.activePath === jot.path, 'is-multi': multiSelected.has(jot.path) }"
          @click="openJot(jot, $event)"
          @contextmenu.prevent="showJotMenu($event, jot)"
        >
          <div class="jot-dot" />
          <div class="jot-content">
            <div class="jot-name">{{ jot.title }}</div>
            <div v-if="jot.tags.length" class="jot-tags">
              <span v-for="tg in jot.tags" :key="tg" class="jot-tag">#{{ tg }}</span>
            </div>
          </div>
          <span v-if="multiSelected.has(jot.path)" class="jot-check-badge">
            <Icon name="check" :size="12" />
          </span>
        </div>
      </div>
      <div v-if="jotNotes.length === 0" class="jot-empty">
        {{ t("jot.empty") }}
      </div>
    </div>

    <!-- 小计右键菜单 -->
    <DropdownMenu :open="jotMenu.visible" :x="jotMenu.x" :y="jotMenu.y" :items="jotMenuItems" @select="onJotMenu" @close="jotMenu.visible = false" />
  </div>
</template>

<script setup lang="ts">
import { computed, reactive, ref } from "vue";
import Icon from "../common/Icon.vue";
import DropdownMenu, { type DropItem } from "../common/DropdownMenu.vue";
import { useEditorStore } from "../../stores/editor";
import { useNotesIndexStore } from "../../stores/notesIndex";
import { useSettingsStore } from "../../stores/settings";
import { useUiStore } from "../../stores/ui";
import { useVaultStore } from "../../stores/vault";
import { t, tf } from "../../i18n";
import type { NoteIndex } from "../../types";
import { api } from "../../ipc/tauri";

const editor = useEditorStore();
const indexStore = useNotesIndexStore();
const settings = useSettingsStore();
const ui = useUiStore();
const vault = useVaultStore();

/** 判断是否为小计 */
function isJot(path: string): boolean {
  const dir = (settings.data.daily_dir || "jots").replace(/^\/+|\/+$/g, "");
  return path.startsWith(dir + "/") || path.startsWith("daily/") || path.startsWith("jots/") || path.includes("小计-");
}

/** 全部小计笔记（按修改时间倒序） */
const multiSelected = ref(new Set<string>());
const jotMenu = reactive({ visible: false, x: 0, y: 0, target: null as NoteIndex | null });
const jotMenuItems = computed<DropItem[]>(() => {
  const items: DropItem[] = [];
  if (jotMenu.target) {
    items.push({ key: "open", label: t("jot.open"), icon: "file-text" });
    items.push({ key: "duplicate", label: t("fe.duplicate"), icon: "copy" });
    items.push({ key: "sep1", label: "", separator: true });
  }
  if (multiSelected.value.size > 0) {
    items.push({ key: "batchDelete", label: t("fe.batchDelete").replace("{}", String(multiSelected.value.size)), icon: "trash-2", danger: true });
  } else if (jotMenu.target) {
    items.push({ key: "delete", label: t("fe.delete"), icon: "trash-2", danger: true });
  }
  return items;
});

function showJotMenu(ev: MouseEvent, jot: NoteIndex) {
  jotMenu.x = ev.clientX; jotMenu.y = ev.clientY; jotMenu.target = jot; jotMenu.visible = true;
}

async function onJotMenu(key: string) {
  jotMenu.visible = false;
  const n = jotMenu.target;
  if (key === "open" && n) { await openJot(n); }
  else if (key === "duplicate" && n) { await duplicateJot(n); }
  else if (key === "delete" && n) {
    if (!confirm(tf("fe.confirmDel", { name: n.title }))) return;
    try { await api.deletePath(n.path); indexStore.rebuild(); } catch (e) { alert(String(e)); }
  }
  else if (key === "batchDelete") { await batchDelete(); }
}

/** Ctrl+点击多选 */
function toggleMulti(path: string) {
  const s = new Set(multiSelected.value);
  if (s.has(path)) s.delete(path); else s.add(path);
  multiSelected.value = s;
}

const jotNotes = computed<NoteIndex[]>(() =>
  indexStore.notes.filter((n) => isJot(n.path)).sort((a, b) => b.mtime - a.mtime),
);

/** 按日期分组 */
const groupedJots = computed(() => {
  const groups: { date: string; jots: NoteIndex[] }[] = [];
  for (const n of jotNotes.value) {
    const d = new Date(n.mtime * 1000);
    const pad = (x: number) => String(x).padStart(2, "0");
    const dateStr = `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
    let g = groups.find((x) => x.date === dateStr);
    if (!g) {
      g = { date: dateStr, jots: [] };
      groups.push(g);
    }
    g.jots.push(n);
  }
  return groups;
});

async function openJot(n: NoteIndex, ev?: MouseEvent) {
  if (ev?.ctrlKey || ev?.metaKey) { toggleMulti(n.path); return; }
  await editor.openNote(n.path);
  ui.view = "jot";
}

/** 批量删除选中 */
async function batchDelete() {
  if (multiSelected.value.size === 0) return;
  if (!confirm(tf("jot.batchDelConfirm", { n: multiSelected.value.size }))) return;
  try {
    await api.deleteFiles(Array.from(multiSelected.value));
    multiSelected.value = new Set();
    indexStore.rebuild();
  } catch (e) { alert(String(e)); }
}

/** 复制副本 */
async function duplicateJot(n: NoteIndex) {
  try {
    await api.duplicateFile(n.path);
    indexStore.rebuild();
  } catch (e) { alert(String(e)); }
}

async function createJot() {
  const now = new Date();
  const pad = (x: number) => String(x).padStart(2, "0");
  const name = `小计-${now.getFullYear()}${pad(now.getMonth() + 1)}${pad(now.getDate())}-${pad(now.getHours())}${pad(now.getMinutes())}`;
  const dir = (settings.data.daily_dir || "jots").replace(/^\/+|\/+$/g, "");
  await vault.newNote(dir || null, name, "");
  ui.view = "jot";
}
</script>

<style scoped>
.jot-panel {
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.jot-new-btn {
  padding: 3px;
  border-radius: var(--radius-s);
  color: var(--text-muted);
}
.jot-new-btn:hover { background: var(--background-modifier-hover); color: var(--text-normal); }
.jot-timeline {
  flex: 1;
  overflow-y: auto;
  padding: 4px 6px;
}
.jot-date-group { margin-bottom: 8px; }
.jot-date-line {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 4px;
}
.jot-date-label {
  font-size: var(--font-ui-smaller);
  color: var(--text-faint);
  font-weight: 600;
  white-space: nowrap;
}
.jot-date-bar {
  flex: 1;
  height: 1px;
  background: var(--background-modifier-border);
}
.jot-item {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 6px 8px;
  border-radius: var(--radius-s);
  cursor: pointer;
  position: relative;
  margin-left: 6px;
  border-left: 2px solid var(--background-modifier-border);
  padding-left: 12px;
  transition: border-color var(--anim-fast), background var(--anim-fast);
}
.jot-item:hover { background: var(--background-modifier-hover); border-left-color: var(--text-accent); }
.jot-item.is-active { background: var(--background-modifier-active-hover); border-left-color: var(--interactive-accent); }
.jot-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--text-accent);
  opacity: 0.5;
  margin-top: 4px;
  flex-shrink: 0;
}
.jot-item:hover .jot-dot, .jot-item.is-active .jot-dot { opacity: 1; }
.jot-content { flex: 1; min-width: 0; }
.jot-name {
  font-size: var(--font-ui-size);
  color: var(--text-normal);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  line-height: 1.4;
}
.jot-tags { display: flex; flex-wrap: wrap; gap: 3px; margin-top: 2px; }
.jot-tag {
  font-size: 10px;
  color: var(--tag-color);
  background: var(--tag-background);
  border-radius: 3px;
  padding: 0 4px;
  line-height: 1.4;
}
.jot-empty {
  color: var(--text-faint);
  text-align: center;
  padding: 32px 12px;
  font-size: var(--font-ui-smaller);
  white-space: pre-line;
  line-height: 1.6;
}
.jot-item.is-multi {
  background: var(--interactive-accent-hover-alt) !important;
  border-left-color: var(--interactive-accent) !important;
}
.jot-item.is-multi .jot-name {
  color: var(--text-accent);
  font-weight: 600;
}
/* 与笔记树多选一致的右侧圆形小勾 */
.jot-check-badge {
  flex-shrink: 0;
  width: 18px;
  height: 18px;
  margin-top: 1px;
  border-radius: 50%;
  background: var(--interactive-accent);
  color: var(--text-on-accent);
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 700;
}
</style>
