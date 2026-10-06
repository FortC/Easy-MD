//! AI 命令：OpenAI 兼容接口（含各类中转站）与 Anthropic 官方接口。
//! 仅在用户显式配置 API Key 后可用；请求由 Rust 发出，Key 不经过前端页面。

use crate::config;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use tauri::AppHandle;

/// 统一对话入口：system + user → 返回模型文本回复
#[tauri::command]
pub async fn ai_chat(
    app: AppHandle,
    prompt: String,
    system: Option<String>,
) -> Result<String, String> {
    let settings = config::load_settings(&app);
    if settings.ai_api_key.trim().is_empty() {
        return Err("未配置 AI API Key，请到 设置 → AI 中填写".into());
    }
    let prompt = truncate(prompt, 60_000);
    tauri::async_runtime::spawn_blocking(move || chat_blocking(&settings, &prompt, system.as_deref()))
        .await
        .map_err(|e| format!("AI 线程异常：{}", e))?
}

fn truncate(s: String, max: usize) -> String {
    if s.chars().count() <= max {
        return s;
    }
    let t: String = s.chars().take(max).collect();
    format!("{}\n…（内容过长已截断）", t)
}

fn chat_blocking(
    settings: &config::AppSettings,
    prompt: &str,
    system: Option<&str>,
) -> Result<String, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("HTTP 客户端创建失败：{}", e))?;

    let base = settings.ai_base_url.trim().trim_end_matches('/').to_string();
    if settings.ai_provider == "anthropic" {
        // Anthropic Messages API（base 缺 /v1 时自动补）
        let url = if base.ends_with("/v1") {
            format!("{}/messages", base)
        } else {
            format!("{}/v1/messages", base)
        };
        let mut body = json!({
            "model": settings.ai_model,
            "max_tokens": 4096,
            "messages": [{ "role": "user", "content": prompt }],
        });
        if let Some(sys) = system {
            body["system"] = json!(sys);
        }
        let resp = client
            .post(&url)
            .header("x-api-key", &settings.ai_api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .map_err(|e| format!("请求失败：{}", e))?;
        to_text(resp, |v| {
            v.get("content")?
                .as_array()?
                .iter()
                .filter_map(|p| {
                    if p.get("type").and_then(|t| t.as_str()) == Some("text") {
                        p.get("text").and_then(|t| t.as_str())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("")
                .into()
        })
    } else {
        // OpenAI 兼容 chat/completions（各类兼容中转站同构）
        let url = if base.ends_with("/v1") {
            format!("{}/chat/completions", base)
        } else {
            format!("{}/v1/chat/completions", base)
        };
        let mut messages = vec![];
        if let Some(sys) = system {
            messages.push(json!({ "role": "system", "content": sys }));
        }
        messages.push(json!({ "role": "user", "content": prompt }));
        let body = json!({ "model": settings.ai_model, "messages": messages, "stream": false });
        let resp = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", settings.ai_api_key))
            .json(&body)
            .send()
            .map_err(|e| format!("请求失败：{}", e))?;
        to_text(resp, |v| {
            v.get("choices")?
                .get(0)?
                .get("message")?
                .get("content")?
                .as_str()?
                .to_string()
                .into()
        })
    }
}

/// 解析响应：先看 HTTP 错误体，再按各家结构取文本
fn to_text(
    resp: reqwest::blocking::Response,
    pick: impl Fn(&Value) -> Option<String>,
) -> Result<String, String> {
    let status = resp.status();
    let text = resp
        .text()
        .map_err(|e| format!("读取响应失败：{}", e))?;
    let v: Value = serde_json::from_str(&text)
        .map_err(|_| anyhow!("响应不是 JSON：{}", &text[..text.len().min(300)]))
        .map_err(|e| e.to_string())?;
    if !status.is_success() {
        let msg = v
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
            .or_else(|| v.get("message").and_then(|m| m.as_str()))
            .unwrap_or("未知错误");
        return Err(format!("AI 服务返回 {}：{}", status.as_u16(), msg));
    }
    pick(&v).ok_or_else(|| format!("无法解析 AI 响应：{}", &text[..text.len().min(300)]))
}
