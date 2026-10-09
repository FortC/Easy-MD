<template>
  <div class="file-explorer" @contextmenu.prevent="showMenu($event, null)">
    <div class="fe-tree">
      <FileTree :key="vault.root" dir="" @menu="showMenu" @open="openNoteFile" />
      <!-- 空库引导：目录为空时给出可点的入口，而不是一片空白 -->
      <div v-if="rootEmpty" class="fe-empty">
        <Icon name="folder-open" :size="28" />
        <p>{{ t("fe.emptyVault") }}</p>
        <button class="emd-btn" @click="ui.openNewNote()">
          <Icon name="file-plus" :size="13" /> {{ t("fe.newNote") }}
        </button>
        <button class="emd-btn" @click="openFolderDialog(null)">
          <Icon name="folder-plus" :size="13" /> {{ t("fe.newFolder") }}
        </button>
        <button class="emd-btn" @click="importHere(null)">
          <Icon name="download" :size="13" /> {{ t("fe.import") }}
        </button>
      </div>
    </div>

    <!-- 右键菜单（通用下拉组件） -->
    <DropdownMenu
      :open="menu.visible"
      :x="menu.x"
      :y="menu.y"
      :items="menuItems"
      @select="onMenuSelect"
      @close="menu.visible = false"
    />

    <InputDialog
      v-if="dialog.mode === 'folder'"
      :title="tf('fe.newFolderIn', { dir: dialog.parent || t('fe.rootDir') })"
      @confirm="confirmNewFolder"
      @cancel="dialog.mode = 'none'"
    />
    <InputDialog
      v-if="dialog.mode === 'rename'"
      :title="t('fe.rename')"
      :initial="dialog.entry?.name"
      @confirm="confirmRename"
      @cancel="dialog.mode = 'none'"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, onUnmounted, reactive } from "vue";
import Icon from "../common/Icon.vue";
import InputDialog from "../common/InputDialog.vue";
import DropdownMenu, { type DropItem } from "../common/DropdownMenu.vue";
import FileTree from "./FileTree.vue";
import { useVaultStore } from "../../stores/vault";
import { useEditorStore } from "../../stores/editor";
import { useNotesIndexStore } from "../../stores/notesIndex";
import { useUiStore } from "../../stores/ui";
import { api } from "../../ipc/tauri";
import { copyText } from "../../lib/clipboard";
import { t, tf } from "../../i18n";
import type { FsEntry } from "../../types";

const vault = useVaultStore();
const editor = useEditorStore();
const indexStore = useNotesIndexStore();
const ui = useUiStore();

const multiSelected = ref(new Set<string>());

/** 根目录为空（空库）：文件树无内容时显示引导 */
const rootEmpty = computed(() => vault.dirCache[""]?.length === 0);

const menu = reactive({
  visible: false,
  x: 0,
  y: 0,
  entry: null as FsEntry | null,
});

const dialog = reactive({
  mode: "none" as "none" | "folder" | "rename",
  parent: null as string | null,
  entry: null as FsEntry | null,
});

/** 菜单项根据右键目标动态生成 */
const menuItems = computed(() => {
  const e = menu.entry;
  if (!e) {
    return [
      { key: "new-note", label: t("fe.newNote"), icon: "file-plus" },
      { key: "new-canvas", label: t("fe.newCanvas"), icon: "layout-grid" },
      { key: "new-folder", label: t("fe.newFolder"), icon: "folder-plus" },
      { key: "sep-i", label: "", separator: true },
      { key: "import", label: t("fe.import"), icon: "download" },
    ];
  }
  if (e.is_dir) {
    return [
      { key: "new-note", label: t("fe.newNote"), icon: "file-plus" },
      { key: "new-canvas", label: t("fe.newCanvas"), icon: "layout-grid" },
      { key: "new-folder", label: t("fe.newFolder"), icon: "folder-plus" },
      { key: "sep-i", label: "", separator: true },
      { key: "import", label: t("fe.import"), icon: "download" },
      { key: "sep-p", label: "", separator: true },
      { key: "copy-full", label: t("fe.copyFullPath"), icon: "copy" },
      { key: "copy-rel", label: t("fe.copyRelPath"), icon: "copy" },
      { key: "sep", label: "", separator: true },
      { key: "delete", label: t("fe.delete"), icon: "trash-2", danger: true },
    ];
  }
  const items: DropItem[] = [
    { key: "copy-full", label: t("fe.copyFullPath"), icon: "copy" },
    { key: "copy-rel", label: t("fe.copyRelPath"), icon: "copy" },
    { key: "sep-p", label: "", separator: true },
    { key: "rename", label: t("fe.rename"), icon: "pencil" },
    { key: "duplicate", label: t("fe.duplicate"), icon: "copy" },
    { key: "sep", label: "", separator: true },
    { key: "delete", label: t("fe.delete"), icon: "trash-2", danger: true },
  ];
  return items;
});

const closeMenu = () => (menu.visible = false);
onMounted(() => {
  window.addEventListener("pointerdown", closeMenu);
  window.addEventListener("emd-multi-select-changed", ((e: CustomEvent<Set<string>>) => {
    multiSelected.value = e.detail;
  }) as EventListener);
});
onUnmounted(() => window.removeEventListener("pointerdown", closeMenu));

function showMenu(ev: MouseEvent, entry: FsEntry | null) {
  // 防御：行内右键即使未来某处漏了 .stop 冒泡上来，也不能覆盖行菜单为空白菜单
  if (!entry && (ev.target as HTMLElement | null)?.closest?.(".ft-row")) return;
  menu.x = ev.clientX;
  menu.y = ev.clientY;
  menu.entry = entry;
  menu.visible = true;
}

async function onMenuSelect(key: string) {
  menu.visible = false;
  const e = menu.entry;
  switch (key) {
    case "new-note":
      ui.openNewNote(e?.is_dir ? e.path : parentOf(e));
      break;
    case "new-canvas":
      vault.newCanvas(e?.is_dir ? e.path : parentOf(e));
      break;
    case "new-folder":
      openFolderDialog(e?.is_dir ? e.path : parentOf(e));
      break;
    case "import":
      importHere(e?.is_dir ? e.path : parentOf(e));
      break;
    case "duplicate":
      if (e) {
        try { await api.duplicateFile(e.path); await vault.refreshParents(e.path); await indexStore.rebuild(); } catch (err) { alert(String(err)); }
      }
      break;
    case "batchDelete":
      if (multiSelected.value.size > 0) {
        if (!confirm(`Delete ${multiSelected.value.size} files?`)) return;
        try {
          await api.deleteFiles(Array.from(multiSelected.value));
          for (const p of multiSelected.value) await vault.refreshParents(p);
          multiSelected.value = new Set();
          await indexStore.rebuild();
        } catch (err) { alert(String(err)); }
      }
      break;
    case "copy-full":
    case "copy-rel": {
      if (!e) break;
      const rel = e.path;
      const full = `${vault.root.replace(/[\\/]+$/, "")}\\${rel.replace(/\//g, "\\")}`;
      const ok = await copyText(key === "copy-full" ? full : rel);
      if (!ok) alert(t("fe.copyFail"));
      break;
    }
    case "rename":
      dialog.mode = "rename";
      dialog.entry = e;
      break;
    case "delete":
      deleteEntry(e);
      break;
  }
}

function openFolderDialog(parent: string | null) {
  dialog.mode = "folder";
  dialog.parent = parent;
}

function openNewFolderDialog() {
  openFolderDialog(null);
}

/** 导入外部 md/图片等文件到当前库根目录 */
async function importHere(folder?: string | null) {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const files = await open({
    multiple: true,
    filters: [{ name: t("fe.importFilter"), extensions: ["md", "markdown", "canvas", "png", "jpg", "jpeg", "gif", "webp", "svg", "pdf"] }],
  });
  if (!files || (Array.isArray(files) && files.length === 0)) return;
  const paths = Array.isArray(files) ? files : [files];
  try {
    const imported = await api.importFiles(paths, folder ?? null);
    for (const rel of imported) await vault.refreshParents(rel);
    await vault.refreshDir("");
    await indexStore.rebuild();
  } catch (e) {
    alert(`${t("fe.importFail")}: ${e}`);
  }
}

async function openNoteFile(path: string) {
  await editor.openNote(path);
}

function parentOf(entry: FsEntry | null): string | null {
  if (!entry) return null;
  const parts = entry.path.split("/");
  parts.pop();
  return parts.length ? parts.join("/") : null;
}

async function confirmNewFolder(name: string) {
  dialog.mode = "none";
  await api.createFolder(dialog.parent, name);
  await vault.refreshDir(dialog.parent || "");
}

async function confirmRename(name: string) {
  const entry = dialog.entry;
  dialog.mode = "none";
  if (!entry) return;
  const newPath = await api.renamePath(entry.path, name);
  // 修正编辑器与树
  if (editor.activePath === entry.path) editor.renameSelf(newPath);
  await vault.refreshParents(newPath);
  await indexStore.rebuild();
}

async function deleteEntry(e: FsEntry | null) {
  if (!e) return;
  if (!confirm(`${tf("fe.confirmDel", { name: e.name })}`)) return;
  await api.deletePath(e.path);
  if (editor.activePath === e.path) editor.reset();
  await vault.refreshParents(e.path);
  await indexStore.rebuild();
}
</script>

<style scoped>
.file-explorer {
  /* ws-left 是 flex 容器：不设 flex:1 会被收缩成内容宽度，
     右侧空白区域右键/点击都不归文件树管 */
  flex: 1 1 0%;
  min-width: 0;
  width: 100%;
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}
.fe-tree {
  flex: 1;
  overflow-y: auto;
  padding: 4px 6px;
}
.fe-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 36px 12px;
  color: var(--text-faint);
  font-size: var(--font-ui-smaller);
  text-align: center;
  white-space: pre-line;
}
.fe-empty .emd-btn {
  width: 100%;
  justify-content: center;
}
</style>
