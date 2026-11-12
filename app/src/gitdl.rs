//! Download Git for the current operating system, choosing the version.
//!
//! * Windows: the official "Git for Windows" releases (the files linked from
//!   git-scm.com): installer, portable or MinGit. MinGit/portable are unpacked
//!   into the app's own folder and used straight away.
//! * macOS / Linux: the official source releases listed on kernel.org (linked
//!   from git-scm.com), plus the package-manager command for the system.

use serde_json::Value;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct GitAsset {
    pub name: String,
    pub url: String,
    pub size: u64,
    pub kind: AssetKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssetKind {
    Installer,
    Portable,
    MinGit,
    Source,
}

impl AssetKind {
    pub fn key(&self) -> &'static str {
        match self {
            AssetKind::Installer => "gitdl.kind_installer",
            AssetKind::Portable => "gitdl.kind_portable",
            AssetKind::MinGit => "gitdl.kind_mingit",
            AssetKind::Source => "gitdl.kind_source",
        }
    }
}

#[derive(Clone, Debug)]
pub struct GitRelease {
    pub version: String,
    pub date: String,
    pub prerelease: bool,
    pub assets: Vec<GitAsset>,
}

pub fn os_name() -> &'static str {
    if cfg!(windows) {
        "Windows"
    } else if cfg!(target_os = "macos") {
        "macOS"
    } else {
        "Linux"
    }
}

pub fn arch() -> &'static str {
    if cfg!(target_arch = "aarch64") {
        "arm64"
    } else if cfg!(target_arch = "x86") {
        "32-bit"
    } else {
        "64-bit"
    }
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(20))
        .user_agent(&format!("GitCerberoCraft/{}", crate::config::APP_VERSION))
        .build()
}

/// Versions available for this system, newest first.
pub fn list_releases() -> Result<Vec<GitRelease>, String> {
    if cfg!(windows) {
        list_windows()
    } else {
        list_source()
    }
}

fn list_windows() -> Result<Vec<GitRelease>, String> {
    let v: Value = agent()
        .get("https://api.github.com/repos/git-for-windows/git/releases?per_page=40")
        .call()
        .map_err(|e| e.to_string())?
        .into_json()
        .map_err(|e| e.to_string())?;
    let arch = arch();
    let mut out = Vec::new();
    for r in v.as_array().cloned().unwrap_or_default() {
        let tag = r["tag_name"].as_str().unwrap_or("");
        let version = tag.trim_start_matches('v').replace(".windows.1", "").replace(".windows.", ".w");
        let mut assets = Vec::new();
        for a in r["assets"].as_array().cloned().unwrap_or_default() {
            let name = a["name"].as_str().unwrap_or("").to_string();
            let url = a["browser_download_url"].as_str().unwrap_or("").to_string();
            let size = a["size"].as_u64().unwrap_or(0);
            if !name.contains(arch) || name.contains("busybox") || name.ends_with(".tar.bz2") {
                continue;
            }
            let kind = if name.starts_with("Git-") && name.ends_with(".exe") {
                AssetKind::Installer
            } else if name.starts_with("PortableGit-") {
                AssetKind::Portable
            } else if name.starts_with("MinGit-") && name.ends_with(".zip") {
                AssetKind::MinGit
            } else {
                continue;
            };
            assets.push(GitAsset { name, url, size, kind });
        }
        assets.sort_by_key(|a| a.kind as u8);
        if !assets.is_empty() {
            out.push(GitRelease {
                version,
                date: r["published_at"].as_str().unwrap_or("").chars().take(10).collect(),
                prerelease: r["prerelease"].as_bool().unwrap_or(false),
                assets,
            });
        }
    }
    Ok(out)
}

fn version_key(v: &str) -> Vec<u32> {
    v.split(|c: char| !c.is_ascii_digit()).filter_map(|x| x.parse().ok()).collect()
}

fn list_source() -> Result<Vec<GitRelease>, String> {
    let base = "https://mirrors.edge.kernel.org/pub/software/scm/git/";
    let html = agent().get(base).call().map_err(|e| e.to_string())?.into_string().map_err(|e| e.to_string())?;
    let mut versions: Vec<String> = Vec::new();
    for part in html.split("href=\"").skip(1) {
        let name = part.split('"').next().unwrap_or("");
        if let Some(v) = name.strip_prefix("git-").and_then(|n| n.strip_suffix(".tar.xz")) {
            if v.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) && !versions.contains(&v.to_string()) {
                versions.push(v.to_string());
            }
        }
    }
    versions.sort_by_key(|v| std::cmp::Reverse(version_key(v)));
    Ok(versions
        .into_iter()
        .take(60)
        .map(|v| GitRelease {
            prerelease: v.contains("rc"),
            assets: vec![GitAsset {
                name: format!("git-{v}.tar.xz"),
                url: format!("{base}git-{v}.tar.xz"),
                size: 0,
                kind: AssetKind::Source,
            }],
            version: v,
            date: String::new(),
        })
        .collect())
}

/// Package-manager command for this system (macOS / Linux).
pub fn package_command() -> Option<String> {
    if cfg!(target_os = "macos") {
        return Some("brew install git".into());
    }
    if cfg!(windows) {
        return Some("winget install --id Git.Git -e --source winget".into());
    }
    let pm = [
        ("apt-get", "sudo apt-get install -y git"),
        ("dnf", "sudo dnf install -y git"),
        ("yum", "sudo yum install -y git"),
        ("pacman", "sudo pacman -S --noconfirm git"),
        ("zypper", "sudo zypper install -y git"),
        ("apk", "sudo apk add git"),
    ];
    pm.iter().find(|(b, _)| crate::util::which(b).is_some()).map(|(_, c)| c.to_string())
}

pub fn downloads_dir() -> PathBuf {
    crate::config::data_dir().join("downloads")
}

/// Download a file with progress (0..1). Returns the saved path.
pub fn download(asset: &GitAsset, progress: &mut dyn FnMut(f32)) -> Result<PathBuf, String> {
    let dir = downloads_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let dest = dir.join(&asset.name);
    let resp = agent().get(&asset.url).call().map_err(|e| e.to_string())?;
    let total = resp
        .header("Content-Length")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(asset.size)
        .max(1);
    let mut reader = resp.into_reader();
    let tmp = dest.with_extension("part");
    let mut f = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; 64 * 1024];
    let mut done: u64 = 0;
    loop {
        let n = reader.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        f.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        done += n as u64;
        progress((done as f32 / total as f32).min(1.0));
    }
    drop(f);
    let _ = std::fs::remove_file(&dest);
    std::fs::rename(&tmp, &dest).map_err(|e| e.to_string())?;
    Ok(dest)
}

/// Install what was downloaded. Returns a message for the person.
pub fn install(asset: &GitAsset, file: &Path) -> Result<String, String> {
    let target = crate::git::managed_git_dir();
    match asset.kind {
        AssetKind::Installer => {
            std::process::Command::new(file).spawn().map_err(|e| e.to_string())?;
            Ok("installer_started".into())
        }
        AssetKind::MinGit => {
            let _ = std::fs::remove_dir_all(&target);
            std::fs::create_dir_all(&target).map_err(|e| e.to_string())?;
            let f = std::fs::File::open(file).map_err(|e| e.to_string())?;
            let mut zip = zip::ZipArchive::new(f).map_err(|e| e.to_string())?;
            zip.extract(&target).map_err(|e| e.to_string())?;
            Ok("installed".into())
        }
        AssetKind::Portable => {
            let _ = std::fs::remove_dir_all(&target);
            std::fs::create_dir_all(&target).map_err(|e| e.to_string())?;
            let status = std::process::Command::new(file)
                .arg(format!("-o{}", target.display()))
                .arg("-y")
                .status()
                .map_err(|e| e.to_string())?;
            if status.success() {
                Ok("installed".into())
            } else {
                Err("The portable package could not be unpacked".into())
            }
        }
        AssetKind::Source => {
            crate::util::open_path(file.parent().unwrap_or(file));
            Ok("source_saved".into())
        }
    }
}

pub fn remove_managed() -> Result<(), String> {
    let d = crate::git::managed_git_dir();
    if d.exists() {
        std::fs::remove_dir_all(d).map_err(|e| e.to_string())?;
    }
    Ok(())
}
