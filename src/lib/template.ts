// 模板工具：列表 / 变量替换（{{title}} {{date}} {{time}}）
import { api } from "../ipc/tauri";
import { useSettingsStore } from "../stores/settings";

export interface TemplateFile {
  name: string;
  path: string;
}

/** 模板文件夹内的全部模板 */
export async function listTemplates(): Promise<TemplateFile[]> {
  const settings = useSettingsStore();
  const dir = (settings.data.templates_dir || "templates").replace(/^\/+|\/+$/g, "");
  try {
    const entries = await api.listDir(dir);
    return entries
      .filter((e) => !e.is_dir && e.kind === "md")
      .map((e) => ({ name: e.name.replace(/\.md$/i, ""), path: e.path }));
  } catch {
    return [];
  }
}

export function applyTemplateVars(text: string, title: string): string {
  const now = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  return text
    .replace(/\{\{title\}\}/gi, title)
    .replace(
      /\{\{date\}\}/gi,
      `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`,
    )
    .replace(/\{\{time\}\}/gi, `${pad(now.getHours())}:${pad(now.getMinutes())}`);
}
