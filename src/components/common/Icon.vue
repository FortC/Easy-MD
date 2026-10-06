// 内联 Lucide 图标（Obsidian 同款图标库，MIT）。禁止 Emoji 当图标。
<template>
  <svg
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="2"
    stroke-linecap="round"
    stroke-linejoin="round"
    class="emd-icon"
    aria-hidden="true"
    v-html="paths[name] || paths['circle']"
  />
</template>

<script setup lang="ts">
import { paths } from "./iconPaths";

withDefaults(defineProps<{ name: string; size?: number }>(), { size: 16 });
</script>

<style scoped>
.emd-icon {
  display: block;
  flex-shrink: 0;
}
</style>
