//! MCP 客户端：通过 stdio 与外部 MCP 服务（腾讯云/百度网盘等）通信。
//! 标准协议流程：initialize → notifications/initialized → tools/list → tools/call
//! 本软件只做 MCP 客户端，不含任何云厂商 SDK。

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::Receiver;
use std::time::Duration;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
}

/// 已连接的 MCP 服务
pub struct McpClient {
    child: Child,
    stdin: ChildStdin,
    rx: Receiver<Value>,
    next_id: u64,
    server_name: String,
}

impl McpClient {
    /// 启动 MCP 服务进程并完成 initialize 握手
    pub fn connect(
        command: &str,
        args: &[String],
        env: &std::collections::HashMap<String, String>,
        server_name: &str,
    ) -> Result<Self> {
        let mut cmd = Command::new(command);
        cmd.args(args)
            .envs(env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = cmd
            .spawn()
            .with_context(|| format!("无法启动 MCP 服务：{}", command))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("MCP 服务无标准输入"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("MCP 服务无标准输出"))?;

        let (tx, rx) = std::sync::mpsc::channel::<Value>();
        // 读线程：逐行读 JSON-RPC 响应（sender 克隆进线程，原句柄留在结构体维持通道）
        let tx_reader = tx.clone();
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                let Ok(line) = line else { break };
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
                    // 只转发带 id 的响应；通知与请求忽略（云盘 MCP 常无）
                    if v.get("id").is_some() && (v.get("result").is_some() || v.get("error").is_some()) {
                        if tx_reader.send(v).is_err() {
                            break;
                        }
                    }
                }
            }
        });

        let mut client = McpClient {
            child,
            stdin,
            rx,
            next_id: 1,
            server_name: server_name.to_string(),
        };

        // initialize 握手
        let result = client.request(
            "initialize",
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "easymd", "version": "0.1.0" }
            }),
            Duration::from_secs(30),
        )?;
        let _ = result
            .get("serverInfo")
            .and_then(|s| s.get("name"))
            .and_then(|n| n.as_str())
            .map(|s| client.server_name = s.to_string());
        client.notify("notifications/initialized", json!({}))?;
        Ok(client)
    }

    pub fn server_name(&self) -> &str {
        &self.server_name
    }

    fn request(&mut self, method: &str, params: Value, timeout: Duration) -> Result<Value> {
        let id = self.next_id;
        self.next_id += 1;
        let req = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        // 清掉旧的超时响应，避免串号
        while self.rx.try_recv().is_ok() {}
        self.stdin
            .write_all((serde_json::to_string(&req)? + "\n").as_bytes())
            .context("写入 MCP 服务失败（进程可能已退出）")?;
        self.stdin.flush()?;
        let resp = self
            .rx
            .recv_timeout(timeout)
            .map_err(|_| anyhow!("MCP 服务响应超时（{}）", method))?;
        // 校验 id 匹配（读线程只转发响应，一般必匹配）
        if resp.get("id").and_then(|v| v.as_u64()) != Some(id) {
            return Err(anyhow!("MCP 响应 id 不匹配"));
        }
        if let Some(err) = resp.get("error") {
            return Err(anyhow!("MCP 错误：{}", err));
        }
        Ok(resp["result"].clone())
    }

    fn notify(&mut self, method: &str, params: Value) -> Result<()> {
        let req = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        self.stdin
            .write_all((serde_json::to_string(&req)? + "\n").as_bytes())
            .context("写入 MCP 服务失败")?;
        self.stdin.flush()?;
        Ok(())
    }

    /// 工具列表
    pub fn list_tools(&mut self) -> Result<Vec<ToolInfo>> {
        let result = self.request("tools/list", json!({}), Duration::from_secs(30))?;
        let tools = result
            .get("tools")
            .and_then(|t| t.as_array())
            .ok_or_else(|| anyhow!("tools/list 返回格式异常"))?;
        Ok(tools
            .iter()
            .filter_map(|t| {
                Some(ToolInfo {
                    name: t.get("name")?.as_str()?.to_string(),
                    description: t
                        .get("description")
                        .and_then(|d| d.as_str())
                        .unwrap_or("")
                        .to_string(),
                })
            })
            .collect())
    }

    /// 调用工具，返回文本内容合并结果
    pub fn call_tool_text(&mut self, name: &str, args: Value, timeout: Duration) -> Result<String> {
        let result = self.request(
            "tools/call",
            json!({ "name": name, "arguments": args }),
            timeout,
        )?;
        if result.get("isError").and_then(|e| e.as_bool()).unwrap_or(false) {
            return Err(anyhow!("工具 {} 返回错误：{}", name, result));
        }
        let mut out = String::new();
        if let Some(content) = result.get("content").and_then(|c| c.as_array()) {
            for part in content {
                if part.get("type").and_then(|t| t.as_str()) == Some("text") {
                    if let Some(t) = part.get("text").and_then(|t| t.as_str()) {
                        out.push_str(t);
                    }
                }
            }
        }
        Ok(out)
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        // 结束子进程
        let _ = self.stdin.write_all(b"\n");
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
