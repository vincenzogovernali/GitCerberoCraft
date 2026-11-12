//! AI assistant: either a web API (OpenAI-compatible, Anthropic, Ollama) or an
//! agent spoken to through the Agent Client Protocol (ACP: JSON-RPC over the
//! agent's stdin/stdout).

use crate::config::{AiMode, AiSettings, ApiKind};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Stdio};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub role: String,
    pub text: String,
}

#[derive(Debug)]
pub enum AiEvent {
    Chunk(String),
    Done,
    Error(String),
    Status(String),
}

/// Ask once and wait for the whole answer (used by the CLI and the API mode).
pub fn ask_api(s: &AiSettings, history: &[ChatMessage]) -> Result<String, String> {
    let base = s.base_url.trim_end_matches('/');
    let agent = ureq::AgentBuilder::new().timeout(Duration::from_secs(180)).build();
    let msgs: Vec<Value> = history.iter().map(|m| json!({"role": m.role, "content": m.text})).collect();
    match s.api_kind {
        ApiKind::OpenAiCompatible => {
            let mut all = vec![json!({"role": "system", "content": s.system_prompt})];
            all.extend(msgs);
            let mut req = agent.post(&format!("{base}/chat/completions")).set("Content-Type", "application/json");
            if !s.api_key.is_empty() {
                req = req.set("Authorization", &format!("Bearer {}", s.api_key));
            }
            let v: Value = req
                .send_json(json!({"model": s.model, "messages": all}))
                .map_err(http_err)?
                .into_json()
                .map_err(|e| e.to_string())?;
            v["choices"][0]["message"]["content"]
                .as_str()
                .map(|x| x.to_string())
                .ok_or_else(|| format!("Unexpected answer: {v}"))
        }
        ApiKind::Anthropic => {
            let v: Value = agent
                .post(&format!("{base}/v1/messages"))
                .set("x-api-key", &s.api_key)
                .set("anthropic-version", "2023-06-01")
                .set("content-type", "application/json")
                .send_json(json!({"model": s.model, "max_tokens": 4096, "system": s.system_prompt, "messages": msgs}))
                .map_err(http_err)?
                .into_json()
                .map_err(|e| e.to_string())?;
            let text: String = v["content"]
                .as_array()
                .map(|a| a.iter().filter_map(|b| b["text"].as_str()).collect::<Vec<_>>().join(""))
                .unwrap_or_default();
            if text.is_empty() {
                Err(format!("Unexpected answer: {v}"))
            } else {
                Ok(text)
            }
        }
        ApiKind::Ollama => {
            let mut all = vec![json!({"role": "system", "content": s.system_prompt})];
            all.extend(msgs);
            let v: Value = agent
                .post(&format!("{base}/api/chat"))
                .send_json(json!({"model": s.model, "messages": all, "stream": false}))
                .map_err(http_err)?
                .into_json()
                .map_err(|e| e.to_string())?;
            v["message"]["content"]
                .as_str()
                .map(|x| x.to_string())
                .ok_or_else(|| format!("Unexpected answer: {v}"))
        }
    }
}

fn http_err(e: ureq::Error) -> String {
    match e {
        ureq::Error::Status(code, r) => {
            let body = r.into_string().unwrap_or_default();
            format!("HTTP {code}: {}", body.chars().take(400).collect::<String>())
        }
        other => other.to_string(),
    }
}

/// List the models offered by the configured API (best effort).
pub fn list_models(s: &AiSettings) -> Result<Vec<String>, String> {
    let base = s.base_url.trim_end_matches('/');
    let agent = ureq::AgentBuilder::new().timeout(Duration::from_secs(20)).build();
    let v: Value = match s.api_kind {
        ApiKind::OpenAiCompatible => {
            let mut r = agent.get(&format!("{base}/models"));
            if !s.api_key.is_empty() {
                r = r.set("Authorization", &format!("Bearer {}", s.api_key));
            }
            r.call().map_err(http_err)?.into_json().map_err(|e| e.to_string())?
        }
        ApiKind::Anthropic => agent
            .get(&format!("{base}/v1/models"))
            .set("x-api-key", &s.api_key)
            .set("anthropic-version", "2023-06-01")
            .call()
            .map_err(http_err)?
            .into_json()
            .map_err(|e| e.to_string())?,
        ApiKind::Ollama => agent.get(&format!("{base}/api/tags")).call().map_err(http_err)?.into_json().map_err(|e| e.to_string())?,
    };
    let mut out: Vec<String> = Vec::new();
    if let Some(a) = v["data"].as_array() {
        out.extend(a.iter().filter_map(|m| m["id"].as_str().map(|s| s.to_string())));
    }
    if let Some(a) = v["models"].as_array() {
        out.extend(a.iter().filter_map(|m| m["name"].as_str().map(|s| s.to_string())));
    }
    out.sort();
    Ok(out)
}

// ---------------------------------------------------------------------------
// ACP client
// ---------------------------------------------------------------------------

struct AcpConn {
    child: Child,
    stdin: ChildStdin,
    lines: Receiver<String>,
    next_id: u64,
    session: Option<String>,
    cwd: PathBuf,
}

impl AcpConn {
    fn start(cmd: &str, args: &str, cwd: &Path) -> Result<Self, String> {
        let argv: Vec<String> = split_args(args);
        let mut c = if cfg!(windows) {
            // npx, uvx and friends are .cmd scripts on Windows.
            let mut c = crate::util::quiet_command("cmd");
            c.arg("/C").arg(cmd);
            c
        } else {
            std::process::Command::new(cmd)
        };
        let mut child = c
            .args(&argv)
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Cannot start the agent '{cmd}': {e}"))?;
        let stdin = child.stdin.take().ok_or("no stdin")?;
        let stdout = child.stdout.take().ok_or("no stdout")?;
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        Ok(Self { child, stdin, lines: rx, next_id: 1, session: None, cwd: cwd.to_path_buf() })
    }

    fn send(&mut self, v: &Value) -> Result<(), String> {
        let s = serde_json::to_string(v).unwrap();
        self.stdin.write_all(s.as_bytes()).map_err(|e| e.to_string())?;
        self.stdin.write_all(b"\n").map_err(|e| e.to_string())?;
        self.stdin.flush().map_err(|e| e.to_string())
    }

    /// Send a request and wait for its result, handling what the agent sends meanwhile.
    fn request(&mut self, method: &str, params: Value, out: &Sender<AiEvent>) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id += 1;
        self.send(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}))?;
        loop {
            let line = self
                .lines
                .recv_timeout(Duration::from_secs(600))
                .map_err(|_| "The agent stopped answering".to_string())?;
            let v: Value = match serde_json::from_str(&line) {
                Ok(v) => v,
                Err(_) => continue,
            };
            if v.get("method").is_some() {
                self.handle_incoming(&v, out)?;
                continue;
            }
            if v["id"].as_u64() == Some(id) {
                if let Some(e) = v.get("error") {
                    return Err(e["message"].as_str().unwrap_or("agent error").to_string());
                }
                return Ok(v["result"].clone());
            }
        }
    }

    fn handle_incoming(&mut self, v: &Value, out: &Sender<AiEvent>) -> Result<(), String> {
        let method = v["method"].as_str().unwrap_or("");
        let id = v.get("id").cloned();
        match method {
            "session/update" => {
                let u = &v["params"]["update"];
                match u["sessionUpdate"].as_str().unwrap_or("") {
                    "agent_message_chunk" => {
                        if let Some(t) = u["content"]["text"].as_str() {
                            let _ = out.send(AiEvent::Chunk(t.to_string()));
                        }
                    }
                    "tool_call" => {
                        if let Some(t) = u["title"].as_str() {
                            let _ = out.send(AiEvent::Status(t.to_string()));
                        }
                    }
                    _ => {}
                }
            }
            "session/request_permission" => {
                // Advice only: refuse changes, allow nothing that writes.
                let opts = v["params"]["options"].as_array().cloned().unwrap_or_default();
                let reject = opts
                    .iter()
                    .find(|o| o["kind"].as_str().map(|k| k.starts_with("reject")).unwrap_or(false))
                    .and_then(|o| o["optionId"].as_str().map(|s| s.to_string()));
                let result = match reject {
                    Some(o) => json!({"outcome": {"outcome": "selected", "optionId": o}}),
                    None => json!({"outcome": {"outcome": "cancelled"}}),
                };
                if let Some(id) = id {
                    self.send(&json!({"jsonrpc": "2.0", "id": id, "result": result}))?;
                }
            }
            "fs/read_text_file" => {
                let p = v["params"]["path"].as_str().unwrap_or("");
                let path = PathBuf::from(p);
                let allowed = path.starts_with(&self.cwd);
                let reply = if allowed {
                    match std::fs::read_to_string(&path) {
                        Ok(t) => json!({"jsonrpc": "2.0", "id": id, "result": {"content": t}}),
                        Err(e) => json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32000, "message": e.to_string()}}),
                    }
                } else {
                    json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32000, "message": "outside the repository"}})
                };
                self.send(&reply)?;
            }
            _ => {
                if let Some(id) = id {
                    self.send(&json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32601, "message": "not supported by GitCerberoCraft"}}))?;
                }
            }
        }
        Ok(())
    }

    fn ensure_session(&mut self, out: &Sender<AiEvent>) -> Result<String, String> {
        if let Some(s) = &self.session {
            return Ok(s.clone());
        }
        let _ = out.send(AiEvent::Status("Connecting to the agent…".into()));
        self.request(
            "initialize",
            json!({
                "protocolVersion": 1,
                "clientCapabilities": {"fs": {"readTextFile": true, "writeTextFile": false}, "terminal": false},
                "clientInfo": {"name": "GitCerberoCraft", "version": crate::config::APP_VERSION}
            }),
            out,
        )?;
        let r = self.request("session/new", json!({"cwd": self.cwd.display().to_string(), "mcpServers": []}), out)?;
        let sid = r["sessionId"].as_str().ok_or("The agent did not open a session")?.to_string();
        self.session = Some(sid.clone());
        Ok(sid)
    }

    fn prompt(&mut self, text: &str, out: &Sender<AiEvent>) -> Result<(), String> {
        let sid = self.ensure_session(out)?;
        self.request("session/prompt", json!({"sessionId": sid, "prompt": [{"type": "text", "text": text}]}), out)?;
        Ok(())
    }
}

impl Drop for AcpConn {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

pub fn split_args(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quote = false;
    for ch in s.chars() {
        match ch {
            '"' => quote = !quote,
            ' ' if !quote => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

pub struct AiRequest {
    pub settings: AiSettings,
    pub history: Vec<ChatMessage>,
    pub cwd: PathBuf,
}

/// Background worker; keeps an ACP agent alive between questions.
pub struct AiWorker {
    tx: Sender<AiRequest>,
    pub rx: Receiver<AiEvent>,
}

impl AiWorker {
    pub fn start() -> Self {
        let (tx, rx_req) = channel::<AiRequest>();
        let (tx_ev, rx) = channel::<AiEvent>();
        std::thread::spawn(move || {
            let mut conn: Option<(AcpConn, String)> = None;
            while let Ok(req) = rx_req.recv() {
                let s = &req.settings;
                match s.mode {
                    AiMode::Off => {
                        let _ = tx_ev.send(AiEvent::Error("AI is off: choose API or ACP in Preferences > AI".into()));
                    }
                    AiMode::Api => match ask_api(s, &req.history) {
                        Ok(t) => {
                            let _ = tx_ev.send(AiEvent::Chunk(t));
                            let _ = tx_ev.send(AiEvent::Done);
                        }
                        Err(e) => {
                            let _ = tx_ev.send(AiEvent::Error(e));
                        }
                    },
                    AiMode::Acp => {
                        let key = format!("{} {} {}", s.acp_command, s.acp_args, req.cwd.display());
                        if conn.as_ref().map(|c| c.1 != key).unwrap_or(true) {
                            conn = None;
                            match AcpConn::start(&s.acp_command, &s.acp_args, &req.cwd) {
                                Ok(c) => conn = Some((c, key)),
                                Err(e) => {
                                    let _ = tx_ev.send(AiEvent::Error(e));
                                    continue;
                                }
                            }
                        }
                        let text = req.history.last().map(|m| m.text.clone()).unwrap_or_default();
                        let first = conn.as_ref().map(|c| c.0.session.is_none()).unwrap_or(true);
                        let text = if first { format!("{}\n\n{}", s.system_prompt, text) } else { text };
                        let res = conn.as_mut().unwrap().0.prompt(&text, &tx_ev);
                        match res {
                            Ok(()) => {
                                let _ = tx_ev.send(AiEvent::Done);
                            }
                            Err(e) => {
                                conn = None;
                                let _ = tx_ev.send(AiEvent::Error(e));
                            }
                        }
                    }
                }
            }
        });
        Self { tx, rx }
    }

    pub fn ask(&self, req: AiRequest) {
        let _ = self.tx.send(req);
    }
}

/// Ask through ACP once and collect the full answer (CLI use).
pub fn ask_once(settings: &AiSettings, question: &str, cwd: &Path) -> Result<String, String> {
    let history = vec![ChatMessage { role: "user".into(), text: question.to_string() }];
    match settings.mode {
        AiMode::Off => Err("AI is off: set it up in the app's Preferences > AI".into()),
        AiMode::Api => ask_api(settings, &history),
        AiMode::Acp => {
            let (tx, rx) = channel();
            let mut c = AcpConn::start(&settings.acp_command, &settings.acp_args, cwd)?;
            c.prompt(&format!("{}\n\n{}", settings.system_prompt, question), &tx)?;
            drop(tx);
            let mut s = String::new();
            for e in rx.try_iter() {
                if let AiEvent::Chunk(t) = e {
                    s.push_str(&t);
                }
            }
            Ok(s)
        }
    }
}

/// Prompts used by the buttons.
pub fn prompt_explain_commit(summary: &str, diff: &str) -> String {
    format!("Explain this commit: what it changes, why it might have been done, and any risk or bug you notice. Suggest improvements if useful.\n\nCommit message:\n{summary}\n\nDiff:\n```diff\n{diff}\n```")
}

pub fn prompt_review(diff: &str) -> String {
    format!("Review these uncommitted changes like a careful code reviewer: list bugs, risks, missing tests and style problems, with short concrete fixes.\n\n```diff\n{diff}\n```")
}

pub fn prompt_commit_message(diff: &str) -> String {
    format!("Write a git commit message for these staged changes. First line: imperative summary under 72 characters. Then a blank line and a short body if needed. Reply with the message only, no code fences.\n\n```diff\n{diff}\n```")
}
