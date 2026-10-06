<template>
  <div class="ai-pane">
    <div class="st-row">
      <label>{{ t("ai.protocol") }}</label>
      <div class="ai-seg">
        <button :class="{ 'is-on': settings.data.ai_provider === 'openai' }" @click="setProvider('openai')">
          {{ t("ai.openai") }}
        </button>
        <button :class="{ 'is-on': settings.data.ai_provider === 'anthropic' }" @click="setProvider('anthropic')">
          Anthropic
        </button>
      </div>
    </div>
    <div class="st-row">
      <label>{{ t("ai.base") }}</label>
      <input v-model="settings.data.ai_base_url" type="text" placeholder="https://api.openai.com/v1" @change="persist" />
    </div>
    <div class="st-row">
      <label>{{ t("ai.key") }}</label>
      <input v-model="settings.data.ai_api_key" type="password" placeholder="sk-…" @change="persist" />
    </div>
    <div class="st-row">
      <label>{{ t("ai.model") }}</label>
      <input v-model="settings.data.ai_model" type="text" :placeholder="modelPlaceholder" @change="persist" />
    </div>
    <div class="st-row st-actions-row">
      <label>{{ t("ai.conn") }}</label>
      <button class="emd-btn" :disabled="testing" @click="test">
        <Icon name="play" :size="12" /> {{ testing ? t("ai.testing") : t("ai.test") }}
      </button>
      <span v-if="testOk" class="ai-ok">{{ t("ai.ok") }}</span>
    </div>
    <p class="st-tip">
      {{ t("ai.tip2") }}
      
      
    </p>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import Icon from "../common/Icon.vue";
import { useSettingsStore } from "../../stores/settings";
import { api } from "../../ipc/tauri";
import { t } from "../../i18n";

const settings = useSettingsStore();
const testing = ref(false);
const testOk = ref(false);

const modelPlaceholder = computed(() =>
  settings.data.ai_provider === "anthropic" ? "claude-sonnet-4-20250514" : "gpt-4o-mini",
);

function setProvider(p: "openai" | "anthropic") {
  settings.data.ai_provider = p;
  settings.data.ai_base_url =
    p === "anthropic" ? "https://api.anthropic.com" : "https://api.openai.com/v1";
  settings.data.ai_model = p === "anthropic" ? "claude-sonnet-4-20250514" : "gpt-4o-mini";
  testOk.value = false;
  persist();
}

async function persist() {
  await settings.persist();
}

async function test() {
  testing.value = true;
  testOk.value = false;
  try {
    const r = await api.aiChat("请只回复两个字：正常", "这是连通性测试。");
    testOk.value = r.trim().length > 0;
    alert(`${t("ai.reply")}${r.slice(0, 100)}`);
  } catch (e) {
    alert(`${t("ai.testFail")}: ${e}`);
  } finally {
    testing.value = false;
  }
}
</script>

<style scoped>
.ai-pane {
  display: flex;
  flex-direction: column;
}
.ai-pane .st-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 0;
  border-bottom: 1px solid var(--background-modifier-border);
}
.ai-pane .st-row label {
  width: 90px;
  flex-shrink: 0;
  color: var(--text-muted);
}
.ai-pane input {
  flex: 1;
}
.ai-seg {
  display: flex;
  background: var(--interactive-normal);
  border-radius: var(--radius-s);
  padding: 2px;
}
.ai-seg button {
  padding: 4px 14px;
  border-radius: var(--radius-s);
  color: var(--text-muted);
}
.ai-seg button.is-on {
  background: var(--background-primary);
  color: var(--text-normal);
}
.ai-ok {
  color: var(--text-success);
  font-size: var(--font-ui-smaller);
}
.st-tip {
  color: var(--text-faint);
  font-size: var(--font-ui-smaller);
  line-height: 1.6;
  padding: 10px 0 0;
}
</style>
