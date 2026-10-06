<template>
  <transition name="emd-fade">
    <div v-if="ui.newNoteOpen" class="emd-modal-bg" @click.self="close">
    <div class="emd-modal nn-modal">
      <div class="nn-title">{{ t("nn.title") }}</div>
      <div class="nn-row">
        <label>{{ t("nn.name") }}</label>
        <input
          ref="nameRef"
          v-model="name"
          type="text"
          :placeholder="t('nn.ph')"
          @keydown.enter="confirm"
          @keydown.esc="close"
        />
      </div>
      <div v-if="templates.length > 0" class="nn-row">
        <label>{{ t("nn.tpl") }}</label>
        <select v-model="template" class="nn-select">
          <option value="">{{ t("nn.noTpl") }}</option>
          <option v-for="t in templates" :key="t.path" :value="t.path">
            {{ t.name }}
          </option>
        </select>
      </div>
      <div class="nn-actions">
        <button class="emd-btn" @click="close">{{ t("c.cancel") }}</button>
        <button class="emd-btn emd-btn-accent" @click="confirm">{{ t("c.create") }}</button>
      </div>
    </div>
  </div>
  </transition>
</template>

<script setup lang="ts">
import { nextTick, onMounted, ref } from "vue";
import { useUiStore } from "../../stores/ui";
import { useVaultStore } from "../../stores/vault";
import { useSettingsStore } from "../../stores/settings";
import { api } from "../../ipc/tauri";
import { t } from "../../i18n";

const ui = useUiStore();
const vault = useVaultStore();
const settings = useSettingsStore();

const name = ref("");
const template = ref("");
const templates = ref<{ name: string; path: string }[]>([]);
const nameRef = ref<HTMLInputElement>();

onMounted(async () => {
  nextTick(() => nameRef.value?.focus());
  // 扫描模板文件夹
  const dir = (settings.data.templates_dir || "templates").replace(/^\/+|\/+$/g, "");
  try {
    const entries = await api.listDir(dir);
    templates.value = entries
      .filter((e) => !e.is_dir && e.kind === "md")
      .map((e) => ({ name: e.name.replace(/\.md$/i, ""), path: e.path }));
  } catch {
    templates.value = [];
  }
});

function applyTemplateVars(text: string, title: string): string {
  const now = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  return text
    .replace(/\{\{title\}\}/gi, title)
    .replace(/\{\{date\}\}/gi, `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`)
    .replace(/\{\{time\}\}/gi, `${pad(now.getHours())}:${pad(now.getMinutes())}`);
}

async function confirm() {
  const title = name.value.trim() || t("nn.untitled");
  let content = "";
  if (template.value) {
    try {
      content = await api.readTextFile(template.value);
      content = applyTemplateVars(content, title);
    } catch {
      content = "";
    }
  }
  close();
  await vault.newNote(ui.newNoteFolder, title, content);
}

function close() {
  ui.newNoteOpen = false;
}
</script>

<style scoped>
.nn-modal {
  width: 420px;
  padding: 16px;
}
.nn-title {
  font-size: var(--font-ui-medium);
  margin-bottom: 12px;
}
.nn-row {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-bottom: 12px;
}
.nn-row label {
  color: var(--text-muted);
  font-size: var(--font-ui-smaller);
}
.nn-select {
  height: var(--input-height);
  background: var(--background-primary);
  border: 1px solid var(--background-modifier-border);
  border-radius: var(--radius-s);
  padding: 0 8px;
  color: var(--text-normal);
}
.nn-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
</style>
