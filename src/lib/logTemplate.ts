// AI 日志模板：文件化的生成规则（md / txt / skill），整个文件内容作为规则发给 AI。
import { api } from "../ipc/tauri";
import { useSettingsStore } from "../stores/settings";

export interface LogTemplateFile {
  name: string;
  path: string;
}

/** 支持的规则文件扩展名（skill = 公司内部技能/规范文档，本质是文本） */
const LOG_TPL_EXTS = [".md", ".markdown", ".txt", ".skill"];

/** 内置默认模板（无文件模板时的兜底规则） */
export const DEFAULT_LOG_TEMPLATE =
  "## {{date}} 日志\n\n### 今日完成\n{{items}}\n\n### 明日计划\n\n### 备注\n";

/** 新建模板的起始内容 */
export const LOG_TEMPLATE_STARTER = `# 日志生成规则

（整个文件会作为生成规则发给 AI，可以写很长的公司规范；支持占位符 {{date}} 和 {{items}}）

## 输出格式

## {{date}} 日志

### 今日完成
{{items}}

### 明日计划

### 备注

## 要求

- 用简体中文，语气客观
- 只输出 Markdown 正文，不要解释
`;

export function logTemplatesDir(): string {
  const s = useSettingsStore();
  return (s.data.log_templates_dir || "log-templates").replace(/^\/+|\/+$/g, "");
}

/** 日志模板文件夹内的全部规则文件 */
export async function listLogTemplates(): Promise<LogTemplateFile[]> {
  const dir = logTemplatesDir();
  try {
    const entries = await api.listDir(dir);
    return entries
      .filter(
        (e) => !e.is_dir && LOG_TPL_EXTS.some((x) => e.name.toLowerCase().endsWith(x)),
      )
      .map((e) => ({
        name: e.name.replace(/\.(md|markdown|txt|skill)$/i, ""),
        path: e.path,
      }));
  } catch {
    return [];
  }
}
