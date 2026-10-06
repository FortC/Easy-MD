<template>
  <transition name="emd-fade">
    <div v-if="ai.open" class="emd-modal-bg" @click.self="ai.close()">
      <div class="emd-modal ai-dialog">
        <div class="ai-head">
          <span class="ai-title">{{ t("ai.assistant") }}</span>
          <span class="ai-ctx">
            <Icon name="pencil" :size="11" />
            {{ ai.contextLabel }}
          </span>
          <button class="ai-close" :title="t('c.close')" @click="ai.close()">
            <Icon name="x" :size="14" />
          </button>
        </div>

        <!-- 快捷操作 -->
        <div class="ai-actions">
          <button class="emd-btn" :disabled="ai.running" @click="ai.run('explain')">{{ t("ai.explain") }}</button>
          <button class="emd-btn" :disabled="ai.running" @click="ai.run('summarize')">{{ t("ai.summarize") }}</button>
          <button class="emd-btn" :disabled="ai.running" @click="ai.run('translate')">{{ t("ai.translate") }}</button>
          <button class="emd-btn" :disabled="ai.running" @click="ai.run('polish')">{{ t("ai.polish") }}</button>
          <button class="emd-btn" :disabled="ai.running" @click="ai.run('continue')">{{ t("ai.continue") }}</button>
        </div>

        <!-- 自定义提问 -->
        <div class="ai-ask">
          <input
            v-model="ai.prompt"
            type="text"
            :placeholder="t('ai.askPh')"
            :disabled="ai.running"
            @keydown.enter="ai.run('ask')"
          />
          <button class="emd-btn emd-btn-accent" :disabled="ai.running || !ai.prompt.trim()" @click="ai.run('ask')">
            {{ t("ai.ask") }}
          </button>
        </div>

        <!-- 结果 -->
        <div class="ai-result">
          <div v-if="ai.running" class="ai-loading">
            <Icon name="refresh-cw" :size="14" /> {{ t("ai.thinking") }}
          </div>
          <div v-else-if="ai.error" class="ai-error">{{ ai.error }}</div>
          <pre v-else-if="ai.result" class="ai-text">{{ ai.result }}</pre>
          <div v-else class="ai-placeholder">
            {{ t("ai.ph") }}
            {{ ai.selection ? t("ai.ctxSel") : t("ai.ctxNote") }}
          </div>
        </div>

        <!-- 结果操作 -->
        <div v-if="ai.result && !ai.running" class="ai-result-actions">
          <button class="emd-btn" @click="ai.copyResult()"><Icon name="copy" :size="12" /> {{ t("c.copy") }}</button>
          <button v-if="ai.selection" class="emd-btn" @click="ai.replaceSelection">{{ t("ai.replace") }}</button>
          <button class="emd-btn emd-btn-accent" @click="ai.insertResult">{{ t("ai.insert") }}</button>
        </div>
      </div>
    </div>
  </transition>
</template>

<script setup lang="ts">
import Icon from "../common/Icon.vue";
import { useAiStore } from "../../stores/ai";
import { t } from "../../i18n";

const ai = useAiStore();
</script>

<style scoped>
.ai-dialog {
  width: 620px;
  height: 480px;
  padding: 0;
}
.ai-head {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 16px;
  border-bottom: 1px solid var(--background-modifier-border);
  flex-shrink: 0;
}
.ai-title {
  font-size: var(--font-ui-medium);
  font-weight: 600;
}
.ai-ctx {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  color: var(--text-accent);
  background: var(--interactive-accent-hover-alt);
  border-radius: var(--radius-m);
  padding: 2px 10px;
  font-size: var(--font-ui-smaller);
}
.ai-close {
  margin-left: auto;
  width: 26px;
  height: 26px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-s);
  color: var(--text-muted);
}
.ai-close:hover {
  background: var(--background-modifier-hover);
  color: var(--text-normal);
}
.ai-actions {
  display: flex;
  gap: 6px;
  padding: 10px 16px 0;
  flex-wrap: wrap;
  flex-shrink: 0;
}
.ai-ask {
  display: flex;
  gap: 8px;
  padding: 10px 16px 0;
  flex-shrink: 0;
}
.ai-result {
  flex: 1;
  margin: 10px 16px;
  border: 1px solid var(--background-modifier-border);
  border-radius: var(--radius-m);
  background: var(--background-primary);
  overflow-y: auto;
  padding: 12px 14px;
  min-height: 0;
}
.ai-loading {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--text-muted);
}
.ai-error {
  color: var(--text-error);
  white-space: pre-wrap;
  word-break: break-all;
}
.ai-text {
  font-family: var(--font-text);
  font-size: 13px;
  line-height: 1.65;
  white-space: pre-wrap;
  word-break: break-word;
  color: var(--text-normal);
  user-select: text;
}
.ai-placeholder {
  color: var(--text-faint);
  font-size: var(--font-ui-smaller);
}
.ai-result-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 0 16px 12px;
  flex-shrink: 0;
}
</style>
