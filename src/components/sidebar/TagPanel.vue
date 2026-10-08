<template>
  <div class="tag-panel">
    <div class="emd-panel-header"><span>{{ t("tp.tags") }}</span></div>
    <div v-if="!selectedTag" class="tp-list">
      <div v-for="tg in tags" :key="tg.tag" class="emd-tree-item tp-item" @click="selectTag(tg.tag)">
        <Icon name="tag" :size="13" class="tp-icon" />
        <span class="tp-name">#{{ tg.tag }}</span>
        <span class="tp-n">{{ tg.count }}</span>
      </div>
      <div v-if="tags.length === 0" class="tp-empty">{{ t("tp.empty") }}</div>
    </div>
    <div v-else class="tp-results">
      <div class="emd-panel-header">
        <button class="tp-back" @click="selectedTag = ''">
          <Icon name="chevron-left" :size="13" />
        </button>
        <span>#{{ selectedTag }}</span>
        <span class="tp-n">{{ matchNotes.length }}</span>
      </div>
      <div class="tp-notes">
        <div v-for="n in matchNotes" :key="n.path" class="emd-tree-item tp-note-item" @click="openNote(n.path)">
          <Icon name="file-text" :size="13" />
          <div class="tp-note-info">
            <span class="tp-note-title">{{ n.title }}</span>
            <span class="tp-note-path">{{ n.path }}</span>
          </div>
        </div>
        <div v-if="matchNotes.length === 0" class="tp-empty">{{ t("tag.noMatch") }}</div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import Icon from "../common/Icon.vue";
import { useNotesIndexStore } from "../../stores/notesIndex";
import { useEditorStore } from "../../stores/editor";
import { useUiStore } from "../../stores/ui";
import { api } from "../../ipc/tauri";
import { t } from "../../i18n";

const indexStore = useNotesIndexStore();
const editor = useEditorStore();
const ui = useUiStore();
const selectedTag = ref("");
const matchNotes = ref<{ path: string; title: string }[]>([]);

const tags = computed(() => indexStore.tags);

watch(selectedTag, async (tag) => {
  if (!tag) { matchNotes.value = []; return; }
  matchNotes.value = await api.searchByTag(tag);
});

function selectTag(tag: string) { selectedTag.value = tag; }

async function openNote(path: string) {
  await editor.openNote(path);
  ui.view = "editor";
}
</script>

<style scoped>
.tag-panel { flex: 1 1 0%; min-width: 0; width: 100%; height: 100%; display: flex; flex-direction: column; overflow: hidden; }
.tp-list { flex: 1; overflow-y: auto; padding: 2px 6px; }
.tp-item { gap: 8px; }
.tp-icon { color: var(--text-accent); }
.tp-name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.tp-n { color: var(--text-faint); font-size: var(--font-ui-smaller); }
.tp-empty { color: var(--text-faint); padding: 16px 10px; text-align: center; font-size: var(--font-ui-smaller); white-space: pre-line; }
.tp-results { flex: 1; display: flex; flex-direction: column; overflow: hidden; }
.tp-back { padding: 3px; border-radius: var(--radius-s); color: var(--text-muted); }
.tp-back:hover { color: var(--text-normal); background: var(--background-modifier-hover); }
.tp-notes { flex: 1; overflow-y: auto; padding: 2px 6px; }
.tp-note-item { flex-direction: column; align-items: stretch; gap: 2px; height: auto; padding: 6px 8px; width: 100%; }
.tp-note-info { display: flex; flex-direction: column; gap: 1px; min-width: 0; width: 100%; flex: 1; }
.tp-note-title { font-size: var(--font-ui-size); color: var(--text-normal); font-weight: 500; }
.tp-note-path { font-size: var(--font-ui-smaller); color: var(--text-faint); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
