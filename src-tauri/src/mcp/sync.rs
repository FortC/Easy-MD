//! MCP 双向同步引擎：本地快照 vs 云端快照 → 差异同步。
//! 软件只下发文件读写指令，具体云存储业务由 MCP 服务完成。

use super::client::McpClient;
use crate::config::{McpServerConfig, McpToolMap};
use anyhow::{anyhow, Result};
use serde::Serialize;
use serde_json::{json, Value};
use std::path::Path;
use std::time::Duration;

#[derive(Serialize, Clone, Debug)]
pub struct RemoteFile {
    pub path: String,
    pub size: Option<u64>,
    pub mtime: Option<u64>,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct SyncReport {
    pub uploaded: Vec<String>,
    pub downloaded: Vec<String>,
    pub skipped: Vec<String>,
    pub errors: Vec<String>,
    pub remote_files: usize,
    pub local_files: usize,
}

#[derive(Clone)]
struct BoundTools {
    list: String,
    download: String,
    upload: String,
    delete: String,
    mkdir: Option<String>,
}

/// 按约定名匹配工具；用户显式绑定优先
fn bind_tools(client: &mut McpClient, map: &McpToolMap) -> Result<BoundTools> {
    let tools = client.list_tools()?;
    let names: Vec<String> = tools.iter().map(|t| t.name.clone()).collect();
    let find = |needles: &[&str], user: &Option<String>| -> Result<String> {
        if let Some(u) = user {
            if names.iter().any(|n| n == u) {
                return Ok(u.clone());
            }
            return Err(anyhow!("配置的工具「{}」不存在", u));
        }
        for n in &names {
            let lower = n.to_lowercase();
            if needles.iter().any(|needle| lower.contains(needle)) {
                return Ok(n.clone());
            }
        }
        Err(anyhow!(
            "在 MCP 工具中找不到匹配「{}」的操作，可用工具：{}",
            needles.join("/"),
            names.join(", ")
        ))
    };
    Ok(BoundTools {
        list: find(&["list", "search", "readdir"], &map.list)?,
        download: find(&["download", "read_file", "get_file", "read"], &map.download)?,
        upload: find(&["upload", "write_file", "put", "write"], &map.upload)?,
        delete: find(&["delete", "remove", "rm"], &map.delete)?,
        mkdir: find(&["mkdir", "create_dir", "makedir"], &map.mkdir).ok(),
    })
}

/// 解析 list 工具的返回（宽松：JSON 数组 / 每行一个路径）
fn parse_remote_list(text: &str) -> Vec<RemoteFile> {
    let mut out = Vec::new();
    if let Ok(v) = serde_json::from_str::<Value>(text) {
        if let Some(arr) = v.as_array() {
            for item in arr {
                match item {
                    Value::String(s) => out.push(RemoteFile {
                        path: s.clone(),
                        size: None,
                        mtime: None,
                    }),
                    Value::Object(o) => {
                        let path = o
                            .get("path")
                            .or_else(|| o.get("name"))
                            .or_else(|| o.get("file"))
                            .and_then(|p| p.as_str())
                            .map(|s| s.to_string());
                        if let Some(path) = path {
                            out.push(RemoteFile {
                                path,
                                size: o.get("size").and_then(|s| s.as_u64()),
                                mtime: o
                                    .get("mtime")
                                    .or_else(|| o.get("modified"))
                                    .and_then(|s| s.as_u64()),
                            });
                        }
                    }
                    _ => {}
                }
            }
            return out;
        }
    }
    // 退化：按行当路径
    for line in text.lines() {
        let l = line.trim();
        if !l.is_empty() && !l.starts_with('{') {
            out.push(RemoteFile {
                path: l.to_string(),
                size: None,
                mtime: None,
            });
        }
    }
    out
}

/// 本地快照：vault 内全部非隐藏文件
fn local_snapshot(root: &Path) -> Vec<(String, u64, u64)> {
    let mut out = Vec::new();
    for entry in walkdir::WalkDir::new(root)
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
        let Ok(meta) = e.metadata() else { continue };
        let rel = e
            .path()
            .strip_prefix(root)
            .unwrap_or(e.path())
            .to_string_lossy()
            .replace('\\', "/");
        out.push((
            rel,
            meta.len(),
            meta.modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0),
        ));
    }
    out
}

/// 执行一次双向同步。on_progress: (阶段描述, 已完成, 总数)
pub fn run_sync<F>(
    root: &Path,
    cfg: &McpServerConfig,
    mut on_progress: F,
) -> Result<SyncReport>
where
    F: FnMut(String, usize, usize),
{
    let mut client = McpClient::connect(&cfg.command, &cfg.args, &cfg.env, &cfg.name)?;
    let tools = bind_tools(&mut client, &cfg.tool_map)?;

    // 1) 双侧快照
    on_progress("获取云端文件列表…".into(), 0, 0);
    let remote_raw = client.call_tool_text(
        &tools.list,
        json!({ "path": "", "prefix": "" }),
        Duration::from_secs(120),
    )?;
    let remote = parse_remote_list(&remote_raw);
    let remote_map: std::collections::HashMap<String, RemoteFile> = remote
        .into_iter()
        .map(|f| (f.path.trim_start_matches('/').to_string(), f))
        .collect();
    let local = local_snapshot(root);

    let mut report = SyncReport {
        remote_files: remote_map.len(),
        local_files: local.len(),
        ..Default::default()
    };

    // 2) 计划
    struct Action {
        path: String,
        kind: &'static str, // upload | download | skip
        reason: String,
    }
    let mut actions: Vec<Action> = Vec::new();
    for (rel, size, _mtime) in &local {
        match remote_map.get(rel.as_str()) {
            None => actions.push(Action {
                path: rel.clone(),
                kind: "upload",
                reason: "云端不存在".into(),
            }),
            Some(r) => {
                let differs = r.size.map(|s| s != *size).unwrap_or(true);
                if differs {
                    // 远端无 mtime 时采用"本地优先"（本地为真相源），有 mtime 则新者胜
                    let local_newer = r
                        .mtime
                        .map(|rm| *_mtime >= rm)
                        .unwrap_or(true);
                    actions.push(Action {
                        path: rel.clone(),
                        kind: if local_newer { "upload" } else { "download" },
                        reason: if local_newer {
                            "本地较新".into()
                        } else {
                            "云端较新".into()
                        },
                    });
                }
            }
        }
    }
    for r in remote_map.values() {
        if !local.iter().any(|(l, _, _)| l == &r.path) {
            actions.push(Action {
                path: r.path.clone(),
                kind: "download",
                reason: "本地不存在".into(),
            });
        }
    }

    let total = actions.len();
    on_progress(format!("共 {} 项待同步", total), 0, total);

    // 3) 执行（删除不自动传播：误删风险，双向同步只做新增/更新）
    for (i, act) in actions.iter().enumerate() {
        on_progress(
            format!(
                "{} {}（{}）",
                if act.kind == "upload" { "上传" } else { "下载" },
                act.path,
                act.reason
            ),
            i + 1,
            total,
        );
        let full = root.join(&act.path);
        let res: Result<()> = (|| {
            match act.kind {
                "upload" => {
                    let content = std::fs::read_to_string(&full)
                        .map_err(|e| anyhow!("读取本地文件失败：{}", e))?;
                    if let Some(parent) = full.parent() {
                        let dir = parent
                            .strip_prefix(root)
                            .unwrap_or(Path::new(""))
                            .to_string_lossy()
                            .replace('\\', "/");
                        if !dir.is_empty() {
                            if let Some(mkdir) = &tools.mkdir {
                                let _ = client.call_tool_text(
                                    mkdir,
                                    json!({ "path": dir }),
                                    Duration::from_secs(60),
                                );
                            }
                        }
                    }
                    client.call_tool_text(
                        &tools.upload,
                        json!({ "path": act.path, "content": content }),
                        Duration::from_secs(180),
                    )?;
                    Ok(())
                }
                _ => {
                    let text = client.call_tool_text(
                        &tools.download,
                        json!({ "path": act.path }),
                        Duration::from_secs(180),
                    )?;
                    // 云端返回可能是纯文本，也可能是 JSON 包装
                    let content = serde_json::from_str::<Value>(&text)
                        .ok()
                        .and_then(|v| {
                            v.get("content")
                                .or_else(|| v.get("text"))
                                .and_then(|c| c.as_str())
                                .map(|s| s.to_string())
                        })
                        .unwrap_or(text);
                    if let Some(parent) = full.parent() {
                        std::fs::create_dir_all(parent)
                            .map_err(|e| anyhow!("创建目录失败：{}", e))?;
                    }
                    std::fs::write(&full, content)
                        .map_err(|e| anyhow!("写入本地失败：{}", e))?;
                    Ok(())
                }
            }
        })();
        match res {
            Ok(()) => {
                if act.kind == "upload" {
                    report.uploaded.push(act.path.clone());
                } else {
                    report.downloaded.push(act.path.clone());
                }
            }
            Err(e) => report.errors.push(format!("{}：{}", act.path, e)),
        }
    }

    let _ = tools.delete; // 保留绑定校验，v1 不自动删除远端文件
    Ok(report)
}
