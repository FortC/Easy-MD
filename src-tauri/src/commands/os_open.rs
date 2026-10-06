//! 系统级打开：Windows 右键菜单 / 双击 md 文件拉起应用。
//! 启动参数中的文件路径 → 定位所属 vault（没有则以其所在文件夹建库）→ 打开笔记。

use crate::state::AppState;
use tauri::{AppHandle, State};

#[derive(serde::Serialize)]
pub struct OpenNoteResult {
    pub root: String,
    pub notes: Vec<crate::index::model::NoteIndex>,
    pub rel: String,
}

/// 取出启动时传入的待打开文件（读取后清空）
#[tauri::command]
pub fn get_pending_file(state: State<'_, AppState>) -> Option<String> {
    let mut pf = state.pending_file.lock().ok()?;
    pf.take()
}

/// 打开一个磁盘上的 md/canvas 文件：
/// 1) 在已知 vault 列表中找包含它的库（最长路径前缀）；
/// 2) 找不到则把它所在文件夹作为新库打开；
/// 3) 返回 vault 上下文 + 文件相对路径，前端直接展示。
#[tauri::command]
pub fn open_path_from_os(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<OpenNoteResult, String> {
    let abs = std::path::PathBuf::from(&path);
    if !abs.is_absolute() {
        return Err("必须使用绝对路径".into());
    }
    let abs_norm = abs.to_string_lossy().replace('\\', "/");
    let abs_norm = abs_norm.trim_end_matches('/').to_string();
    let lower = abs_norm.to_lowercase();

    // 找包含该文件的 vault（最长前缀优先）
    let vaults = crate::config::load_vaults(&app);
    let mut best: Option<String> = None;
    for v in &vaults {
        let vp = v.path.replace('\\', "/");
        let vp = vp.trim_end_matches('/').to_string();
        let vpl = vp.to_lowercase();
        if lower.starts_with(&format!("{}/", vpl)) {
            match &best {
                Some(b) if b.len() >= vp.len() => {}
                _ => best = Some(vp),
            }
        }
    }

    let root_str = match best {
        Some(v) => v,
        None => {
            // 不属于任何库：以文件所在文件夹为库
            abs_norm
                .rsplit_once('/')
                .map(|(dir, _)| dir.to_string())
                .unwrap_or_else(|| abs_norm.clone())
        }
    };

    let rel = abs_norm
        .strip_prefix(&root_str)
        .unwrap_or("")
        .trim_start_matches('/')
        .to_string();

    let res = super::vault::open_vault_inner(
        app,
        state.clone(),
        std::path::PathBuf::from(root_str.clone()),
        root_str,
    )?;
    Ok(OpenNoteResult {
        root: res.root,
        notes: res.notes,
        rel,
    })
}

// ---- Windows 右键菜单注册（绿色版也可用，写 HKCU 无需管理员） ----

fn write_context_reg(exe: &str) -> Result<(), String> {
    use winreg::enums::*;
    use winreg::RegKey;
    let cmd = format!("\"{}\" \"%1\"", exe);
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let classes = hkcu
        .open_subkey_with_flags("Software\\Classes", KEY_WRITE)
        .map_err(|e| format!("打开注册表失败：{}", e))?;

    for ext in ["md", "markdown", "canvas"] {
        // 1) 直接的右键菜单项："用 EasyMD 打开"
        let shell_path = format!("SystemFileAssociations\\.{}\\shell\\EasyMD", ext);
        let (key, _) = classes
            .create_subkey(&shell_path)
            .map_err(|e| format!("写入注册表失败：{}", e))?;
        key.set_value("", &"用 EasyMD 打开").map_err(|e| e.to_string())?;
        key.set_value("Icon", &exe).map_err(|e| e.to_string())?;
        let (cmd_key, _) = key
            .create_subkey("command")
            .map_err(|e| format!("写入注册表失败：{}", e))?;
        cmd_key.set_value("", &cmd).map_err(|e| e.to_string())?;

        // 2) "打开方式"候选列表
        let (prog_key, _) = classes
            .create_subkey("EasyMD.file")
            .map_err(|e| format!("写入注册表失败：{}", e))?;
        prog_key
            .set_value("", &"EasyMD 文档")
            .map_err(|e| e.to_string())?;
        let (icon_key, _) = prog_key
            .create_subkey("DefaultIcon")
            .map_err(|e| format!("写入注册表失败：{}", e))?;
        icon_key
            .set_value("", &format!("{},0", exe))
            .map_err(|e| e.to_string())?;
        let (open_key, _) = prog_key
            .create_subkey(r"shell\open\command")
            .map_err(|e| format!("写入注册表失败：{}", e))?;
        open_key.set_value("", &cmd).map_err(|e| e.to_string())?;
        let (ext_key, _) = classes
            .create_subkey(format!(".{}\\OpenWithProgids", ext))
            .map_err(|e| format!("写入注册表失败：{}", e))?;
        ext_key.set_value("EasyMD.file", &"").map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn remove_context_reg() -> Result<(), String> {
    use winreg::enums::*;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let classes = hkcu
        .open_subkey_with_flags("Software\\Classes", KEY_WRITE)
        .map_err(|e| format!("打开注册表失败：{}", e))?;
    for ext in ["md", "markdown", "canvas"] {
        let _ = classes.delete_subkey_all(format!("SystemFileAssociations\\.{}\\shell\\EasyMD", ext));
        let _ = classes.delete_subkey_all(format!(".{}\\OpenWithProgids", ext));
        let _ = classes.delete_subkey_all("EasyMD.file");
    }
    Ok(())
}



/// 临时文件打开：拷贝到内置临时目录后以该目录为 vault 打开
#[tauri::command]
pub fn open_as_temp(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<OpenNoteResult, String> {
    let src = std::path::PathBuf::from(&path);
    if !src.is_absolute() || !src.exists() {
        return Err("文件不存在".into());
    }

    // 临时目录：app_config_dir/temp/
    let temp_dir = crate::config::config_dir(&app).join("temp");
    std::fs::create_dir_all(&temp_dir).map_err(|e| format!("创建临时目录失败：{}", e))?;

    // 拷贝文件
    let name = src.file_name().unwrap_or_default().to_string_lossy().to_string();
    let dst = temp_dir.join(&name);
    std::fs::copy(&src, &dst).map_err(|e| format!("拷贝失败：{}", e))?;

    // 以临时目录为 vault 打开
    let root_str = temp_dir.to_string_lossy().to_string();
    let res = super::vault::open_vault_inner(
        app,
        state.clone(),
        std::path::PathBuf::from(temp_dir),
        root_str,
    )?;
    Ok(OpenNoteResult {
        root: res.root,
        notes: res.notes,
        rel: name,
    })
}


/// 打开 Windows 设置 → 默认应用（让用户设 EasyMD 为 .md 默认打开方式）
#[tauri::command]
pub fn open_default_apps_settings() -> Result<(), String> {
    std::process::Command::new("cmd")
        .args(["/C", "start", "ms-settings:defaultapps"])
        .spawn()
        .map_err(|e| format!("Open settings failed: {}", e))?;
    Ok(())
}

/// 注册右键菜单（使用当前运行的 exe 路径）
#[tauri::command]
pub fn register_context_menu() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| format!("获取程序路径失败：{}", e))?;
    let exe = exe.to_string_lossy().to_string();
    write_context_reg(&exe)
}

/// 取消注册
#[tauri::command]
pub fn unregister_context_menu() -> Result<(), String> {
    remove_context_reg()
}
