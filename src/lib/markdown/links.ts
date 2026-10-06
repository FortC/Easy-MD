// 链接解析（前端本地索引版，供渲染/补全即时使用；后端 resolveLink 供单独查询）
import type { NoteIndex } from "../../types";

export interface ResolvedTarget {
  path: string | null;
  ambiguous: boolean;
}

const IMAGE_EXT = ["png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "ico", "avif"];

export function isImageName(name: string): boolean {
  const lower = name.toLowerCase();
  return IMAGE_EXT.some((e) => lower.endsWith("." + e));
}

/** 从索引镜像解析 [[目标]] → 笔记相对路径 */
export function resolveTarget(target: string, notes: NoteIndex[]): ResolvedTarget {
  const t = target.trim().replace(/\\/g, "/").toLowerCase();
  if (!t) return { path: null, ambiguous: false };
  if (t.includes("/")) {
    for (const n of notes) {
      const p = n.path.toLowerCase();
      if (p === t || p === t + ".md") return { path: n.path, ambiguous: false };
    }
  }
  // 目标带 .md 后缀时按去后缀比较，与 Obsidian 一致
  const stem = t.endsWith(".md") ? t.slice(0, -3) : t;
  const byTitle = notes.filter((n) => n.title.toLowerCase() === stem);
  if (byTitle.length === 1) return { path: byTitle[0].path, ambiguous: false };
  if (byTitle.length > 1) return { path: byTitle[0].path, ambiguous: true };
  const byAlias = notes.filter((n) =>
    n.aliases.some((a) => a.toLowerCase() === t),
  );
  if (byAlias.length === 1) return { path: byAlias[0].path, ambiguous: false };
  if (byAlias.length > 1) return { path: byAlias[0].path, ambiguous: true };
  return { path: null, ambiguous: false };
}

/** 图片资源目标 → 相对路径（有路径用路径，无路径拼附件目录） */
export function resolveAssetPath(
  target: string,
  attachmentsDir: string,
  sourcePath: string,
): string {
  const t = target.trim().replace(/\\/g, "/");
  if (!t) return t;
  if (t.includes("/")) return t;
  // 与当前笔记同目录优先？Obsidian 行为：先附件目录
  const dir = attachmentsDir.trim().replace(/^\/+|\/+$/g, "") || "assets";
  return `${dir}/${t}`;
}
