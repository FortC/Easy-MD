<template>
  <VaultPicker v-if="!vault.opened" />
  <Workspace v-else />

  <!-- 系统打开文件时的模式选择弹窗 -->
  <transition name="emd-fade">
    <div v-if="openChoice.visible" class="emd-modal-bg" @click.self="doOpen('cancel')">
      <div class="emd-modal oc-modal">
        <div class="oc-header">
          <svg viewBox="0 0 48 48" width="40" height="40" class="oc-logo">
            <rect x="4" y="4" width="40" height="40" rx="10" fill="var(--interactive-accent)" />
            <path d="M17 14h4v20h-4zM17 14h14v4H17zM17 23h11v4H17zM17 30h14v4H17z" fill="#fff" />
          </svg>
          <div class="oc-header-text">
            <div class="oc-title">EasyMD</div>
            <div class="oc-subtitle">{{ t("oc.title") }}</div>
          </div>
        </div>

        <div class="oc-file-box">
          <svg viewBox="0 0 24 24" width="18" height="18" class="oc-file-icon">
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" fill="var(--text-accent)" opacity="0.3"/>
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" fill="none" stroke="var(--text-accent)" stroke-width="1.5"/>
            <polyline points="14 2 14 8 20 8" fill="none" stroke="var(--text-accent)" stroke-width="1.5"/>
          </svg>
          <span class="oc-file-name">{{ openChoice.fileName }}</span>
        </div>

        <div class="oc-cards">
          <button class="oc-card" @click="doOpen('vault')">
            <div class="oc-card-icon">
              <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M4 19.5v-15A2.5 2.5 0 0 1 6.5 2H20v20H6.5a2.5 2.5 0 0 1 0-5H20"/>
              </svg>
            </div>
            <div class="oc-card-body">
              <div class="oc-card-title">{{ t("oc.vault") }}</div>
              <div class="oc-card-desc">{{ t("oc.vaultDesc") }}</div>
            </div>
          </button>

          <button class="oc-card oc-card-temp" @click="doOpen('temp')">
            <div class="oc-card-icon">
              <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
                <polyline points="14 2 14 8 20 8"/>
                <line x1="9" y1="15" x2="15" y2="15"/>
              </svg>
            </div>
            <div class="oc-card-body">
              <div class="oc-card-title">{{ t("oc.temp") }}</div>
              <div class="oc-card-desc">{{ t("oc.tempDesc") }}</div>
            </div>
          </button>
        </div>

        <label class="oc-remember">
          <input type="checkbox" v-model="openChoice.remember" />
          {{ t("oc.remember") }}
        </label>

        <button class="oc-cancel" @click="doOpen('cancel')">{{ t("c.cancel") }}</button>
      </div>
    </div>
  </transition>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import VaultPicker from "./views/VaultPicker.vue";
import Workspace from "./views/Workspace.vue";
import { t } from "./i18n";
import { api } from "./ipc/tauri";
import { useSettingsStore } from "./stores/settings";
import { useVaultStore } from "./stores/vault";
import { useNotesIndexStore } from "./stores/notesIndex";
import { useEditorStore } from "./stores/editor";
import { useUiStore } from "./stores/ui";
import { useSyncStore } from "./stores/sync";
import { openDailyNote } from "./lib/actions";

const settings = useSettingsStore();
const vault = useVaultStore();
const notesIndex = useNotesIndexStore();
const editor = useEditorStore();
const ui = useUiStore();
const sync = useSyncStore();

onMounted(async () => {
  await settings.init();
  await sync.init();
  applyCssSnippets();
  window.addEventListener("emd-snippets-changed", applyCssSnippets);

  // 定时同步（settings.sync_interval_minutes > 0 时启用）
  setInterval(() => {
    const min = settings.data.sync_interval_minutes;
    if (min > 0 && vault.opened && !sync.running && settings.data.active_mcp) {
      sync.syncNow();
    }
  }, 60_000);

  // 索引增量事件：镜像更新 + 刷新文件树受影响目录
  api.onVaultChanged((payload) => {
    notesIndex.applyVaultChanged(payload);
    for (const rel of [
      ...payload.updated.map((u) => u.path),
      ...payload.removed,
      ...payload.canvas_changed,
      ...payload.graph_changed,
    ]) {
      vault.refreshParents(rel);
    }
    if (payload.canvas_changed.length > 0) {
      for (const c of payload.canvas_changed) {
        window.dispatchEvent(new CustomEvent("emd-canvas-changed", { detail: c }));
      }
    }
    if (payload.graph_changed.length > 0) {
      for (const g of payload.graph_changed) {
        window.dispatchEvent(new CustomEvent("emd-graph-changed", { detail: g }));
      }
    }
  });

  // 系统打开文件：运行中右键/双击 → 事件转发
  api.onOsOpenFile((path) => openOsFile(path));

  // 自动恢复上次 vault（启动参数带文件时优先打开文件）
  const pending = await api.getPendingFile();
  if (pending) {
    await openOsFile(pending);
    return;
  }
  const last = settings.data.last_vault;
  if (last) {
    try {
      await vault.open(last);
    } catch {
      settings.data.last_vault = null;
      await settings.persist();
    }
  }
});

/** 打开模式选择弹窗状态 */
const openChoice = ref<{
  visible: boolean;
  path: string;
  fileName: string;
  remember: boolean;
}>({ visible: false, path: "", fileName: "", remember: false });

/** 打开磁盘上的 md/canvas：已设默认方式直接执行，否则显示选择弹窗 */
function openOsFile(abs: string) {
  const mode = settings.data.os_open_mode;
  if (mode === "vault" || mode === "temp") {
    doOpen(mode);
    return;
  }
  const fileName = abs.replace(/\\/g, "/").split("/").pop() || t("oc.file");
  openChoice.value = { visible: true, path: abs, fileName, remember: false };
}

/** 用户选择后执行 */
async function doOpen(mode: "vault" | "temp" | "cancel") {
  const abs = openChoice.value.path;
  const remember = openChoice.value.remember;
  openChoice.value.visible = false;
  if (mode === "cancel") {
    await fallbackOpen();
    return;
  }
  if (remember) {
    settings.data.os_open_mode = mode;
    await settings.persist();
  }
  try {
    const res =
      mode === "vault"
        ? await api.openPathFromOs(abs)
        : await api.openAsTemp(abs);
    vault.applyOpened(res.root, res.notes);
    if (res.rel.toLowerCase().endsWith(".canvas")) {
      ui.canvasPath = res.rel;
      ui.view = "canvas";
    } else {
      await editor.openNote(res.rel);
      ui.view = "editor";
    }
  } catch (e) {
    console.error("打开文件失败：", e);
    await fallbackOpen();
  }
}

async function fallbackOpen() {
  const last = settings.data.last_vault;
  if (last && !vault.opened) {
    try {
      await vault.open(last);
    } catch {
      /* 保持欢迎页 */
    }
  }
}

// ---- 全局快捷键 ----
function onKeydown(e: KeyboardEvent) {
  const ctrl = e.ctrlKey || e.metaKey;
  if (!ctrl || e.altKey) return;
  const key = e.key.toLowerCase();
  if (key === "e") {
    e.preventDefault();
    editor.cycleMode();
  } else if (key === "p" && !e.shiftKey) {
    e.preventDefault();
    ui.openSearch("file");
  } else if (key === "f" && e.shiftKey) {
    e.preventDefault();
    ui.openSearch("content");
  } else if (key === "n") {
    e.preventDefault();
    ui.openNewNote();
  } else if (key === "d") {
    e.preventDefault();
    openDailyNote();
  } else if (key === "g") {
    e.preventDefault();
    if (editor.isDirty) editor.save();
    ui.view = ui.view === "graph" ? "editor" : "graph";
  } else if (key === ",") {
    e.preventDefault();
    ui.settingsOpen = true;
  } else if (key === "s") {
    e.preventDefault();
    editor.save();
  }
}

onMounted(() => window.addEventListener("keydown", onKeydown));
onUnmounted(() => window.removeEventListener("keydown", onKeydown));

/** 应用启用的 CSS 片段（配置目录 snippets/*.css → 注入 <style>） */
async function applyCssSnippets() {
  document.querySelectorAll("style[data-emd-snippet]").forEach((n) => n.remove());
  try {
    const files = await api.snippetFiles();
    // settings 里没有记录的新文件自动补上（默认停用）
    for (const f of files) {
      if (!settings.data.css_snippets.find((s) => s.name === f)) {
        settings.data.css_snippets.push({ name: f, enabled: false });
      }
    }
    settings.data.css_snippets = settings.data.css_snippets.filter((s) =>
      files.includes(s.name),
    );
    for (const s of settings.data.css_snippets) {
      if (!s.enabled) continue;
      const css = await api.readSnippetFile(s.name);
      const style = document.createElement("style");
      style.setAttribute("data-emd-snippet", s.name);
      style.textContent = css;
      document.head.appendChild(style);
    }
    await settings.persist();
  } catch {
    /* 片段目录不可用时静默 */
  }
}
</script>

<style>
#app {
  height: 100%;
}

/* ---------- 打开文件弹窗 ---------- */
.oc-modal {
  width: 460px;
  padding: 24px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.oc-header {
  display: flex;
  align-items: center;
  gap: 14px;
}
.oc-logo {
  flex-shrink: 0;
  border-radius: 10px;
}
.oc-title {
  font-size: 20px;
  font-weight: 700;
  color: var(--text-normal);
  letter-spacing: 0.02em;
}
.oc-subtitle {
  font-size: 13px;
  color: var(--text-muted);
  margin-top: 2px;
}
.oc-file-box {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 16px;
  border-radius: var(--radius-m);
  background: var(--background-primary);
  border: 1px solid var(--background-modifier-border);
}
.oc-file-icon {
  flex-shrink: 0;
}
.oc-file-name {
  font-size: 15px;
  font-weight: 600;
  color: var(--text-normal);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.oc-cards {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.oc-card {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 14px 16px;
  border-radius: var(--radius-l);
  border: 2px solid var(--background-modifier-border);
  background: var(--background-primary);
  text-align: left;
  cursor: pointer;
  transition: border-color var(--anim-fast), background var(--anim-fast), transform var(--anim-fast);
}
.oc-card:hover {
  border-color: var(--interactive-accent);
  background: var(--interactive-accent-hover-alt);
  transform: translateY(-1px);
}
.oc-card-icon {
  width: 44px;
  height: 44px;
  border-radius: var(--radius-m);
  background: var(--interactive-accent-hover-alt);
  color: var(--interactive-accent);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}
.oc-card:hover .oc-card-icon {
  background: var(--interactive-accent);
  color: var(--text-on-accent);
}
.oc-card-temp .oc-card-icon {
  background: var(--code-background);
  color: var(--text-muted);
}
.oc-card-temp:hover .oc-card-icon {
  background: var(--text-muted);
  color: var(--text-on-accent);
}
.oc-card-body {
  flex: 1;
  min-width: 0;
}
.oc-card-title {
  font-size: 15px;
  font-weight: 600;
  color: var(--text-normal);
  margin-bottom: 3px;
}
.oc-card-desc {
  font-size: 12px;
  color: var(--text-muted);
  line-height: 1.4;
}
.oc-cancel {
  align-self: center;
  padding: 6px 20px;
  border-radius: var(--radius-m);
  color: var(--text-faint);
  font-size: 13px;
  transition: color var(--anim-fast);
}
.oc-cancel:hover {
  color: var(--text-normal);
}
.oc-remember {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--text-muted);
  font-size: 13px;
  cursor: pointer;
  user-select: none;
}
.oc-remember input {
  accent-color: var(--interactive-accent);
  width: 15px;
  height: 15px;
  cursor: pointer;
}
</style>
