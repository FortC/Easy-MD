// 全局动作（ribbon / 快捷键共用）
import { useSettingsStore } from "../stores/settings";
import { useVaultStore } from "../stores/vault";
import { useEditorStore } from "../stores/editor";
import { formatDailyName } from "./daily";
import { applyTemplateVars, listTemplates } from "./template";
import { addTagToContent } from "./tags";

/**
 * 打开/创建某天的日记：
 * - 不存在时按设置套用日记模板（支持 {{title}}/{{date}}/{{time}} 变量）
 * - 配置了自动标签时写入 frontmatter tags（与标签面板/图谱/搜索联动）
 */
export async function openDailyNote(date = new Date()) {
  const settings = useSettingsStore();
  const vault = useVaultStore();
  const editor = useEditorStore();
  const name = formatDailyName(settings.data.daily_format || "YYYY-MM-DD", date);
  const dir = (settings.data.daily_dir || "daily").replace(/^\/+|\/+$/g, "");
  const path = dir ? `${dir}/${name}.md` : `${name}.md`;
  try {
    await editor.openNote(path); // 已存在直接打开
    return;
  } catch {
    /* 不存在则按模板创建 */
  }

  let content = "";
  const tplName = settings.data.daily_template;
  if (tplName) {
    const tpls = await listTemplates();
    const tpl = tpls.find((t) => t.name === tplName);
    if (tpl) {
      try {
        const { api } = await import("../ipc/tauri");
        const raw = await api.readTextFile(tpl.path);
        content = applyTemplateVars(raw, name);
      } catch {
        content = "";
      }
    }
  }
  const tag = (settings.data.daily_tag || "").trim().replace(/^#/, "");
  if (tag) content = addTagToContent(content, tag);

  await vault.newNote(dir || null, name, content);
}
