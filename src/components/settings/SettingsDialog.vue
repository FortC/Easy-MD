<template>
  <transition name="emd-fade">
    <div v-if="ui.settingsOpen" class="emd-modal-bg" @click.self="ui.settingsOpen = false">
    <div class="emd-modal st-modal">
      <div class="st-tabs">
        <button
          v-for="t in tabs"
          :key="t.key"
          class="st-tab"
          :class="{ 'is-active': tab === t.key }"
          @click="tab = t.key"
        >
          {{ t.label }}
        </button>
      </div>
      <div class="st-body">
        <transition name="emd-pane" mode="out-in">
        <!-- 通用 -->
        <div v-if="tab === 'general'" key="general" class="st-pane">
          <div class="st-row">
            <label>{{ t("st.language") }}</label>
            <div class="st-seg">
              <button :class="{ 'is-on': settings.data.language !== 'en' }" @click="settings.setLanguage('zh')">
                简体中文
              </button>
              <button :class="{ 'is-on': settings.data.language === 'en' }" @click="settings.setLanguage('en')">
                English (US)
              </button>
            </div>
          </div>
          <div class="st-row">
            <label>{{ t("st.fontSize") }}</label>
            <div class="st-seg">
              <button v-for="fs in fontScales" :key="fs.key" :class="{ 'is-on': settings.data.font_scale === fs.key }" @click="settings.setFontScale(fs.key)">
                {{ t(fs.label) }}
              </button>
            </div>
          </div>
          <div class="st-row">
            <label>{{ t("st.theme") }}</label>
            <div class="st-seg">
              <button :class="{ 'is-on': settings.data.theme === 'dark' }" @click="settings.setTheme('dark')">
                {{ t('st.dark') }}
              </button>
              <button :class="{ 'is-on': settings.data.theme === 'light' }" @click="settings.setTheme('light')">
                {{ t('st.light') }}
              </button>
            </div>
          </div>
          <div class="st-row">
            <label>{{ t("st.attach") }}</label>
            <input v-model="settings.data.attachments_dir" type="text" @change="persist" />
          </div>
          <div class="st-row">
            <label>{{ t("st.tplDir") }}</label>
            <input v-model="settings.data.templates_dir" type="text" @change="persist" />
          </div>
          <div class="st-row">
            <label>{{ t("st.dailyDir") }}</label>
            <input v-model="settings.data.daily_dir" type="text" @change="persist" />
          </div>
          <div class="st-row">
            <label>{{ t("st.dailyFmt") }}</label>
            <input v-model="settings.data.daily_format" type="text" @change="persist" />
          </div>
          <div class="st-row">
            <label>{{ t("st.dailyTpl") }}</label>
            <select v-model="settings.data.daily_template" class="st-select" @change="persist">
              <option :value="null">{{ t("st.noTpl") }}</option>
              <option v-for="t in dailyTemplates" :key="t.path" :value="t.name">{{ t.name }}</option>
            </select>
          </div>
          <div class="st-row">
            <label>{{ t("st.dailyTag") }}</label>
            <input
              v-model="settings.data.daily_tag"
              type="text"
              :placeholder="t('st.dailyTagPh')"
              @change="persist"
            />
          </div>
          <div class="st-row st-actions-row">
            <label>{{ t("st.index") }}</label>
            <button class="emd-btn" @click="rebuild">
              <Icon name="refresh-cw" :size="13" /> {{ t("st.rebuild") }}
            </button>
            <span v-if="rebuilt" class="st-ok">{{ tf("st.rebuilt", { n: count }) }}</span>
          </div>
          <div class="st-row">
            <label>{{ t("st.osOpen") }}</label>
            <select v-model="settings.data.os_open_mode" class="st-select" @change="persist">
              <option value="ask">{{ t("st.osAsk") }}</option>
              <option value="vault">{{ t("st.osVault") }}</option>
              <option value="temp">{{ t("st.osTemp") }}</option>
            </select>
          </div>
          <div class="st-row st-actions-row">
            <label>{{ t("st.ctx") }}</label>
            <button class="emd-btn" @click="regMenu(true)">
              <Icon name="check" :size="13" /> {{ t('st.ctxReg') }}
            </button>
            <button class="emd-btn" @click="regMenu(false)">
              <Icon name="x" :size="13" /> {{ t("st.ctxUnreg") }}
            </button>
          </div>
          <p class="st-tip">
            {{ t("st.ctxTip") }}</p>
        </div>

        <!-- 外观（CSS 片段） -->
        <div v-if="tab === 'appearance'" key="appearance" class="st-pane">
          <p class="st-tip">
            {{ t("st.snippetDir") }}<code>{{ settings.snippetsDir }}</code
            >{{ t("st.snippetTip2") }}
          </p>
          <div v-for="s in settings.data.css_snippets" :key="s.name" class="st-row">
            <label>{{ s.name }}</label>
            <label class="st-switch">
              <input
                type="checkbox"
                :checked="s.enabled"
                @change="toggleSnippet(s.name)"
              />
              {{ t("st.enable") }}
            </label>
            <button class="emd-btn" @click="removeSnippet(s.name)">{{ t("c.delete") }}</button>
          </div>
          <div class="st-row">
            <label>{{ t("st.addSnippet") }}</label>
            <button class="emd-btn" @click="addSnippet">
              <Icon name="plus" :size="13" /> {{ t("st.newSnippet") }}
            </button>
          </div>
        </div>

        <!-- 模板管理 -->
        <div v-if="tab === 'templates'" key="templates" class="st-pane">
          <TemplatePane />
        </div>

        <!-- AI -->
        <div v-if="tab === 'ai'" key="ai" class="st-pane">
          <AiPane />
        </div>

        <!-- MCP 同步 -->
        <div v-if="tab === 'mcp'" key="mcp" class="st-pane">
          <McpSettings />
        </div>

        <!-- 关于 -->
        <div v-if="tab === 'about'" key="about" class="st-pane">
          <AboutPane />
        </div>
        </transition>
      </div>
    </div>
  </div>
  </transition>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import Icon from "../common/Icon.vue";
import McpSettings from "./McpSettings.vue";
import TemplatePane from "./TemplatePane.vue";
import AiPane from "./AiPane.vue";
import AboutPane from "./AboutPane.vue";
import { useUiStore } from "../../stores/ui";
import { useSettingsStore } from "../../stores/settings";
import { useNotesIndexStore } from "../../stores/notesIndex";
import { api } from "../../ipc/tauri";
import { listTemplates, type TemplateFile } from "../../lib/template";
import { t, tf } from "../../i18n";

const ui = useUiStore();
const settings = useSettingsStore();
const indexStore = useNotesIndexStore();
const tab = ref("general");
const rebuilt = ref(false);
const count = ref(0);
const dailyTemplates = ref<TemplateFile[]>([]);

const fontScales = [
  { key: "xs", label: "st.fsXs" },
  { key: "sm", label: "st.fsSm" },
  { key: "md", label: "st.fsMd" },
  { key: "lg", label: "st.fsLg" },
  { key: "xl", label: "st.fsXl" },
];

// computed：切换语言时页签文案同步更新
const tabs = computed(() => [
  { key: "general", label: t("st.general") },
  { key: "templates", label: t("st.templates") },
  { key: "ai", label: t("st.ai") },
  { key: "appearance", label: t("st.appearance") },
  { key: "mcp", label: t("st.mcp") },
  { key: "about", label: t("st.about") },
]);

onMounted(async () => {
  dailyTemplates.value = await listTemplates();
});

async function persist() {
  await settings.persist();
}

async function rebuild() {
  count.value = (await indexStore.rebuild()).length;
  rebuilt.value = true;
  setTimeout(() => (rebuilt.value = false), 4000);
}

async function setDefaultApp() {
  try {
    await api.openDefaultAppsSettings();
  } catch (e) {
    alert(String(e));
  }
}

async function regMenu(register: boolean) {
  try {
    if (register) await api.registerContextMenu();
    else await api.unregisterContextMenu();
    alert(register ? t("st.ctxOk") : t("st.ctxRemoved"));
  } catch (e) {
    alert(`${t("st.opFail")}: ${e}`);
  }
}

async function toggleSnippet(name: string) {
  const s = settings.data.css_snippets.find((x) => x.name === name);
  if (s) s.enabled = !s.enabled;
  await settings.persist();
  window.dispatchEvent(new CustomEvent("emd-snippets-changed"));
}

async function removeSnippet(name: string) {
  await api.deleteSnippetFile(name);
  settings.data.css_snippets = settings.data.css_snippets.filter(
    (x) => x.name !== name,
  );
  await settings.persist();
  window.dispatchEvent(new CustomEvent("emd-snippets-changed"));
}

async function addSnippet() {
  const name = prompt(t("st.snippetName"));
  if (!name) return;
  try {
    await api.createSnippetFile(name.trim());
  } catch (e) {
    alert(String(e));
    return;
  }
  settings.data.css_snippets.push({ name: name.trim() + ".css", enabled: true });
  await settings.persist();
  window.dispatchEvent(new CustomEvent("emd-snippets-changed"));
}
</script>

<style scoped>
.st-modal {
  width: 640px;
  height: 480px;
}
.st-tabs {
  display: flex;
  gap: 4px;
  padding: 10px 12px 0;
  border-bottom: 1px solid var(--background-modifier-border);
}
.st-tab {
  padding: 6px 14px;
  border-radius: var(--radius-s);
  color: var(--text-muted);
}
.st-tab:hover {
  background: var(--background-modifier-hover);
}
.st-tab.is-active {
  color: var(--text-normal);
  box-shadow: inset 0 -2px 0 var(--interactive-accent);
}
.st-body {
  flex: 1;
  overflow-y: auto;
}
.st-pane {
  padding: 14px 16px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.st-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 0;
  border-bottom: 1px solid var(--background-modifier-border);
}
.st-row label:first-child {
  width: 190px;
  flex-shrink: 0;
  color: var(--text-muted);
}
.st-row input[type="text"] {
  flex: 1;
}
.st-select {
  flex: 1;
  height: var(--input-height);
  background: var(--background-primary);
  border: 1px solid var(--background-modifier-border);
  border-radius: var(--radius-s);
  padding: 0 8px;
  color: var(--text-normal);
}
.st-seg {
  display: flex;
  background: var(--interactive-normal);
  border-radius: var(--radius-s);
  padding: 2px;
}
.st-seg button {
  padding: 4px 14px;
  border-radius: var(--radius-s);
  color: var(--text-muted);
}
.st-seg button.is-on {
  background: var(--background-primary);
  color: var(--text-normal);
}
.st-ok {
  color: var(--text-success);
  font-size: var(--font-ui-smaller);
}
.st-tip {
  color: var(--text-muted);
  padding: 6px 0;
}
.st-tip code {
  background: var(--code-background);
  border-radius: var(--radius-s);
  padding: 2px 6px;
  font-family: var(--font-mono);
  font-size: var(--font-ui-smaller);
  user-select: text;
}
.st-switch {
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--text-muted);
}
</style>
