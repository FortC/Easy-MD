<template>
  <div class="ft-children">
    <div v-for="entry in children" :key="entry.path" class="ft-item">
      <!-- 目录 -->
      <div
        v-if="entry.is_dir"
        class="emd-tree-item ft-row"
        :style="{ paddingLeft: 6 + depth * 14 + 'px' }"
        @click="vault.toggleExpand(entry.path)"
        @contextmenu.prevent="emitMenu($event, entry)"
      >
        <Icon
          :name="vault.expanded[entry.path] ? 'chevron-down' : 'chevron-right'"
          :size="12"
        />
        <span class="ft-name">{{ entry.name }}</span>
      </div>
      <!-- 文件 -->
      <div
        v-else
        class="emd-tree-item ft-row"
        :class="{ 'is-active': isActive(entry), 'is-multi-selected': multiSelected.has(entry.path) }"
        :style="{ paddingLeft: 6 + depth * 14 + 'px' }"
        @click="openEntry(entry, $event)"
        @contextmenu.prevent="emitMenu($event, entry)"
      >
        <Icon :name="iconFor(entry)" :size="14" class="ft-fileicon" />
        <span class="ft-name">{{ entry.name }}</span>
        <span v-if="multiSelected.has(entry.path)" class="ft-check-badge"><Icon name="check" :size="12" /></span>
      </div>
      <FileTree
        v-if="entry.is_dir && vault.expanded[entry.path]"
        :dir="entry.path"
        :depth="depth + 1"
        @menu="(e: unknown, entry: FsEntry) => emitMenu(e as MouseEvent, entry)"
        @open="(p: string) => emit('open', p)"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import Icon from "../common/Icon.vue";
import { useVaultStore } from "../../stores/vault";
import { useUiStore } from "../../stores/ui";
import { useEditorStore } from "../../stores/editor";
import { useSettingsStore } from "../../stores/settings";
import type { FsEntry } from "../../types";

const props = withDefaults(defineProps<{ dir: string; depth?: number }>(), {
  depth: 0,
});
const emit = defineEmits<{
  menu: [event: MouseEvent, entry: FsEntry];
  open: [path: string];
}>();

const vault = useVaultStore();
const ui = useUiStore();
const editor = useEditorStore();
const children = ref<FsEntry[]>([]);
const multiSelected = ref(new Set<string>());

// 响应父组件的多选集合变化（通过事件同步）
window.addEventListener("emd-multi-select-changed", ((e: CustomEvent<Set<string>>) => {
  multiSelected.value = e.detail;
}) as EventListener);

/** 载入当前目录一层（切换库/目录内容变化时自动重载，避免残留上一个库的文件列表） */
async function load() {
  let list = await vault.loadDir(props.dir);
  // 笔记树只展示笔记相关内容：隐藏小计目录与图谱文件（图谱在图谱模块里管理）
  if (props.dir === "" || props.dir === "jots" || props.dir === "daily") {
    const jotDir = (useSettingsStore().data.daily_dir || "jots").replace(/^\/+$|\/+$/g, "");
    if (props.dir === "") {
      // 根目录：隐藏 jots/daily 文件夹本身
      list = list.filter((e) => e.name !== jotDir && e.name !== "daily" && e.name !== "jots");
    }
  }
  list = list.filter((e) => e.kind !== "graph");
  children.value = list;
}

// 切换库（root 变化）或目录内容刷新（dirCache 更新）都重新载入
watch(
  () => [vault.root, vault.dirCache[props.dir]] as const,
  () => load(),
  { immediate: true },
);

function iconFor(e: FsEntry): string {
  if (e.kind === "md") return "file-text";
  if (e.kind === "canvas") return "layout-grid";
  if (e.kind === "graph") return "share-2";
  if (e.kind === "image") return "image";
  return "file-text";
}

function isActive(e: FsEntry): boolean {
  return e.path === editor.activePath && ui.view === "editor";
}

function openEntry(e: FsEntry, ev?: MouseEvent) {
  // Ctrl+点击 = 多选/取消多选
  if (ev?.ctrlKey || ev?.metaKey) {
    const s = new Set(multiSelected.value);
    if (s.has(e.path)) s.delete(e.path);
    else s.add(e.path);
    multiSelected.value = s;
    window.dispatchEvent(new CustomEvent("emd-multi-select-changed", { detail: s }));
    return;
  }
  // 普通点击：清除多选
  if (multiSelected.value.size > 0) {
    multiSelected.value = new Set();
    window.dispatchEvent(new CustomEvent("emd-multi-select-changed", { detail: new Set() }));
  }
  if (e.kind === "md") {
    if (editor.isDirty) editor.save();
    emit("open", e.path);
    ui.view = "editor";
  } else if (e.kind === "canvas") {
    if (editor.isDirty) editor.save();
    ui.canvasPath = e.path;
    ui.view = "canvas";
  } else if (e.kind === "graph") {
    if (editor.isDirty) editor.save();
    ui.graphPath = e.path;
    ui.view = "graph";
  } else if (e.kind === "image") {
    emit("open", e.path);
  }
}

function emitMenu(ev: MouseEvent, entry: FsEntry) {
  emit("menu", ev, entry);
}
</script>

<style scoped>
.ft-row {
  gap: 5px;
  font-size: var(--font-ui-size);
}
.ft-fileicon {
  color: var(--text-faint);
}
.ft-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
/* 多选高亮：紫色背景 + 左侧紫条 + 右侧圆形勾 */
.ft-row.is-multi-selected {
  background: var(--interactive-accent-hover-alt) !important;
  border-left: 3px solid var(--interactive-accent);
  padding-left: 4px !important;
}
.ft-row.is-multi-selected .ft-name {
  color: var(--text-accent);
  font-weight: 600;
}
.ft-check-badge {
  margin-left: auto;
  flex-shrink: 0;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: var(--interactive-accent);
  color: var(--text-on-accent);
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 700;
}
</style>
