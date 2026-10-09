// 导出：md（保留扩展标记）/ html（扩展语法转标准链接+内联资源）/ pdf（打印渲染）
import { save } from "@tauri-apps/plugin-dialog";
import { api, assetUrl } from "../ipc/tauri";
import { useEditorStore } from "../stores/editor";
import { useNotesIndexStore } from "../stores/notesIndex";
import { useSettingsStore } from "../stores/settings";
import { stripFrontmatter, slugify, fixImageDests } from "./markdown/renderer";
import { resolveTarget } from "./markdown/links";
import { tf } from "../i18n";

/** 简易导出渲染：[[x]] → 相对链接，![[ ]] → 内联，#tag → 徽章 */
function renderExportMarkdown(content: string): string {
  const indexStore = useNotesIndexStore();
  const { body } = stripFrontmatter(content);
  const src = fixImageDests(body);

  // 1) 嵌入 ![[...]]
  let out = src.replace(/!\[\[([^\[\]\n]+)\]\]/g, (_m, inner: string) => {
    const [main] = inner.split("|");
    const target = main.split("#")[0].trim();
    return `%%EMBED::${target}%%`;
  });
  // 2) 链接 [[x|alias]] [[x#h]]
  out = out.replace(/(?<!\!)\[\[([^\[\]\n]+)\]\]/g, (_m, inner: string) => {
    const [main, alias] = inner.split("|");
    const [t, sub] = main.trim().split("#");
    const label = (alias || main).trim();
    const res = resolveTarget(t, indexStore.notes);
    const name = res.path ? res.path.replace(/\.md$/i, "") : t;
    const href = `${name}.html${sub ? `#emd-h-${slugify(sub)}` : ""}`;
    return `[${label}](${href})`;
  });
  // 3) 标签 #x → 纯文本徽章样式（导出用 code 包裹避免歧义）
  out = out.replace(/(^|\s)#([\p{L}\p{N}_][\p{L}\p{N}_/-]*)/gu, (_m, sp, tag) => `${sp}\`#${tag}\``);
  // 4) 高亮 ==x== → 粗体（导出降级）
  out = out.replace(/==([^=\n]+)==/g, "**$1**");
  return out;
}

/** markdown-it 实例（导出独立，避免预览插件副作用）；html:true 与预览一致 */
async function renderExportHtml(content: string): Promise<string> {
  const { default: MarkdownIt } = await import("markdown-it");
  const md = new MarkdownIt({ html: true, linkify: true });
  // 同预览：放行 file: / data:，仅拦脚本类协议
  md.validateLink = (url: string) => !/^(vbscript|javascript):/i.test(url.trim());
  let html = md.render(renderExportMarkdown(content));

  // 嵌入内容替换（读取文件正文再渲染）
  const embedRe = /<p>%%EMBED::([^%]+)%%<\/p>|%%EMBED::([^%]+)%%/g;
  const embeds: { key: string; target: string }[] = [];
  let m: RegExpExecArray | null;
  while ((m = embedRe.exec(html))) {
    const target = (m[1] || m[2]).trim();
    embeds.push({ key: m[0], target });
  }
  const indexStore = useNotesIndexStore();
  for (const e of embeds) {
    const res = resolveTarget(e.target, indexStore.notes);
    let replacement = `<blockquote>${tf("ex.embedMissing", { name: e.target })}</blockquote>`;
    if (res.path) {
      try {
        const raw = await api.readTextFile(res.path);
        replacement = md.render(stripFrontmatter(raw).body);
      } catch {
        /* 保持占位 */
      }
    }
    html = html.split(e.key).join(replacement);
  }
  return html;
}

/** 相对资源路径 → base64（单文件 html 内联）；file:// 本地图片同样内联 */
async function inlineImages(html: string): Promise<string> {
  const srcRe = /src="(?!data:|blob:)([^"]+)"/g;
  const urls = new Set<string>();
  let m: RegExpExecArray | null;
  while ((m = srcRe.exec(html))) urls.add(m[1]);
  const indexStore = useNotesIndexStore();
  const settings = useSettingsStore();
  const { resolveImageSrc, fileUrlToPath, emdExternalUrl } = await import("./markdown/renderer");
  const { useEditorStore } = await import("../stores/editor");
  const editor = useEditorStore();
  const mimeOf = (p: string): string => {
    const ext = p.split(".").pop()?.toLowerCase() || "";
    return (
      { png: "image/png", jpg: "image/jpeg", jpeg: "image/jpeg", gif: "image/gif", webp: "image/webp", svg: "image/svg+xml", bmp: "image/bmp", ico: "image/x-icon", avif: "image/avif" }[ext] ||
      "application/octet-stream"
    );
  };
  for (const rel of urls) {
    try {
      let dataUrl: string;
      if (/^https?:/i.test(rel) && !/emdasset/i.test(rel)) {
        // 外链：ORB/防盗链环境下浏览器拉不到，走 Rust 代理下载再内联
        const name = await api.fetchExternalImage(rel);
        const resp = await fetch(emdExternalUrl(name));
        const blob = await resp.blob();
        dataUrl = await blobToDataUrl(blob);
      } else if (/^file:/i.test(rel)) {
        // 本地绝对路径图片（file:/// 或盘符路径）：读文件内联
        const bytes = await api.readExternalBinary(fileUrlToPath(rel));
        const blob = new Blob([new Uint8Array(bytes)], { type: mimeOf(rel) });
        dataUrl = await blobToDataUrl(blob);
      } else if (/emdasset/i.test(rel)) {
        const resp = await fetch(rel);
        const blob = await resp.blob();
        dataUrl = await blobToDataUrl(blob);
      } else {
        let target = rel;
        try {
          target = decodeURIComponent(rel);
        } catch {
          /* 保留原文 */
        }
        const resolved = resolveImageSrc(target, {
          resolveNote: () => null,
          attachmentsDir: settings.data.attachments_dir,
          sourcePath: editor.activePath,
          assetPaths: indexStore.assets,
        });
        const resp = await fetch(assetUrl(resolved));
        const blob = await resp.blob();
        dataUrl = await blobToDataUrl(blob);
      }
      html = html.split(`src="${rel}"`).join(`src="${dataUrl}"`);
    } catch {
      /* 跳过失败资源 */
    }
  }
  return html;
}

function blobToDataUrl(blob: Blob): Promise<string> {
  return new Promise<string>((resolve, reject) => {
    const fr = new FileReader();
    fr.onload = () => resolve(fr.result as string);
    fr.onerror = reject;
    fr.readAsDataURL(blob);
  });
}

function exportCss(): string {
  const dark = useSettingsStore().data.theme === "dark";
  return `
  body { font-family: "Segoe UI", "PingFang SC", "Microsoft YaHei", sans-serif;
    color: ${dark ? "#dcddde" : "#222"}; background: ${dark ? "#1e1e1e" : "#fff"};
    max-width: 820px; margin: 0 auto; padding: 40px 28px; line-height: 1.7; }
  h1,h2,h3,h4 { line-height: 1.35; margin: 1.4em 0 0.5em; }
  a { color: ${dark ? "#a882ff" : "#705dcf"}; }
  blockquote { border-left: 3px solid ${dark ? "#4a4a4a" : "#cfcfcf"}; margin: 1em 0; padding: 0.1em 1em; color: ${dark ? "#999" : "#666"}; }
  code { background: ${dark ? "rgba(255,255,255,0.06)" : "rgba(0,0,0,0.05)"}; border-radius: 4px; padding: 0.1em 0.35em; font-size: 0.9em; }
  pre { background: ${dark ? "#161616" : "#f6f6f6"}; border: 1px solid ${dark ? "#333" : "#dbdbdc"}; border-radius: 8px; padding: 12px 14px; overflow-x: auto; }
  pre code { background: none; padding: 0; }
  img { max-width: 100%; }
  table { border-collapse: collapse; } th,td { border: 1px solid ${dark ? "#333" : "#dbdbdc"}; padding: 6px 12px; }
  hr { border: none; border-top: 1px solid ${dark ? "#333" : "#d8d8d8"}; margin: 2em 0; }
  @media print { body { padding: 0; } }
  `;
}

export function noteTitle(): string {
  const editor = useEditorStore();
  return editor.activePath.split("/").pop()?.replace(/\.md$/i, "") || "未命名";
}

export async function exportMarkdown() {
  const editor = useEditorStore();
  if (!editor.isOpen) return;
  const path = await save({
    defaultPath: `${noteTitle()}.md`,
    filters: [{ name: "Markdown", extensions: ["md"] }],
  });
  if (!path) return;
  await api.writeExternal(path, editor.content); // 原样导出，保留 [[ ]] 标记
}

export async function exportHtml() {
  const editor = useEditorStore();
  if (!editor.isOpen) return;
  const path = await save({
    defaultPath: `${noteTitle()}.html`,
    filters: [{ name: "HTML", extensions: ["html"] }],
  });
  if (!path) return;
  let body = await renderExportHtml(editor.content);
  body = await inlineImages(body);
  const title = noteTitle();
  const doc = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>${title}</title>
<style>${exportCss()}</style>
</head>
<body>
<h1 class="export-title">${title}</h1>
${body}
</body>
</html>`;
  await api.writeExternal(path, doc);
}

/** PDF：渲染到打印层，调起系统打印（另存为 PDF） */
export async function exportPdf() {
  const editor = useEditorStore();
  if (!editor.isOpen) return;
  let body = await renderExportHtml(editor.content);
  const printRoot = document.getElementById("print-root");
  if (!printRoot) return;
  printRoot.innerHTML = `<style>${exportCss()}</style><h1>${noteTitle()}</h1>${body}`;
  document.body.classList.add("is-printing");
  window.print();
  const cleanup = () => {
    document.body.classList.remove("is-printing");
    printRoot.innerHTML = "";
    window.removeEventListener("afterprint", cleanup);
  };
  window.addEventListener("afterprint", cleanup);
}
