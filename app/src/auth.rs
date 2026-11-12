//! HTTPS sign-in for GitHub and GitLab with the tokens saved in Preferences.
//!
//! The Git program normally asks the system's credential manager, which may hold
//! a password or token of a different account (a common cause of "403" on push).
//! When a token is saved, it is handed to Git for that host only, through
//! environment variables (never on the command line), and the credential
//! manager is skipped for that host so the right account is always used.
//!
//! When the server still refuses, [`explain_refusal`] asks the host's API what is
//! wrong with the token (wrong account, missing permission, expired...).

use crate::i18n::{t, tf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Host {
    GitHub,
    GitLab,
}

/// Which provider an URL points to, and its `https://host/` prefix.
pub fn host_of(url: &str) -> Option<(Host, String)> {
    let rest = url.strip_prefix("https://").or_else(|| url.strip_prefix("http://"))?;
    let scheme = if url.starts_with("https://") { "https" } else { "http" };
    let hostport = rest.split('/').next()?;
    let host = hostport.rsplit('@').next()?.to_ascii_lowercase();
    let kind = if host.contains("github") {
        Host::GitHub
    } else if host.contains("gitlab") {
        Host::GitLab
    } else {
        return None;
    };
    Some((kind, format!("{scheme}://{host}/")))
}

/// Username Git should send with a token for this provider.
pub fn token_user(h: Host) -> &'static str {
    match h {
        Host::GitHub => "x-access-token",
        Host::GitLab => "oauth2",
    }
}

pub fn clean_token(t: &str) -> String {
    t.trim().trim_matches('"').trim().to_string()
}

fn b64(data: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        out.push(A[(n >> 18) as usize & 63] as char);
        out.push(A[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 { A[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if c.len() > 2 { A[n as usize & 63] as char } else { '=' });
    }
    out
}

/// Git configuration (key, value) pairs that sign in to each https host found
/// in `urls` with the matching saved token.
pub fn git_auth_config(urls: &[String], github: &str, gitlab: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut done: Vec<String> = Vec::new();
    for u in urls {
        let Some((kind, prefix)) = host_of(u) else { continue };
        if done.contains(&prefix) {
            continue;
        }
        let tok = clean_token(match kind {
            Host::GitHub => github,
            Host::GitLab => gitlab,
        });
        if tok.is_empty() {
            continue;
        }
        done.push(prefix.clone());
        let basic = b64(format!("{}:{}", token_user(kind), tok).as_bytes());
        // Skip the system credential manager for this host (it may hold another account)...
        out.push((format!("credential.{}.helper", prefix.trim_end_matches('/')), String::new()));
        // ...and send the token with every request to it.
        out.push((format!("http.{prefix}.extraHeader"), format!("Authorization: Basic {basic}")));
    }
    out
}

/// Turn configuration pairs into environment variables Git reads (Git 2.31+).
pub fn as_env(cfg: &[(String, String)]) -> Vec<(String, String)> {
    if cfg.is_empty() {
        return Vec::new();
    }
    let mut env = vec![("GIT_CONFIG_COUNT".to_string(), cfg.len().to_string())];
    for (i, (k, v)) in cfg.iter().enumerate() {
        env.push((format!("GIT_CONFIG_KEY_{i}"), k.clone()));
        env.push((format!("GIT_CONFIG_VALUE_{i}"), v.clone()));
    }
    env
}

/// Does this error text look like the server refused the sign-in or the write?
pub fn is_auth_refusal(err: &str) -> bool {
    let e = err.to_ascii_lowercase();
    ["403", "401", "permission to", "denied", "authentication failed", "could not read username", "unauthorized", "forbidden", "terminal prompts disabled", "invalid username or password", "no credentials"]
        .iter()
        .any(|k| e.contains(k))
}

fn owner_repo(url: &str) -> Option<String> {
    let rest = url.split_once("://").map(|x| x.1).unwrap_or(url);
    let path = rest.split_once('/').map(|x| x.1)?;
    let p = path.trim_end_matches('/').trim_end_matches(".git");
    if p.is_empty() {
        None
    } else {
        Some(p.to_string())
    }
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new().timeout(std::time::Duration::from_secs(15)).user_agent("GitCerberoCraft").build()
}

/// What the host says about a token: the account it belongs to and a note on its rights.
#[derive(Clone, Debug, Default)]
pub struct TokenInfo {
    pub login: String,
    pub scopes: Option<String>,
    pub fine_grained: bool,
}

/// Check a GitHub token: who it belongs to and which scopes it has.
pub fn github_whoami(token: &str) -> Result<TokenInfo, String> {
    let tok = clean_token(token);
    if tok.is_empty() {
        return Err(t("auth.no_token"));
    }
    match agent().get("https://api.github.com/user").set("Authorization", &format!("Bearer {tok}")).set("Accept", "application/vnd.github+json").call() {
        Ok(r) => {
            let scopes = r.header("x-oauth-scopes").map(|s| s.to_string());
            let v: serde_json::Value = r.into_json().unwrap_or_default();
            Ok(TokenInfo {
                login: v["login"].as_str().unwrap_or("?").to_string(),
                fine_grained: scopes.is_none() || tok.starts_with("github_pat_"),
                scopes,
            })
        }
        Err(ureq::Error::Status(401, _)) => Err(t("auth.token_invalid")),
        Err(e) => Err(e.to_string()),
    }
}

/// Check a GitLab token: who it belongs to.
pub fn gitlab_whoami(token: &str, base: &str) -> Result<TokenInfo, String> {
    let tok = clean_token(token);
    if tok.is_empty() {
        return Err(t("auth.no_token"));
    }
    let url = format!("{}api/v4/user", base);
    match agent().get(&url).set("PRIVATE-TOKEN", &tok).call() {
        Ok(r) => {
            let v: serde_json::Value = r.into_json().unwrap_or_default();
            Ok(TokenInfo { login: v["username"].as_str().unwrap_or("?").to_string(), scopes: None, fine_grained: false })
        }
        Err(ureq::Error::Status(401, _)) => Err(t("auth.token_invalid")),
        Err(e) => Err(e.to_string()),
    }
}

/// Short sentence for the "Test token" button.
pub fn describe(info: &TokenInfo) -> String {
    match (&info.scopes, info.fine_grained) {
        (Some(s), false) => tf("auth.token_ok_classic", &[&info.login, if s.is_empty() { "-" } else { s }]),
        _ => tf("auth.token_ok", &[&info.login]),
    }
}

/// The server refused a push/pull/fetch: find out why and say it plainly.
/// `remote_url` is the remote's address; `raw` is Git's own message.
pub fn explain_refusal(remote_url: &str, github: &str, gitlab: &str, raw: &str) -> String {
    let mut lines: Vec<String> = vec![raw.lines().filter(|l| !l.trim().is_empty()).take(3).collect::<Vec<_>>().join("\n")];
    let Some((kind, prefix)) = host_of(remote_url) else {
        if remote_url.starts_with("git@") || remote_url.starts_with("ssh://") {
            lines.push(t("auth.ssh_remote"));
        }
        return lines.join("\n\n");
    };
    let repo = owner_repo(remote_url).unwrap_or_default();
    match kind {
        Host::GitHub => {
            let tok = clean_token(github);
            if tok.is_empty() {
                lines.push(t("auth.no_token_github"));
                return lines.join("\n\n");
            }
            let info = match github_whoami(&tok) {
                Ok(i) => i,
                Err(e) => {
                    lines.push(e);
                    return lines.join("\n\n");
                }
            };
            // Can this account write to the repository?
            let url = format!("https://api.github.com/repos/{repo}");
            let resp = agent().get(&url).set("Authorization", &format!("Bearer {tok}")).set("Accept", "application/vnd.github+json").call();
            match resp {
                Ok(r) => {
                    let sso = r.header("x-github-sso").map(|s| s.to_string());
                    let v: serde_json::Value = r.into_json().unwrap_or_default();
                    let can_push = v["permissions"]["push"].as_bool();
                    let private = v["private"].as_bool().unwrap_or(false);
                    if let Some(s) = sso {
                        lines.push(tf("auth.sso", &[&s]));
                    }
                    if can_push == Some(false) {
                        lines.push(tf("auth.no_write", &[&info.login, &repo]));
                    } else if let (Some(sc), false) = (&info.scopes, info.fine_grained) {
                        let has = sc.split(',').map(|x| x.trim()).any(|x| x == "repo" || (x == "public_repo" && !private));
                        if !has {
                            lines.push(tf("auth.missing_scope", &[&info.login, if sc.is_empty() { "-" } else { sc }]));
                        } else if raw.to_ascii_lowercase().contains("workflow") && !sc.contains("workflow") {
                            lines.push(t("auth.workflow_scope"));
                        } else {
                            lines.push(tf("auth.looks_ok", &[&info.login]));
                        }
                    } else {
                        lines.push(tf("auth.fine_grained", &[&info.login, &repo]));
                    }
                }
                Err(ureq::Error::Status(404, _)) => lines.push(tf("auth.repo_hidden", &[&info.login, &repo])),
                Err(ureq::Error::Status(403, r)) => {
                    let sso = r.header("x-github-sso").map(|s| s.to_string()).unwrap_or_default();
                    lines.push(if sso.is_empty() { tf("auth.fine_grained", &[&info.login, &repo]) } else { tf("auth.sso", &[&sso]) });
                }
                Err(e) => lines.push(e.to_string()),
            }
        }
        Host::GitLab => {
            let tok = clean_token(gitlab);
            if tok.is_empty() {
                lines.push(t("auth.no_token_gitlab"));
            } else {
                match gitlab_whoami(&tok, &prefix) {
                    Ok(i) => lines.push(tf("auth.gitlab_scope", &[&i.login, &repo])),
                    Err(e) => lines.push(e),
                }
            }
        }
    }
    lines.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn base64_matches() {
        assert_eq!(b64(b"x-access-token:abc"), "eC1hY2Nlc3MtdG9rZW46YWJj");
        assert_eq!(b64(b"a"), "YQ==");
        assert_eq!(b64(b"ab"), "YWI=");
    }
    #[test]
    fn hosts() {
        assert_eq!(host_of("https://bob@github.com/a/b.git").unwrap().1, "https://github.com/");
        assert!(host_of("git@github.com:a/b.git").is_none());
        assert_eq!(owner_repo("https://github.com/a/b.git").unwrap(), "a/b");
        let c = git_auth_config(&["https://github.com/a/b".into()], " tok ", "");
        assert_eq!(c.len(), 2);
    }
}
