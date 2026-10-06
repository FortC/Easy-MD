# EasyMD 项目规则（AI 工具必读）

本项目是桌面端 Markdown 知识库（Tauri 2 + Vue 3 + TypeScript），UI **严格复刻 Obsidian 默认主题**，不做自由发挥。任何代码/样式改动必须遵守以下规则。

## 设计令牌（唯一色值来源：`src/assets/theme.css`）

- 禁止在组件里硬编码 hex 色值，一律使用 CSS 变量。
- 暗色：`--background-primary:#1e1e1e` / `--background-secondary:#161616` / 边框 `#333` / 正文 `#dcddde` / 弱字 `#999` / 强调紫 `#8b6cef`。
- 亮色：`#ffffff` / `#f6f6f6` / 边框 `#dbdbdc` / 正文 `#222` / 弱字 `#666` / 强调紫 `#8b6cef`。
- 字体：UI 13px、正文 16px，系统栈 `Inter → Segoe UI Variable → Segoe UI → 微软雅黑`；等宽 `JetBrains Mono → Cascadia Code → Consolas`。
- 圆角：4 / 8 / 12px 三档；阴影只用 `--shadow-l1/l2`。

## 布局（对照 Obsidian）

- 结构：左侧 44px 竖排 Ribbon → 左侧栏（文件/搜索/标签，宽 250px 可拖）→ 主编辑区 → 右侧栏（大纲/反链/属性，宽 300px 可拖）→ 底部状态栏（24px，字数/反链数）。
- 无框窗口：顶栏 40px 兼作拖拽区，右侧自绘最小化/最大化/关闭按钮。
- 弹窗 = Obsidian 模态：居中、圆角 12px、背景 `--background-secondary`。
- 命令面板 = 居中偏上搜索弹窗。

## 禁止项

- ❌ 禁止 Emoji 当图标；图标一律用内联 Lucide SVG（Obsidian 同款图标库），见 `src/components/common/Icon.vue`。
- ❌ 禁止蓝紫渐变、玻璃拟态、营销页式 Hero、"三卡片"模板化布局。
- ❌ 禁止 ease-in-out 默认缓动；动效用 `--anim-fast/--anim-medium`（cubic-bezier(0.33,0.66,0.5,1)），只做克制微反馈（hover/展开），不做入场炫技。
- ❌ 界面文案使用简体中文、具体直白，不写"赋能/极致/智能"类套话。

## 架构红线（业务）

- 磁盘 md 文件是唯一真相源；索引只是缓存（存 app 配置目录，可随时重建），永远不要用缓存反写 md。
- vault 内只允许 .md / .canvas / 资源文件；软件私有数据一律放 Tauri app 配置目录。
- 所有文件 IO 走 Rust 命令（src-tauri），前端不直接碰文件系统。
- 禁止引入插件系统、账号体系、云 SDK（云同步走 MCP 外部服务）。

## 技术栈

- 前端：Vue 3 `<script setup lang="ts">` + Pinia + CodeMirror 6 + markdown-it + force-graph。
- 后端：Rust（tauri 2 命令层 + index/ 索引引擎 + mcp/ 同步客户端）。
- 验证：改前端必须过 `npm run build`；改 Rust 必须过 `cargo check` / `cargo test`。
