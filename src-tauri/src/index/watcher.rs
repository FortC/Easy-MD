//! 文件监听：notify 递归监听 vault，防抖后增量更新索引并向前端推送事件。

use crate::state::{AppState, VaultContext};
use anyhow::Result;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

pub struct WatcherHandle {
    stop: Sender<()>,
    _watcher: RecommendedWatcher,
}

impl WatcherHandle {
    pub fn stop(&self) {
        let _ = self.stop.send(());
    }
}

/// 监听事件推送给前端的数据
#[derive(serde::Serialize, Clone)]
pub struct VaultChangedPayload {
    /// 更新后的笔记索引（仅变化的）
    pub updated: Vec<crate::index::model::NoteIndex>,
    /// 已删除的笔记路径
    pub removed: Vec<String>,
    /// 变化的 canvas 文件（相对路径）
    pub canvas_changed: Vec<String>,
    /// 变化的 graph 文件（相对路径）
    pub graph_changed: Vec<String>,
}

pub fn spawn(root: PathBuf, app: AppHandle) -> Result<WatcherHandle> {
    let (tx, rx): (Sender<Event>, Receiver<Event>) = std::sync::mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |res: std::result::Result<Event, notify::Error>| {
        if let Ok(ev) = res {
            let _ = tx.send(ev);
        }
    })?;
    watcher.watch(&root, RecursiveMode::Recursive)?;

    let (stop_tx, stop_rx) = std::sync::mpsc::channel::<()>();
    std::thread::spawn(move || {
        let mut pending: HashSet<PathBuf> = HashSet::new();
        loop {
            if stop_rx.try_recv().is_ok() {
                break;
            }
            match rx.recv_timeout(Duration::from_millis(200)) {
                Ok(ev) => {
                    for p in ev.paths {
                        // 只关心 vault 内文件（忽略目录事件本身）
                        if p.is_file() || p.extension().is_some() {
                            pending.insert(p);
                        }
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    if pending.is_empty() {
                        continue;
                    }
                    handle_batch(&app, &root, std::mem::take(&mut pending));
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    });

    Ok(WatcherHandle { stop: stop_tx, _watcher: watcher })
}

fn handle_batch(app: &AppHandle, root: &std::path::Path, paths: HashSet<PathBuf>) {
    let mut payload = VaultChangedPayload {
        updated: vec![],
        removed: vec![],
        canvas_changed: vec![],
        graph_changed: vec![],
    };
    {
        let state = app.state::<AppState>();
        let mut guard = state.vault.lock().unwrap();
        let vc: &mut VaultContext = match guard.as_mut() {
            Some(vc) => vc,
            None => return,
        };
        if vc.root != root {
            return; // 已经切换了 vault，丢弃旧事件
        }
        for full in paths {
            let rel = full
                .strip_prefix(root)
                .unwrap_or(&full)
                .to_string_lossy()
                .replace('\\', "/");
            let is_md = rel.to_lowercase().ends_with(".md");
            let is_canvas = rel.to_lowercase().ends_with(".canvas");
            let is_graph = rel.to_lowercase().ends_with(".graph");
            if !is_md && !is_canvas && !is_graph {
                continue; // 资源文件变化不影响索引
            }
            if is_canvas {
                payload.canvas_changed.push(rel);
                continue;
            }
            if is_graph {
                payload.graph_changed.push(rel);
                continue;
            }
            match vc.engine.update_file(root, &rel) {
                Some(note) => payload.updated.push(note),
                None => payload.removed.push(rel),
            }
        }
        if !payload.updated.is_empty() || !payload.removed.is_empty() {
            vc.engine.save_cache(&vc.cache_path);
        }
    }
    let _ = app.emit("vault-changed", payload);
}
