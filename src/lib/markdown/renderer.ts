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
  /** vault 内全部资源文件路径（提供时按 Obsidian 行为做全库文件名解析） */
  assetPaths?: string[];
}

// html:true 让论坛/网页复制来的原生 <img> 等 HTML 片段正常渲染
// （VSCode/Obsidian 同为 true）；script 仍被 CSP script-src 'self' 阻断
const md: MarkdownIt = new MarkdownIt({ html: true, linkify: true, breaks: false });

// markdown-it 默认把 file: / data: 也当危险协议拒绝解析（链接/图片都不出），
// 本地知识库需要 file:// 与 base64 图片；脚本安全由 CSP script-src 'self' 把关
md.validateLink = (url: string) => {
  const str = url.trim().toLowerCase();
  return !/^(vbscript|javascript):/.test(str);
};

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
  const src = finalImgSrc(rawSrc, CURRENT);
  return `<img src="${escAttr(src)}" alt="${alt}" loading="lazy" />`;
};

/**
 * 图片 src 最终定位（分类必须完整，否则网图/本地路径会被误当 vault 相对路径）：
 * - 外链 http(s)/data/blob/emdasset：原样；
 * - `//host/…`（协议相对）与裸域名（pic1.zhimg.com/…）：补 https:// 前缀；
 * - `file://` / 盘符绝对路径 / UNC：统一为 file:/// URL（预览后置处理读文件转 blob）；
 * - 其余（含裸文件名）：走 vault 全库解析 → emdasset。
 */
export function finalImgSrc(rawSrc: string, opts: RenderOpts | null): string {
  const raw = rawSrc.trim();
  if (!raw) return "";
  // 分类前解码一次：%5C/%20 是 fixImageDests 为保护路径加的，识别本地路径需还原
  let logical = raw;
  try {
    logical = decodeURIComponent(raw);
  } catch {
    /* 非法编码序列时保留原文 */
  }
  if (/^(https?:|data:|blob:|emdasset:)/i.test(logical)) return raw;
  if (logical.startsWith("//")) return "https:" + raw;
  if (/^(www\.|[a-z0-9-]+(?:\.[a-z0-9-]+){2,}\/)/i.test(logical)) return "https://" + raw;
  if (/^(file:|[a-z]:[\\/]|\\\\)/i.test(logical)) return toFileUrl(logical);
  return emdAssetUrl(resolveImageSrc(raw, opts));
}

/** 本地绝对路径 / file:// → 规范 file:/// URL（含空格/中文按段编码） */
export function toFileUrl(p: string): string {
  const s = p.trim();
  if (/^file:/i.test(s)) return s;
  const norm = s.replace(/\\/g, "/");
  return (
    "file:///" +
    norm
      .split("/")
      .map((seg, i) => (i === 0 ? seg : encodeURIComponent(seg)))
      .join("/")
  );
}

/** file:/// URL → 本地路径（配合 read_external_binary 读取） */
export function fileUrlToPath(u: string): string {
  const s = u.replace(/^file:\/*/i, "");
  return s
    .split("/")
    .map((seg) => {
      try {
        return decodeURIComponent(seg);
      } catch {
        return seg;
      }
    })
    .join("/");
}

/** 图片/相对资源目标 → vault 相对路径。候选顺序（对齐 Obsidian）：
 * 1. 按字面（vault 根相对）；2. 相对当前笔记目录；3. 裸文件名拼附件目录；
 * 4. 以上都不在库中时，全库按文件名唯一匹配（同名取最短路径）。 */
export function resolveImageSrc(rawSrc: string, opts: RenderOpts | null): string {
  let t = rawSrc.trim().replace(/\\/g, "/");
  try {
    t = decodeURIComponent(t);
  } catch {
    /* 非法编码序列时保留原文 */
  }
  t = t.split("#")[0].split("?")[0];
  if (!t || t === ".") return "";

  const noteDir = (opts?.sourcePath || "").split("/").slice(0, -1);
  const attachDir =
    (opts?.attachmentsDir || "").trim().replace(/^\/+|\/+$/g, "") || "assets";

  // 展开相对段（./ ../）并归一化
  const expand = (parts: string[], base: string[]): string => {
    const out = [...parts.filter((s) => s !== ".")];
    for (const seg of base) out.push(seg);
    const res: string[] = [];
    for (const seg of out) {
      if (seg === "..") res.pop();
      else if (seg) res.push(seg);
    }
    return res.join("/");
  };

  const candidates: string[] = [];
  if (t.startsWith("/")) {
    candidates.push(expand(t.slice(1).split("/"), []));
  } else if (t.startsWith("./") || t.startsWith("../")) {
    candidates.push(expand(t.split("/"), noteDir));
  } else if (t.includes("/")) {
    candidates.push(expand(t.split("/"), []));
    candidates.push(expand(t.split("/"), noteDir));
  } else {
    candidates.push(`${attachDir}/${t}`);
    candidates.push(expand([t], noteDir));
  }

  const index = assetIndexOf(opts?.assetPaths);
  if (index) {
    for (const c of candidates) {
      const hit = index.byPath.get(c.toLowerCase());
      if (hit) return hit;
    }
    // 全库按文件名唯一匹配（大小写不敏感；多同名取最短路径）
    const name = t.split("/").pop()!.toLowerCase();
    const hits = index.byName.get(name);
    if (hits && hits.length > 0) {
      return hits.reduce((a, b) => (a.length <= b.length ? a : b));
    }
  }
  return candidates[0] ?? t;
}

/** assetPaths 的查找索引（按数组引用缓存，渲染期间复用） */
interface AssetIndex {
  byPath: Map<string, string>;
  byName: Map<string, string[]>;
}
const assetIndexCache = new WeakMap<string[], AssetIndex>();

function assetIndexOf(paths?: string[]): AssetIndex | null {
  if (!paths || paths.length === 0) return null;
  let idx = assetIndexCache.get(paths);
  if (!idx) {
    idx = { byPath: new Map(), byName: new Map() };
    for (const p of paths) {
      const lower = p.toLowerCase();
      if (!idx.byPath.has(lower)) idx.byPath.set(lower, p);
      const name = lower.split("/").pop()!;
      const arr = idx.byName.get(name);
      if (arr) arr.push(p);
      else idx.byName.set(name, [p]);
    }
    assetIndexCache.set(paths, idx);
  }
  return idx;
}

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
  // 逐段编码：保留 / 分隔符，空格与中文按段编码，避免整串 encodeURIComponent 把 / 变成 %2F
  const clean = relPath.replace(/\\/g, "/").replace(/^\/+/, "");
  const encoded = clean.split("/").map(encodeURIComponent).join("/");
  // Windows/Android 的 WebView 不支持非标准 scheme 的页面内请求，Tauri 将自定义协议
  // 映射为 http://<scheme>.localhost；其余平台用原生 scheme
  const isWinLike =
    /win/i.test(navigator.userAgent) || /android/i.test(navigator.userAgent);
  return isWinLike
    ? `http://emdasset.localhost/vault/${encoded}`
    : `emdasset://vault/${encoded}`;
}

/** 渲染入口（同步渲染，嵌入内容由 PreviewView 异步填充） */
export function renderMarkdown(content: string, opts: RenderOpts): string {
  CURRENT = opts;
  try {
    const body = fixImageDests(stripFrontmatter(content).body);
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

/**
 * 图片目标预处理（渲染/导出共用）：
 * 1. 编码裸反斜杠 → %5C：markdown 解析会吃掉 `\x` 形式的反斜杠，Windows 路径
 *    （`![](C:\Users\…)`）不经保护会被毁掉；
 * 2. 空格 → %20：CommonMark 裸目标不允许空格（粘贴截图 `Pasted image …`、
 *    本地路径常见），否则整段不被解析成图片；
 * 3. 含 " 或 <> 或括号的（合法标题写法）不动。
 * 目标端（resolveImageSrc/fileUrlToPath）会 decodeURIComponent 还原。
 */
export function fixImageDests(body: string): string {
  return body.replace(
    /!\[([^\]\n]*)\]\((<[^<>\n]*>|[^()\n<>"']*)\)/g,
    (m: string, alt: string, destRaw: string) => {
      const wrapped = destRaw.startsWith("<");
      const dest = wrapped ? destRaw.slice(1, -1) : destRaw;
      if (!dest) return m;
      let out = dest;
      if (out.includes("\\")) out = out.replace(/\\/g, "%5C");
      if (/\s/.test(out)) out = out.replace(/ /g, "%20");
      return out === dest && !wrapped ? m : `![${alt}](${out})`;
    },
  );
}

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
