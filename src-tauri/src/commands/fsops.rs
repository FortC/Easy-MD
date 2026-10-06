//! 文件操作命令：读写 md/canvas、文件树、新建/重命名/删除、粘贴图片。
//! 所有相对路径都经 safe_join 校验，出不了 vault。

use crate::config;
use crate::state::{AppState, VaultContext};
use chrono::Local;
use std::path::Path;
use tauri::{AppHandle, State};

fn with_vault<T>(
    state: &State<'_, AppState>,
    f: impl FnOnce(&VaultContext) -> Result<T, String>,
) -> Result<T, String> {
    let guard = state.vault.lock().map_err(|e| e.to_string())?;
    match guard.as_ref() {
        Some(vc) => f(vc),
        None => Err("尚未打开知识库".into()),
    }
}

// ---------------- 读写 ----------------

#[tauri::command]
pub fn read_text_file(state: State<'_, AppState>, path: String) -> Result<String, String> {
    with_vault(&state, |vc| {
        let full = vc.safe_join(&path).map_err(|e| e.to_string())?;
        std::fs::read_to_string(&full).map_err(|e| format!("读取失败：{}", e))
    })
}

/// 写文件（md / canvas / css 片段以外的 vault 内文本）
#[tauri::command]
pub fn write_text_file(
    state: State<'_, AppState>,
    path: String,
    content: String,
) -> Result<(), String> {
    with_vault(&state, |vc| {
        let full = vc.safe_join(&path).map_err(|e| e.to_string())?;
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("创建目录失败：{}", e))?;
        }
        std::fs::write(&full, content).map_err(|e| format!("写入失败：{}", e))
    })
}

#[tauri::command]
pub fn path_exists(state: State<'_, AppState>, path: String) -> Result<bool, String> {
    with_vault(&state, |vc| {
        let full = vc.safe_join(&path).map_err(|e| e.to_string())?;
        Ok(full.exists())
    })
}

// ---------------- 文件树 ----------------

#[derive(serde::Serialize)]
pub struct FsEntry {
    pub name: String,
    /// 相对路径（'/' 分隔）
    pub path: String,
    pub is_dir: bool,
    /// md / canvas / image / other
    pub kind: String,
}

fn entry_kind(name: &str) -> &'static str {
    let lower = name.to_lowercase();
    for ext in ["md", "markdown"] {
        if lower.ends_with(ext) {
            return "md";
        }
    }
    if lower.ends_with(".canvas") {
        return "canvas";
    }
    if lower.ends_with(".graph") {
        return "graph";
    }
    if [
        ".png", ".jpg", ".jpeg", ".gif", ".webp", ".svg", ".bmp", ".ico", ".avif",
    ]
    .iter()
    .any(|e| lower.ends_with(e))
    {
        return "image";
    }
    "other"
}

/// 列出某目录下一层内容（目录在前，按名称排序）
#[tauri::command]
pub fn list_dir(state: State<'_, AppState>, path: Option<String>) -> Result<Vec<FsEntry>, String> {
    with_vault(&state, |vc| {
        let full = vc
            .safe_join(path.as_deref().unwrap_or(""))
            .map_err(|e| e.to_string())?;
        let mut dirs: Vec<FsEntry> = vec![];
        let mut files: Vec<FsEntry> = vec![];
        let entries = std::fs::read_dir(&full).map_err(|e| format!("读取目录失败：{}", e))?;
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || name == "node_modules" || name == "Thumbs.db" {
                continue;
            }
            let rel = e
                .path()
                .strip_prefix(&vc.root)
                .unwrap_or(&e.path())
                .to_string_lossy()
                .replace('\\', "/");
            let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
            let item = FsEntry {
                name: name.clone(),
                path: rel,
                is_dir,
                kind: if is_dir { "dir".into() } else { entry_kind(&name).into() },
            };
            if is_dir {
                dirs.push(item);
            } else {
                files.push(item);
            }
        }
        dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        dirs.extend(files);
        Ok(dirs)
    })
}

// ---------------- 新建 / 重命名 / 删除 ----------------

/// 文件名净化：去掉 Windows 非法字符
fn sanitize_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            _ => c,
        })
        .collect();
    let trimmed = cleaned.trim().trim_end_matches('.');
    if trimmed.is_empty() {
        "未命名".to_string()
    } else {
        trimmed.to_string()
    }
}

/// 同名自动加序号：foo.md → foo 2.md
fn unique_path(mut full: std::path::PathBuf) -> std::path::PathBuf {
    if !full.exists() {
        return full;
    }
    let stem = full.file_stem().unwrap_or_default().to_string_lossy().to_string();
    let ext = full
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    let parent = full.parent().unwrap_or(Path::new("")).to_path_buf();
    for i in 2..1000 {
        full = parent.join(format!("{} {}{}", stem, i, ext));
        if !full.exists() {
            return full;
        }
    }
    full
}

/// 新建笔记（可选初始内容，例如模板）
#[tauri::command]
pub fn create_note(
    state: State<'_, AppState>,
    folder: Option<String>,
    title: String,
    content: Option<String>,
) -> Result<String, String> {
    with_vault(&state, |vc| {
        let dir = folder.unwrap_or_default();
        let name = sanitize_name(&title);
        let name = if name.to_lowercase().ends_with(".md") { name } else { format!("{}.md", name) };
        let dir_full = vc.safe_join(&dir).map_err(|e| e.to_string())?;
        // 父目录不存在时自动创建（新建日记/模板等场景）
        std::fs::create_dir_all(&dir_full).map_err(|e| format!("创建目录失败：{}", e))?;
        let full = unique_path(dir_full.join(&name));
        std::fs::write(&full, content.unwrap_or_default())
            .map_err(|e| format!("创建失败：{}", e))?;
        Ok(full
            .strip_prefix(&vc.root)
            .unwrap_or(&full)
            .to_string_lossy()
            .replace('\\', "/"))
    })
}

#[tauri::command]
pub fn create_folder(
    state: State<'_, AppState>,
    parent: Option<String>,
    name: String,
) -> Result<String, String> {
    with_vault(&state, |vc| {
        let full = unique_path(
            vc.safe_join(parent.as_deref().unwrap_or(""))
                .map_err(|e| e.to_string())?
                .join(sanitize_name(&name)),
        );
        std::fs::create_dir_all(&full).map_err(|e| format!("创建失败：{}", e))?;
        Ok(full
            .strip_prefix(&vc.root)
            .unwrap_or(&full)
            .to_string_lossy()
            .replace('\\', "/"))
    })
}

/// 重命名（同目录）；md 笔记自动保留扩展名
#[tauri::command]
pub fn rename_path(
    state: State<'_, AppState>,
    path: String,
    new_name: String,
) -> Result<String, String> {
    with_vault(&state, |vc| {
        let old = vc.safe_join(&path).map_err(|e| e.to_string())?;
        if !old.exists() {
            return Err("文件不存在".into());
        }
        let mut name = sanitize_name(&new_name);
        // 未带扩展名时保留原扩展（md / canvas / graph 等）
        let orig_ext = old
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy().to_lowercase()));
        if let Some(ext) = orig_ext {
            if !name.to_lowercase().ends_with(&ext) {
                name.push_str(&ext);
            }
        }
        let parent = old.parent().unwrap_or(Path::new("")).to_path_buf();
        let new_full = unique_path(parent.join(&name));
        std::fs::rename(&old, &new_full).map_err(|e| format!("重命名失败：{}", e))?;
        Ok(new_full
            .strip_prefix(&vc.root)
            .unwrap_or(&new_full)
            .to_string_lossy()
            .replace('\\', "/"))
    })
}

/// 删除到系统回收站
#[tauri::command]
pub fn delete_path(state: State<'_, AppState>, path: String) -> Result<(), String> {
    with_vault(&state, |vc| {
        let full = vc.safe_join(&path).map_err(|e| e.to_string())?;
        if full == vc.root {
            return Err("不能删除知识库根目录".into());
        }
        trash::delete(&full).map_err(|e| format!("删除失败：{}", e))
    })
}

// ---------------- 粘贴图片 ----------------

/// 保存粘贴的图片字节到附件目录，返回相对路径
#[tauri::command]
pub fn save_image(
    app: AppHandle,
    state: State<'_, AppState>,
    data: Vec<u8>,
    ext: String,
) -> Result<String, String> {
    let settings = config::load_settings(&app);
    with_vault(&state, |vc| {
        let dir = settings.attachments_dir.trim().trim_matches('/').to_string();
        let dir = if dir.is_empty() { "assets".to_string() } else { dir };
        let ext = ext.trim_start_matches('.').to_lowercase();
        let ext = if ext.is_empty() { "png".to_string() } else { ext };
        let now = Local::now();
        let name = format!("Pasted image {}.{}", now.format("%Y%m%d%H%M%S"), ext);
        let dir_full = vc.safe_join(&dir).map_err(|e| e.to_string())?;
        std::fs::create_dir_all(&dir_full).map_err(|e| format!("创建附件目录失败：{}", e))?;
        let full = unique_path(dir_full.join(&name));
        std::fs::write(&full, &data).map_err(|e| format!("写入图片失败：{}", e))?;
        Ok(full
            .strip_prefix(&vc.root)
            .unwrap_or(&full)
            .to_string_lossy()
            .replace('\\', "/"))
    })
}

/// 全库文件列表（含修改时间）：日历看板按日期聚合
#[derive(serde::Serialize)]
pub struct FileWithTime {
    pub path: String,
    pub mtime: u64,
    /// md | canvas | image | other
    pub kind: String,
}

#[tauri::command]
pub fn list_all_files_with_mtime(state: State<'_, AppState>) -> Result<Vec<FileWithTime>, String> {
    with_vault(&state, |vc| {
        let mut out = Vec::new();
        for entry in walkdir::WalkDir::new(&vc.root)
            .into_iter()
            .filter_entry(|e| {
                e.depth() == 0
                    || (!e.file_name().to_string_lossy().starts_with('.')
                        && e.file_name() != "node_modules"
                        && e.file_name().to_string_lossy() != "Thumbs.db")
            })
        {
            let Ok(e) = entry else { continue };
            if !e.file_type().is_file() {
                continue;
            }
            let name = e.file_name().to_string_lossy().to_string();
            let kind = entry_kind(&name).to_string();
            if kind == "other" {
                continue; // 只关注 md / canvas / graph / image
            }
            let rel = e
                .path()
                .strip_prefix(&vc.root)
                .unwrap_or(e.path())
                .to_string_lossy()
                .replace('\\', "/");
            let mtime = e
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            out.push(FileWithTime { path: rel, mtime, kind });
        }
        out.sort_by(|a, b| b.mtime.cmp(&a.mtime));
        Ok(out)
    })
}

/// 复制文件为副本：foo.md → foo 副本.md
#[tauri::command]
pub fn duplicate_file(state: State<'_, AppState>, path: String) -> Result<String, String> {
    with_vault(&state, |vc| {
        let src = vc.safe_join(&path).map_err(|e| e.to_string())?;
        if !src.exists() {
            return Err("文件不存在".into());
        }
        let _name = src.file_name().unwrap_or_default().to_string_lossy().to_string();
        let stem = src.file_stem().unwrap_or_default().to_string_lossy().to_string();
        let ext = src.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
        let parent = src.parent().unwrap_or(Path::new("")).to_path_buf();
        let dst = unique_path(parent.join(format!("{} 副本{}", stem, ext)));
        std::fs::copy(&src, &dst).map_err(|e| format!("复制失败：{}", e))?;
        Ok(dst
            .strip_prefix(&vc.root)
            .unwrap_or(&dst)
            .to_string_lossy()
            .replace('\\', "/"))
    })
}

/// 批量删除（到回收站）
#[tauri::command]
pub fn delete_files(state: State<'_, AppState>, paths: Vec<String>) -> Result<Vec<String>, String> {
    with_vault(&state, |vc| {
        let mut deleted = Vec::new();
        let mut errors = Vec::new();
        for p in &paths {
            let full = vc.safe_join(p).map_err(|e| e.to_string())?;
            if full == vc.root {
                continue;
            }
            match trash::delete(&full) {
                Ok(()) => deleted.push(p.clone()),
                Err(e) => errors.push(format!("{}: {}", p, e)),
            }
        }
        if !errors.is_empty() {
            return Err(errors.join("; "));
        }
        Ok(deleted)
    })
}

/// 导出用：把文本写到 vault 外（用户通过保存对话框选定的绝对路径）
#[tauri::command]
pub fn write_external(path: String, content: String) -> Result<(), String> {
    let full = Path::new(&path);
    if !full.is_absolute() {
        return Err("必须使用绝对路径".into());
    }
    std::fs::write(full, content).map_err(|e| format!("写入失败：{}", e))
}

/// 读取 vault 外文件的字节（图片导入画布等场景，路径来自系统文件对话框）
#[tauri::command]
pub fn read_external_binary(path: String) -> Result<Vec<u8>, String> {
    let full = Path::new(&path);
    if !full.is_absolute() {
        return Err("必须使用绝对路径".into());
    }
    std::fs::read(full).map_err(|e| format!("读取失败：{}", e))
}

/// 导入文件：把外部文件复制进 vault 指定目录（同名自动加序号）
#[tauri::command]
pub fn import_files(
    state: State<'_, AppState>,
    folder: Option<String>,
    paths: Vec<String>,
) -> Result<Vec<String>, String> {
    with_vault(&state, |vc| {
        let dir = folder.unwrap_or_default();
        let dir_full = vc.safe_join(&dir).map_err(|e| e.to_string())?;
        std::fs::create_dir_all(&dir_full).map_err(|e| format!("创建目录失败：{}", e))?;
        let mut imported = Vec::new();
        for p in &paths {
            let src = Path::new(p);
            if !src.is_absolute() || !src.exists() {
                continue;
            }
            let name = src
                .file_name()
                .map(|n| sanitize_name(&n.to_string_lossy()))
                .unwrap_or_else(|| "导入文件".into());
            let full = unique_path(dir_full.join(&name));
            std::fs::copy(src, &full).map_err(|e| format!("复制 {} 失败：{}", name, e))?;
            imported.push(
                full.strip_prefix(&vc.root)
                    .unwrap_or(&full)
                    .to_string_lossy()
                    .replace('\\', "/"),
            );
        }
        Ok(imported)
    })
}
