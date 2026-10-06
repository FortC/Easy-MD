# -*- coding: utf-8 -*-
import io

def patch(path, repls, imp=None):
    s = io.open(path, encoding='utf-8').read()
    n = 0
    for a, b in repls:
        if a in s:
            s = s.replace(a, b)
            n += 1
    if imp and 'from "../../i18n"' not in s and 'from "../i18n"' not in s:
        s = s.replace(imp[0], imp[0] + '\n' + imp[1])
        n += 1
    io.open(path, 'w', encoding='utf-8').write(s)
    print(n, path)

patch('components/ai/AiDialog.vue', [
    ('title="关闭"', ':title="t(\'c.close\')"'),
    ("'explain')\">解释</button>", "'explain')\">{{ t(\"ai.explain\") }}</button>"),
    ("'summarize')\">总结</button>", "'summarize')\">{{ t(\"ai.summarize\") }}</button>"),
    ("'translate')\">翻译</button>", "'translate')\">{{ t(\"ai.translate\") }}</button>"),
    ("'polish')\">润色</button>", "'polish')\">{{ t(\"ai.polish\") }}</button>"),
    ("'continue')\">续写</button>", "'continue')\">{{ t(\"ai.continue\") }}</button>"),
])

patch('components/canvas/CanvasEditor.vue', [
    ('title="添加节点（小卡片，用于连接/分组）"', ':title="t(\'cv.addNode\')"'),
    ('<Icon name="circle" :size="12" /> 节点', '<Icon name="circle" :size="12" /> {{ t("cv.node") }}'),
    ('title="添加文本卡片（不参与连线）"', ':title="t(\'cv.addText\')"'),
    ('<Icon name="plus" :size="12" /> 文本', '<Icon name="plus" :size="12" /> {{ t("cv.text") }}'),
    ('title="添加笔记卡片（双向绑定源 md）"', ':title="t(\'cv.addNote\')"'),
    ('<Icon name="file-text" :size="12" /> 笔记', '<Icon name="file-text" :size="12" /> {{ t("cv.note") }}'),
    ('title="添加图片卡片"', ':title="t(\'cv.addImage\')"'),
    ('<Icon name="image" :size="12" /> 图片', '<Icon name="image" :size="12" /> {{ t("cv.image") }}'),
    ('title="缩放（滚轮）"', ':title="t(\'cv.zoomTitle\')"'),
    ('title="缩小"', ':title="t(\'cv.zoomOut2\')"'),
    ('title="放大"', ':title="t(\'cv.zoomIn2\')"'),
    ('title="返回笔记编辑"', ':title="t(\'cv.backEd\')"'),
    ('{{ n.text || "节点" }}', '{{ n.text || t("cv.node") }}'),
    ('text: "节点",', 'text: t("cv.node"),'),
    ('return "文本";', 'return t("cv.text");'),
    ('filters: [{ name: "图片", extensions:', 'filters: [{ name: t("cv.imageFilter"), extensions:'),
    (":title=\"'从' + sideName(s) + '拉出连线'\"", ':title="tf(\'cv.fromSide\', { side: sideName(s) })"'),
])
s = io.open('components/canvas/CanvasEditor.vue', encoding='utf-8').read()
s = s.replace('import { t } from "../../i18n";', 'import { t, tf } from "../../i18n";')
io.open('components/canvas/CanvasEditor.vue', 'w', encoding='utf-8').write(s)

patch('components/graph/GraphView.vue', [
    ('type="checkbox" /> 显示孤立笔记', 'type="checkbox" /> {{ t("gf.orphans") }}'),
    ('title="重新布局"', ':title="t(\'gf.relayout\')"'),
])

patch('components/search/SearchPalette.vue', [
    ('>全文</button>', '>{{ t("sp.content") }}</button>'),
    ('>没有匹配结果</div>', '>{{ t("sp.none") }}</div>'),
])

patch('components/common/NewNoteDialog.vue', [
    ('const title = name.value.trim() || "未命名";', 'const title = name.value.trim() || t("nn.untitled");'),
])

patch('components/editor/SourceEditor.vue', [
    ('title="问 AI（基于选中内容）"', ':title="t(\'se.askAi\')"'),
    ('wrapSel(v, "==", "==", "高亮")', 'wrapSel(v, "==", "==", t("se.phMark"))'),
    ('wrapSel(v, "[[", "]]", "笔记名")', 'wrapSel(v, "[[", "]]", t("se.phNote"))'),
    ('insertBlock(v, "| 列1 | 列2 |\\n| --- | --- |\\n| 内容 | 内容 |")', 'insertBlock(v, t("se.table"))'),
    ('placeholder = "文本")', 'placeholder = t("se.phText"))'),
    ('(isImage ? "描述" : "链接文字")', '(isImage ? t("se.phAlt") : t("se.phLink"))'),
    ('const url = isImage ? "assets/图片.png" : "https://";', 'const url = isImage ? t("se.phImg") : "https://";'),
    ('`${t.count} 篇`', 'tf("se.nNotes", { n: t.count })'),
], ('import { useAiStore } from "../../stores/ai";', 'import { t, tf } from "../../i18n";'))

patch('components/panels/OutlinePanel.vue', [
    ('{{ h.text || "(空标题)" }}', '{{ h.text || t("ol.emptyHeading") }}'),
])

patch('lib/export.ts', [
    ('`<blockquote>未找到嵌入：${e.target}</blockquote>`', '`<blockquote>${tf("ex.embedMissing", { name: e.target })}</blockquote>`'),
], ('import { resolveTarget } from "./markdown/links";', 'import { tf } from "../i18n";'))

patch('lib/markdown/renderer.ts', [
    ('subpath || "同页链接"', 'subpath || t("pv.self2")'),
], ('import MarkdownIt from "markdown-it";', 'import { t } from "../../i18n";'))

patch('stores/ai.ts', [
    ('s.selection ? `已选中 ${s.selection.length} 字` : "整篇笔记",', 's.selection ? tf("ai.selected", { n: s.selection.length }) : t("ai.whole"),'),
    ('const sys = "你是 EasyMD 笔记软件里的 AI 助手。Answer in the same language as the user\'s content. Output plain text or Markdown only.";', 'const sys = locale.value === "en"\n        ? "You are the AI assistant inside EasyMD, a note-taking app. Answer in English unless the note content is in another language. Output plain text or Markdown only."\n        : "你是 EasyMD 笔记软件里的 AI 助手。回答使用简体中文（除非笔记内容是其它语言），输出纯文本或 Markdown，不要输出无关内容。";'),
    ('? `以下是我笔记中选中的一段内容：\\n\\n<<<\\n${ctx}\\n>>>\\n\\n`', '? `${t("ai.pSelCtx")}\\n\\n<<<\\n${ctx}\\n>>>\\n\\n`'),
    (': `以下是我的整篇笔记：\\n\\n<<<\\n${ctx}\\n>>>\\n\\n`;', ': `${t("ai.pNoteCtx")}\\n\\n<<<\\n${ctx}\\n>>>\\n\\n`;'),
    ('ask: `${q}我的问题是：${this.prompt || "请分析这段内容"}`,', 'ask: `${q}${t("ai.pAsk")}${this.prompt || t("ai.pAskDef")}`,'),
    ('explain: `${q}请解释这段内容，讲清楚它在讲什么、关键概念和要点。`,', 'explain: `${q}${t("ai.pExplain")}`,'),
    ('summarize: `${q}请用要点（Markdown 无序列表）总结这段内容，控制在 8 条以内，每条一句话。`,', 'summarize: `${q}${t("ai.pSum")}`,'),
    ('translate: `${q}请把这段内容翻译成简体中文（如果已是中文则翻译成英文），保持原有格式。`,', 'translate: `${q}${t("ai.pTr")}`,'),
    ('polish: `${q}请润色这段文字：保持原意，使表达更通顺清晰，直接输出润色后的全文，不要解释。`,', 'polish: `${q}${t("ai.pPolish")}`,'),
    ('continue: `${q}请顺着内容自然续写 2-3 段，风格与原文一致，直接输出续写内容。`,', 'continue: `${q}${t("ai.pCont")}`,'),
], ('import { t, tf } from "../i18n";', 'import { locale } from "../i18n";'))

patch('stores/sync.ts', [
    ('this.lastReport = `同步完成：上传 ${r.uploaded.length} · 下载 ${r.downloaded.length} · 失败 ${r.errors.length}（本地 ${r.local_files} / 云端 ${r.remote_files}）`;', 'this.lastReport = tf("sy.done", { u: r.uploaded.length, d: r.downloaded.length, e: r.errors.length, l: r.local_files, r: r.remote_files });'),
    ('this.lastReport = "同步失败";', 'this.lastReport = t("sy.fail");'),
    ('this.message = "连接 MCP 服务…";', 'this.message = t("sy.connecting");'),
], ('import { api } from "../ipc/tauri";', 'import { t, tf } from "../i18n";'))

patch('stores/vault.ts', [
    ('name = "新画布")', 'name = t("cv.newCanvasName"))'),
], ('import { useUiStore } from "./ui";', 'import { t } from "../i18n";'))

patch('views/VaultPicker.vue', [
    ('title: "选择知识库文件夹"', 'title: t("vp.pickOpen")'),
    ('title: "选择新知识库的存放位置"', 'title: t("vp.pickCreate")'),
])

patch('components/settings/SettingsDialog.vue', [
    ('注册"用 EasyMD 打开 .md"', "{{ t('st.ctxReg') }}"),
    ('<Icon name="x" :size="13" /> 取消注册', '<Icon name="x" :size="13" /> {{ t("st.ctxUnreg") }}'),
    ('（可手动放入 .css 文件，重启或切回本页即识别）', '{{ t("st.snippetTip2") }}'),
])

patch('components/settings/TemplatePane.vue', [
    ('<Icon name="plus" :size="13" /> 新建模板', '<Icon name="plus" :size="13" /> {{ t("tp2.create") }}'),
    ('<Icon name="refresh-cw" :size="13" /> 刷新', '<Icon name="refresh-cw" :size="13" /> {{ t("tp2.refresh") }}'),
    ('模板存放于知识库内 {{ dirLabel }}/ 文件夹，支持变量 {{ varsHint }}', '{{ tf("tp2.tip", { dir: dirLabel, vars: varsHint }) }}'),
    ('title="编辑内容"', ':title="t(\'c.edit\')"'),
    ('title="删除"', ':title="t(\'c.delete\')"'),
])

patch('components/settings/AiPane.vue', [
    ('OpenAI 兼容协议适用于官方接口及各类兼容中转站（自填 Base URL）；Anthropic 走', '{{ t("ai.tip2") }}'),
    ('Messages API（x-api-key）。Key 仅保存在本机设置文件中，请求由软件后端直接发出。', ''),
    ('未配置时 AI 功能不可用，不影响其他功能。', ''),
])

patch('components/settings/McpSettings.vue', [
    ('<p>未配置 MCP 服务。云同步是可选功能，不配置不影响本地使用。</p>', '<p>{{ t("mcp.empty") }}</p>'),
    ('<Icon name="plus" :size="13" /> 添加 MCP 服务', '<Icon name="plus" :size="13" /> {{ t("mcp.add") }}'),
    (":title=\"c.name === settings.data.active_mcp ? '当前使用' : '切换为当前'\"", ':title="c.name === settings.data.active_mcp ? t(\'mcp.active\') : t(\'mcp.switchTo\')"'),
    ('title="测试连接"', ':title="t(\'mcp.test\')"'),
    ('title="编辑"', ':title="t(\'c.edit\')"'),
    ('title="删除"', ':title="t(\'c.delete\')"'),
    ('<Icon name="plus" :size="13" /> 添加\n        </button>', '<Icon name="plus" :size="13" /> {{ t("mcp.addShort") }}\n        </button>'),
    ('{{ sync.running ? sync.message || "同步中…" : "立即同步" }}', '{{ sync.running ? sync.message || t("mcp.syncing") : t("mcp.syncNow") }}'),
    ('<label>定时同步（分钟，0 = 仅手动）</label>', '<label>{{ t("mcp.schedule") }}</label>'),
    ('进度：{{ sync.done }} / {{ sync.total }}', '{{ t("mcp.progress") }}: {{ sync.done }} / {{ sync.total }}'),
    ('{{ editingOriginal ? "编辑 MCP 服务" : "添加 MCP 服务" }}', '{{ editingOriginal ? t("mcp.editTitle") : t("mcp.addTitle") }}'),
    ('<label>名称</label>', '<label>{{ t("mcp.name") }}</label>'),
    ('placeholder="如：腾讯云 / 百度网盘"', ':placeholder="t(\'mcp.namePh\')"'),
    ('<label>command</label>', '<label>{{ t("mcp.command") }}</label>'),
    ('placeholder="如 npx / 云盘mcp程序路径"', ':placeholder="t(\'mcp.cmdPh\')"'),
    ('<label>args</label>', '<label>{{ t("mcp.args") }}</label>'),
    ('placeholder="参数，空格分隔"', ':placeholder="t(\'mcp.argsPh\')"'),
    ('<label>env（每行 KEY=值）</label>', '<label>{{ t("mcp.env") }}</label>'),
    ('placeholder="TENCENT_SECRET_KEY=xxx"', ':placeholder="t(\'mcp.envPh\')"'),
    ('@click="editing = false">取消</button>', '@click="editing = false">{{ t("c.cancel") }}</button>'),
    ('@click="saveConfig">保存</button>', '@click="saveConfig">{{ t("c.save") }}</button>'),
    ('{ ok: false, text: "连接中…" };', '{ ok: false, text: t("mcp.connecting") };'),
    ('`连接成功（${res.server_name}）：${names || "无工具"}`', 'tf("mcp.connOk", { s: res.server_name, tools: names || t("mcp.noTools") })'),
    ('`连接失败：${res.error}`', '`${t("mcp.connFail")}: ${res.error}`'),
    ('`连接失败：${e}`', '`${t("mcp.connFail")}: ${e}`'),
], ('import { useSyncStore } from "../../stores/sync";', 'import { t, tf } from "../../i18n";'))

print('ALLDONE')
