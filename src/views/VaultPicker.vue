<template>
  <div class="vault-picker">
    <div class="vp-card">
      <div class="vp-logo">
        <svg viewBox="0 0 48 48" width="56" height="56">
          <rect x="4" y="4" width="40" height="40" rx="10" fill="#8b6cef" />
          <path
            d="M17 14h4v20h-4zM17 14h14v4H17zM17 23h11v4H17zM17 30h14v4H17z"
            fill="#fff"
          />
        </svg>
      </div>
      <h1 class="vp-title">EasyMD</h1>
      <p class="vp-sub">{{ t("vp.sub") }}</p>

      <div class="vp-list">
        <div class="emd-panel-header">
          <span>{{ t("vp.myVaults") }}</span>
        </div>
        <div v-if="vaultList.length === 0" class="vp-empty">
          {{ t("vp.empty") }}
        </div>
        <div
          v-for="v in vaultList"
          :key="v.path"
          class="emd-tree-item vp-item"
          @click="open(v.path)"
        >
          <Icon name="book" :size="16" />
          <span class="vp-name">{{ v.name }}</span>
          <span class="vp-path">{{ v.path }}</span>
          <button class="vp-remove" :title="t('vp.remove')" @click.stop="forget(v.path)">
            <Icon name="x" :size="12" />
          </button>
        </div>
      </div>

      <div class="vp-actions">
        <button class="emd-btn emd-btn-accent" @click="pickOpen">
          <Icon name="folder" :size="14" /> {{ t("vp.open") }}
        </button>
        <button class="emd-btn" @click="pickCreate">
          <Icon name="folder-plus" :size="14" /> {{ t("vp.create") }}
        </button>
      </div>
      <div v-if="error" class="vp-error">{{ error }}</div>
    </div>

    <InputDialog
      v-if="creating"
      :title="t('vp.newName')"
      :placeholder="t('vp.namePh')"
      @confirm="doCreate"
      @cancel="creating = false"
    />
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from "vue";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import Icon from "../components/common/Icon.vue";
import InputDialog from "../components/common/InputDialog.vue";
import { useVaultStore } from "../stores/vault";
import { t } from "../i18n";

const vault = useVaultStore();
const vaultList = ref(vault.vaultList);
const error = ref("");
const creating = ref(false);
const createParent = ref("");

onMounted(async () => {
  await vault.loadVaultList();
  vaultList.value = vault.vaultList;
});

async function open(path: string) {
  error.value = "";
  try {
    await vault.open(path);
  } catch (e) {
    error.value = String(e);
  }
}

async function pickOpen() {
  const dir = await openDialog({ directory: true, title: t("vp.pickOpen") });
  if (typeof dir === "string" && dir) await open(dir);
}

async function pickCreate() {
  const dir = await openDialog({ directory: true, title: t("vp.pickCreate") });
  if (typeof dir === "string" && dir) {
    createParent.value = dir;
    creating.value = true;
  }
}

async function doCreate(name: string) {
  creating.value = false;
  const path = `${createParent.value.replace(/[\\/]+$/, "")}/${name}`;
  error.value = "";
  try {
    await vault.create(path);
  } catch (e) {
    error.value = String(e);
  }
}

async function forget(path: string) {
  await vault.forget(path);
  vaultList.value = vault.vaultList;
}
</script>

<style scoped>
.vault-picker {
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--background-secondary);
}
.vp-card {
  width: 520px;
  max-width: 90vw;
  background: var(--background-primary);
  border: 1px solid var(--background-modifier-border);
  border-radius: var(--radius-l);
  padding: 28px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.vp-logo {
  display: flex;
  justify-content: center;
}
.vp-title {
  text-align: center;
  font-size: 22px;
  font-weight: 700;
  color: var(--text-normal);
}
.vp-sub {
  text-align: center;
  color: var(--text-muted);
  margin-top: -10px;
}
.vp-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
  max-height: 240px;
  overflow-y: auto;
}
.vp-empty {
  color: var(--text-faint);
  padding: 12px 8px;
  text-align: center;
}
.vp-item {
  gap: 8px;
}
.vp-name {
  font-weight: 500;
}
.vp-path {
  flex: 1;
  color: var(--text-faint);
  font-size: var(--font-ui-smaller);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.vp-remove {
  opacity: 0;
  color: var(--text-muted);
  padding: 4px;
  border-radius: var(--radius-s);
}
.vp-item:hover .vp-remove {
  opacity: 1;
}
.vp-remove:hover {
  color: var(--text-error);
}
.vp-actions {
  display: flex;
  justify-content: center;
  gap: 10px;
}
.vp-error {
  color: var(--text-error);
  text-align: center;
  font-size: var(--font-ui-smaller);
}
</style>
