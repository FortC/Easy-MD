// 快速加标签：写回 frontmatter（与属性面板同一真相源：md 文本）
import YAML from "yaml";
import { stripFrontmatter } from "./markdown/renderer";

export function addTagToContent(content: string, tag: string): string {
  tag = tag.trim().replace(/^#/, "");
  if (!tag) return content;
  const { fm, body } = stripFrontmatter(content);
  let obj: Record<string, unknown> = {};
  if (fm !== null) {
    try {
      obj = (YAML.parse(fm) as Record<string, unknown>) || {};
    } catch {
      return content; // frontmatter 非法时不動
    }
  }
  const v = obj["tags"] ?? obj["tag"];
  let arr: string[] = [];
  if (typeof v === "string") arr = v.split(",").map((s) => s.trim()).filter(Boolean);
  else if (Array.isArray(v)) arr = v.map(String);
  if (!arr.includes(tag)) arr.push(tag);
  obj["tags"] = arr;
  delete obj["tag"];
  const yamlText = YAML.stringify(obj).trimEnd();
  return `---\n${yamlText}\n---\n${body}`;
}
