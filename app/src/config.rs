//! Settings stored as JSON in the user's config folder.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const APP_NAME: &str = "GitCerberoCraft";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct RecentRepo {
    pub name: String,
    pub path: String,
}

impl Default for RecentRepo {
    fn default() -> Self {
        Self { name: String::new(), path: String::new() }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Profile {
    pub name: String,
    pub user_name: String,
    pub user_email: String,
}

impl Default for Profile {
    fn default() -> Self {
        Self { name: "Default Profile".into(), user_name: String::new(), user_email: String::new() }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AiMode {
    Off,
    Api,
    Acp,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ApiKind {
    OpenAiCompatible,
    Anthropic,
    Ollama,
}

impl ApiKind {
    pub fn label(&self) -> &'static str {
        match self {
            ApiKind::OpenAiCompatible => "OpenAI-compatible",
            ApiKind::Anthropic => "Anthropic",
            ApiKind::Ollama => "Ollama (local)",
        }
    }
    pub fn default_url(&self) -> &'static str {
        match self {
            ApiKind::OpenAiCompatible => "https://api.openai.com/v1",
            ApiKind::Anthropic => "https://api.anthropic.com",
            ApiKind::Ollama => "http://localhost:11434",
        }
    }
    pub fn default_model(&self) -> &'static str {
        match self {
            ApiKind::OpenAiCompatible => "gpt-4o-mini",
            ApiKind::Anthropic => "claude-sonnet-5-5",
            ApiKind::Ollama => "llama3.1",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AiSettings {
    pub mode: AiMode,
    pub api_kind: ApiKind,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    /// Command that starts an Agent Client Protocol agent (stdio JSON-RPC).
    pub acp_command: String,
    pub acp_args: String,
    pub system_prompt: String,
    pub max_diff_chars: usize,
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            mode: AiMode::Off,
            api_kind: ApiKind::Anthropic,
            base_url: ApiKind::Anthropic.default_url().into(),
            api_key: String::new(),
            model: ApiKind::Anthropic.default_model().into(),
            acp_command: "npx".into(),
            acp_args: "@zed-industries/claude-code-acp".into(),
            system_prompt: "You are a senior software engineer helping inside the GitCerberoCraft Git client. Answer concisely, in the user's language, with concrete advice.".into(),
            max_diff_chars: 60_000,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PullMode {
    FetchAll,
    FastForwardIfPossible,
    FastForwardOnly,
    Rebase,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// "auto" follows the system language; otherwise a language code (file name).
    pub language: String,
    pub onboarding_done: bool,
    pub profiles: Vec<Profile>,
    pub active_profile: usize,
    pub recent_repos: Vec<RecentRepo>,
    pub open_tabs: Vec<String>,
    pub zoom: f32,
    pub git_path_override: String,
    pub prefer_git_cli: bool,
    pub ai: AiSettings,
    pub external_editor: String,
    pub external_terminal: String,
    pub default_clone_dir: String,
    pub default_branch: String,
    pub auto_fetch_minutes: u32,
    pub pull_mode: PullMode,
    pub graph_max_commits: usize,
    pub show_avatars_initials: bool,
    pub left_panel_width: f32,
    pub github_token: String,
    pub gitlab_token: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: "auto".into(),
            onboarding_done: false,
            profiles: vec![Profile::default()],
            active_profile: 0,
            recent_repos: vec![],
            open_tabs: vec![],
            zoom: 1.0,
            git_path_override: String::new(),
            prefer_git_cli: true,
            ai: AiSettings::default(),
            external_editor: String::new(),
            external_terminal: String::new(),
            default_clone_dir: dirs::document_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            default_branch: "main".into(),
            auto_fetch_minutes: 0,
            pull_mode: PullMode::FastForwardIfPossible,
            graph_max_commits: 2000,
            show_avatars_initials: true,
            left_panel_width: 206.0,
            github_token: String::new(),
            gitlab_token: String::new(),
        }
    }
}

pub fn config_dir() -> PathBuf {
    if let Ok(p) = std::env::var("GITCERBEROCRAFT_HOME") {
        return PathBuf::from(p);
    }
    dirs::config_dir().unwrap_or_else(std::env::temp_dir).join(APP_NAME)
}

pub fn data_dir() -> PathBuf {
    if let Ok(p) = std::env::var("GITCERBEROCRAFT_HOME") {
        return PathBuf::from(p).join("data");
    }
    dirs::data_local_dir().unwrap_or_else(std::env::temp_dir).join(APP_NAME)
}

impl Settings {
    pub fn path() -> PathBuf {
        config_dir().join("settings.json")
    }

    pub fn load() -> Self {
        let p = Self::path();
        let mut s: Settings = std::fs::read_to_string(&p)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        if s.profiles.is_empty() {
            s.profiles.push(Profile::default());
        }
        if s.active_profile >= s.profiles.len() {
            s.active_profile = 0;
        }
        if s.zoom < 0.5 || s.zoom > 2.5 {
            s.zoom = 1.0;
        }
        s
    }

    pub fn save(&self) -> std::io::Result<()> {
        let p = Self::path();
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d)?;
        }
        std::fs::write(p, serde_json::to_string_pretty(self).unwrap_or_default())
    }

    pub fn profile(&self) -> &Profile {
        &self.profiles[self.active_profile.min(self.profiles.len() - 1)]
    }

    pub fn profile_mut(&mut self) -> &mut Profile {
        let i = self.active_profile.min(self.profiles.len() - 1);
        &mut self.profiles[i]
    }

    pub fn add_recent(&mut self, path: &str) {
        let norm = path.trim_end_matches(['/', '\\']).to_string();
        self.recent_repos.retain(|r| r.path != norm);
        let name = std::path::Path::new(&norm)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| norm.clone());
        self.recent_repos.insert(0, RecentRepo { name, path: norm });
        self.recent_repos.truncate(20);
    }
}
