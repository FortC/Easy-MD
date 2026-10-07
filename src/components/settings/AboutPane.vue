<template>
  <div class="ab-head">
    <div class="ab-name">EasyMD</div>
    <p class="ab-tagline">{{ t("st.about.tagline") }}</p>
  </div>
  <div class="st-row">
    <label>{{ t("st.about.version") }}</label>
    <span class="ab-value">v{{ version }}</span>
  </div>
  <div class="st-row">
    <label>{{ t("st.about.author") }}</label>
    <span class="ab-value ab-select">{{ AUTHOR }}</span>
  </div>
  <div class="st-row">
    <label>{{ t("st.about.email") }}</label>
    <span class="ab-value ab-select">{{ EMAIL }}</span>
    <button class="emd-btn" @click="copyEmail">
      <Icon :name="copied ? 'check' : 'copy'" :size="13" />
      {{ copied ? t("st.about.copied") : t("st.about.copy") }}
    </button>
  </div>
  <div class="st-row">
    <label>{{ t("st.about.repo") }}</label>
    <a class="ab-link" :href="REPO" @click.prevent="openRepo">
      <Icon name="external-link" :size="13" /> github.com/FortC/Easy-MD
    </a>
  </div>
  <div class="st-row">
    <label>{{ t("st.about.license") }}</label>
    <span class="ab-value">MIT</span>
  </div>
  <p class="st-tip">{{ t("st.about.tip") }}</p>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import Icon from "../common/Icon.vue";
import { t } from "../../i18n";
import pkg from "../../../package.json";

const AUTHOR = "clb";
const EMAIL = "lamthebest@foxmail.com";
const REPO = "https://github.com/FortC/Easy-MD";

// 与 PreviewView 外链一致：走 window.open 由系统默认浏览器打开
function openRepo() {
  window.open(REPO, "_blank");
}

// 版本运行时从 Tauri 读取，与安装包保持一致；读取失败退回清单版本
const version = ref(pkg.version);
getVersion()
  .then((v) => (version.value = v))
  .catch(() => {});

const copied = ref(false);
let timer: ReturnType<typeof setTimeout> | undefined;

async function copyEmail() {
  try {
    await navigator.clipboard.writeText(EMAIL);
    copied.value = true;
    clearTimeout(timer);
    timer = setTimeout(() => (copied.value = false), 2000);
  } catch {
    // 剪贴板不可用时不提示，邮箱文本本身可选中手动复制
  }
}
</script>

<style scoped>
.ab-head {
  padding: 10px 0 8px;
}
.ab-name {
  font-size: 20px;
  font-weight: 700;
  color: var(--text-normal);
}
.ab-tagline {
  margin-top: 4px;
  color: var(--text-muted);
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
.ab-value {
  color: var(--text-normal);
}
.ab-select {
  user-select: text;
}
.st-tip {
  color: var(--text-muted);
  padding: 6px 0;
}
.ab-link {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  color: var(--link-color);
}
.ab-link:hover {
  color: var(--link-color-hover);
}
</style>
