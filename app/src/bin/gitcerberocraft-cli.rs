//! GitCerberoCraft command line: everyday Git actions and an MCP server.
//!
//!   gitcerberocraft-cli mcp [--repo PATH]       start the MCP server on stdio
//!   gitcerberocraft-cli [-C PATH] status|log|fetch|pull|push|commit|...

use gcc_core::config::{PullMode, Settings};
use gcc_core::git::{self, Engine};
use gcc_core::{ai, gitdl, i18n, mcp};
use std::path::PathBuf;

fn usage() -> &'static str {
    "GitCerberoCraft CLI

USAGE:
  gitcerberocraft-cli [-C <repo>] <command> [options]

COMMANDS:
  mcp [--repo <path>]            Start the MCP server on stdin/stdout
  mcp-config                     Print a ready-to-paste MCP client configuration
  tools                          List the MCP tools
  status                         Branch and changed files
  log [-n <count>]               Recent commits
  show <rev>                     A commit and its diff
  diff [--staged]                Uncommitted changes
  fetch [<remote>]               Fetch (all remotes by default)
  pull [--ff-only|--rebase]      Pull the current branch
  push [--force]                 Push the current branch
  add [<file>...]                Stage files (all when none given)
  reset-files [<file>...]        Unstage files
  commit -m <msg> [-a] [--amend] [--push]
                                 Commit (and push with --push)
  check-token                    Test the saved GitHub/GitLab tokens
  branch [<name>] [--from <rev>] List or create branches
  checkout <branch>              Switch branch
  merge <branch>                 Merge into the current branch
  stash [-m <msg>] | pop         Stash changes / pop the latest stash
  tag <name> [<rev>]             Create a tag
  clone <url> <dest> [--shallow] Clone a repository
  init <path> [--branch <name>]  Create a repository
  git-versions                   Git versions available for this system
  git-download <version> [installer|portable|mingit|source]
  languages                      Languages found in the lang folders
  ask <question>                 Ask the configured AI about this repository
  version                        Show versions
"
}

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut repo = std::env::current_dir().unwrap_or_default();
    if let Some(i) = args.iter().position(|a| a == "-C") {
        if i + 1 < args.len() {
            repo = PathBuf::from(&args[i + 1]);
            args.drain(i..=i + 1);
        }
    }
    if args.is_empty() || args[0] == "-h" || args[0] == "--help" || args[0] == "help" {
        print!("{}", usage());
        return;
    }
    let settings = Settings::load();
    i18n::init(&settings.language);
    let cmd = args.remove(0);
    let code = match run(&cmd, &args, repo, &settings) {
        Ok(out) => {
            if !out.is_empty() {
                println!("{out}");
            }
            0
        }
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    };
    std::process::exit(code);
}

fn flag(args: &[String], f: &str) -> bool {
    args.iter().any(|a| a == f)
}

fn opt(args: &[String], f: &str) -> Option<String> {
    args.iter().position(|a| a == f).and_then(|i| args.get(i + 1).cloned())
}

fn positional(args: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut skip = false;
    for a in args {
        if skip {
            skip = false;
            continue;
        }
        if matches!(a.as_str(), "-m" | "-n" | "--from" | "--repo" | "--branch") {
            skip = true;
            continue;
        }
        if a.starts_with('-') {
            continue;
        }
        out.push(a.clone());
    }
    out
}

fn engine(s: &Settings) -> Engine {
    let mut e = Engine::detect(&s.git_path_override, s.prefer_git_cli);
    e.user_name = s.profile().user_name.clone();
    e.user_email = s.profile().user_email.clone();
    e.token_github = s.github_token.clone();
    e.token_gitlab = s.gitlab_token.clone();
    e
}

fn run(cmd: &str, args: &[String], repo: PathBuf, s: &Settings) -> Result<String, String> {
    let e = engine(s);
    let pos = positional(args);
    let mut progress = |f: f32, _l: &str| {
        eprint!("\r{:>3}%", (f * 100.0) as u32);
    };
    match cmd {
        "mcp" | "serve" => {
            let r = opt(args, "--repo").map(PathBuf::from).unwrap_or(repo);
            mcp::serve_stdio(r);
            Ok(String::new())
        }
        "mcp-config" => {
            let exe = std::env::current_exe().map(|p| p.display().to_string()).unwrap_or_default();
            Ok(serde_json::to_string_pretty(&serde_json::json!({
                "mcpServers": {"gitcerberocraft": {"command": exe, "args": ["mcp", "--repo", repo.display().to_string()]}}
            }))
            .unwrap())
        }
        "tools" => Ok(mcp::tools()
            .iter()
            .map(|t| format!("{:<20} {}", t["name"].as_str().unwrap_or(""), t["description"].as_str().unwrap_or("")))
            .collect::<Vec<_>>()
            .join("\n")),
        "status" => git::status_text(&repo),
        "log" => git::log_text(&repo, opt(args, "-n").and_then(|n| n.parse().ok()).unwrap_or(20)),
        "show" => {
            let rev = pos.first().cloned().unwrap_or_else(|| "HEAD".into());
            mcp::Server::new(repo).call("git_show", &serde_json::json!({ "rev": rev }))
        }
        "diff" => mcp::Server::new(repo).call("git_diff", &serde_json::json!({ "staged": flag(args, "--staged") })),
        "fetch" => git::fetch(&e, &repo, pos.first().map(|s| s.as_str()), Some(&mut progress)),
        "pull" => {
            let mode = if flag(args, "--rebase") {
                PullMode::Rebase
            } else if flag(args, "--ff-only") {
                PullMode::FastForwardOnly
            } else {
                PullMode::FastForwardIfPossible
            };
            git::pull(&e, &repo, mode, Some(&mut progress))
        }
        "push" => git::push(&e, &repo, flag(args, "--force"), Some(&mut progress)),
        "check-token" => {
            let gh = match gcc_core::auth::github_whoami(&s.github_token) {
                Ok(i) => gcc_core::auth::describe(&i),
                Err(err) => err,
            };
            let gl = if s.gitlab_token.trim().is_empty() {
                "-".to_string()
            } else {
                match gcc_core::auth::gitlab_whoami(&s.gitlab_token, "https://gitlab.com/") {
                    Ok(i) => gcc_core::auth::describe(&i),
                    Err(err) => err,
                }
            };
            Ok(format!("GitHub: {gh}\nGitLab: {gl}"))
        }
        "add" | "stage" => {
            if pos.is_empty() {
                git::stage_all(&repo)?
            } else {
                git::stage(&repo, &pos)?
            }
            Ok("Staged".into())
        }
        "reset-files" | "unstage" => {
            if pos.is_empty() {
                git::unstage_all(&repo)?
            } else {
                git::unstage(&repo, &pos)?
            }
            Ok("Unstaged".into())
        }
        "commit" => {
            let msg = opt(args, "-m").ok_or("use -m <message>")?;
            if flag(args, "-a") || flag(args, "--all") {
                git::stage_all(&repo)?;
            }
            let oid = git::commit(&e, &repo, &msg, flag(args, "--amend"))?;
            let done = format!("Committed {}", &oid.to_string()[..7]);
            if flag(args, "--push") {
                return match git::push(&e, &repo, false, Some(&mut progress)) {
                    Ok(m) => Ok(format!("{done}\n{m}")),
                    Err(err) => Err(format!("{done}, but the push failed:\n{err}")),
                };
            }
            Ok(done)
        }
        "branch" => match pos.first() {
            None => mcp::Server::new(repo).call("git_branch_list", &serde_json::json!({})),
            Some(name) => {
                let from = opt(args, "--from").map(|r| git::parse_oid(&repo, &r)).transpose()?;
                git::create_branch(&repo, name, from, false)?;
                Ok(format!("Created {name}"))
            }
        },
        "checkout" | "switch" => {
            let b = pos.first().ok_or("which branch?")?;
            git::checkout(&repo, b)?;
            Ok(format!("Switched to {b}"))
        }
        "merge" => git::merge(&e, &repo, pos.first().ok_or("which branch?")?, flag(args, "--ff-only")),
        "stash" => {
            if pos.first().map(|s| s == "pop").unwrap_or(false) {
                git::stash_pop(&repo, 0)?;
                Ok("Popped".into())
            } else {
                git::stash_save(&e, &repo, &opt(args, "-m").unwrap_or_default())?;
                Ok("Stashed".into())
            }
        }
        "pop" => git::stash_pop(&repo, 0).map(|_| "Popped".into()),
        "tag" => {
            let name = pos.first().ok_or("tag name?")?;
            let oid = git::parse_oid(&repo, pos.get(1).map(|s| s.as_str()).unwrap_or("HEAD"))?;
            git::create_tag(&repo, name, oid, &opt(args, "-m").unwrap_or_default(), &e)?;
            Ok(format!("Tagged {name}"))
        }
        "clone" => {
            let url = pos.first().ok_or("url?")?;
            let dest = pos.get(1).map(PathBuf::from).unwrap_or_else(|| {
                repo.join(url.trim_end_matches(".git").rsplit(['/', ':']).next().unwrap_or("repo"))
            });
            let p = git::clone(&e, url, &dest, flag(args, "--shallow"), Some(&mut progress))?;
            Ok(format!("\nCloned into {}", p.display()))
        }
        "init" => {
            let path = pos.first().map(PathBuf::from).unwrap_or(repo);
            let o = git::InitOptions {
                default_branch: opt(args, "--branch").unwrap_or_else(|| s.default_branch.clone()),
                gitignore: String::new(),
                license: String::new(),
                lfs: false,
                remote_url: String::new(),
                initial_commit: false,
            };
            git::init(&e, &path, &o)?;
            Ok(format!("Initialized {}", path.display()))
        }
        "git-versions" => {
            let rels = gitdl::list_releases()?;
            Ok(rels
                .iter()
                .map(|r| {
                    format!(
                        "{:<12} {:<10} {}{}",
                        r.version,
                        r.date,
                        r.assets.iter().map(|a| a.name.clone()).collect::<Vec<_>>().join(", "),
                        if r.prerelease { " (pre-release)" } else { "" }
                    )
                })
                .collect::<Vec<_>>()
                .join("\n"))
        }
        "git-download" => {
            let v = pos.first().ok_or("which version? (see git-versions)")?;
            let kind = pos.get(1).map(|s| s.as_str()).unwrap_or(if cfg!(windows) { "mingit" } else { "source" });
            let rels = gitdl::list_releases()?;
            let r = rels.iter().find(|r| &r.version == v).ok_or("version not found")?;
            let a = r
                .assets
                .iter()
                .find(|a| match kind {
                    "installer" => a.kind == gitdl::AssetKind::Installer,
                    "portable" => a.kind == gitdl::AssetKind::Portable,
                    "source" => a.kind == gitdl::AssetKind::Source,
                    _ => a.kind == gitdl::AssetKind::MinGit,
                })
                .ok_or("that package is not available for this version")?;
            let file = gitdl::download(a, &mut |f| eprint!("\r{:>3}%", (f * 100.0) as u32))?;
            let m = gitdl::install(a, &file)?;
            Ok(format!("\n{} ({m})", file.display()))
        }
        "languages" => {
            let g = i18n::global().read().map_err(|e| e.to_string())?;
            let mut out = format!("Folders: {}\n", i18n::lang_dirs().iter().map(|d| d.display().to_string()).collect::<Vec<_>>().join("; "));
            for l in &g.languages {
                out.push_str(&format!("{:<6} {}{}\n", l.code, l.name, if l.code == g.current { "  (current)" } else { "" }));
            }
            Ok(out)
        }
        "ask" => {
            let q = pos.join(" ");
            let ctx = git::status_text(&repo).unwrap_or_default();
            ai::ask_once(&s.ai, &format!("{q}\n\nRepository context:\n{ctx}"), &repo)
        }
        "version" | "--version" => {
            let cli = e.cli.as_ref().and_then(|p| git::git_version(p)).unwrap_or_else(|| "not found".into());
            Ok(format!(
                "GitCerberoCraft CLI {}\nbuilt-in engine libgit2 {}\nGit program: {}",
                gcc_core::config::APP_VERSION,
                git::engine_version(),
                cli
            ))
        }
        other => Err(format!("unknown command '{other}'\n\n{}", usage())),
    }
}
