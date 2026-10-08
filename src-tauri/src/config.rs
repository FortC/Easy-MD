//! 应用配置：settings.json / vaults.json，存放于 Tauri app 配置目录（vault 之外）。
//! vault 列表仅记录路径，不存任何笔记业务数据。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

// ---------------- 数据结构 ----------------

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct AppSettings {
    pub theme: String,               // "dark" | "light"
    pub attachments_dir: String,     // 粘贴图片存放目录（vault 内相对路径）
    pub templates_dir: String,       // 模板文件夹
    pub daily_dir: String,           // 每日笔记存放目录
    pub daily_format: String,        // 日记文件名格式
    /// 日记使用的模板文件名（templates_dir 内，不含路径）
    pub daily_template: Option<String>,
    /// 新建日记自动附加的标签（空 = 不加）
    pub daily_tag: String,
    pub last_vault: Option<String>,
    pub mcp_configs: Vec<McpServerConfig>,
    pub active_mcp: Option<String>,
    pub sync_interval_minutes: u64, // 0 = 仅手动同步
    pub css_snippets: Vec<SnippetState>,
    /// AI 配置：openai 兼容接口 或 anthropic
    pub ai_provider: String, // "openai" | "anthropic"
    pub ai_base_url: String, // 自定义 baseURL（中转/兼容站）
    pub ai_api_key: String,
    pub ai_model: String,
    /// 界面语言：zh | en
    pub language: String,
    /// 全局字体缩放：xs | sm | md | lg | xl
    pub font_scale: String,
    /// AI 日志模板
    pub log_template: String,
    /// AI 日志模板文件夹（vault 内相对路径，放 md/txt/skill 规则文件）
    pub log_templates_dir: String,
    /// 选中的日志模板文件名（空 = 使用内置默认模板）
    pub log_template_name: String,
    /// 外部文件（右键/双击打开）默认处理方式：ask = 每次询问 | vault | temp
    pub os_open_mode: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SnippetState {
    pub name: String,
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub enabled: bool,
    /// 同步四操作绑定的工具名（空 = 按约定名自动匹配）
    pub tool_map: McpToolMap,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct McpToolMap {
    pub list: Option<String>,
    pub download: Option<String>,
    pub upload: Option<String>,
    pub delete: Option<String>,
    pub mkdir: Option<String>,
}

impl Default for McpServerConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            command: String::new(),
            args: vec![],
            env: HashMap::new(),
            enabled: true,
            tool_map: McpToolMap::default(),
        }
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: "dark".into(),
            attachments_dir: "assets".into(),
            templates_dir: "templates".into(),
            daily_dir: "daily".into(),
            daily_format: "YYYY-MM-DD".into(),
            daily_template: None,
            daily_tag: String::new(),
            last_vault: None,
            mcp_configs: vec![],
            active_mcp: None,
            sync_interval_minutes: 0,
            css_snippets: vec![],
            ai_provider: "openai".into(),
            ai_base_url: "https://api.openai.com/v1".into(),
            ai_api_key: String::new(),
            ai_model: "gpt-4o-mini".into(),
            language: "zh".into(),
            font_scale: "sm".into(),
            log_template: String::new(),
            log_templates_dir: "log-templates".into(),
            log_template_name: String::new(),
            os_open_mode: "ask".into(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct VaultEntry {
    pub path: String,
    pub name: String,
    pub last_opened: String,
}

// ---------------- 读写 ----------------

pub fn config_dir(app: &AppHandle) -> PathBuf {
    let dir = app
        .path()
        .app_config_dir()
        .expect("无法获取配置目录");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

pub fn snippets_dir(app: &AppHandle) -> PathBuf {
    let dir = config_dir(app).join("snippets");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &std::path::Path, default: T) -> T {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or(default),
        Err(_) => default,
    }
}

fn write_json<T: Serialize + ?Sized>(path: &std::path::Path, value: &T) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_vec_pretty(value) {
        let tmp = path.with_extension("tmp");
        if std::fs::write(&tmp, &json).is_ok() {
            let _ = std::fs::rename(&tmp, path);
        }
    }
}

pub fn load_settings(app: &AppHandle) -> AppSettings {
    read_json(&config_dir(app).join("settings.json"), AppSettings::default())
}

pub fn save_settings(app: &AppHandle, settings: &AppSettings) {
    write_json(&config_dir(app).join("settings.json"), settings);
}

pub fn load_vaults(app: &AppHandle) -> Vec<VaultEntry> {
    read_json(&config_dir(app).join("vaults.json"), Vec::new())
}

pub fn save_vaults(app: &AppHandle, vaults: &[VaultEntry]) {
    write_json(&config_dir(app).join("vaults.json"), vaults);
}

/// 记录 vault 打开（新增或更新 last_opened）
pub fn touch_vault(app: &AppHandle, path: &str) {
    let mut vaults = load_vaults(app);
    let name = std::path::Path::new(path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string());
    let now = chrono::Local::now().to_rfc3339();
    match vaults.iter_mut().find(|v| v.path == path) {
        Some(v) => {
            v.last_opened = now;
            v.name = name;
        }
        None => vaults.push(VaultEntry {
            path: path.to_string(),
            name,
            last_opened: now,
        }),
    }
    vaults.sort_by(|a, b| b.last_opened.cmp(&a.last_opened));
    save_vaults(app, &vaults);
}
