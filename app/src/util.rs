//! Small helpers shared by the app, the CLI and the MCP server.

use std::path::{Path, PathBuf};
use std::process::Command;

/// A process command that never flashes a console window on Windows.
pub fn quiet_command<S: AsRef<std::ffi::OsStr>>(program: S) -> Command {
    #[allow(unused_mut)]
    let mut c = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000);
    }
    c
}

/// Look for an executable on PATH.
pub fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    let exts: Vec<String> = if cfg!(windows) {
        std::env::var("PATHEXT")
            .unwrap_or_else(|_| ".EXE;.CMD;.BAT".into())
            .split(';')
            .map(|s| s.to_lowercase())
            .collect()
    } else {
        vec![String::new()]
    };
    for dir in std::env::split_paths(&path) {
        for ext in &exts {
            let cand = dir.join(format!("{name}{ext}"));
            if cand.is_file() {
                return Some(cand);
            }
        }
        let cand = dir.join(name);
        if cand.is_file() && !cfg!(windows) {
            return Some(cand);
        }
    }
    None
}

/// "5 years ago", "11 hours ago"...
pub fn relative_time(ts: i64) -> String {
    let now = chrono::Utc::now().timestamp();
    let d = (now - ts).max(0);
    let (n, unit) = if d < 60 {
        return crate::i18n::t("time.now");
    } else if d < 3600 {
        (d / 60, "minute")
    } else if d < 86_400 {
        (d / 3600, "hour")
    } else if d < 86_400 * 30 {
        (d / 86_400, "day")
    } else if d < 86_400 * 365 {
        (d / (86_400 * 30), "month")
    } else {
        (d / (86_400 * 365), "year")
    };
    let key = if n == 1 { format!("time.{unit}") } else { format!("time.{unit}s") };
    crate::i18n::tf(&key, &[&n.to_string()])
}

pub fn format_date(ts: i64) -> String {
    use chrono::TimeZone;
    match chrono::Local.timestamp_opt(ts, 0) {
        chrono::LocalResult::Single(d) => d.format("%d/%m/%Y %H:%M").to_string(),
        _ => String::new(),
    }
}

pub fn initials(name: &str) -> String {
    let mut s: String = name
        .split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase();
    if s.is_empty() {
        s = "?".into();
    }
    s
}

pub fn hash_str(s: &str) -> u32 {
    let mut h: u32 = 2166136261;
    for b in s.bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(16777619);
    }
    h
}

pub fn open_path(p: &Path) {
    let _ = open_detached(p.as_os_str());
}

pub fn open_url(url: &str) {
    let _ = open_detached(std::ffi::OsStr::new(url));
}

fn open_detached(target: &std::ffi::OsStr) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        quiet_command("explorer").arg(target).spawn().map(|_| ())
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(target).spawn().map(|_| ())
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Command::new("xdg-open").arg(target).spawn().map(|_| ())
    }
}

/// Open a terminal window in a folder.
pub fn open_terminal(dir: &Path, custom: &str) -> Result<(), String> {
    if !custom.trim().is_empty() {
        let mut parts = custom.split_whitespace();
        let prog = parts.next().unwrap_or_default();
        return Command::new(prog)
            .args(parts)
            .current_dir(dir)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string());
    }
    #[cfg(windows)]
    {
        if which("wt").is_some() {
            return Command::new("wt").arg("-d").arg(dir).spawn().map(|_| ()).map_err(|e| e.to_string());
        }
        Command::new("cmd")
            .args(["/C", "start", "cmd", "/K"])
            .current_dir(dir)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open").args(["-a", "Terminal"]).arg(dir).spawn().map(|_| ()).map_err(|e| e.to_string())
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        for t in ["x-terminal-emulator", "gnome-terminal", "konsole", "xfce4-terminal", "xterm"] {
            if which(t).is_some() {
                return Command::new(t).current_dir(dir).spawn().map(|_| ()).map_err(|e| e.to_string());
            }
        }
        Err("no terminal found".into())
    }
}

/// Open a folder in an editor (custom command, or VS Code if present).
pub fn open_editor(dir: &Path, custom: &str) -> Result<(), String> {
    let cmd = if custom.trim().is_empty() {
        if which("code").is_some() {
            "code".to_string()
        } else {
            open_path(dir);
            return Ok(());
        }
    } else {
        custom.to_string()
    };
    let mut parts = cmd.split_whitespace();
    let prog = parts.next().unwrap_or_default().to_string();
    let mut c = if cfg!(windows) {
        let mut c = quiet_command("cmd");
        c.arg("/C").arg(&prog);
        c
    } else {
        Command::new(&prog)
    };
    c.args(parts).arg(dir).spawn().map(|_| ()).map_err(|e| e.to_string())
}

/// Run a shell command line in a folder and return its combined output.
pub fn run_shell(dir: &Path, line: &str, extra_path: Option<&Path>) -> String {
    let mut c = if cfg!(windows) {
        let mut c = quiet_command("cmd");
        c.arg("/C").arg(line);
        c
    } else {
        let mut c = Command::new("sh");
        c.arg("-c").arg(line);
        c
    };
    if let Some(p) = extra_path {
        let mut paths = vec![p.to_path_buf()];
        if let Some(old) = std::env::var_os("PATH") {
            paths.extend(std::env::split_paths(&old));
        }
        if let Ok(j) = std::env::join_paths(paths) {
            c.env("PATH", j);
        }
    }
    c.current_dir(dir).env("GIT_TERMINAL_PROMPT", "0").stdin(std::process::Stdio::null());
    match c.output() {
        Ok(o) => {
            let mut s = String::from_utf8_lossy(&o.stdout).to_string();
            let e = String::from_utf8_lossy(&o.stderr);
            if !e.is_empty() {
                if !s.is_empty() && !s.ends_with('\n') {
                    s.push('\n');
                }
                s.push_str(&e);
            }
            s
        }
        Err(e) => format!("error: {e}"),
    }
}

pub fn human_size(n: u64) -> String {
    let f = n as f64;
    if f > 1024.0 * 1024.0 {
        format!("{:.1} MB", f / 1024.0 / 1024.0)
    } else if f > 1024.0 {
        format!("{:.0} KB", f / 1024.0)
    } else {
        format!("{n} B")
    }
}
