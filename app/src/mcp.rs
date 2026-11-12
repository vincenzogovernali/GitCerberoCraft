//! Model Context Protocol server over stdio (newline-delimited JSON-RPC 2.0).
//! Exposes the Git actions (fetch, pull, push, commit, ...) as MCP tools.

use crate::config::{PullMode, Settings};
use crate::git::{self, Engine};
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::path::PathBuf;

const SUPPORTED: [&str; 4] = ["2025-06-18", "2025-03-26", "2024-11-05", "2024-10-07"];

fn repo_prop() -> Value {
    json!({"type": "string", "description": "Path of the repository (defaults to the server's working folder)"})
}

fn tool(name: &str, desc: &str, props: Value, required: &[&str]) -> Value {
    let mut p = props.as_object().cloned().unwrap_or_default();
    p.insert("repo".into(), repo_prop());
    json!({
        "name": name,
        "description": desc,
        "inputSchema": {"type": "object", "properties": p, "required": required}
    })
}

pub fn tools() -> Vec<Value> {
    let s = |d: &str| json!({"type": "string", "description": d});
    let b = |d: &str| json!({"type": "boolean", "description": d});
    let n = |d: &str| json!({"type": "integer", "description": d});
    vec![
        tool("git_status", "Show the current branch, upstream and changed files.", json!({}), &[]),
        tool("git_log", "List recent commits.", json!({"limit": n("How many commits (default 20)")}), &[]),
        tool("git_show", "Show a commit: message, author and diff.", json!({"rev": s("Commit id, branch or tag")}), &["rev"]),
        tool("git_diff", "Show uncommitted changes.", json!({"staged": b("Only staged changes"), "file": s("Only this file")}), &[]),
        tool("git_fetch", "Fetch from a remote (all remotes by default).", json!({"remote": s("Remote name")}), &[]),
        tool("git_pull", "Pull the current branch.", json!({"mode": s("ff (default), ff-only, rebase or fetch")}), &[]),
        tool("git_push", "Push the current branch (sets the upstream if missing).", json!({"force": b("Force with lease")}), &[]),
        tool("git_stage", "Stage files (all files when 'files' is empty).", json!({"files": {"type": "array", "items": {"type": "string"}}}), &[]),
        tool("git_unstage", "Unstage files (all when 'files' is empty).", json!({"files": {"type": "array", "items": {"type": "string"}}}), &[]),
        tool("git_commit", "Commit staged changes.", json!({"message": s("Commit message"), "all": b("Stage every change first"), "amend": b("Amend the last commit"), "push": b("Push the branch to its remote after committing")}), &["message"]),
        tool("git_branch_list", "List local and remote branches.", json!({}), &[]),
        tool("git_branch_create", "Create a branch.", json!({"name": s("Branch name"), "from": s("Start point (default HEAD)"), "checkout": b("Switch to it")}), &["name"]),
        tool("git_branch_delete", "Delete a local branch.", json!({"name": s("Branch name")}), &["name"]),
        tool("git_checkout", "Switch to a branch (remote branches get a tracking branch).", json!({"branch": s("Branch name")}), &["branch"]),
        tool("git_merge", "Merge a branch into the current branch.", json!({"branch": s("Branch to merge"), "ff_only": b("Only fast-forward")}), &["branch"]),
        tool("git_stash", "Stash uncommitted changes.", json!({"message": s("Stash message")}), &[]),
        tool("git_stash_pop", "Apply and remove a stash.", json!({"index": n("Stash index (default 0)")}), &[]),
        tool("git_tag", "Create a tag.", json!({"name": s("Tag name"), "rev": s("Commit (default HEAD)"), "message": s("Annotation (optional)")}), &["name"]),
        tool("git_reset", "Reset the current branch to a commit.", json!({"rev": s("Commit"), "mode": s("soft, mixed (default) or hard")}), &["rev"]),
        tool("git_cherry_pick", "Cherry-pick a commit onto the current branch.", json!({"rev": s("Commit")}), &["rev"]),
        tool("git_revert", "Revert a commit.", json!({"rev": s("Commit")}), &["rev"]),
        tool("git_clone", "Clone a repository.", json!({"url": s("Remote URL"), "dest": s("Destination folder"), "shallow": b("Only the latest commit")}), &["url", "dest"]),
        tool("git_init", "Create a new repository.", json!({"path": s("Folder"), "branch": s("Default branch (main)")}), &["path"]),
        tool("git_remote_list", "List remotes and their URLs.", json!({}), &[]),
    ]
}

pub struct Server {
    pub settings: Settings,
    pub default_repo: PathBuf,
}

impl Server {
    pub fn new(default_repo: PathBuf) -> Self {
        Self { settings: Settings::load(), default_repo }
    }

    fn engine(&self) -> Engine {
        let mut e = Engine::detect(&self.settings.git_path_override, self.settings.prefer_git_cli);
        let p = self.settings.profile();
        e.user_name = p.user_name.clone();
        e.user_email = p.user_email.clone();
        e.token_github = self.settings.github_token.clone();
        e.token_gitlab = self.settings.gitlab_token.clone();
        e
    }

    fn repo(&self, args: &Value) -> PathBuf {
        args["repo"].as_str().map(PathBuf::from).unwrap_or_else(|| self.default_repo.clone())
    }

    pub fn call(&self, name: &str, args: &Value) -> Result<String, String> {
        let e = self.engine();
        let repo = self.repo(args);
        let str_arg = |k: &str| args[k].as_str().unwrap_or("").to_string();
        let files: Vec<String> = args["files"]
            .as_array()
            .map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();
        match name {
            "git_status" => git::status_text(&repo),
            "git_log" => git::log_text(&repo, args["limit"].as_u64().unwrap_or(20) as usize),
            "git_show" => {
                let oid = git::parse_oid(&repo, &str_arg("rev"))?;
                let d = git::commit_detail(&repo, oid)?;
                let diff = git::diff_text(&git::commit_diff(&repo, oid, None)?);
                Ok(format!(
                    "commit {}\nAuthor: {} <{}>\nDate: {}\n\n{}\n\n{}",
                    d.id,
                    d.author,
                    d.email,
                    crate::util::format_date(d.time),
                    d.message.trim(),
                    diff
                ))
            }
            "git_diff" => {
                let f = args["file"].as_str();
                let staged = args["staged"].as_bool().unwrap_or(false);
                let mut t = git::diff_text(&git::wip_diff(&repo, f, true)?);
                if !staged {
                    let u = git::diff_text(&git::wip_diff(&repo, f, false)?);
                    t = format!("# staged\n{t}\n# unstaged\n{u}");
                }
                Ok(t)
            }
            "git_fetch" => git::fetch(&e, &repo, args["remote"].as_str(), None),
            "git_pull" => {
                let mode = match args["mode"].as_str().unwrap_or("ff") {
                    "ff-only" => PullMode::FastForwardOnly,
                    "rebase" => PullMode::Rebase,
                    "fetch" => PullMode::FetchAll,
                    _ => PullMode::FastForwardIfPossible,
                };
                git::pull(&e, &repo, mode, None)
            }
            "git_push" => git::push(&e, &repo, args["force"].as_bool().unwrap_or(false), None),
            "git_stage" => {
                if files.is_empty() {
                    git::stage_all(&repo)?
                } else {
                    git::stage(&repo, &files)?
                }
                Ok("Staged".into())
            }
            "git_unstage" => {
                if files.is_empty() {
                    git::unstage_all(&repo)?
                } else {
                    git::unstage(&repo, &files)?
                }
                Ok("Unstaged".into())
            }
            "git_commit" => {
                if args["all"].as_bool().unwrap_or(false) {
                    git::stage_all(&repo)?;
                }
                let oid = git::commit(&e, &repo, &str_arg("message"), args["amend"].as_bool().unwrap_or(false))?;
                let done = format!("Committed {}", &oid.to_string()[..7]);
                if args["push"].as_bool().unwrap_or(false) {
                    return match git::push(&e, &repo, false, None) {
                        Ok(m) => Ok(format!("{done}\n{m}")),
                        Err(err) => Err(format!("{done}, but the push failed:\n{err}")),
                    };
                }
                Ok(done)
            }
            "git_branch_list" => {
                let s = git::load(&repo, 1)?;
                let mut out = String::new();
                for b in &s.local {
                    out.push_str(&format!(
                        "{} {}{}\n",
                        if b.is_head { "*" } else { " " },
                        b.name,
                        b.upstream.as_ref().map(|u| format!(" -> {u} (+{} -{})", b.ahead, b.behind)).unwrap_or_default()
                    ));
                }
                for r in &s.remotes {
                    for b in &r.branches {
                        out.push_str(&format!("  remotes/{}/{}\n", r.name, b.name));
                    }
                }
                Ok(out)
            }
            "git_branch_create" => {
                let from = args["from"].as_str().map(|r| git::parse_oid(&repo, r)).transpose()?;
                git::create_branch(&repo, &str_arg("name"), from, args["checkout"].as_bool().unwrap_or(false))?;
                Ok(format!("Created branch {}", str_arg("name")))
            }
            "git_branch_delete" => git::delete_branch(&repo, &str_arg("name")).map(|_| "Deleted".into()),
            "git_checkout" => git::checkout(&repo, &str_arg("branch")).map(|_| format!("Switched to {}", str_arg("branch"))),
            "git_merge" => git::merge(&e, &repo, &str_arg("branch"), args["ff_only"].as_bool().unwrap_or(false)),
            "git_stash" => git::stash_save(&e, &repo, &str_arg("message")).map(|_| "Stashed".into()),
            "git_stash_pop" => git::stash_pop(&repo, args["index"].as_u64().unwrap_or(0) as usize).map(|_| "Popped".into()),
            "git_tag" => {
                let rev = args["rev"].as_str().unwrap_or("HEAD");
                let oid = git::parse_oid(&repo, rev)?;
                git::create_tag(&repo, &str_arg("name"), oid, &str_arg("message"), &e).map(|_| "Tagged".into())
            }
            "git_reset" => {
                let oid = git::parse_oid(&repo, &str_arg("rev"))?;
                git::reset(&repo, oid, args["mode"].as_str().unwrap_or("mixed")).map(|_| "Reset done".into())
            }
            "git_cherry_pick" => git::cherry_pick(&e, &repo, git::parse_oid(&repo, &str_arg("rev"))?),
            "git_revert" => git::revert(&e, &repo, git::parse_oid(&repo, &str_arg("rev"))?),
            "git_clone" => git::clone(&e, &str_arg("url"), &PathBuf::from(str_arg("dest")), args["shallow"].as_bool().unwrap_or(false), None)
                .map(|p| format!("Cloned into {}", p.display())),
            "git_init" => {
                let o = git::InitOptions {
                    default_branch: args["branch"].as_str().unwrap_or("main").to_string(),
                    gitignore: String::new(),
                    license: String::new(),
                    lfs: false,
                    remote_url: String::new(),
                    initial_commit: false,
                };
                git::init(&e, &PathBuf::from(str_arg("path")), &o).map(|_| "Initialized".into())
            }
            "git_remote_list" => {
                let s = git::load(&repo, 1)?;
                Ok(s.remotes.iter().map(|r| format!("{}\t{}", r.name, r.url)).collect::<Vec<_>>().join("\n"))
            }
            other => Err(format!("Unknown tool: {other}")),
        }
    }

    /// Handle one JSON-RPC message; returns the reply (None for notifications).
    pub fn handle(&self, msg: &Value) -> Option<Value> {
        let id = msg.get("id").cloned();
        let method = msg["method"].as_str().unwrap_or("");
        let result: Result<Value, (i64, String)> = match method {
            "initialize" => {
                let asked = msg["params"]["protocolVersion"].as_str().unwrap_or(SUPPORTED[0]);
                let v = if SUPPORTED.contains(&asked) { asked } else { SUPPORTED[0] };
                Ok(json!({
                    "protocolVersion": v,
                    "capabilities": {"tools": {"listChanged": false}},
                    "serverInfo": {"name": "gitcerberocraft", "title": "GitCerberoCraft", "version": crate::config::APP_VERSION},
                    "instructions": "Git tools from GitCerberoCraft. Pass 'repo' with the repository path, or start the server inside the repository."
                }))
            }
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({"tools": tools()})),
            "tools/call" => {
                let name = msg["params"]["name"].as_str().unwrap_or("");
                let args = msg["params"]["arguments"].clone();
                let args = if args.is_null() { json!({}) } else { args };
                match self.call(name, &args) {
                    Ok(t) => Ok(json!({"content": [{"type": "text", "text": if t.is_empty() { "OK".to_string() } else { t }}], "isError": false})),
                    Err(e) => Ok(json!({"content": [{"type": "text", "text": e}], "isError": true})),
                }
            }
            "resources/list" => Ok(json!({"resources": []})),
            "prompts/list" => Ok(json!({"prompts": []})),
            m if m.starts_with("notifications/") => return None,
            _ => Err((-32601, format!("Method not found: {method}"))),
        };
        let id = id?;
        Some(match result {
            Ok(r) => json!({"jsonrpc": "2.0", "id": id, "result": r}),
            Err((code, m)) => json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": m}}),
        })
    }
}

/// Serve on stdin/stdout until stdin closes.
pub fn serve_stdio(default_repo: PathBuf) {
    let server = Server::new(default_repo);
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines().map_while(Result::ok) {
        if line.trim().is_empty() {
            continue;
        }
        let msg: Value = match serde_json::from_str(line.trim_start_matches('\u{feff}')) {
            Ok(v) => v,
            Err(e) => {
                let r = json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": e.to_string()}});
                let _ = writeln!(stdout, "{r}");
                let _ = stdout.flush();
                continue;
            }
        };
        let replies: Vec<Value> = if let Some(batch) = msg.as_array() {
            batch.iter().filter_map(|m| server.handle(m)).collect()
        } else {
            server.handle(&msg).into_iter().collect()
        };
        for r in replies {
            let _ = writeln!(stdout, "{r}");
        }
        let _ = stdout.flush();
    }
}
