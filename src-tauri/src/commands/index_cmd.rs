//! 索引查询命令：重建、标签统计、反链、链接解析。

use crate::index::model::{Backlink, NoteIndex, ResolveResult, TagCount};
use crate::state::AppState;
use tauri::State;

/// 手动重建索引（丢弃缓存全量扫描）
#[tauri::command]
pub fn rebuild_index(state: State<'_, AppState>) -> Result<Vec<NoteIndex>, String> {
    let mut guard = state.vault.lock().map_err(|e| e.to_string())?;
    let vc = guard.as_mut().ok_or("尚未打开知识库")?;
    let engine = crate::index::engine::IndexEngine::scan(&vc.root)
        .map_err(|e| format!("扫描失败：{}", e))?;
    let notes = engine.all_notes();
    vc.engine = engine;
    vc.engine.save_cache(&vc.cache_path);
    Ok(notes)
}

#[tauri::command]
pub fn get_all_notes(state: State<'_, AppState>) -> Result<Vec<NoteIndex>, String> {
    let guard = state.vault.lock().map_err(|e| e.to_string())?;
    match guard.as_ref() {
        Some(vc) => Ok(vc.engine.all_notes()),
        None => Ok(vec![]),
    }
}

#[tauri::command]
pub fn get_tags(state: State<'_, AppState>) -> Result<Vec<TagCount>, String> {
    let guard = state.vault.lock().map_err(|e| e.to_string())?;
    match guard.as_ref() {
        Some(vc) => Ok(vc.engine.tag_counts()),
        None => Ok(vec![]),
    }
}

#[tauri::command]
pub fn get_backlinks(state: State<'_, AppState>, path: String) -> Result<Vec<Backlink>, String> {
    let guard = state.vault.lock().map_err(|e| e.to_string())?;
    match guard.as_ref() {
        Some(vc) => Ok(vc.engine.backlinks(&path)),
        None => Ok(vec![]),
    }
}

#[tauri::command]
pub fn resolve_link(state: State<'_, AppState>, target: String) -> Result<ResolveResult, String> {
    let guard = state.vault.lock().map_err(|e| e.to_string())?;
    match guard.as_ref() {
        Some(vc) => Ok(vc.engine.resolve(&target)),
        None => Ok(ResolveResult { path: None, ambiguous: false }),
    }
}
