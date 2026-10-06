//! 设置读写命令。

use crate::config::AppSettings;
use tauri::{AppHandle, State};

/// 额外暴露配置目录给前端（CSS 片段管理用，只读信息）
#[derive(serde::Serialize)]
pub struct ConfigDirs {
    pub config_dir: String,
    pub snippets_dir: String,
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> AppSettings {
    crate::config::load_settings(&app)
}

#[tauri::command]
pub fn save_settings(app: AppHandle, settings: AppSettings) -> Result<(), String> {
    crate::config::save_settings(&app, &settings);
    Ok(())
}

#[tauri::command]
pub fn get_config_dirs(app: AppHandle) -> ConfigDirs {
    ConfigDirs {
        config_dir: crate::config::config_dir(&app).to_string_lossy().to_string(),
        snippets_dir: crate::config::snippets_dir(&app).to_string_lossy().to_string(),
    }
}

/// 当前 vault 根（前端拼接 emdasset:// 资源 URL 用）
#[tauri::command]
pub fn get_vault_root(state: State<'_, crate::state::AppState>) -> Result<String, String> {
    let guard = state.vault.lock().map_err(|e| e.to_string())?;
    Ok(guard
        .as_ref()
        .map(|vc| vc.root.to_string_lossy().to_string())
        .unwrap_or_default())
}

// ---- CSS 片段（存配置目录 snippets/，不属于 vault） ----

/// 列出片段目录中的 .css 文件名
#[tauri::command]
pub fn snippet_files(app: AppHandle) -> Vec<String> {
    let dir = crate::config::snippets_dir(&app);
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.to_lowercase().ends_with(".css") {
                out.push(name);
            }
        }
    }
    out.sort();
    out
}

/// 读取片段内容（仅限 snippets 目录内的 .css 文件名）
#[tauri::command]
pub fn read_snippet_file(app: AppHandle, name: String) -> Result<String, String> {
    if name.contains('/') || name.contains('\\') || !name.to_lowercase().ends_with(".css") {
        return Err("非法片段文件名".into());
    }
    let path = crate::config::snippets_dir(&app).join(&name);
    std::fs::read_to_string(&path).map_err(|e| format!("读取失败：{}", e))
}

/// 新建片段文件
#[tauri::command]
pub fn create_snippet_file(app: AppHandle, name: String) -> Result<(), String> {
    if name.contains('/') || name.contains('\\') || name.trim().is_empty() {
        return Err("非法片段文件名".into());
    }
    let file = if name.to_lowercase().ends_with(".css") {
        name
    } else {
        format!("{}.css", name)
    };
    let path = crate::config::snippets_dir(&app).join(&file);
    if path.exists() {
        return Err("同名片段已存在".into());
    }
    std::fs::write(&path, "/* EasyMD 自定义样式片段 */\n").map_err(|e| format!("创建失败：{}", e))
}

/// 删除片段文件
#[tauri::command]
pub fn delete_snippet_file(app: AppHandle, name: String) -> Result<(), String> {
    if name.contains('/') || name.contains('\\') || !name.to_lowercase().ends_with(".css") {
        return Err("非法片段文件名".into());
    }
    let path = crate::config::snippets_dir(&app).join(&name);
    std::fs::remove_file(&path).map_err(|e| format!("删除失败：{}", e))
}
