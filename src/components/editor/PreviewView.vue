<template>
  <div ref="previewEl" class="md-preview markdown-rendered" @click="onClick" />
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { api } from "../../ipc/tauri";
import { useEditorStore } from "../../stores/editor";
import { useNotesIndexStore } from "../../stores/notesIndex";
import { useSettingsStore } from "../../stores/settings";
import { useUiStore } from "../../stores/ui";
import {
  emdAssetUrl,
  emdExternalUrl,
  extractBlock,
  extractSection,
  fileUrlToPath,
  renderMarkdown,
  resolveImageSrc,
  slugify,
} from "../../lib/markdown/renderer";
import { isImageName, resolveTarget } from "../../lib/markdown/links";
import { t } from "../../i18n";
import { clamp01, isScrollSyncLocked, lockScrollSync } from "../../lib/scrollSync";

const editor = useEditorStore();
const indexStore = useNotesIndexStore();
const settings = useSettingsStore();
const ui = useUiStore();
const previewEl = ref<HTMLElement>();
const emit = defineEmits<{ jump: [target: { path?: string; anchor?: string; line?: number }] }>();

/** 当前渲染上下文（html 与 fillEmbeds 共用） */
const renderOpts = computed(() => ({
  resolveNote: (tgt: string) => resolveTarget(tgt, indexStore.notes).path,
  attachmentsDir: settings.data.attachments_dir,
  sourcePath: editor.activePath,
  // 访问 assetVersion 使 computed 依赖资源表版本（注册表更新后自动重渲染）
  assetPaths: (indexStore.assetVersion, indexStore.assets),
}));

const html = computed(() => {
  if (!editor.activePath) return "";
  return renderMarkdown(editor.content, renderOpts.value);
});

watch(
  html,
  async () => {
    if (!previewEl.value) return;
    // 重渲染（打字时的实时刷新）保持可视区顶部对应的源行不变，避免预览跳顶
    const keepLine = previewTopLine();
    previewEl.value.innerHTML = html.value;
    await fillEmbeds(previewEl.value, 0);
    await fixupRawImages(previewEl.value);
    scrollToLine(keepLine);
    handlePendingJump();
  },
  { immediate: false },
);

// 切换笔记时清理图片错误计数
watch(
  () => editor.activePath,
  () => imgErrCount.clear(),
);

// 接收编辑器滚动上报 → 预览按锚点对齐（分屏滚动同步）
function onSourceScroll(e: Event) {
  if (isScrollSyncLocked() || editor.mode !== "split") return;
  const { line, frac } = (e as CustomEvent<{ line: number; frac: number }>).detail;
  scrollToLine(line + frac);
}

onMounted(async () => {
  previewEl.value?.addEventListener("error", onImageError, true);
  previewEl.value?.addEventListener("scroll", onPreviewScroll);
  window.addEventListener("emd-scroll-source", onSourceScroll);
  if (previewEl.value && html.value) {
    previewEl.value.innerHTML = html.value;
    await fillEmbeds(previewEl.value, 0);
    await fixupRawImages(previewEl.value);
    handlePendingJump();
  }
});

onUnmounted(() => {
  previewEl.value?.removeEventListener("error", onImageError, true);
  previewEl.value?.removeEventListener("scroll", onPreviewScroll);
  window.removeEventListener("emd-scroll-source", onSourceScroll);
});

// ---- 分屏滚动同步：预览侧 ----
// 元素相对预览内容顶部的偏移（getBoundingClientRect 差值 + scrollTop，稳定于滚动）
function contentTop(root: HTMLElement, el: HTMLElement): number {
  return el.getBoundingClientRect().top - root.getBoundingClientRect().top + root.scrollTop;
}

/** 可视区顶部对应的源行（可在锚点块内按比例取小数） */
function previewTopLine(): number {
  const root = previewEl.value;
  if (!root) return 0;
  const y = root.scrollTop + 8;
  let top: HTMLElement | null = null;
  let next: HTMLElement | null = null;
  for (const el of Array.from(root.querySelectorAll<HTMLElement>("[data-ls]"))) {
    if (contentTop(root, el) <= y) top = el;
    else {
      next = el;
      break;
    }
  }
  if (!top) return 0;
  const ls = Number(top.dataset.ls);
  const le = Number(top.dataset.le);
  if (next) {
    const span = Math.max(1, contentTop(root, next) - contentTop(root, top));
    const t = clamp01((y - contentTop(root, top)) / span);
    return ls + t * Math.max(0, Number(next.dataset.ls) - ls);
  }
  const f = clamp01((y - contentTop(root, top)) / Math.max(1, top.offsetHeight));
  return ls + f * Math.max(1, le - 1 - ls);
}

/** 滚动预览，使源行 line 对齐到可视区顶部 */
function scrollToLine(line: number) {
  const root = previewEl.value;
  if (!root) return;
  const blocks = root.querySelectorAll<HTMLElement>("[data-ls]");
  if (!blocks.length) return;
  let top: HTMLElement | null = null;
  let next: HTMLElement | null = null;
  for (const el of Array.from(blocks)) {
    if (Number(el.dataset.ls) <= line) top = el;
    else {
      next = el;
      break;
    }
  }
  if (!top) {
    lockScrollSync();
    root.scrollTop = 0;
    return;
  }
  let target = contentTop(root, top) - 8;
  if (next) {
    const span = Math.max(1, Number(next.dataset.ls) - Number(top.dataset.ls));
    const t = clamp01((line - Number(top.dataset.ls)) / span);
    target = contentTop(root, top) + t * (contentTop(root, next) - contentTop(root, top)) - 8;
  }
  lockScrollSync();
  root.scrollTop = Math.max(0, target);
}

function onPreviewScroll() {
  if (isScrollSyncLocked() || editor.mode !== "split") return;
  window.dispatchEvent(new CustomEvent("emd-scroll-preview", { detail: { line: previewTopLine() } }));
}

// ---- 原生 <img>（论坛/网页复制的 HTML 片段）后置修正 ----
// markdown 语法图片已在渲染期改写；原生 <img> 直通到这里才处理：
// 相对路径 → vault 解析；file:/// 与盘符路径 → 读文件转 blob URL
async function fixupRawImages(root: HTMLElement) {
  for (const img of Array.from(root.querySelectorAll("img"))) {
    const src = img.getAttribute("src") || "";
    if (!src) continue;
    if (/^file:/i.test(src)) {
      try {
        const bytes = await api.readExternalBinary(fileUrlToPath(src));
        img.src = URL.createObjectURL(new Blob([new Uint8Array(bytes)]));
      } catch {
        /* 读取失败交给 error 占位 */
      }
    } else if (!/^(https?:|data:|blob:|emdasset:)/i.test(src)) {
      img.src = emdAssetUrl(resolveImageSrc(src, renderOpts.value));
    }
  }
}

// ---- 图片加载失败：外链走 Rust 代理下载（绕过 ORB/防盗链），库内刷新注册表重试 ----
const imgErrCount = new Map<string, number>();
// 外链代理缓存：原 URL → 本地 external 资源 URL（跨重渲染复用，避免重复下载）
const extUrlCache = new Map<string, string>();

function onImageError(ev: ErrorEvent) {
  const el = ev.target as HTMLElement | null;
  if (!(el instanceof HTMLImageElement) || !previewEl.value?.contains(el)) return;
  const src = el.getAttribute("src") || "";
  const isHttp = /^https?:/i.test(src) && !/emdasset/i.test(src);
  const isFile = /^file:/i.test(src);
  const isExtCache = /emdasset[^/]*\/external\//i.test(src);
  const count = (imgErrCount.get(src) || 0) + 1;
  imgErrCount.set(src, count);
  if (count === 1 && !isExtCache) {
    if (isHttp) {
      // 外链直连失败（ORB / 防盗链 / octet-stream）→ Rust 代理下载后指向本地缓存
      const cached = extUrlCache.get(src);
      if (cached) {
        el.src = cached;
        return;
      }
      api
        .fetchExternalImage(src)
        .catch(() => api.fetchExternalImage(src)) // 重试一次：并发同图竞争写缓存 / 瞬时网络抖动
        .then((name) => {
          const u = emdExternalUrl(name);
          extUrlCache.set(src, u);
          if (el.isConnected) el.src = u;
        })
        .catch(() => showImgPlaceholder(el, src, true));
      return;
    }
    if (!isFile) {
      // 库内图片可能是刚放进库的（粘贴/拖入/外部写入），注册表尚不知道 → 刷新后重渲染
      indexStore.refreshAssets();
      return;
    }
  }
  showImgPlaceholder(el, src, isHttp || isFile || isExtCache);
}

function showImgPlaceholder(el: HTMLImageElement, src: string, isExternal: boolean) {
  let name = src;
  try {
    name = decodeURIComponent(src.split("/").pop() || src);
  } catch {
    /* 保留原文 */
  }
  const ph = document.createElement("div");
  ph.className = "md-img-missing";
  ph.textContent = `${isExternal ? t("pv.imgLoadFail") : t("pv.imgMissing")}  ${name}`;
  el.replaceWith(ph);
}

// ---- 嵌入内容异步填充（![[ ]]） ----
async function fillEmbeds(root: HTMLElement, depth: number) {
  if (depth > 3) return;
  const nodes = Array.from(root.querySelectorAll<HTMLElement>(".emd-embed"));
  for (const node of nodes) {
    const target = node.dataset.embedTarget || "";
    const sub = node.dataset.embedSub || "";
    if (!target && !sub) continue;
    try {
      if (isImageName(target)) {
        const rel = resolveImageSrc(target, renderOpts.value);
        const alt = target.replace(/"/g, "&quot;");
        node.innerHTML = `<img src="${emdAssetUrl(rel)}" alt="${alt}" />`;
        continue;
      }
      const resolved = resolveTarget(target, indexStore.notes).path;
      if (!resolved) {
        node.innerHTML = `<div class="emd-embed-missing">${t("pv.notFound")}${target || sub}</div>`;
        continue;
      }
      const raw = await api.readTextFile(resolved);
      const noteIndex = indexStore.byPath[resolved];
      let body = raw;
      if (sub.startsWith("^")) {
        body = extractBlock(raw, sub.slice(1));
      } else if (sub) {
        body = extractSection(raw, sub);
      }
      const inner = renderMarkdown(body, {
        ...renderOpts.value,
        sourcePath: resolved,
      });
      const title = noteIndex?.title || target;
      node.innerHTML = `<div class="emd-embed-head"><a class="internal-link" data-target="${target}" data-subpath="" href="javascript:void(0)">${title}</a></div><div class="emd-embed-body">${inner}</div>`;
      await fillEmbeds(node, depth + 1);
    } catch {
      node.innerHTML = `<div class="emd-embed-missing">${t("pv.readFail")}</div>`;
    }
  }
}

// ---- 点击：内链跳转 / 未解析创建 / 标签 / 外链 ----
function onClick(ev: MouseEvent) {
  const a = (ev.target as HTMLElement).closest("a");
  if (!a) return;
  if (a.classList.contains("internal-link")) {
    const target = a.dataset.target || "";
    const sub = a.dataset.subpath || "";
    goInternal(target, sub);
  } else if (a.classList.contains("md-tag")) {
    ui.leftVisible = true;
    ui.leftTab = "tags";
    window.dispatchEvent(new CustomEvent("emd-select-tag", { detail: a.dataset.tag }));
  } else if (a.href && /^https?:/i.test(a.href)) {
    ev.preventDefault();
    window.open(a.href, "_blank");
  }
}

async function goInternal(target: string, sub: string) {
  if (!target && sub) {
    // 同页引用
    if (sub.startsWith("^")) {
      scrollBlock(sub.slice(1));
    } else {
      scrollAnchor(sub);
    }
    return;
  }
  const res = resolveTarget(target, indexStore.notes);
  if (!res.path) {
    // 未解析 → 询问创建
    if (confirm(`${t("pv.createConfirm").replace("{name}", target)}`)) {
      const { useVaultStore } = await import("../../stores/vault");
      const vault = useVaultStore();
      await vault.newNote(null, target);
    }
    return;
  }
  if (sub.startsWith("^")) {
    const noteIdx = indexStore.byPath[res.path];
    const blk = noteIdx?.block_ids.find((b) => b.id === sub.slice(1));
    emit("jump", { path: res.path, line: blk?.line });
  } else if (sub) {
    emit("jump", { path: res.path, anchor: sub });
  } else {
    emit("jump", { path: res.path });
  }
}

function scrollAnchor(headingText: string) {
  const slug = slugify(headingText);
  const el =
    previewEl.value?.querySelector(`#emd-h-${CSS.escape(slug)}`) ||
    Array.from(previewEl.value?.querySelectorAll("[data-heading]") || []).find(
      (e) => (e as HTMLElement).dataset.heading === headingText,
    );
  el?.scrollIntoView({ behavior: "smooth", block: "start" });
}

function scrollBlock(id: string) {
  const noteIdx = indexStore.byPath[editor.activePath];
  const blk = noteIdx?.block_ids.find((b) => b.id === id);
  if (blk && ui.view === "editor") {
    // 分屏/预览模式下直接滚动行不可行，跳源码行
    window.dispatchEvent(new CustomEvent("emd-goto-line", { detail: blk.line }));
  }
}

function handlePendingJump() {
  const jump = editor.pendingJump;
  if (!jump) return;
  editor.pendingJump = null;
  if (jump.anchor) setTimeout(() => scrollAnchor(jump.anchor!), 50);
  if (jump.line !== undefined)
    setTimeout(
      () => window.dispatchEvent(new CustomEvent("emd-goto-line", { detail: jump.line })),
      50,
    );
}

onUnmounted(() => {});
</script>

<style>
.md-preview {
  height: 100%;
  overflow-y: auto;
  padding: 24px 32px 60vh;
  background: var(--background-primary);
  color: var(--text-normal);
  font-family: var(--font-text);
  font-size: var(--font-text-size);
  line-height: 1.65;
  user-select: text;
}

/* ---------- Obsidian 风格 markdown 渲染 ---------- */
.markdown-rendered h1,
.markdown-rendered h2,
.markdown-rendered h3,
.markdown-rendered h4,
.markdown-rendered h5,
.markdown-rendered h6 {
  font-weight: 650;
  line-height: 1.35;
  margin: 1.4em 0 0.5em;
  scroll-margin-top: 16px;
}
.markdown-rendered h1 { font-size: 1.75em; }
.markdown-rendered h2 { font-size: 1.5em; }
.markdown-rendered h3 { font-size: 1.25em; }
.markdown-rendered h4 { font-size: 1.1em; }
.markdown-rendered p { margin: 0.4em 0 0.9em; }
.markdown-rendered ul,
.markdown-rendered ol { margin: 0.4em 0 0.9em; padding-left: 1.6em; }
.markdown-rendered li { margin: 0.15em 0; }
.markdown-rendered blockquote {
  border-left: 3px solid var(--blockquote-border-color);
  margin: 0.9em 0;
  padding: 0.1em 1em;
  color: var(--text-muted);
}
.markdown-rendered code {
  font-family: var(--font-mono);
  font-size: 0.86em;
  background: var(--code-background);
  color: var(--code-normal);
  border-radius: var(--radius-s);
  padding: 0.1em 0.35em;
}
.markdown-rendered pre {
  background: var(--background-secondary);
  border: 1px solid var(--background-modifier-border);
  border-radius: var(--radius-m);
  padding: 12px 14px;
  overflow-x: auto;
  margin: 0.9em 0;
}
.markdown-rendered pre code {
  background: none;
  padding: 0;
}
.markdown-rendered a.internal-link {
  color: var(--link-color);
  text-decoration: none;
}
.markdown-rendered a.internal-link:hover {
  color: var(--link-color-hover);
  text-decoration: underline;
}
.markdown-rendered a.internal-link.is-unresolved {
  color: var(--link-unresolved-color);
  opacity: var(--link-unresolved-opacity);
  text-decoration-style: dashed;
}
.markdown-rendered a:not(.internal-link):not(.md-tag) {
  color: var(--link-color);
}
.markdown-rendered a.md-tag {
  display: inline-block;
  background: var(--tag-background);
  color: var(--tag-color);
  border-radius: var(--radius-m);
  padding: 1px 8px;
  font-size: 0.85em;
  text-decoration: none;
  line-height: 1.5;
}
.markdown-rendered a.md-tag:hover {
  background: var(--tag-background-hover);
}
.markdown-rendered mark.md-mark {
  background: rgba(255, 208, 0, 0.28);
  color: inherit;
  border-radius: var(--radius-s);
  padding: 0 3px;
}
.markdown-rendered img {
  max-width: 100%;
  border-radius: var(--radius-s);
}
.markdown-rendered .md-img-missing {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  border: 1px dashed var(--background-modifier-border-hover);
  border-radius: var(--radius-m);
  background: var(--background-secondary);
  color: var(--text-faint);
  font-size: 0.85em;
  padding: 10px 14px;
  margin: 0.3em 0;
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  user-select: text;
}
.markdown-rendered hr {
  border: none;
  border-top: 1px solid var(--hr-color);
  margin: 1.8em 0;
}
.markdown-rendered table {
  border-collapse: collapse;
  margin: 0.9em 0;
}
.markdown-rendered th,
.markdown-rendered td {
  border: 1px solid var(--background-modifier-border);
  padding: 6px 12px;
}
.markdown-rendered th { background: var(--background-secondary); }
.markdown-rendered input.md-task {
  margin-right: 6px;
  accent-color: var(--interactive-accent);
}

/* ---------- 嵌入块 ---------- */
.markdown-rendered .emd-embed {
  margin: 0.6em 0;
}
.markdown-rendered .emd-embed .emd-embed-head {
  border-left: 3px solid var(--text-accent);
  padding-left: 8px;
  font-size: 0.85em;
  margin-bottom: 2px;
}
.markdown-rendered .emd-embed .emd-embed-body {
  border-left: 3px solid var(--background-modifier-border);
  padding: 0.2em 0 0.2em 12px;
  font-size: 0.95em;
}
.markdown-rendered .emd-embed-missing {
  border: 1px dashed var(--background-modifier-border-hover);
  border-radius: var(--radius-m);
  color: var(--text-faint);
  padding: 6px 10px;
  font-size: 0.85em;
}
</style>
