//! Git engine: reading uses the built-in libgit2 engine; actions use the Git
//! program when it is installed (or downloaded by the app) and fall back to the
//! built-in engine otherwise, so the app works even without Git installed.

use crate::util::quiet_command;
use git2::{
    build::CheckoutBuilder, BranchType, Cred, CredentialType, DiffFormat, DiffOptions, FetchOptions, IndexAddOption,
    Oid, PushOptions, RemoteCallbacks, Repository, ResetType, Signature, Sort, StashFlags, StatusOptions,
};
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Stdio;

pub type GResult<T> = Result<T, String>;

trait Es<T> {
    fn es(self) -> GResult<T>;
}
impl<T> Es<T> for Result<T, git2::Error> {
    fn es(self) -> GResult<T> {
        self.map_err(|e| e.message().to_string())
    }
}

// ---------------------------------------------------------------------------
// Finding the Git program
// ---------------------------------------------------------------------------

/// Folder where the app keeps a Git it downloaded itself.
pub fn managed_git_dir() -> PathBuf {
    crate::config::data_dir().join("git")
}

fn managed_git_exe() -> Option<PathBuf> {
    let base = managed_git_dir();
    let candidates = if cfg!(windows) {
        vec![base.join("cmd").join("git.exe"), base.join("mingw64").join("bin").join("git.exe"), base.join("bin").join("git.exe")]
    } else {
        vec![base.join("bin").join("git")]
    };
    candidates.into_iter().find(|p| p.is_file())
}

/// Find a usable `git` program: the user's choice, the app's own copy, PATH,
/// then the usual install folders.
pub fn find_git(override_path: &str) -> Option<PathBuf> {
    if !override_path.trim().is_empty() {
        let p = PathBuf::from(override_path.trim());
        if p.is_file() {
            return Some(p);
        }
    }
    if let Some(p) = managed_git_exe() {
        return Some(p);
    }
    if let Some(p) = crate::util::which("git") {
        return Some(p);
    }
    let mut extra: Vec<PathBuf> = Vec::new();
    if cfg!(windows) {
        for var in ["ProgramFiles", "ProgramW6432", "ProgramFiles(x86)", "LOCALAPPDATA"] {
            if let Ok(v) = std::env::var(var) {
                extra.push(PathBuf::from(&v).join("Git").join("cmd").join("git.exe"));
                extra.push(PathBuf::from(&v).join("Programs").join("Git").join("cmd").join("git.exe"));
            }
        }
    } else {
        extra.extend(["/usr/bin/git", "/usr/local/bin/git", "/opt/homebrew/bin/git"].iter().map(PathBuf::from));
    }
    extra.into_iter().find(|p| p.is_file())
}

pub fn git_version(git: &Path) -> Option<String> {
    let out = quiet_command(git).arg("--version").output().ok()?;
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s.trim_start_matches("git version ").to_string())
    }
}

pub fn engine_version() -> String {
    let v = git2::Version::get();
    let (a, b, c) = v.libgit2_version();
    format!("{a}.{b}.{c}")
}

/// How actions are carried out.
#[derive(Clone, Debug)]
pub struct Engine {
    pub cli: Option<PathBuf>,
    pub token_github: String,
    pub token_gitlab: String,
    pub user_name: String,
    pub user_email: String,
}

impl Engine {
    pub fn detect(override_path: &str, prefer_cli: bool) -> Self {
        Self {
            cli: if prefer_cli { find_git(override_path) } else { None },
            token_github: String::new(),
            token_gitlab: String::new(),
            user_name: String::new(),
            user_email: String::new(),
        }
    }
}

pub type Progress<'a> = &'a mut dyn FnMut(f32, &str);

/// Run the git program and wait for it. Progress lines from stderr are reported.
pub fn run_git(engine: &Engine, dir: &Path, args: &[&str], progress: Option<Progress>) -> GResult<String> {
    let git = engine.cli.as_ref().ok_or_else(|| "Git program not found".to_string())?;
    let mut cmd = quiet_command(git);
    cmd.args(args)
        .current_dir(dir)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if !engine.user_name.is_empty() {
        cmd.env("GIT_AUTHOR_NAME", &engine.user_name).env("GIT_COMMITTER_NAME", &engine.user_name);
    }
    if !engine.user_email.is_empty() {
        cmd.env("GIT_AUTHOR_EMAIL", &engine.user_email).env("GIT_COMMITTER_EMAIL", &engine.user_email);
    }
    // Sign in to GitHub/GitLab with the saved tokens instead of whatever the
    // system credential manager remembers (often another account -> 403).
    if !engine.token_github.trim().is_empty() || !engine.token_gitlab.trim().is_empty() {
        let mut urls: Vec<String> = args.iter().filter(|a| a.starts_with("http")).map(|a| a.to_string()).collect();
        urls.extend(remote_urls(dir));
        for (k, v) in crate::auth::as_env(&crate::auth::git_auth_config(&urls, &engine.token_github, &engine.token_gitlab)) {
            cmd.env(k, v);
        }
    }
    let mut child = cmd.spawn().map_err(|e| format!("cannot start git: {e}"))?;
    let mut stdout = child.stdout.take().unwrap();
    let out_thread = std::thread::spawn(move || {
        let mut s = Vec::new();
        let _ = stdout.read_to_end(&mut s);
        s
    });
    let mut stderr = child.stderr.take().unwrap();
    let mut err_all = Vec::new();
    let mut line = Vec::new();
    let mut buf = [0u8; 1024];
    let mut progress = progress;
    loop {
        let n = stderr.read(&mut buf).unwrap_or(0);
        if n == 0 {
            break;
        }
        for &b in &buf[..n] {
            err_all.push(b);
            if b == b'\r' || b == b'\n' {
                if let Some(p) = progress.as_mut() {
                    let l = String::from_utf8_lossy(&line).to_string();
                    if let Some(pct) = parse_percent(&l) {
                        p(pct, &l);
                    }
                }
                line.clear();
            } else {
                line.push(b);
            }
        }
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    let out = out_thread.join().unwrap_or_default();
    let out_s = String::from_utf8_lossy(&out).to_string();
    let err_s = String::from_utf8_lossy(&err_all).to_string();
    if status.success() {
        let mut s = out_s;
        if s.trim().is_empty() {
            s = err_s.lines().filter(|l| !l.contains('%')).collect::<Vec<_>>().join("\n");
        }
        Ok(s.trim().to_string())
    } else {
        let msg = err_s
            .split(['\r', '\n'])
            .filter(|l| !l.trim().is_empty() && !l.contains('%'))
            .collect::<Vec<_>>()
            .join("\n");
        Err(if msg.is_empty() { out_s.trim().to_string() } else { msg })
    }
}

/// Push and fetch addresses of every remote of the repository at `dir`.
pub fn remote_urls(dir: &Path) -> Vec<String> {
    let Ok(repo) = Repository::open(dir) else { return Vec::new() };
    let mut out = Vec::new();
    if let Ok(names) = repo.remotes() {
        for n in names.iter().flatten() {
            if let Ok(r) = repo.find_remote(n) {
                for u in [r.url(), r.pushurl()].into_iter().flatten() {
                    out.push(u.to_string());
                }
            }
        }
    }
    out
}

fn remote_url_of(dir: &Path, remote: &str) -> String {
    Repository::open(dir)
        .ok()
        .and_then(|r| r.find_remote(remote).ok().and_then(|x| x.pushurl().or(x.url()).map(|s| s.to_string())))
        .unwrap_or_default()
}

/// When the server refused, replace Git's message with a plain explanation.
fn explain(engine: &Engine, dir: &Path, remote: Option<&str>, r: GResult<String>) -> GResult<String> {
    match r {
        Err(e) if crate::auth::is_auth_refusal(&e) => {
            let url = match remote {
                Some(n) => remote_url_of(dir, n),
                None => remote_urls(dir).into_iter().next().unwrap_or_default(),
            };
            Err(crate::auth::explain_refusal(&url, &engine.token_github, &engine.token_gitlab, &e))
        }
        other => other,
    }
}

fn parse_percent(l: &str) -> Option<f32> {
    let i = l.find('%')?;
    let start = l[..i].rfind(|c: char| !c.is_ascii_digit()).map(|x| x + 1).unwrap_or(0);
    let n: f32 = l[start..i].parse().ok()?;
    // Weight the phases of a clone/fetch so the bar keeps moving forward.
    let base = if l.starts_with("Receiving") {
        0.1
    } else if l.starts_with("Resolving") {
        0.8
    } else if l.starts_with("Updating files") || l.starts_with("Checking out") {
        0.9
    } else {
        0.0
    };
    let span = if base == 0.0 { 0.1 } else if base == 0.1 { 0.7 } else { 0.1 };
    Some(base + span * n / 100.0)
}

fn callbacks<'a>(engine: &Engine, progress: Option<Progress<'a>>) -> RemoteCallbacks<'a> {
    let mut cb = RemoteCallbacks::new();
    let mut tries = 0;
    let gh = crate::auth::clean_token(&engine.token_github);
    let gl = crate::auth::clean_token(&engine.token_gitlab);
    cb.credentials(move |url, username, allowed| {
        tries += 1;
        if tries > 5 {
            return Err(git2::Error::from_str("authentication failed"));
        }
        if allowed.contains(CredentialType::SSH_KEY) {
            if tries <= 2 {
                return Cred::ssh_key_from_agent(username.unwrap_or("git"));
            }
            if let Some(home) = dirs::home_dir() {
                for k in ["id_ed25519", "id_rsa", "id_ecdsa"] {
                    let p = home.join(".ssh").join(k);
                    if p.exists() {
                        return Cred::ssh_key(username.unwrap_or("git"), None, &p, None);
                    }
                }
            }
        }
        if allowed.contains(CredentialType::USER_PASS_PLAINTEXT) {
            let host = crate::auth::host_of(url).map(|h| h.0);
            if host == Some(crate::auth::Host::GitHub) && !gh.is_empty() && tries <= 2 {
                return Cred::userpass_plaintext("x-access-token", &gh);
            }
            if host == Some(crate::auth::Host::GitLab) && !gl.is_empty() && tries <= 2 {
                return Cred::userpass_plaintext("oauth2", &gl);
            }
            if let Ok(cfg) = git2::Config::open_default() {
                if let Ok(c) = Cred::credential_helper(&cfg, url, username) {
                    return Ok(c);
                }
            }
        }
        if allowed.contains(CredentialType::DEFAULT) {
            return Cred::default();
        }
        Err(git2::Error::from_str("no credentials available (add a token in Preferences or install Git)"))
    });
    if let Some(p) = progress {
        cb.transfer_progress(move |s| {
            let total = s.total_objects().max(1) as f32;
            let frac = (s.received_objects() as f32 / total) * 0.8 + (s.indexed_deltas() as f32 / s.total_deltas().max(1) as f32) * 0.2;
            p(frac, &format!("{}/{} objects", s.received_objects(), s.total_objects()));
            true
        });
    }
    cb
}

// ---------------------------------------------------------------------------
// Data read from a repository
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct CommitRow {
    pub id: Oid,
    pub short: String,
    pub parents: Vec<Oid>,
    pub author: String,
    pub email: String,
    pub time: i64,
    pub summary: String,
    pub body: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefKind {
    Local,
    Remote,
    Tag,
}

#[derive(Clone, Debug)]
pub struct RefLabel {
    pub name: String,
    pub kind: RefKind,
    pub is_head: bool,
}

#[derive(Clone, Debug, Default)]
pub struct BranchInfo {
    pub name: String,
    pub oid: Option<Oid>,
    pub is_head: bool,
    pub upstream: Option<String>,
    pub ahead: usize,
    pub behind: usize,
}

#[derive(Clone, Debug, Default)]
pub struct RemoteInfo {
    pub name: String,
    pub url: String,
    pub branches: Vec<BranchInfo>,
}

#[derive(Clone, Debug)]
pub struct StashInfo {
    pub index: usize,
    pub message: String,
    pub oid: Oid,
}

#[derive(Clone, Debug)]
pub struct FileChange {
    pub path: String,
    pub old_path: Option<String>,
    /// A added, M modified, D deleted, R renamed, T type change, C conflict.
    pub status: char,
    pub additions: usize,
    pub deletions: usize,
}

#[derive(Clone, Debug, Default)]
pub struct Wip {
    pub unstaged: Vec<FileChange>,
    pub staged: Vec<FileChange>,
    pub conflicted: Vec<String>,
}

impl Wip {
    pub fn is_empty(&self) -> bool {
        self.unstaged.is_empty() && self.staged.is_empty() && self.conflicted.is_empty()
    }
    pub fn count(&self) -> usize {
        let mut set: std::collections::BTreeSet<&str> = self.unstaged.iter().map(|f| f.path.as_str()).collect();
        set.extend(self.staged.iter().map(|f| f.path.as_str()));
        set.extend(self.conflicted.iter().map(|s| s.as_str()));
        set.len()
    }
}

#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub path: PathBuf,
    pub name: String,
    pub head_name: String,
    pub head_oid: Option<Oid>,
    pub detached: bool,
    pub unborn: bool,
    pub state: String,
    pub commits: Vec<CommitRow>,
    pub refs: HashMap<Oid, Vec<RefLabel>>,
    pub local: Vec<BranchInfo>,
    pub remotes: Vec<RemoteInfo>,
    pub tags: Vec<(String, Oid)>,
    pub stashes: Vec<StashInfo>,
    pub submodules: Vec<(String, String)>,
    pub worktrees: Vec<String>,
    pub wip: Wip,
    pub truncated: bool,
}

impl Snapshot {
    pub fn row_of(&self, oid: Oid) -> Option<usize> {
        self.commits.iter().position(|c| c.id == oid)
    }
    pub fn current_upstream(&self) -> Option<&BranchInfo> {
        self.local.iter().find(|b| b.is_head)
    }
}

pub fn open(path: &Path) -> GResult<Repository> {
    Repository::open(path).es()
}

pub fn is_repo(path: &Path) -> bool {
    Repository::open(path).is_ok()
}

pub fn repo_name(path: &Path) -> String {
    path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| path.display().to_string())
}

pub fn load(path: &Path, max_commits: usize) -> GResult<Snapshot> {
    let mut repo = open(path)?;
    let workdir = repo.workdir().map(|p| p.to_path_buf()).unwrap_or_else(|| path.to_path_buf());
    let mut s = Snapshot {
        path: workdir.clone(),
        name: repo_name(&workdir),
        ..Default::default()
    };
    s.state = format!("{:?}", repo.state());

    match repo.head() {
        Ok(h) => {
            s.head_oid = h.target();
            s.detached = repo.head_detached().unwrap_or(false);
            s.head_name = if s.detached {
                h.target().map(|o| o.to_string()[..7].to_string()).unwrap_or_default()
            } else {
                h.shorthand().unwrap_or("HEAD").to_string()
            };
        }
        Err(_) => {
            s.unborn = true;
            s.head_name = repo
                .find_reference("HEAD")
                .ok()
                .and_then(|r| r.symbolic_target().map(|t| t.trim_start_matches("refs/heads/").to_string()))
                .unwrap_or_else(|| "main".into());
        }
    }

    // Branches
    let head_ref = repo.head().ok().and_then(|h| h.name().map(|s| s.to_string()));
    if let Ok(branches) = repo.branches(Some(BranchType::Local)) {
        for (b, _) in branches.flatten() {
            let name = b.name().ok().flatten().unwrap_or("").to_string();
            let oid = b.get().target();
            let is_head = b.get().name().map(|n| Some(n.to_string()) == head_ref).unwrap_or(false);
            let mut info = BranchInfo { name, oid, is_head, ..Default::default() };
            if let Ok(up) = b.upstream() {
                info.upstream = up.name().ok().flatten().map(|s| s.to_string());
                if let (Some(l), Some(u)) = (oid, up.get().target()) {
                    if let Ok((a, bh)) = repo.graph_ahead_behind(l, u) {
                        info.ahead = a;
                        info.behind = bh;
                    }
                }
            }
            s.local.push(info);
        }
    }
    s.local.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    if let Ok(remotes) = repo.remotes() {
        for r in remotes.iter().flatten() {
            let url = repo.find_remote(r).ok().and_then(|x| x.url().map(|u| u.to_string())).unwrap_or_default();
            s.remotes.push(RemoteInfo { name: r.to_string(), url, branches: vec![] });
        }
    }
    if let Ok(branches) = repo.branches(Some(BranchType::Remote)) {
        for (b, _) in branches.flatten() {
            let full = b.name().ok().flatten().unwrap_or("").to_string();
            if full.ends_with("/HEAD") {
                continue;
            }
            let (remote, name) = match full.split_once('/') {
                Some((r, n)) => (r.to_string(), n.to_string()),
                None => continue,
            };
            let oid = b.get().target();
            if let Some(ri) = s.remotes.iter_mut().find(|x| x.name == remote) {
                ri.branches.push(BranchInfo { name, oid, ..Default::default() });
            }
        }
    }
    for r in &mut s.remotes {
        r.branches.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    }

    // Tags
    if let Ok(names) = repo.tag_names(None) {
        for n in names.iter().flatten() {
            if let Ok(r) = repo.find_reference(&format!("refs/tags/{n}")) {
                if let Ok(c) = r.peel_to_commit() {
                    s.tags.push((n.to_string(), c.id()));
                }
            }
        }
    }

    // Stashes
    let mut stashes = Vec::new();
    let _ = repo.stash_foreach(|i, msg, oid| {
        stashes.push(StashInfo { index: i, message: msg.to_string(), oid: *oid });
        true
    });
    s.stashes = stashes;

    if let Ok(subs) = repo.submodules() {
        for sm in subs {
            s.submodules.push((sm.name().unwrap_or("").to_string(), sm.path().display().to_string()));
        }
    }
    if let Ok(wts) = repo.worktrees() {
        s.worktrees = wts.iter().flatten().map(|w| w.to_string()).collect();
    }

    // Ref labels per commit
    for b in &s.local {
        if let Some(o) = b.oid {
            s.refs.entry(o).or_default().push(RefLabel { name: b.name.clone(), kind: RefKind::Local, is_head: b.is_head });
        }
    }
    for r in &s.remotes {
        for b in &r.branches {
            if let Some(o) = b.oid {
                s.refs.entry(o).or_default().push(RefLabel {
                    name: format!("{}/{}", r.name, b.name),
                    kind: RefKind::Remote,
                    is_head: false,
                });
            }
        }
    }
    for (n, o) in &s.tags {
        s.refs.entry(*o).or_default().push(RefLabel { name: n.clone(), kind: RefKind::Tag, is_head: false });
    }

    // Commits
    if !s.unborn || !s.local.is_empty() || !s.remotes.is_empty() {
        let mut walk = repo.revwalk().es()?;
        walk.set_sorting(Sort::TOPOLOGICAL | Sort::TIME).es()?;
        let _ = walk.push_head();
        for b in &s.local {
            if let Some(o) = b.oid {
                let _ = walk.push(o);
            }
        }
        for r in &s.remotes {
            for b in &r.branches {
                if let Some(o) = b.oid {
                    let _ = walk.push(o);
                }
            }
        }
        for (_, o) in &s.tags {
            let _ = walk.push(*o);
        }
        for oid in walk.flatten() {
            if s.commits.len() >= max_commits {
                s.truncated = true;
                break;
            }
            if let Ok(c) = repo.find_commit(oid) {
                let msg = c.message().unwrap_or("").to_string();
                let summary = c.summary().unwrap_or("").to_string();
                let body = msg
                    .splitn(2, '\n')
                    .nth(1)
                    .map(|b| b.trim().replace('\n', " "))
                    .unwrap_or_default();
                let a = c.author();
                s.commits.push(CommitRow {
                    id: oid,
                    short: oid.to_string()[..7].to_string(),
                    parents: c.parent_ids().collect(),
                    author: a.name().unwrap_or("").to_string(),
                    email: a.email().unwrap_or("").to_string(),
                    time: c.time().seconds(),
                    summary,
                    body,
                });
            }
        }
    }

    s.wip = wip_status(&repo)?;
    Ok(s)
}

fn status_char(st: git2::Delta) -> char {
    match st {
        git2::Delta::Added | git2::Delta::Untracked => 'A',
        git2::Delta::Deleted => 'D',
        git2::Delta::Renamed => 'R',
        git2::Delta::Copied => 'A',
        git2::Delta::Typechange => 'T',
        git2::Delta::Conflicted => 'C',
        _ => 'M',
    }
}

fn diff_files(diff: &git2::Diff) -> Vec<FileChange> {
    let mut out = Vec::new();
    for (i, d) in diff.deltas().enumerate() {
        let path = d
            .new_file()
            .path()
            .or_else(|| d.old_file().path())
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        let old = d.old_file().path().map(|p| p.to_string_lossy().replace('\\', "/"));
        let (mut add, mut del) = (0, 0);
        if let Ok(Some(p)) = git2::Patch::from_diff(diff, i) {
            if let Ok((_, a, dl)) = p.line_stats() {
                add = a;
                del = dl;
            }
        }
        out.push(FileChange {
            old_path: if d.status() == git2::Delta::Renamed { old } else { None },
            path,
            status: status_char(d.status()),
            additions: add,
            deletions: del,
        });
    }
    out
}

pub fn wip_status(repo: &Repository) -> GResult<Wip> {
    let mut w = Wip::default();
    if repo.is_bare() {
        return Ok(w);
    }
    let mut opts = StatusOptions::new();
    opts.include_untracked(true).recurse_untracked_dirs(true).renames_head_to_index(true);
    let statuses = repo.statuses(Some(&mut opts)).es()?;
    for e in statuses.iter() {
        let st = e.status();
        let path = e.path().unwrap_or("").to_string();
        if st.is_conflicted() {
            w.conflicted.push(path.clone());
            continue;
        }
        let idx = if st.is_index_new() {
            Some('A')
        } else if st.is_index_modified() {
            Some('M')
        } else if st.is_index_deleted() {
            Some('D')
        } else if st.is_index_renamed() {
            Some('R')
        } else if st.is_index_typechange() {
            Some('T')
        } else {
            None
        };
        let wt = if st.is_wt_new() {
            Some('A')
        } else if st.is_wt_modified() {
            Some('M')
        } else if st.is_wt_deleted() {
            Some('D')
        } else if st.is_wt_renamed() {
            Some('R')
        } else if st.is_wt_typechange() {
            Some('T')
        } else {
            None
        };
        if let Some(c) = idx {
            w.staged.push(FileChange { path: path.clone(), old_path: None, status: c, additions: 0, deletions: 0 });
        }
        if let Some(c) = wt {
            w.unstaged.push(FileChange { path, old_path: None, status: c, additions: 0, deletions: 0 });
        }
    }
    Ok(w)
}

/// Files changed by a commit (compared with its first parent).
pub fn commit_files(path: &Path, oid: Oid) -> GResult<Vec<FileChange>> {
    let repo = open(path)?;
    let c = repo.find_commit(oid).es()?;
    let tree = c.tree().es()?;
    let parent_tree = c.parent(0).ok().and_then(|p| p.tree().ok());
    let mut diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None).es()?;
    let _ = diff.find_similar(None);
    Ok(diff_files(&diff))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiffLineKind {
    Header,
    Hunk,
    Add,
    Del,
    Context,
}

#[derive(Clone, Debug)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub old_no: Option<u32>,
    pub new_no: Option<u32>,
    pub text: String,
}

fn diff_lines(diff: &git2::Diff) -> Vec<DiffLine> {
    let mut out = Vec::new();
    let _ = diff.print(DiffFormat::Patch, |_d, _h, l| {
        let text = String::from_utf8_lossy(l.content()).trim_end_matches(['\n', '\r']).to_string();
        let kind = match l.origin() {
            '+' | '>' => DiffLineKind::Add,
            '-' | '<' => DiffLineKind::Del,
            ' ' | '=' => DiffLineKind::Context,
            'H' => DiffLineKind::Hunk,
            _ => DiffLineKind::Header,
        };
        if kind == DiffLineKind::Header {
            return true;
        }
        out.push(DiffLine { kind, old_no: l.old_lineno(), new_no: l.new_lineno(), text });
        true
    });
    out
}

pub fn diff_text(lines: &[DiffLine]) -> String {
    lines
        .iter()
        .map(|l| {
            let p = match l.kind {
                DiffLineKind::Add => "+",
                DiffLineKind::Del => "-",
                DiffLineKind::Context => " ",
                _ => "",
            };
            format!("{p}{}", l.text)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The diff of one file in a commit, or of the whole commit when `file` is None.
pub fn commit_diff(path: &Path, oid: Oid, file: Option<&str>) -> GResult<Vec<DiffLine>> {
    let repo = open(path)?;
    let c = repo.find_commit(oid).es()?;
    let tree = c.tree().es()?;
    let parent_tree = c.parent(0).ok().and_then(|p| p.tree().ok());
    let mut opts = DiffOptions::new();
    if let Some(f) = file {
        opts.pathspec(f).disable_pathspec_match(true);
    }
    let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut opts)).es()?;
    Ok(diff_lines(&diff))
}

/// The working-copy diff of one file (staged or not), or all of them.
pub fn wip_diff(path: &Path, file: Option<&str>, staged: bool) -> GResult<Vec<DiffLine>> {
    let repo = open(path)?;
    let mut opts = DiffOptions::new();
    opts.include_untracked(true).recurse_untracked_dirs(true).show_untracked_content(true);
    if let Some(f) = file {
        opts.pathspec(f).disable_pathspec_match(true);
    }
    let diff = if staged {
        let head_tree = repo.head().ok().and_then(|h| h.peel_to_tree().ok());
        repo.diff_tree_to_index(head_tree.as_ref(), None, Some(&mut opts)).es()?
    } else {
        repo.diff_index_to_workdir(None, Some(&mut opts)).es()?
    };
    Ok(diff_lines(&diff))
}

#[derive(Clone, Debug, Default)]
pub struct CommitDetail {
    pub id: String,
    pub message: String,
    pub author: String,
    pub email: String,
    pub committer: String,
    pub time: i64,
    pub parents: Vec<String>,
    pub files: Vec<FileChange>,
}

pub fn commit_detail(path: &Path, oid: Oid) -> GResult<CommitDetail> {
    let repo = open(path)?;
    let c = repo.find_commit(oid).es()?;
    let d = CommitDetail {
        id: oid.to_string(),
        message: c.message().unwrap_or("").to_string(),
        author: c.author().name().unwrap_or("").to_string(),
        email: c.author().email().unwrap_or("").to_string(),
        committer: c.committer().name().unwrap_or("").to_string(),
        time: c.time().seconds(),
        parents: c.parent_ids().map(|p| p.to_string()[..7].to_string()).collect(),
        files: commit_files(path, oid)?,
    };
    Ok(d)
}

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

fn signature<'a>(repo: &Repository, engine: &Engine) -> GResult<Signature<'a>> {
    if !engine.user_name.is_empty() && !engine.user_email.is_empty() {
        return Signature::now(&engine.user_name, &engine.user_email).es();
    }
    repo.signature()
        .map_err(|_| "Set your name and e-mail in Preferences > Profile before committing".to_string())
}

pub fn stage(path: &Path, files: &[String]) -> GResult<()> {
    let repo = open(path)?;
    let mut idx = repo.index().es()?;
    let wd = repo.workdir().ok_or("bare repository")?.to_path_buf();
    for f in files {
        if wd.join(f).exists() {
            idx.add_path(Path::new(f)).es()?;
        } else {
            idx.remove_path(Path::new(f)).es()?;
        }
    }
    idx.write().es()
}

pub fn stage_all(path: &Path) -> GResult<()> {
    let repo = open(path)?;
    let mut idx = repo.index().es()?;
    idx.add_all(["*"], IndexAddOption::DEFAULT, None).es()?;
    idx.update_all(["*"], None).es()?;
    idx.write().es()
}

pub fn unstage(path: &Path, files: &[String]) -> GResult<()> {
    let repo = open(path)?;
    let head = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let res = match head {
        Some(c) => repo.reset_default(Some(c.as_object()), files.iter().map(|s| s.as_str())).es(),
        None => {
            let mut idx = repo.index().es()?;
            for f in files {
                let _ = idx.remove_path(Path::new(f));
            }
            idx.write().es()
        }
    };
    res
}

pub fn unstage_all(path: &Path) -> GResult<()> {
    let w = wip_status(&open(path)?)?;
    let files: Vec<String> = w.staged.iter().map(|f| f.path.clone()).collect();
    if files.is_empty() {
        return Ok(());
    }
    unstage(path, &files)
}

/// Throw away working-copy changes for these files (untracked files are deleted).
pub fn discard(path: &Path, files: &[String]) -> GResult<()> {
    let repo = open(path)?;
    let wd = repo.workdir().ok_or("bare repository")?.to_path_buf();
    let idx = repo.index().es()?;
    let mut tracked = Vec::new();
    for f in files {
        if idx.get_path(Path::new(f), 0).is_none() {
            let p = wd.join(f);
            if p.is_dir() {
                let _ = std::fs::remove_dir_all(&p);
            } else {
                let _ = std::fs::remove_file(&p);
            }
        } else {
            tracked.push(f.clone());
        }
    }
    if !tracked.is_empty() {
        let mut cb = CheckoutBuilder::new();
        cb.force();
        for f in &tracked {
            cb.path(f.as_str());
        }
        repo.checkout_index(None, Some(&mut cb)).es()?;
    }
    Ok(())
}

pub fn discard_all(path: &Path) -> GResult<()> {
    let repo = open(path)?;
    let w = wip_status(&repo)?;
    if let Some(c) = repo.head().ok().and_then(|h| h.peel_to_commit().ok()) {
        repo.reset(c.as_object(), ResetType::Hard, None).es()?;
    }
    let wd = repo.workdir().ok_or("bare repository")?.to_path_buf();
    for f in w.unstaged.iter().filter(|f| f.status == 'A') {
        let p = wd.join(&f.path);
        if p.is_dir() {
            let _ = std::fs::remove_dir_all(&p);
        } else {
            let _ = std::fs::remove_file(&p);
        }
    }
    Ok(())
}

/// Commit what is staged. Returns the new commit id.
pub fn commit(engine: &Engine, path: &Path, message: &str, amend: bool) -> GResult<Oid> {
    if message.trim().is_empty() {
        return Err("The commit message is empty".into());
    }
    let mut repo = open(path)?;
    let mut merge_heads = Vec::new();
    let _ = repo.mergehead_foreach(|o| {
        merge_heads.push(*o);
        true
    });
    let repo = repo;
    let sig = signature(&repo, engine)?;
    let mut idx = repo.index().es()?;
    if idx.has_conflicts() {
        return Err("Resolve the conflicts before committing".into());
    }
    let tree_id = idx.write_tree().es()?;
    let tree = repo.find_tree(tree_id).es()?;
    let head = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    if amend {
        let h = head.ok_or("Nothing to amend")?;
        let oid = h.amend(Some("HEAD"), None, Some(&sig), None, Some(message), Some(&tree)).es()?;
        return Ok(oid);
    }
    if let Some(h) = &head {
        if h.tree_id() == tree_id && repo.state() != git2::RepositoryState::Merge {
            return Err("No staged changes to commit".into());
        }
    }
    let mut parents = Vec::new();
    if let Some(h) = head {
        parents.push(h);
    }
    for o in merge_heads {
        if let Ok(c) = repo.find_commit(o) {
            parents.push(c);
        }
    }
    let prefs: Vec<&git2::Commit> = parents.iter().collect();
    let oid = repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &prefs).es()?;
    let _ = repo.cleanup_state();
    Ok(oid)
}

pub fn create_branch(path: &Path, name: &str, at: Option<Oid>, checkout_it: bool) -> GResult<()> {
    let repo = open(path)?;
    let name = name.trim();
    if name.is_empty() {
        return Err("Branch name is empty".into());
    }
    {
        let target = match at {
            Some(o) => repo.find_commit(o).es()?,
            None => repo.head().es()?.peel_to_commit().es()?,
        };
        repo.branch(name, &target, false).es()?;
    }
    drop(repo);
    if checkout_it {
        checkout(path, name)?;
    }
    Ok(())
}

/// Switch to a local branch; a remote branch `origin/x` makes a tracking branch `x`.
pub fn checkout(path: &Path, branch: &str) -> GResult<()> {
    let repo = open(path)?;
    let local = if repo.find_branch(branch, BranchType::Local).is_ok() {
        branch.to_string()
    } else if let Ok(rb) = repo.find_branch(branch, BranchType::Remote) {
        let local_name = branch.split_once('/').map(|x| x.1).unwrap_or(branch).to_string();
        if repo.find_branch(&local_name, BranchType::Local).is_err() {
            let c = rb.get().peel_to_commit().es()?;
            let mut nb = repo.branch(&local_name, &c, false).es()?;
            nb.set_upstream(Some(branch)).es()?;
        }
        local_name
    } else {
        return Err(format!("Branch '{branch}' not found"));
    };
    let refname = format!("refs/heads/{local}");
    let obj = repo.revparse_single(&refname).es()?;
    let mut cb = CheckoutBuilder::new();
    cb.safe();
    repo.checkout_tree(&obj, Some(&mut cb)).es()?;
    repo.set_head(&refname).es()
}

/// True when `name` is a local branch (as opposed to a remote one like origin/x).
pub fn checkout_is_local(path: &Path, name: &str) -> bool {
    open(path).map(|r| r.find_branch(name, BranchType::Local).is_ok()).unwrap_or(false)
}

pub fn checkout_commit(path: &Path, oid: Oid) -> GResult<()> {
    let repo = open(path)?;
    let obj = repo.find_object(oid, None).es()?;
    let mut cb = CheckoutBuilder::new();
    cb.safe();
    repo.checkout_tree(&obj, Some(&mut cb)).es()?;
    repo.set_head_detached(oid).es()
}

pub fn delete_branch(path: &Path, name: &str) -> GResult<Option<Oid>> {
    let repo = open(path)?;
    let mut b = repo.find_branch(name, BranchType::Local).es()?;
    if b.is_head() {
        return Err("You cannot delete the branch you are on".into());
    }
    let oid = b.get().target();
    b.delete().es()?;
    Ok(oid)
}

pub fn rename_branch(path: &Path, old: &str, new: &str) -> GResult<()> {
    let repo = open(path)?;
    let mut b = repo.find_branch(old, BranchType::Local).es()?;
    b.rename(new, false).es().map(|_| ())
}

pub fn reset(path: &Path, oid: Oid, mode: &str) -> GResult<()> {
    let repo = open(path)?;
    let obj = repo.find_object(oid, None).es()?;
    let kind = match mode {
        "soft" => ResetType::Soft,
        "hard" => ResetType::Hard,
        _ => ResetType::Mixed,
    };
    repo.reset(&obj, kind, None).es()
}

pub fn create_tag(path: &Path, name: &str, oid: Oid, message: &str, engine: &Engine) -> GResult<()> {
    let repo = open(path)?;
    let obj = repo.find_object(oid, None).es()?;
    if message.trim().is_empty() {
        repo.tag_lightweight(name, &obj, false).es().map(|_| ())
    } else {
        let sig = signature(&repo, engine)?;
        repo.tag(name, &obj, &sig, message, false).es().map(|_| ())
    }
}

pub fn delete_tag(path: &Path, name: &str) -> GResult<()> {
    open(path)?.tag_delete(name).es()
}

pub fn stash_save(engine: &Engine, path: &Path, message: &str) -> GResult<()> {
    let mut repo = open(path)?;
    let sig = signature(&repo, engine)?;
    let msg = if message.trim().is_empty() { None } else { Some(message) };
    repo.stash_save(&sig, msg.unwrap_or("WIP"), Some(StashFlags::INCLUDE_UNTRACKED)).es().map(|_| ())
}

pub fn stash_pop(path: &Path, index: usize) -> GResult<()> {
    open(path)?.stash_pop(index, None).es()
}

pub fn stash_apply(path: &Path, index: usize) -> GResult<()> {
    open(path)?.stash_apply(index, None).es()
}

pub fn stash_drop(path: &Path, index: usize) -> GResult<()> {
    open(path)?.stash_drop(index).es()
}

fn commit_after_pick(engine: &Engine, repo: &Repository, message: &str, author: Option<Signature>) -> GResult<String> {
    let mut idx = repo.index().es()?;
    if idx.has_conflicts() {
        return Ok("conflicts".into());
    }
    let tree_id = idx.write_tree().es()?;
    let tree = repo.find_tree(tree_id).es()?;
    let sig = signature(repo, engine)?;
    let head = repo.head().es()?.peel_to_commit().es()?;
    let author = author.unwrap_or_else(|| sig.clone());
    repo.commit(Some("HEAD"), &author, &sig, message, &tree, &[&head]).es()?;
    let _ = repo.cleanup_state();
    Ok("ok".into())
}

pub fn cherry_pick(engine: &Engine, path: &Path, oid: Oid) -> GResult<String> {
    if engine.cli.is_some() {
        return run_git(engine, path, &["cherry-pick", &oid.to_string()], None);
    }
    let repo = open(path)?;
    let c = repo.find_commit(oid).es()?;
    repo.cherrypick(&c, None).es()?;
    let author = c.author().to_owned();
    commit_after_pick(engine, &repo, c.message().unwrap_or(""), Some(author))
}

pub fn revert(engine: &Engine, path: &Path, oid: Oid) -> GResult<String> {
    if engine.cli.is_some() {
        return run_git(engine, path, &["revert", "--no-edit", &oid.to_string()], None);
    }
    let repo = open(path)?;
    let c = repo.find_commit(oid).es()?;
    repo.revert(&c, None).es()?;
    let msg = format!("Revert \"{}\"\n\nThis reverts commit {}.", c.summary().unwrap_or(""), oid);
    commit_after_pick(engine, &repo, &msg, None)
}

/// Merge a branch (local or remote, e.g. `origin/main`) into the current branch.
pub fn merge(engine: &Engine, path: &Path, branch: &str, ff_only: bool) -> GResult<String> {
    if engine.cli.is_some() {
        let mut args = vec!["merge", "--no-edit"];
        if ff_only {
            args.push("--ff-only");
        }
        args.push(branch);
        return run_git(engine, path, &args, None);
    }
    let repo = open(path)?;
    let reference = repo
        .find_branch(branch, BranchType::Local)
        .or_else(|_| repo.find_branch(branch, BranchType::Remote))
        .es()?;
    let annotated = repo.reference_to_annotated_commit(reference.get()).es()?;
    let (analysis, _) = repo.merge_analysis(&[&annotated]).es()?;
    if analysis.is_up_to_date() {
        return Ok("Already up to date".into());
    }
    if analysis.is_fast_forward() || analysis.is_unborn() {
        let target = annotated.id();
        let mut head = repo.head().es()?;
        let obj = repo.find_object(target, None).es()?;
        let mut cb = CheckoutBuilder::new();
        cb.safe();
        repo.checkout_tree(&obj, Some(&mut cb)).es()?;
        head.set_target(target, "fast-forward").es()?;
        return Ok("Fast-forward".into());
    }
    if ff_only {
        return Err("Cannot fast-forward; the branches have diverged".into());
    }
    repo.merge(&[&annotated], None, None).es()?;
    let mut idx = repo.index().es()?;
    if idx.has_conflicts() {
        return Err("Merge conflicts: resolve them, stage the files and commit".into());
    }
    let tree_id = idx.write_tree().es()?;
    let tree = repo.find_tree(tree_id).es()?;
    let sig = signature(&repo, engine)?;
    let head = repo.head().es()?.peel_to_commit().es()?;
    let other = repo.find_commit(annotated.id()).es()?;
    let msg = format!("Merge branch '{}'", branch);
    repo.commit(Some("HEAD"), &sig, &sig, &msg, &tree, &[&head, &other]).es()?;
    let _ = repo.cleanup_state();
    let mut cb = CheckoutBuilder::new();
    cb.force();
    let _ = repo.checkout_head(Some(&mut cb));
    Ok("Merged".into())
}

pub fn abort_merge(engine: &Engine, path: &Path) -> GResult<()> {
    if engine.cli.is_some() {
        return run_git(engine, path, &["merge", "--abort"], None).map(|_| ());
    }
    let repo = open(path)?;
    let head = repo.head().es()?.peel_to_commit().es()?;
    repo.reset(head.as_object(), ResetType::Hard, None).es()?;
    repo.cleanup_state().es()
}

pub fn fetch(engine: &Engine, path: &Path, remote: Option<&str>, progress: Option<Progress>) -> GResult<String> {
    explain(engine, path, remote, fetch_inner(engine, path, remote, progress))
}

pub fn pull(engine: &Engine, path: &Path, mode: crate::config::PullMode, progress: Option<Progress>) -> GResult<String> {
    let remote = upstream_remote(path);
    explain(engine, path, remote.as_deref(), pull_inner(engine, path, mode, progress))
}

pub fn push(engine: &Engine, path: &Path, force: bool, progress: Option<Progress>) -> GResult<String> {
    let remote = upstream_remote(path).or_else(|| remote_names(path).into_iter().next());
    explain(engine, path, remote.as_deref(), push_inner(engine, path, force, progress))
}

fn remote_names(path: &Path) -> Vec<String> {
    Repository::open(path).ok().and_then(|r| r.remotes().ok().map(|n| n.iter().flatten().map(|s| s.to_string()).collect())).unwrap_or_default()
}

/// The remote the current branch tracks, if any.
fn upstream_remote(path: &Path) -> Option<String> {
    let repo = Repository::open(path).ok()?;
    let head = repo.head().ok()?;
    let name = head.shorthand()?.to_string();
    let b = repo.find_branch(&name, BranchType::Local).ok()?;
    let up = b.upstream().ok()?;
    let n = up.name().ok().flatten()?.to_string();
    n.split('/').next().map(|s| s.to_string())
}

fn fetch_inner(engine: &Engine, path: &Path, remote: Option<&str>, progress: Option<Progress>) -> GResult<String> {
    if engine.cli.is_some() {
        let mut args = vec!["fetch", "--progress", "--prune"];
        match remote {
            Some(r) => args.push(r),
            None => args.push("--all"),
        }
        return run_git(engine, path, &args, progress);
    }
    let repo = open(path)?;
    let names: Vec<String> = match remote {
        Some(r) => vec![r.to_string()],
        None => repo.remotes().es()?.iter().flatten().map(|s| s.to_string()).collect(),
    };
    if names.is_empty() {
        return Ok("No remotes to fetch".into());
    }
    let mut progress = progress;
    for n in &names {
        let mut r = repo.find_remote(n).es()?;
        let mut fo = FetchOptions::new();
        let pr: Option<&mut dyn FnMut(f32, &str)> = match progress.as_mut() {
            Some(p) => Some(&mut **p),
            None => None,
        };
        fo.remote_callbacks(callbacks(engine, pr));
        fo.prune(git2::FetchPrune::On);
        fo.download_tags(git2::AutotagOption::All);
        r.fetch(&[] as &[&str], Some(&mut fo), None).es()?;
    }
    Ok(format!("Fetched {}", names.join(", ")))
}

fn pull_inner(engine: &Engine, path: &Path, mode: crate::config::PullMode, progress: Option<Progress>) -> GResult<String> {
    use crate::config::PullMode;
    if engine.cli.is_some() {
        let args: Vec<&str> = match mode {
            PullMode::FetchAll => vec!["fetch", "--all", "--prune", "--progress"],
            PullMode::FastForwardIfPossible => vec!["pull", "--ff", "--no-rebase", "--progress"],
            PullMode::FastForwardOnly => vec!["pull", "--ff-only", "--progress"],
            PullMode::Rebase => vec!["pull", "--rebase", "--progress"],
        };
        return run_git(engine, path, &args, progress);
    }
    if mode == PullMode::Rebase {
        return Err("Pull with rebase needs the Git program: download it from Preferences > Git".into());
    }
    let (remote, upstream) = {
        let repo = open(path)?;
        let head = repo.head().es()?;
        let name = head.shorthand().unwrap_or("").to_string();
        let b = repo.find_branch(&name, BranchType::Local).es()?;
        let up = b.upstream().map_err(|_| format!("Branch '{name}' has no upstream; push it first"))?;
        let up_name = up.name().ok().flatten().unwrap_or("").to_string();
        let remote = up_name.split('/').next().unwrap_or("origin").to_string();
        (remote, up_name)
    };
    fetch_inner(engine, path, Some(&remote), progress)?;
    if mode == PullMode::FetchAll {
        return Ok("Fetched".into());
    }
    merge(engine, path, &upstream, mode == PullMode::FastForwardOnly)
}

fn push_inner(engine: &Engine, path: &Path, force: bool, progress: Option<Progress>) -> GResult<String> {
    let (branch, remote, has_upstream) = {
        let repo = open(path)?;
        let head = repo.head().es()?;
        let name = head.shorthand().unwrap_or("").to_string();
        let b = repo.find_branch(&name, BranchType::Local).es()?;
        let up = b.upstream().ok().and_then(|u| u.name().ok().flatten().map(|s| s.to_string()));
        let remote = up
            .as_ref()
            .and_then(|u| u.split('/').next().map(|s| s.to_string()))
            .or_else(|| repo.remotes().ok().and_then(|r| r.get(0).map(|s| s.to_string())))
            .ok_or("This repository has no remote. Add one first (Actions > Add remote)")?;
        (name, remote, up.is_some())
    };
    if engine.cli.is_some() {
        let mut args = vec!["push", "--progress"];
        if force {
            args.push("--force-with-lease");
        }
        if !has_upstream {
            args.push("-u");
        }
        args.push(&remote);
        args.push(&branch);
        return run_git(engine, path, &args, progress);
    }
    let repo = open(path)?;
    let mut r = repo.find_remote(&remote).es()?;
    let mut po = PushOptions::new();
    po.remote_callbacks(callbacks(engine, progress));
    let spec = format!("{}refs/heads/{b}:refs/heads/{b}", if force { "+" } else { "" }, b = branch);
    r.push(&[spec.as_str()], Some(&mut po)).es()?;
    if !has_upstream {
        if let Ok(mut b) = repo.find_branch(&branch, BranchType::Local) {
            let _ = b.set_upstream(Some(&format!("{remote}/{branch}")));
        }
    }
    Ok(format!("Pushed {branch} to {remote}"))
}

pub fn clone(engine: &Engine, url: &str, dest: &Path, shallow: bool, progress: Option<Progress>) -> GResult<PathBuf> {
    if url.trim().is_empty() {
        return Err("The URL is empty".into());
    }
    if dest.exists() && std::fs::read_dir(dest).map(|mut d| d.next().is_some()).unwrap_or(false) {
        return Err(format!("{} already exists and is not empty", dest.display()));
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    if engine.cli.is_some() {
        let d = dest.display().to_string();
        let mut args = vec!["clone", "--progress"];
        if shallow {
            args.extend(["--depth", "1"]);
        }
        args.push(url.trim());
        args.push(&d);
        let cwd = dest.parent().map(|p| p.to_path_buf()).unwrap_or_else(std::env::temp_dir);
        run_git(engine, &cwd, &args, progress)?;
        return Ok(dest.to_path_buf());
    }
    let mut fo = FetchOptions::new();
    fo.remote_callbacks(callbacks(engine, progress));
    if shallow {
        fo.depth(1);
    }
    git2::build::RepoBuilder::new().fetch_options(fo).clone(url.trim(), dest).es()?;
    Ok(dest.to_path_buf())
}

pub struct InitOptions {
    pub default_branch: String,
    pub gitignore: String,
    pub license: String,
    pub lfs: bool,
    pub remote_url: String,
    pub initial_commit: bool,
}

pub fn init(engine: &Engine, dir: &Path, o: &InitOptions) -> GResult<String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut opts = git2::RepositoryInitOptions::new();
    let branch = if o.default_branch.trim().is_empty() { "main" } else { o.default_branch.trim() };
    opts.initial_head(branch);
    let repo = Repository::init_opts(dir, &opts).es()?;
    let mut files = Vec::new();
    if !o.gitignore.is_empty() {
        std::fs::write(dir.join(".gitignore"), &o.gitignore).map_err(|e| e.to_string())?;
        files.push(".gitignore".to_string());
    }
    if !o.license.is_empty() {
        std::fs::write(dir.join("LICENSE"), &o.license).map_err(|e| e.to_string())?;
        files.push("LICENSE".to_string());
    }
    let readme = dir.join("README.md");
    if !readme.exists() {
        let _ = std::fs::write(&readme, format!("# {}\n", repo_name(dir)));
        files.push("README.md".into());
    }
    let mut notes = Vec::new();
    if o.lfs {
        if engine.cli.is_some() {
            match run_git(engine, dir, &["lfs", "install", "--local"], None) {
                Ok(_) => notes.push("Git LFS enabled".to_string()),
                Err(e) => notes.push(format!("Git LFS not available: {e}")),
            }
        } else {
            notes.push("Git LFS needs the Git program".into());
        }
    }
    if !o.remote_url.trim().is_empty() {
        repo.remote("origin", o.remote_url.trim()).es()?;
    }
    drop(repo);
    if o.initial_commit && !files.is_empty() {
        stage(dir, &files)?;
        commit(engine, dir, "Initial commit", false)?;
    }
    Ok(notes.join("; "))
}

pub fn add_remote(path: &Path, name: &str, url: &str) -> GResult<()> {
    open(path)?.remote(name, url).es().map(|_| ())
}

pub fn remove_remote(path: &Path, name: &str) -> GResult<()> {
    open(path)?.remote_delete(name).es()
}

pub fn read_config(path: Option<&Path>, key: &str) -> Option<String> {
    let cfg = match path {
        Some(p) => open(p).ok()?.config().ok()?,
        None => git2::Config::open_default().ok()?,
    };
    cfg.get_string(key).ok()
}

pub fn write_global_identity(name: &str, email: &str) -> GResult<()> {
    let mut cfg = git2::Config::open_default().es()?;
    let mut global = cfg.open_global().or_else(|_| {
        let p = dirs::home_dir().ok_or_else(|| git2::Error::from_str("no home"))?.join(".gitconfig");
        if !p.exists() {
            let _ = std::fs::write(&p, "");
        }
        git2::Config::open(&p)
    });
    if let Ok(g) = global.as_mut() {
        g.set_str("user.name", name).es()?;
        g.set_str("user.email", email).es()?;
        return Ok(());
    }
    cfg.set_str("user.name", name).es()?;
    cfg.set_str("user.email", email).es()
}

/// Short text summary of the repository, used by the CLI and the AI prompts.
pub fn status_text(path: &Path) -> GResult<String> {
    let s = load(path, 30)?;
    let mut out = format!("Repository: {}\nBranch: {}\n", s.path.display(), s.head_name);
    if let Some(b) = s.current_upstream() {
        if let Some(u) = &b.upstream {
            out.push_str(&format!("Upstream: {u} (ahead {}, behind {})\n", b.ahead, b.behind));
        }
    }
    if s.wip.is_empty() {
        out.push_str("Working tree clean\n");
    } else {
        for f in &s.wip.staged {
            out.push_str(&format!("staged    {} {}\n", f.status, f.path));
        }
        for f in &s.wip.unstaged {
            out.push_str(&format!("unstaged  {} {}\n", f.status, f.path));
        }
        for f in &s.wip.conflicted {
            out.push_str(&format!("conflict  {f}\n"));
        }
    }
    Ok(out)
}

pub fn log_text(path: &Path, n: usize) -> GResult<String> {
    let s = load(path, n)?;
    Ok(s.commits
        .iter()
        .map(|c| format!("{} {} | {} | {}", c.short, crate::util::format_date(c.time), c.author, c.summary))
        .collect::<Vec<_>>()
        .join("\n"))
}

pub fn parse_oid(path: &Path, rev: &str) -> GResult<Oid> {
    let repo = open(path)?;
    let obj = repo.revparse_single(rev).es()?;
    let id = obj.peel_to_commit().es()?.id();
    Ok(id)
}
