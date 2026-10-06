//! Vault 管理命令：打开/创建/关闭/移除知识库。

use crate::config;
use crate::index::engine::IndexEngine;
use crate::index::model::NoteIndex;
use crate::index::watcher;
use crate::state::{AppState, VaultContext};
use std::path::PathBuf;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn list_vaults(app: AppHandle) -> Vec<config::VaultEntry> {
    config::load_vaults(&app)
}

/// 打开已有 vault：增量加载索引、启动文件监听
#[tauri::command]
pub fn open_vault(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<OpenVaultResult, String> {
    let root = PathBuf::from(&path);
    let meta = std::fs::metadata(&root).map_err(|e| format!("无法访问文件夹：{}", e))?;
    if !meta.is_dir() {
        return Err("所选路径不是文件夹".into());
    }
    open_vault_inner(app, state, root, path)
}

/// 创建新 vault（空文件夹）并打开
#[tauri::command]
pub fn create_vault(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<OpenVaultResult, String> {
    let root = PathBuf::from(&path);
    if root.exists() {
        return Err("目录已存在".into());
    }
    std::fs::create_dir_all(&root).map_err(|e| format!("创建失败：{}", e))?;
    open_vault_inner(app, state, root, path)
}

pub(crate) fn open_vault_inner(
    app: AppHandle,
    state: State<'_, AppState>,
    root: PathBuf,
    path: String,
) -> Result<OpenVaultResult, String> {
    let cache_path = IndexEngine::cache_path(&config::config_dir(&app), &root);

    // 关闭旧 vault 的监听
    {
        let mut guard = state.vault.lock().map_err(|e| e.to_string())?;
        if let Some(old) = guard.as_mut() {
            if let Some(w) = old.watcher.take() {
                w.stop();
            }
        }
        *guard = None;
    }

    // 索引：优先缓存，然后对磁盘校验做增量更新（缓存永远可被磁盘推翻）
    let mut engine = IndexEngine::load_cache(&cache_path)
        .or_else(|| IndexEngine::scan(&root).ok())
        .ok_or_else(|| "索引扫描失败".to_string())?;
    engine.refresh_against_disk(&root);
    engine.save_cache(&cache_path);

    let watch = watcher::spawn(root.clone(), app.clone())
        .map_err(|e| format!("文件监听启动失败：{}", e))?;

    let notes = engine.all_notes();
    {
        let mut guard = state.vault.lock().map_err(|e| e.to_string())?;
        *guard = Some(VaultContext {
            root: root.clone(),
            engine,
            cache_path,
            watcher: Some(watch),
        });
    }

    // 记录 vault 列表与最近打开
    config::touch_vault(&app, &path);
    let mut settings = config::load_settings(&app);
    settings.last_vault = Some(path.clone());
    config::save_settings(&app, &settings);

    Ok(OpenVaultResult { root: path, notes })
}

#[tauri::command]
pub fn close_vault(state: State<'_, AppState>) -> Result<(), String> {
    let mut guard = state.vault.lock().map_err(|e| e.to_string())?;
    if let Some(vc) = guard.as_mut() {
        if let Some(w) = vc.watcher.take() {
            w.stop();
        }
    }
    *guard = None;
    Ok(())
}

#[tauri::command]
pub fn forget_vault(app: AppHandle, path: String) -> Result<(), String> {
    let mut vaults = config::load_vaults(&app);
    vaults.retain(|v| v.path != path);
    config::save_vaults(&app, &vaults);
    let mut settings = config::load_settings(&app);
    if settings.last_vault.as_deref() == Some(path.as_str()) {
        settings.last_vault = vaults.first().map(|v| v.path.clone());
        config::save_settings(&app, &settings);
    }
    Ok(())
}

#[derive(serde::Serialize)]
pub struct OpenVaultResult {
    pub root: String,
    pub notes: Vec<NoteIndex>,
}
