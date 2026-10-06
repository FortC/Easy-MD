<template>
  <div class="tpl-pane">
    <div class="tpl-toolbar">
      <button class="emd-btn emd-btn-accent" @click="createTpl">
        <Icon name="plus" :size="13" /> {{ t("tp2.create") }}
      </button>
      <button class="emd-btn" @click="refresh">
        <Icon name="refresh-cw" :size="13" /> {{ t("tp2.refresh") }}
      </button>
      <span class="tpl-tip">{{ tf("tp2.tip", { dir: dirLabel, vars: varsHint }) }}</span>
    </div>

    <div class="tpl-list">
      <div v-for="tp in templates" :key="tp.path" class="tpl-item">
        <Icon name="file-text" :size="14" />
        <span class="tpl-name" :title="tp.path">{{ tp.name }}</span>
        <button class="tpl-op" :title="t('c.edit')" @click="editTpl(tp)">
          <Icon name="pencil" :size="12" />
        </button>
        <button class="tpl-op tpl-op-danger" :title="t('c.delete')" @click="deleteTpl(tp)">
          <Icon name="trash-2" :size="12" />
        </button>
      </div>
      <div v-if="templates.length === 0" class="tpl-empty">
        {{ t("tp2.empty") }}
      </div>
    </div>

    <!-- 编辑弹窗 -->
    <transition name="emd-fade">
      <div v-if="editing" class="emd-modal-bg" @click.self="editing = null">
        <div class="emd-modal tpl-edit">
          <div class="tpl-edit-title">{{ tf("tp2.edit", { name: editing.name }) }}</div>
          <textarea
            v-model="editingContent"
            class="emd-input tpl-textarea"
            :placeholder="tf('tp2.ph', { vars: varsHint })"
          />
          <div class="tpl-edit-actions">
            <button class="emd-btn" @click="editing = null">{{ t("c.cancel") }}</button>
            <button class="emd-btn emd-btn-accent" @click="saveTpl">{{ t("c.save") }}</button>
          </div>
        </div>
      </div>
    </transition>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import Icon from "../common/Icon.vue";
import { api } from "../../ipc/tauri";
import { useSettingsStore } from "../../stores/settings";
import { useVaultStore } from "../../stores/vault";
import { listTemplates, type TemplateFile } from "../../lib/template";
import { t, tf } from "../../i18n";

const settings = useSettingsStore();
const vault = useVaultStore();
const templates = ref<TemplateFile[]>([]);
const editing = ref<TemplateFile | null>(null);
const editingContent = ref("");

const dir = () => (settings.data.templates_dir || "templates").replace(/^\/+|\/+$/g, "");
const dirLabel = computed(() => dir());
const varsHint = ["{{title}}", "{{date}}", "{{time}}"].join("  ");

async function refresh() {
  templates.value = await listTemplates();
}
onMounted(refresh);

async function createTpl() {
  const name = prompt(t("tp2.namePh"));
  if (!name) return;
  const rel = await api.createNote(dir(), name.trim(), "");
  await vault.refreshParents(rel);
  await refresh();
  // 创建后直接进入编辑
  const created = templates.value.find((x) => x.path === rel);
  if (created) editTpl(created);
}

function editTpl(tp: TemplateFile) {
  editing.value = tp;
  api.readTextFile(tp.path).then((c) => (editingContent.value = c));
}

async function saveTpl() {
  if (!editing.value) return;
  await api.writeTextFile(editing.value.path, editingContent.value);
  editing.value = null;
}

async function deleteTpl(tp: TemplateFile) {
  if (!confirm(tf("tp2.del", { name: t.name }))) return;
  await api.deletePath(tp.path);
  await vault.refreshParents(tp.path);
  await refresh();
}
</script>

<style scoped>
.tpl-pane {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.tpl-toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}
.tpl-tip {
  color: var(--text-faint);
  font-size: var(--font-ui-smaller);
}
.tpl-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.tpl-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 7px 10px;
  border-radius: var(--radius-s);
  color: var(--text-normal);
}
.tpl-item:hover {
  background: var(--background-modifier-hover);
}
.tpl-name {
  flex: 1;
  font-weight: 500;
}
.tpl-op {
  padding: 4px;
  border-radius: var(--radius-s);
  color: var(--text-muted);
}
.tpl-op:hover {
  background: var(--background-modifier-hover);
}
.tpl-op-danger:hover {
  color: var(--text-error);
}
.tpl-empty {
  color: var(--text-faint);
  text-align: center;
  padding: 28px 0;
}
.tpl-edit {
  width: 640px;
  height: 480px;
  padding: 16px;
}
.tpl-edit-title {
  font-size: var(--font-ui-medium);
  margin-bottom: 12px;
}
.tpl-textarea {
  flex: 1;
  height: auto;
  resize: none;
  font-family: var(--font-text);
  line-height: 1.6;
  padding: 10px 12px;
}
.tpl-edit-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 12px;
}
</style>
