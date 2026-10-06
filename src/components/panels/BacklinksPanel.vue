<template>
  <div class="backlinks-panel">
    <div class="emd-panel-header">
      <span>{{ t("bl.title") }}</span>
      <span class="bl-n">{{ backlinks.length }}</span>
    </div>
    <div class="bl-scroll">
      <!-- 反向链接：谁引用了本篇 -->
      <div class="bl-group">{{ t("bl.incoming") }} <span class="bl-n">{{ backlinks.length }}</span></div>
      <div
        v-for="(b, i) in backlinks"
        :key="i"
        class="bl-item"
        @click="jump(b)"
      >
        <div class="bl-title">
          <Icon name="file-text" :size="13" />
          <span>{{ b.source_title }}</span>
        </div>
        <div class="bl-ctx">{{ b.link.alias || b.link.raw }}</div>
      </div>
      <div v-if="backlinks.length === 0" class="bl-empty">{{ t("bl.empty") }}</div>

      <!-- 出链：本篇引用了谁 -->
      <div class="bl-group">{{ t("bl.outgoing") }} <span class="bl-n">{{ outgoing.length }}</span></div>
      <div
        v-for="o in outgoing"
        :key="o.key"
        class="bl-item"
        :class="{ 'is-unresolved': !o.path }"
        @click="openOutgoing(o)"
      >
        <div class="bl-title">
          <Icon name="external-link" :size="13" />
          <span>{{ o.label }}</span>
        </div>
        <div v-if="!o.path" class="bl-ctx">{{ t("bl.unresolved") }}</div>
      </div>
      <div v-if="outgoing.length === 0" class="bl-empty">{{ t("bl.outEmpty") }}</div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import Icon from "../common/Icon.vue";
import { useEditorStore } from "../../stores/editor";
import { useNotesIndexStore } from "../../stores/notesIndex";
import { resolveTarget } from "../../lib/markdown/links";
import type { Backlink, NoteLink } from "../../types";
import { t } from "../../i18n";

const editor = useEditorStore();
const indexStore = useNotesIndexStore();
const backlinks = ref<Backlink[]>([]);

/** 出链条目：解析到目标路径则为可跳转 */
interface OutLink {
  key: string;
  label: string;
  target: string;
  subpath: string | null;
  path: string | null;
}

const outgoing = computed<OutLink[]>(() => {
  const note = indexStore.byPath[editor.activePath];
  if (!note) return [];
  const seen = new Set<string>();
  const out: OutLink[] = [];
  for (const link of note.links) {
    if (!link.target || seen.has(link.target)) continue; // 同页引用/重复目标去重
    seen.add(link.target);
    const res = resolveTarget(link.target, indexStore.notes);
    const title = res.path ? indexStore.byPath[res.path]?.title : null;
    out.push({
      key: link.target,
      label: link.alias || title || link.target,
      target: link.target,
      subpath: link.subpath,
      path: res.path,
    });
  }
  return out;
});

/** 快速切换笔记时丢弃过期响应，避免旧请求晚到覆盖新结果 */
let reqToken = 0;
async function refresh() {
  if (!editor.activePath) {
    backlinks.value = [];
    return;
  }
  const token = ++reqToken;
  const res = await indexStore.backlinksOf(editor.activePath);
  if (token !== reqToken) return;
  backlinks.value = res;
}

watch(
  // notes 引用：索引增量推送（数量不变、内容变化）时也会刷新反链
  () => [editor.activePath, editor.savedContent, indexStore.notes] as const,
  () => refresh(),
  { immediate: true },
);

async function jump(b: Backlink) {
  await editor.openNote(b.source, { line: b.link.line });
}

async function openOutgoing(o: OutLink) {
  if (!o.path) {
    // 未解析目标 → 询问创建（与预览内链行为一致）
    if (confirm(t("pv.createConfirm").replace("{name}", o.target))) {
      const { useVaultStore } = await import("../../stores/vault");
      await useVaultStore().newNote(null, o.target);
    }
    return;
  }
  let jump: { line?: number; anchor?: string } | null = null;
  if (o.subpath) {
    if (o.subpath.startsWith("^")) {
      const blk = indexStore.byPath[o.path]?.block_ids.find(
        (b) => b.id === o.subpath!.slice(1),
      );
      if (blk) jump = { line: blk.line };
    } else {
      jump = { anchor: o.subpath };
    }
  }
  await editor.openNote(o.path, jump);
}
</script>

<style scoped>
.backlinks-panel {
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.bl-n {
  color: var(--text-faint);
  font-size: var(--font-ui-smaller);
}
.bl-scroll {
  flex: 1;
  overflow-y: auto;
  padding: 2px 6px 8px;
}
.bl-group {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 8px 8px 4px;
  font-size: var(--font-ui-smaller);
  color: var(--text-faint);
  font-weight: 600;
  user-select: none;
}
.bl-item {
  padding: 6px 8px;
  border-radius: var(--radius-s);
  cursor: pointer;
}
.bl-item:hover {
  background: var(--background-modifier-hover);
}
.bl-title {
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--text-normal);
}
.bl-item.is-unresolved .bl-title {
  color: var(--text-muted);
}
.bl-ctx {
  color: var(--text-muted);
  font-size: var(--font-ui-smaller);
  margin: 3px 0 0 19px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.bl-empty {
  color: var(--text-faint);
  text-align: center;
  padding: 10px 8px;
  font-size: var(--font-ui-smaller);
}
</style>
