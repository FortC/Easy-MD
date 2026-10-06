// markdown-it 实例与渲染管线：wikilink / 标签 / 图片资源改写 / 标题锚点
import MarkdownIt from "markdown-it";
import { t } from "../../i18n";

export interface RenderOpts {
  /** [[目标]] → 笔记相对路径；未解析返回 null */
  resolveNote: (target: string) => string | null;
  /** 附件目录（无路径的图片名兜底） */
  attachmentsDir: string;
  /** 当前笔记相对路径（解析相对资源用） */
  sourcePath?: string;
}

const md: MarkdownIt = new MarkdownIt({ html: false, linkify: true, breaks: false });

// ---------- wikilink / embed ----------
md.inline.ruler.before("link", "emd_wikilink", (state, silent) => {
  const src = state.src;
  let pos = state.pos;
  let embed = false;
  if (src[pos] === "!") {
    embed = true;
    pos++;
  }
  if (src[pos] !== "[" || src[pos + 1] !== "[") return false;
  const end = src.indexOf("]]", pos + 2);
  if (end === -1 || end > state.posMax) return false;
  const inner = src.slice(pos + 2, end);
  if (inner.includes("[") || inner.includes("\n")) return false;

  if (!silent) {
    let main = inner;
    let alias: string | null = null;
    const bar = inner.indexOf("|");
    if (bar !== -1) {
      main = inner.slice(0, bar);
      alias = inner.slice(bar + 1).trim();
    }
    main = main.trim();
    let target = main;
    let subpath: string | null = null;
    const hash = main.indexOf("#");
    if (hash !== -1) {
      target = main.slice(0, hash).trim();
      subpath = main.slice(hash + 1);
    }
    const token = state.push(embed ? "emd_embed" : "emd_wikilink", "", 0);
    token.meta = { target, subpath, alias };
    token.content = state.src.slice(state.pos, end + 2);
  }
  state.pos = end + 2;
  return true;
});

function escHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}
function escAttr(s: string): string {
  return escHtml(s).replace(/'/g, "&#39;");
}

// ---------- 标签 #tag ----------
md.inline.ruler.before("emphasis", "emd_tag", (state, silent) => {
  const src = state.src;
  const pos = state.pos;
  if (src[pos] !== "#") return false;
  // 前面必须是行首或空白
  if (pos > 0 && !/[\s(\u3000]/.test(src[pos - 1])) return false;
  const m = /^#([\p{L}\p{N}_][\p{L}\p{N}_/-]*)/u.exec(src.slice(pos));
  if (!m) return false;
  const tag = m[1].replace(/\/+$/, "");
  if (!tag || /^\d+$/.test(tag)) return false;
  if (!silent) {
    const token = state.push("emd_tag", "", 0);
    token.content = tag;
  }
  state.pos = pos + m[0].length;
  return true;
});

// ---------- 高亮 ==text== ----------
md.inline.ruler.before("emphasis", "emd_mark", (state, silent) => {
  const src = state.src;
  const pos = state.pos;
  if (src[pos] !== "=" || src[pos + 1] !== "=") return false;
  const end = src.indexOf("==", pos + 2);
  if (end === -1 || end === pos + 2 || end > state.posMax) return false;
  const inner = src.slice(pos + 2, end);
  if (inner.includes("\n") || inner.includes("=")) return false;
  if (!silent) {
    const token = state.push("emd_mark", "", 0);
    token.content = inner;
  }
  state.pos = end + 2;
  return true;
});

md.renderer.rules.emd_mark = (tokens, idx) => {
  return `<mark class="md-mark">${escHtml(tokens[idx].content)}</mark>`;
};

// ---------- 图片资源改写（相对路径 → emdasset） ----------
md.renderer.rules.image = (tokens, idx) => {
  const tok = tokens[idx];
  const rawSrc = tok.attrGet("src") || "";
  const alt = escAttr(tok.content || "");
  let src = rawSrc;
  if (!/^(https?:|emdasset:|data:|blob:)/i.test(rawSrc) && rawSrc) {
    src = emdAssetUrl(rawSrc.startsWith("/") ? rawSrc.slice(1) : rawSrc);
  }
  return `<img src="${escAttr(src)}" alt="${alt}" loading="lazy" />`;
};

// ---------- 标题锚点 ----------
md.renderer.rules.heading_open = (tokens, idx) => {
  const tok = tokens[idx];
  const inline = tokens[idx + 1];
  const slug = slugify(inline ? inline.content : "");
  return `<${tok.tag} id="emd-h-${slug}" data-heading="${escAttr(inline ? inline.content : "")}">`;
};

// ---------- 默认规则绑定（渲染时机注入 resolve 上下文） ----------
let CURRENT: RenderOpts | null = null;

md.renderer.rules.emd_wikilink = (tokens, idx) => {
  const { target, subpath, alias } = tokens[idx].meta;
  const label =
    alias ||
    (target ? target + (subpath ? `#${subpath}` : "") : subpath || t("pv.self2"));
  const resolved = CURRENT?.resolveNote(target || "") ?? null;
  const cls = resolved ? "internal-link" : "internal-link is-unresolved";
  return `<a class="${cls}" data-target="${escAttr(target)}" data-subpath="${escAttr(subpath || "")}" href="javascript:void(0)">${escHtml(label)}</a>`;
};

md.renderer.rules.emd_embed = (tokens, idx) => {
  const { target, subpath } = tokens[idx].meta;
  return `<div class="emd-embed" data-embed-target="${escAttr(target)}" data-embed-sub="${escAttr(subpath || "")}"></div>`;
};

md.renderer.rules.emd_tag = (tokens, idx) => {
  return `<a class="md-tag" data-tag="${escAttr(tokens[idx].content)}" href="javascript:void(0)">#${escHtml(tokens[idx].content)}</a>`;
};

export function emdAssetUrl(relPath: string): string {
  return `emdasset://vault/${encodeURIComponent(relPath)}`;
}

/** 渲染入口（同步渲染，嵌入内容由 PreviewView 异步填充） */
export function renderMarkdown(content: string, opts: RenderOpts): string {
  CURRENT = opts;
  try {
    const body = stripFrontmatter(content).body;
    let html = md.render(body);
    // 任务列表复选框
    html = html.replace(
      /<li>(\s*)\[([ xX])\]\s/g,
      (_all, sp, mark) =>
        `<li><input type="checkbox" class="md-task" disabled ${mark.trim() ? "checked" : ""}>${sp ? "" : " "}`,
    );
    return html;
  } finally {
    CURRENT = null;
  }
}

// ---------- 文本工具（与 Rust parser 行为对齐的 JS 版） ----------

export function stripFrontmatter(text: string): { fm: string | null; body: string; bodyStartLine: number } {
  const t = text.startsWith("\u{feff}") ? text.slice(1) : text;
  if (t.startsWith("---\n") || t.startsWith("---\r\n")) {
    const nl = t.startsWith("---\r\n") ? 5 : 4;
    const rest = t.slice(nl);
    const m = /^(---|\.\.\.)\s*(\r?\n|$)/m.exec(rest);
    if (m && m.index !== undefined) {
      const fm = rest.slice(0, m.index);
      const body = rest.slice(m.index + m[0].length);
      return { fm, body, bodyStartLine: fm.split("\n").length + 1 };
    }
  }
  return { fm: null, body: t, bodyStartLine: 0 };
}

/** 从 body 提取某标题的章节（到同级或更高级标题为止） */
export function extractSection(body: string, heading: string): string {
  const lines = body.split("\n");
  let start = -1;
  let level = 0;
  const headingNorm = heading.trim().toLowerCase();
  for (let i = 0; i < lines.length; i++) {
    const m = /^(#{1,6})\s+(.*)$/.exec(lines[i]);
    if (m && m[2].trim().toLowerCase() === headingNorm) {
      start = i + 1;
      level = m[1].length;
      break;
    }
  }
  if (start === -1) return "";
  const out: string[] = [];
  for (let i = start; i < lines.length; i++) {
    const m = /^(#{1,6})\s+/.exec(lines[i]);
    if (m && m[1].length <= level) break;
    out.push(lines[i]);
  }
  return out.join("\n").trim();
}

/** 提取块 id 对应的块内容（^id 所在的空行分隔块） */
export function extractBlock(body: string, blockId: string): string {
  const lines = body.split("\n");
  let hit = -1;
  for (let i = 0; i < lines.length; i++) {
    const m = /\^([A-Za-z0-9-]+)\s*$/.exec(lines[i]);
    if (m && m[1] === blockId) {
      hit = i;
      break;
    }
  }
  if (hit === -1) return "";
  let start = hit;
  while (start > 0 && lines[start - 1].trim() !== "") start--;
  let end = hit;
  while (end < lines.length - 1 && lines[end + 1].trim() !== "") end++;
  return lines
    .slice(start, end + 1)
    .join("\n")
    .replace(/\s*\^[A-Za-z0-9-]+\s*$/, "")
    .trim();
}

export function slugify(text: string): string {
  return (
    text
      .trim()
      .toLowerCase()
      .replace(/[\s]+/g, "-")
      .replace(/[^\p{L}\p{N}\-_]/gu, "") || "h"
  );
}

/** 行内标题解析（大纲面板用） */
export function parseHeadings(text: string): { level: number; text: string; line: number }[] {
  const { body, bodyStartLine } = stripFrontmatter(text);
  const out: { level: number; text: string; line: number }[] = [];
  body.split("\n").forEach((line, i) => {
    const m = /^ {0,3}(#{1,6})[ \t]+(\S.*)$/.exec(line);
    if (m) {
      out.push({
        level: m[1].length,
        text: m[2].trim().replace(/\s*#+\s*$/, ""),
        line: i + bodyStartLine,
      });
    }
  });
  return out;
}
