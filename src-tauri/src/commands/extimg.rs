//! 外链图片代理：下载到本地缓存，经 emdasset://external/ 以正确 MIME 提供。
//! 动机：Chromium 的 ORB（Opaque Response Blocking）会把 Content-Type 非图片
//! （如 application/octet-stream，有道/常见图床）的跨域图片响应拦掉，导致
//! `<img src="https://…">` 不显示；代理下载后按真实格式（魔数嗅探）提供可绕过。

use sha2::{Digest, Sha256};
use std::time::Duration;
use tauri::AppHandle;

/// 按魔数嗅探图片格式 → 扩展名
fn sniff_ext(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        return Some("png");
    }
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some("jpg");
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Some("gif");
    }
    if bytes.len() > 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Some("webp");
    }
    if bytes.starts_with(b"BM") {
        return Some("bmp");
    }
    if bytes.starts_with(&[0x00, 0x00, 0x01, 0x00]) {
        return Some("ico");
    }
    if bytes.len() > 12 && &bytes[4..12] == b"ftypavif" {
        return Some("avif");
    }
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(1024)]).to_lowercase();
    if head.contains("<svg") {
        return Some("svg");
    }
    None
}

/// 下载外链图片到本地缓存（按 URL 哈希去重），返回缓存资源名 `external/<hash>.<ext>`
#[tauri::command]
pub fn fetch_external_image(app: AppHandle, url: String) -> Result<String, String> {
    let u = url.trim();
    if !(u.starts_with("https://") || u.starts_with("http://")) {
        return Err("仅支持 http(s) 图片链接".into());
    }
    if u.len() > 4096 {
        return Err("链接过长".into());
    }
    let dir = crate::config::config_dir(&app).join("external_cache");
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建缓存目录失败：{}", e))?;
    let hash: String = Sha256::digest(u.as_bytes())
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect();

    // 命中缓存直接返回（同一 URL 只下载一次）
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with(&hash) {
                return Ok(format!("external/{}", name));
            }
        }
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(25))
        .build()
        .map_err(|e| format!("网络初始化失败：{}", e))?;
    let resp = client
        .get(u)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36",
        )
        .header("Accept", "image/avif,image/webp,image/apng,image/svg+xml,image/*,*/*;q=0.8")
        .send()
        .map_err(|e| format!("下载失败：{}", e))?;
    if !resp.status().is_success() {
        return Err(format!("下载失败：HTTP {}", resp.status().as_u16()));
    }
    let bytes = resp.bytes().map_err(|e| format!("读取响应失败：{}", e))?;
    if bytes.is_empty() {
        return Err("下载内容为空".into());
    }
    if bytes.len() > 30 * 1024 * 1024 {
        return Err("图片超过 30MB 上限".into());
    }
    let ext = sniff_ext(&bytes).ok_or_else(|| "无法识别的图片格式（可能需要登录或链接已失效）".to_string())?;
    let name = format!("{}.{}", hash, ext);
    std::fs::write(dir.join(&name), &bytes).map_err(|e| format!("写缓存失败：{}", e))?;
    Ok(format!("external/{}", name))
}
