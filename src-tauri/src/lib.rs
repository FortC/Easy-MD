//! EasyMD 入口：命令注册 + 自定义资源协议。
//! emdasset://vault/<percent-encoded 相对路径> —— 只允许读当前 vault 内文件。

mod commands;
mod config;
mod index;
mod mcp;
mod state;

use std::path::{Component, Path, PathBuf};
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        // 单实例：已运行时右键/双击打开文件 → 聚焦窗口并转发路径（必须最先注册）
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            use tauri::Emitter;
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
            if let Some(file) = args.iter().skip(1).rev().find(|a| {
                let l = a.to_lowercase();
                l.ends_with(".md") || l.ends_with(".markdown") || l.ends_with(".canvas")
            }) {
                let _ = app.emit("os-open-file", file.clone());
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .manage(state::AppState::default())
        .register_uri_scheme_protocol("emdasset", |ctx, request| {
            serve_asset(ctx.app_handle(), &request.uri().to_string())
        })
        .setup(|app| {
            // 捕获启动参数中的文件路径（右键"用 EasyMD 打开"）
            let st = app.state::<state::AppState>();
            if let Some(file) = std::env::args().skip(1).rev().find(|a| {
                let l = a.to_lowercase();
                l.ends_with(".md") || l.ends_with(".markdown") || l.ends_with(".canvas")
            }) {
                if let Ok(mut pf) = st.pending_file.lock() {
                    *pf = Some(file);
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::vault::list_vaults,
            commands::vault::open_vault,
            commands::vault::create_vault,
            commands::vault::close_vault,
            commands::vault::forget_vault,
            commands::fsops::read_text_file,
            commands::fsops::write_text_file,
            commands::fsops::path_exists,
            commands::fsops::list_dir,
            commands::fsops::create_note,
            commands::fsops::create_folder,
            commands::fsops::rename_path,
            commands::fsops::delete_path,
            commands::fsops::save_image,
            commands::fsops::write_external,
            commands::search::search_files,
            commands::search::search_content,
            commands::search::search_by_tag,
            commands::index_cmd::rebuild_index,
            commands::index_cmd::get_all_notes,
            commands::index_cmd::get_tags,
            commands::index_cmd::get_backlinks,
            commands::index_cmd::resolve_link,
            commands::settings::get_settings,
            commands::settings::save_settings,
            commands::settings::get_config_dirs,
            commands::settings::get_vault_root,
            commands::mcp_cmd::mcp_test,
            commands::mcp_cmd::mcp_sync,
            commands::settings::snippet_files,
            commands::settings::read_snippet_file,
            commands::settings::create_snippet_file,
            commands::settings::delete_snippet_file,
            commands::os_open::get_pending_file,
            commands::os_open::open_path_from_os,
            commands::os_open::open_as_temp,
            commands::os_open::register_context_menu,
            commands::os_open::unregister_context_menu,
            commands::os_open::open_default_apps_settings,
            commands::fsops::import_files,
            commands::fsops::read_external_binary,
            commands::fsops::list_all_files_with_mtime,
            commands::fsops::duplicate_file,
            commands::fsops::delete_files,
            commands::ai::ai_chat,
        ])
        .run(tauri::generate_context!())
        .expect("EasyMD 启动失败");
}

fn serve_asset(app: &tauri::AppHandle, uri: &str) -> tauri::http::Response<Vec<u8>> {
    use tauri::http::{Response, StatusCode};
    let not_found = || {
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Vec::new())
            .unwrap()
    };

    // emdasset://vault/<encoded-rel-path>（scheme/host 大小写容错）
    let uri_lower = uri.to_lowercase();
    let rest = match uri_lower.strip_prefix("emdasset://vault/") {
        Some(r) => &uri[uri.len() - r.len()..],
        None => return not_found(),
    };
    let rel = percent_encoding::percent_decode_str(rest)
        .decode_utf8_lossy()
        .to_string();

    // 相对路径安全校验（拒绝绝对路径与 .. 穿越）
    let rel_path = Path::new(&rel);
    if rel_path.is_absolute()
        || rel_path
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::RootDir | Component::Prefix(_)))
    {
        return not_found();
    }

    let state = app.state::<state::AppState>();
    let guard = match state.vault.lock() {
        Ok(g) => g,
        Err(_) => return not_found(),
    };
    let vc = match guard.as_ref() {
        Some(vc) => vc,
        None => return not_found(),
    };
    let full: PathBuf = vc.root.join(rel_path);
    if !full.starts_with(&vc.root) {
        return not_found();
    }

    // 大小写不敏感二次校验（Windows 路径 join 后仍需确认未越界）
    let root_lower = vc.root.to_string_lossy().to_lowercase();
    let full_lower = full.to_string_lossy().to_lowercase();
    if !full_lower.starts_with(&root_lower) {
        return not_found();
    }

    // 双重编码防御：webview 偶发把已编码 URL 再编码一次（% → %25），解码失败时重试
    let mut bytes = std::fs::read(&full).ok();
    if bytes.is_none() && rest.contains('%') {
        let rel2 = percent_encoding::percent_decode_str(&rel)
            .decode_utf8_lossy()
            .to_string();
        if !rel2.is_empty()
            && !rel2.contains("..")
            && !Path::new(&rel2).is_absolute()
        {
            bytes = std::fs::read(vc.root.join(rel2.replace('\\', "/"))).ok();
        }
    }
    let bytes = match bytes {
        Some(b) => b,
        None => return not_found(),
    };
    let mime = mime_of(full.extension().and_then(|e| e.to_str()).unwrap_or(""));
    Response::builder()
        .header("Content-Type", mime)
        .header("Cache-Control", "max-age=300")
        .body(bytes)
        .unwrap()
}

fn mime_of(ext: &str) -> &'static str {
    match ext.to_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "avif" => "image/avif",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
}
