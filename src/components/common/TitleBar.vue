<template>
  <div class="titlebar">
    <div class="titlebar-left">
      <span class="titlebar-name">EasyMD</span>
      <button
        v-if="vaultName"
        ref="vaultBtn"
        class="titlebar-vault-btn"
        :title="t('vs.switch')"
        @pointerdown.stop
        @click="vaultMenuOpen = !vaultMenuOpen"
      >
        <span class="titlebar-vault-prefix">{{ t("vs.prefix") }}</span>{{ vaultName }}
        <Icon name="chevron-down" :size="10" class="titlebar-vault-caret" />
      </button>
    </div>

    <!-- 知识库切换下拉 -->
    <DropdownMenu
      :open="vaultMenuOpen"
      :anchor="vaultBtn"
      align="left"
      :title="t('vs.switch')"
      @close="vaultMenuOpen = false"
    >
      <button
        v-for="v in vaultList"
        :key="v.path"
        class="tb-vault-item"
        :class="{ 'is-current': v.path === vault.root }"
        @click="switchVault(v.path)"
      >
        <Icon :name="v.path === vault.root ? 'check' : 'book'" :size="13" />
        <span class="tb-vault-name">{{ v.name }}</span>
      </button>
      <div class="emd-dd-sep" />
      <button class="tb-vault-item" @click="vaultMenuOpen = false; ui.view = 'editor'; vault.close()">
        <Icon name="external-link" :size="13" />
        <span>{{ t("vs.back") }}</span>
      </button>
    </DropdownMenu>

    <!-- 全局搜索入口：点击唤起搜索面板 -->
    <div class="titlebar-search" data-tauri-drag-region>
      <div class="ts-box" :title="t('tb.searchTitle')" @pointerdown.stop @click="openSearch">
        <Icon name="search" :size="13" />
        <span class="ts-hint">{{ t("tb.searchHint") }}</span>
        <span class="ts-kbd">Ctrl+Shift+F</span>
      </div>
    </div>

    <div class="titlebar-drag" data-tauri-drag-region />
    <div class="titlebar-controls">
      <button class="tb-btn" :title="t('tb.min')" @click="minimize">
        <Icon name="minus" :size="14" />
      </button>
      <button class="tb-btn" :title="t('tb.max')" @click="toggleMax">
        <Icon name="square" :size="12" />
      </button>
      <button class="tb-btn tb-close" :title="t('c.close')" @click="closeWin">
        <Icon name="x" :size="14" />
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import Icon from "./Icon.vue";
import DropdownMenu from "./DropdownMenu.vue";
import { useVaultStore } from "../../stores/vault";
import { useUiStore } from "../../stores/ui";
import { t } from "../../i18n";
import type { VaultEntry } from "../../types";

const vault = useVaultStore();
const ui = useUiStore();
const vaultMenuOpen = ref(false);
const vaultBtn = ref<HTMLElement>();
const vaultList = ref<VaultEntry[]>([]);

const vaultName = computed(() => {
  if (!vault.opened || !vault.root) return "";
  const parts = vault.root.replace(/\\/g, "/").split("/");
  return parts[parts.length - 1] || vault.root;
});

onMounted(async () => {
  await vault.loadVaultList();
  vaultList.value = vault.vaultList;
});

async function switchVault(path: string) {
  vaultMenuOpen.value = false;
  if (path === vault.root) return;
  try {
    if (vault.opened) await vault.close();
    await vault.open(path);
  } catch (e) {
    console.error("switch vault failed:", e);
  }
}

function openSearch() {
  ui.openSearch("content");
}

const win = getCurrentWindow();
const minimize = () => win.minimize();
const toggleMax = () => win.toggleMaximize();
const closeWin = () => win.close();
</script>

<style scoped>
.titlebar {
  height: var(--titlebar-height);
  display: flex;
  align-items: stretch;
  background: var(--titlebar-background);
  border-bottom: 1px solid var(--background-modifier-border);
  user-select: none;
  flex-shrink: 0;
}
.titlebar-left {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 12px;
  color: var(--text-muted);
  font-size: var(--font-ui-size);
  flex-shrink: 0;
}
.titlebar-name {
  color: var(--text-normal);
  font-weight: 600;
  letter-spacing: 0.02em;
}
.titlebar-vault-btn {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  padding: 3px 10px;
  border-radius: var(--radius-s);
  font-size: var(--font-ui-size);
  font-weight: 600;
  color: var(--text-normal);
  border: 1px solid var(--background-modifier-border);
  background: var(--background-primary);
  transition: background var(--anim-fast);
}
.titlebar-vault-btn:hover {
  background: var(--background-modifier-hover);
  border-color: var(--background-modifier-border-hover);
}
.titlebar-vault-prefix {
  font-weight: 400;
  font-size: var(--font-ui-smaller);
  color: var(--text-muted);
}
.titlebar-vault-caret {
  opacity: 0.6;
}
.tb-vault-item {
  display: flex;
  align-items: center;
  gap: 9px;
  width: 100%;
  padding: 6px 10px;
  border-radius: var(--radius-s);
  color: var(--text-normal);
  font-size: var(--font-ui-size);
  text-align: left;
}
.tb-vault-item:hover {
  background: var(--background-modifier-hover);
}
.tb-vault-item.is-current {
  color: var(--text-accent);
}
.tb-vault-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.titlebar-search {
  flex: 1 1 auto;
  min-width: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 4px 12px;
}
.ts-box {
  width: min(360px, 100%);
  height: 26px;
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 0 10px;
  border-radius: var(--radius-m);
  border: 1px solid var(--background-modifier-border);
  background: var(--background-primary);
  color: var(--text-faint);
  cursor: pointer;
  transition: border-color var(--anim-fast), background var(--anim-fast);
}
.ts-box:hover {
  border-color: var(--background-modifier-border-hover);
  background: var(--background-primary-alt);
}
.ts-hint {
  flex: 1;
  font-size: var(--font-ui-smaller);
  text-align: left;
}
.ts-kbd {
  font-size: 10px;
  padding: 1px 5px;
  border-radius: var(--radius-s);
  border: 1px solid var(--background-modifier-border);
  color: var(--text-faint);
  white-space: nowrap;
}
.titlebar-drag {
  flex: 0 1 40px;
}
.titlebar-controls {
  display: flex;
}
.tb-btn {
  width: 46px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-muted);
  transition: background var(--anim-fast);
}
.tb-btn:hover {
  background: var(--background-modifier-hover);
  color: var(--text-normal);
}
.tb-close:hover {
  background: #e81123;
  color: #fff;
}
</style>
