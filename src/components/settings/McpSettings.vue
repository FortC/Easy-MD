<template>
  <div class="mcp-pane">
    <div v-if="configs.length === 0" class="mcp-empty">
      <Icon name="cloud" :size="28" />
      <p>{{ t("mcp.empty") }}</p>
      <button class="emd-btn emd-btn-accent" @click="startAdd">
        <Icon name="plus" :size="13" /> {{ t("mcp.add") }}
      </button>
    </div>

    <template v-else>
      <div
        v-for="c in configs"
        :key="c.name"
        class="mcp-item"
        :class="{ 'is-active': c.name === settings.data.active_mcp }"
      >
        <div class="mcp-head">
          <button class="mcp-radio" :title="c.name === settings.data.active_mcp ? t('mcp.active') : t('mcp.switchTo')" @click="setActive(c.name)">
            <span class="mcp-dot" />
          </button>
          <span class="mcp-name">{{ c.name }}</span>
          <span class="mcp-cmd" :title="c.command + ' ' + c.args.join(' ')">
            {{ c.command }} {{ c.args.join(" ") }}
          </span>
          <button class="mcp-op" :title="t('mcp.test')" @click="testConfig(c)">
            <Icon name="play" :size="12" />
          </button>
          <button class="mcp-op" :title="t('c.edit')" @click="startEdit(c)">
            <Icon name="pencil" :size="12" />
          </button>
          <button class="mcp-op mcp-op-danger" :title="t('c.delete')" @click="removeConfig(c.name)">
            <Icon name="trash-2" :size="12" />
          </button>
        </div>
        <div v-if="testResults[c.name]" class="mcp-test" :class="{ 'is-ok': testResults[c.name].ok }">
          {{ testResults[c.name].text }}
        </div>
      </div>
      <div class="mcp-toolbar">
        <button class="emd-btn" @click="startAdd">
          <Icon name="plus" :size="13" /> {{ t("mcp.addShort") }}
        </button>
        <button
          class="emd-btn emd-btn-accent"
          :disabled="!settings.data.active_mcp || sync.running"
          @click="syncNow"
        >
          <Icon name="refresh-cw" :size="13" /> {{ sync.running ? sync.message || t("mcp.syncing") : t("mcp.syncNow") }}
        </button>
      </div>
      <div class="mcp-schedule">
        <label>{{ t("mcp.schedule") }}</label>
        <input
          v-model.number="settings.data.sync_interval_minutes"
          type="text"
          style="width: 80px"
          @change="persist"
        />
      </div>
      <p v-if="sync.running && sync.total > 0" class="mcp-msg">
        {{ t("mcp.progress") }}: {{ sync.done }} / {{ sync.total }}
      </p>
      <p v-if="sync.lastReport" class="mcp-msg" :class="{ 'is-err': !!sync.lastError }">
        {{ sync.lastReport }}
      </p>
      <p v-if="sync.lastError" class="mcp-msg is-err" style="white-space: pre-wrap">{{ sync.lastError }}</p>
    </template>

    <transition name="emd-fade">
      <div v-if="editing" class="emd-modal-bg nested" @click.self="editing = false">
      <div class="emd-modal mcp-edit">
        <div class="me-title">{{ editingOriginal ? t("mcp.editTitle") : t("mcp.addTitle") }}</div>
        <div class="me-row">
          <label>{{ t("mcp.name") }}</label>
          <input v-model="form.name" type="text" :placeholder="t('mcp.namePh')" />
        </div>
        <div class="me-row">
          <label>{{ t("mcp.command") }}</label>
          <input v-model="form.command" type="text" :placeholder="t('mcp.cmdPh')" />
        </div>
        <div class="me-row">
          <label>{{ t("mcp.args") }}</label>
          <input v-model="form.argsText" type="text" :placeholder="t('mcp.argsPh')" />
        </div>
        <div class="me-row">
          <label>{{ t("mcp.env") }}</label>
          <textarea v-model="form.envText" class="emd-input me-env" rows="4" :placeholder="t('mcp.envPh')" />
        </div>
        <div class="me-actions">
          <button class="emd-btn" @click="editing = false">{{ t("c.cancel") }}</button>
          <button class="emd-btn emd-btn-accent" @click="saveConfig">{{ t("c.save") }}</button>
        </div>
      </div>
    </div>
    </transition>
  </div>
</template>

<script setup lang="ts">
import { computed, reactive, ref } from "vue";
import Icon from "../common/Icon.vue";
import { useSettingsStore } from "../../stores/settings";
import { useSyncStore } from "../../stores/sync";
import { t, tf } from "../../i18n";
import { api } from "../../ipc/tauri";
import type { McpServerConfig } from "../../types";

const settings = useSettingsStore();
const sync = useSyncStore();
const configs = computed(() => settings.data.mcp_configs);

const editing = ref(false);
const editingOriginal = ref<string | null>(null);
const form = reactive({
  name: "",
  command: "",
  argsText: "",
  envText: "",
});
const testResults = reactive<Record<string, { ok: boolean; text: string }>>({});

function startAdd() {
  Object.assign(form, { name: "", command: "", argsText: "", envText: "" });
  editingOriginal.value = null;
  editing.value = true;
}

function startEdit(c: { name: string; command: string; args: string[]; env: Record<string, string> }) {
  Object.assign(form, {
    name: c.name,
    command: c.command,
    argsText: c.args.join(" "),
    envText: Object.entries(c.env)
      .map(([k, v]) => `${k}=${v}`)
      .join("\n"),
  });
  editingOriginal.value = c.name;
  editing.value = true;
}

async function saveConfig() {
  const env: Record<string, string> = {};
  for (const line of form.envText.split("\n")) {
    const i = line.indexOf("=");
    if (i > 0) env[line.slice(0, i).trim()] = line.slice(i + 1).trim();
  }
  const cfg = {
    name: form.name.trim(),
    command: form.command.trim(),
    args: form.argsText.split(/\s+/).filter(Boolean),
    env,
    enabled: true,
    tool_map: {},
  };
  if (!cfg.name || !cfg.command) return;
  const list = [...configs.value];
  const idx = list.findIndex((c) => c.name === editingOriginal.value);
  if (idx >= 0) list[idx] = cfg;
  else list.push(cfg);
  settings.data.mcp_configs = list;
  if (!settings.data.active_mcp) settings.data.active_mcp = cfg.name;
  await settings.persist();
  editing.value = false;
}

async function removeConfig(name: string) {
  settings.data.mcp_configs = settings.data.mcp_configs.filter((c) => c.name !== name);
  if (settings.data.active_mcp === name) {
    settings.data.active_mcp = settings.data.mcp_configs[0]?.name || null;
  }
  await settings.persist();
}

async function setActive(name: string) {
  settings.data.active_mcp = name;
  await settings.persist();
}

async function testConfig(c: McpServerConfig) {
  testResults[c.name] = { ok: false, text: t("mcp.connecting") };
  try {
    const res = await api.mcpTest(c);
    if (res.ok) {
      const names = res.tools.map((t) => t.name).join("、");
      testResults[c.name] = {
        ok: true,
        text: tf("mcp.connOk", { s: res.server_name, tools: names || t("mcp.noTools") }),
      };
    } else {
      testResults[c.name] = { ok: false, text: `${t("mcp.connFail")}: ${res.error}` };
    }
  } catch (e) {
    testResults[c.name] = { ok: false, text: `${t("mcp.connFail")}: ${e}` };
  }
}

async function syncNow() {
  await sync.syncNow();
}

async function persist() {
  await settings.persist();
}
</script>

<style scoped>
.mcp-pane {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.mcp-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  color: var(--text-faint);
  padding: 40px 0;
  text-align: center;
}
.mcp-item {
  border: 1px solid var(--background-modifier-border);
  border-radius: var(--radius-m);
  padding: 8px 10px;
}
.mcp-item.is-active {
  border-color: var(--interactive-accent);
}
.mcp-head {
  display: flex;
  align-items: center;
  gap: 8px;
}
.mcp-radio {
  width: 14px;
  height: 14px;
  border-radius: 50%;
  border: 2px solid var(--text-faint);
  display: flex;
  align-items: center;
  justify-content: center;
}
.mcp-item.is-active .mcp-radio {
  border-color: var(--interactive-accent);
}
.mcp-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: transparent;
}
.mcp-item.is-active .mcp-dot {
  background: var(--interactive-accent);
}
.mcp-name {
  font-weight: 600;
}
.mcp-cmd {
  flex: 1;
  color: var(--text-faint);
  font-family: var(--font-mono);
  font-size: var(--font-ui-smaller);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.mcp-op {
  padding: 4px;
  border-radius: var(--radius-s);
  color: var(--text-muted);
}
.mcp-op:hover {
  background: var(--background-modifier-hover);
}
.mcp-op-danger:hover {
  color: var(--text-error);
}
.mcp-toolbar {
  display: flex;
  gap: 8px;
}
.mcp-schedule {
  display: flex;
  align-items: center;
  gap: 10px;
  color: var(--text-muted);
  font-size: var(--font-ui-smaller);
}
.mcp-test {
  margin-top: 6px;
  color: var(--text-error);
  font-size: var(--font-ui-smaller);
  white-space: pre-wrap;
  word-break: break-all;
}
.mcp-test.is-ok {
  color: var(--text-success);
}
.mcp-msg {
  font-size: var(--font-ui-smaller);
  color: var(--text-success);
}
.mcp-msg.is-err {
  color: var(--text-error);
}
.nested {
  position: absolute;
  inset: 0;
}
.mcp-edit {
  width: 480px;
  padding: 16px;
  gap: 10px;
}
.me-title {
  font-size: var(--font-ui-medium);
  margin-bottom: 6px;
}
.me-row {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-bottom: 10px;
}
.me-row label {
  color: var(--text-muted);
  font-size: var(--font-ui-smaller);
}
.me-env {
  height: auto;
  padding: 6px 8px;
  font-family: var(--font-mono);
  font-size: var(--font-ui-smaller);
  resize: vertical;
}
.me-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
</style>
