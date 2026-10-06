//! MCP 命令：测试连接 / 触发同步（进度通过事件推送前端）。

use crate::config::{self, McpServerConfig};
use crate::state::AppState;
use tauri::{AppHandle, Emitter, State};

#[derive(serde::Serialize)]
pub struct TestResult {
    pub ok: bool,
    pub server_name: String,
    pub tools: Vec<crate::mcp::client::ToolInfo>,
    pub error: String,
}

/// 测试 MCP 配置：连接 + 列出工具（设置页兼容性自检）
#[tauri::command]
pub async fn mcp_test(_app: AppHandle, config: McpServerConfig) -> Result<TestResult, String> {
    let res = tauri::async_runtime::spawn_blocking(move || {
        match crate::mcp::client::McpClient::connect(&config.command, &config.args, &config.env, &config.name) {
            Ok(mut client) => match client.list_tools() {
                Ok(tools) => TestResult {
                    ok: true,
                    server_name: client.server_name().to_string(),
                    tools,
                    error: String::new(),
                },
                Err(e) => TestResult {
                    ok: false,
                    server_name: config.name.clone(),
                    tools: vec![],
                    error: format!("工具列表获取失败：{}", e),
                },
            },
            Err(e) => TestResult {
                ok: false,
                server_name: config.name.clone(),
                tools: vec![],
                error: format!("{}", e),
            },
        }
    })
    .await
    .map_err(|e| format!("{}", e))?;
    Ok(res)
}

#[derive(serde::Serialize, Clone)]
pub struct SyncProgress {
    pub message: String,
    pub done: usize,
    pub total: usize,
}

/// 手动/定时同步入口：使用当前激活的 MCP 配置
#[tauri::command]
pub async fn mcp_sync(app: AppHandle, state: State<'_, AppState>) -> Result<crate::mcp::sync::SyncReport, String> {
    let settings = config::load_settings(&app);
    let name = settings
        .active_mcp
        .clone()
        .ok_or("未选择 MCP 服务，请先在设置中配置并激活")?;
    let cfg = settings
        .mcp_configs
        .iter()
        .find(|c| c.name == name)
        .cloned()
        .ok_or(format!("找不到 MCP 配置：{}", name))?;
    if !cfg.enabled {
        return Err(format!("MCP 服务「{}」已停用", cfg.name));
    }

    let root = {
        let guard = state.vault.lock().map_err(|e| e.to_string())?;
        guard
            .as_ref()
            .map(|vc| vc.root.clone())
            .ok_or("尚未打开知识库")?
    };

    let app2 = app.clone();
    let handle = tauri::async_runtime::spawn_blocking(move || {
        crate::mcp::sync::run_sync(&root, &cfg, move |message, done, total| {
            let _ = app2.emit(
                "mcp-sync-progress",
                SyncProgress { message, done, total },
            );
        })
    });
    match handle.await {
        Ok(Ok(report)) => {
            let _ = app.emit("mcp-sync-done", &report);
            Ok(report)
        }
        Ok(Err(e)) => {
            let _ = app.emit("mcp-sync-error", e.to_string());
            Err(e.to_string())
        }
        Err(e) => Err(format!("同步线程异常：{}", e)),
    }
}
