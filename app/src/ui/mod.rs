//! The desktop app: tabs, background jobs, refresh, the screenshot and
//! self-test modes. Window chrome is in chrome.rs.

pub mod actions;
pub mod chrome;
pub mod home;
pub mod repo;
pub mod repo_graph;
pub mod repo_left;
pub mod repo_panels;
pub mod screens;
pub mod screens_prefs;
pub mod selftest;
pub mod theme;

use eframe::egui;
use egui_phosphor::regular as ic;
use gcc_core::ai::AiWorker;
use gcc_core::config::{AiMode, Settings};
use gcc_core::git::{self, Engine};
use gcc_core::gitdl::GitRelease;
use gcc_core::i18n::{self, t, tf};
use repo::RepoTab;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use theme::*;

pub enum TabKind {
    Launchpad,
    ReleaseNotes,
    Home,
    Guide,
    Repo(Box<RepoTab>),
}

pub struct Tab {
    pub id: u64,
    pub kind: TabKind,
}

impl Tab {
    pub fn title(&self) -> String {
        match &self.kind {
            TabKind::Launchpad => t("tabs.launchpad"),
            TabKind::ReleaseNotes => t("tabs.release_notes"),
            TabKind::Home => t("tabs.new_tab"),
            TabKind::Guide => t("tabs.guide"),
            TabKind::Repo(r) => r.name(),
        }
    }
    pub fn icon(&self) -> &'static str {
        match &self.kind {
            TabKind::Launchpad => ic::ROCKET_LAUNCH,
            TabKind::ReleaseNotes => ic::CLIPBOARD_TEXT,
            TabKind::Home => "",
            TabKind::Guide => ic::BOOK_OPEN,
            TabKind::Repo(_) => ic::GIT_BRANCH,
        }
    }
    pub fn key(&self) -> String {
        match &self.kind {
            TabKind::Launchpad => "#launchpad".into(),
            TabKind::ReleaseNotes => "#release".into(),
            TabKind::Home => "#home".into(),
            TabKind::Guide => "#guide".into(),
            TabKind::Repo(r) => r.path.display().to_string(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Screen {
    Main,
    Onboarding(u8),
    Clone,
    Prefs,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PrefPage {
    General,
    Profiles,
    Language,
    Ai,
    Git,
    Integrations,
    Mcp,
    Shortcuts,
}

#[derive(Clone, Debug)]
pub enum InputKind {
    NewBranch { tab: u64, at: Option<git2::Oid> },
    RenameBranch { tab: u64, old: String },
    Tag { tab: u64, oid: git2::Oid },
    Stash { tab: u64 },
    AddRemote { tab: u64 },
}

#[derive(Clone, Debug)]
pub enum ConfirmAction {
    DiscardAll(u64),
    Discard(u64, Vec<String>),
    ResetHard(u64, git2::Oid),
    DeleteBranch(u64, String),
    ForcePush(u64),
    DropStash(u64, usize),
    RemoveRemote(u64, String),
}

pub struct InitForm {
    pub provider: usize,
    pub name: String,
    pub parent: String,
    pub branch: String,
    pub gitignore: String,
    pub license: String,
    pub lfs: bool,
    pub remote: String,
}

pub enum Modal {
    Init(InitForm),
    Input { title: String, label: String, value: String, label2: String, value2: String, kind: InputKind },
    Confirm { title: String, text: String, action: ConfirmAction },
    About,
    Shortcuts,
}

pub struct Toast {
    pub text: String,
    pub error: bool,
    pub until: f64,
}

pub struct Job {
    pub id: u64,
    pub title: String,
    pub progress: Arc<Mutex<f32>>,
}

pub enum JobOut {
    Msg(String),
    Cloned(PathBuf),
    Releases(Vec<GitRelease>),
    GitInstalled(String),
    Models(Vec<String>),
    RepoList(Vec<(String, String)>),
}

pub struct JobDone {
    pub id: u64,
    pub tab: Option<u64>,
    pub title: String,
    pub result: Result<JobOut, String>,
}

#[derive(Default)]
pub struct CloneForm {
    pub provider: usize,
    pub url: String,
    pub parent: String,
    pub shallow: bool,
    pub sparse: bool,
    pub owner: String,
    pub repos: Vec<(String, String)>,
    pub loading: bool,
}

#[derive(Default)]
pub struct GitDlState {
    pub releases: Vec<GitRelease>,
    pub loading: bool,
    pub selected: usize,
    pub asset: usize,
    pub error: Option<String>,
    pub show_pre: bool,
    pub busy: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AiPurpose {
    Chat,
    CommitMessage,
}

pub struct App {
    pub ctx: egui::Context,
    pub settings: Settings,
    pub tabs: Vec<Tab>,
    pub active: usize,
    pub next_id: u64,
    pub closed: Vec<String>,
    pub screen: Screen,
    pub pref_page: PrefPage,
    pub modal: Option<Modal>,
    pub toasts: Vec<Toast>,
    pub notifications: Vec<(String, bool)>,
    pub unread: usize,
    pub jobs: Vec<Job>,
    pub job_tx: Sender<JobDone>,
    pub job_rx: Receiver<JobDone>,
    pub job_seq: u64,
    pub engine: Engine,
    pub git_version: Option<String>,
    pub ai: AiWorker,
    pub ai_tab: Option<u64>,
    pub ai_purpose: AiPurpose,
    pub lang_poll: f64,
    pub clone_form: CloneForm,
    pub gitdl: GitDlState,
    pub onboard_name: String,
    pub onboard_email: String,
    pub screenshot: Option<PathBuf>,
    pub shot_requested: bool,
    pub selftest: Option<selftest::SelfTest>,
    pub frame: u64,
    pub status_drawn: bool,
    pub ai_models: Vec<String>,
    pub persist: bool,
    pub profile_open: bool,
    pub bell_open: bool,
    pub tabs_open: bool,
}

pub struct Options {
    pub screenshot: Option<PathBuf>,
    pub selftest: Option<PathBuf>,
    pub open: Option<PathBuf>,
}

pub fn make_engine(s: &Settings) -> Engine {
    let mut e = Engine::detect(&s.git_path_override, s.prefer_git_cli);
    let p = s.profile();
    e.user_name = p.user_name.clone();
    e.user_email = p.user_email.clone();
    e.token_github = s.github_token.clone();
    e.token_gitlab = s.gitlab_token.clone();
    e
}

impl App {
    pub fn new(cc: &eframe::CreationContext, opts: Options) -> Self {
        theme::setup(&cc.egui_ctx);
        let automated = opts.screenshot.is_some() || opts.selftest.is_some();
        let mut settings = if automated { Settings::default() } else { Settings::load() };
        if automated {
            settings.onboarding_done = true;
            settings.language = "en".into();
        }
        i18n::init(&settings.language);
        if let Ok(mut g) = i18n::global().write() {
            g.set_language(&settings.language);
        }
        if settings.profile().user_name.is_empty() && !automated {
            if let Some(n) = git::read_config(None, "user.name") {
                settings.profile_mut().user_name = n;
            }
            if let Some(m) = git::read_config(None, "user.email") {
                settings.profile_mut().user_email = m;
            }
        }
        let engine = make_engine(&settings);
        let git_version = engine.cli.as_ref().and_then(|p| git::git_version(p));
        let (job_tx, job_rx) = channel();
        let mut app = Self {
            ctx: cc.egui_ctx.clone(),
            onboard_name: settings.profile().user_name.clone(),
            onboard_email: settings.profile().user_email.clone(),
            screen: if settings.onboarding_done { Screen::Main } else { Screen::Onboarding(0) },
            settings,
            tabs: vec![],
            active: 0,
            next_id: 1,
            closed: vec![],
            pref_page: PrefPage::General,
            modal: None,
            toasts: vec![],
            notifications: vec![],
            unread: 0,
            jobs: vec![],
            job_tx,
            job_rx,
            job_seq: 0,
            engine,
            git_version,
            ai: AiWorker::start(),
            ai_tab: None,
            ai_purpose: AiPurpose::Chat,
            lang_poll: 0.0,
            clone_form: CloneForm::default(),
            gitdl: GitDlState::default(),
            screenshot: opts.screenshot,
            shot_requested: false,
            selftest: opts.selftest.map(selftest::SelfTest::new),
            frame: 0,
            status_drawn: false,
            ai_models: vec![],
            persist: !automated,
            profile_open: false,
            bell_open: false,
            tabs_open: false,
        };
        app.clone_form.parent = app.settings.default_clone_dir.clone();
        app.ctx.set_zoom_factor(app.settings.zoom);
        app.add_tab(TabKind::Launchpad, false);
        let restore: Vec<String> = if automated { vec![] } else { app.settings.open_tabs.clone() };
        for p in restore {
            let pb = PathBuf::from(&p);
            if git::is_repo(&pb) {
                app.open_repo_tab(pb, false);
            }
        }
        app.add_tab(TabKind::ReleaseNotes, false);
        app.add_tab(TabKind::Home, true);
        if let Some(p) = opts.open {
            app.open_repo(p);
        }
        if app.engine.cli.is_none() && app.persist {
            app.notify(t("notify.git_missing"), false);
        }
        app
    }

    pub fn add_tab(&mut self, kind: TabKind, focus: bool) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.tabs.push(Tab { id, kind });
        if focus {
            self.active = self.tabs.len() - 1;
            self.screen = if self.screen == Screen::Prefs { Screen::Main } else { self.screen };
        }
        id
    }

    pub fn open_repo_tab(&mut self, path: PathBuf, focus: bool) -> u64 {
        if let Some(i) = self.tabs.iter().position(|t| matches!(&t.kind, TabKind::Repo(r) if r.path == path)) {
            if focus {
                self.active = i;
            }
            return self.tabs[i].id;
        }
        let mut r = RepoTab::new(path);
        r.reload(self.settings.graph_max_commits);
        let pos = self
            .tabs
            .iter()
            .position(|t| matches!(t.kind, TabKind::ReleaseNotes | TabKind::Home))
            .unwrap_or(self.tabs.len());
        let id = self.next_id;
        self.next_id += 1;
        self.tabs.insert(pos, Tab { id, kind: TabKind::Repo(Box::new(r)) });
        if focus {
            self.active = pos;
        } else if self.active >= pos && self.tabs.len() > 1 {
            self.active += 1;
        }
        id
    }

    /// Open a repository folder, as the user would with "Open".
    pub fn open_repo(&mut self, path: PathBuf) -> Option<u64> {
        let root = git2::Repository::discover(&path)
            .ok()
            .and_then(|r| r.workdir().map(|w| w.to_path_buf()))
            .unwrap_or(path.clone());
        if !git::is_repo(&root) {
            self.toast(tf("notify.not_a_repo", &[&path.display().to_string()]), true);
            return None;
        }
        let clean = PathBuf::from(root.display().to_string().trim_end_matches(['/', '\\']).to_string());
        let was_home = matches!(self.tabs.get(self.active).map(|t| &t.kind), Some(TabKind::Home));
        let home_id = self.tabs.get(self.active).map(|t| t.id);
        let id = self.open_repo_tab(clean.clone(), true);
        if was_home {
            if let Some(i) = self.tabs.iter().position(|t| Some(t.id) == home_id) {
                self.tabs.remove(i);
                if self.active > i {
                    self.active -= 1;
                }
            }
        }
        self.screen = Screen::Main;
        self.settings.add_recent(&clean.display().to_string());
        self.save_settings();
        Some(id)
    }

    pub fn save_settings(&mut self) {
        if !self.persist {
            return;
        }
        self.settings.open_tabs = self
            .tabs
            .iter()
            .filter_map(|t| match &t.kind {
                TabKind::Repo(r) => Some(r.path.display().to_string()),
                _ => None,
            })
            .collect();
        let _ = self.settings.save();
    }

    pub fn toast(&mut self, text: impl Into<String>, error: bool) {
        let now = self.ctx.input(|i| i.time);
        let text = text.into();
        self.notifications.insert(0, (text.clone(), error));
        self.notifications.truncate(50);
        self.toasts.push(Toast { text, error, until: now + if error { 8.0 } else { 4.0 } });
    }

    pub fn notify(&mut self, text: impl Into<String>, error: bool) {
        self.notifications.insert(0, (text.into(), error));
        self.unread += 1;
    }

    pub fn ai_on(&self) -> bool {
        self.settings.ai.mode != AiMode::Off
    }

    pub fn spawn_job<F>(&mut self, title: String, tab: Option<u64>, f: F) -> u64
    where
        F: FnOnce(&mut dyn FnMut(f32, &str)) -> Result<JobOut, String> + Send + 'static,
    {
        self.job_seq += 1;
        let id = self.job_seq;
        let progress = Arc::new(Mutex::new(0.0f32));
        self.jobs.push(Job { id, title: title.clone(), progress: progress.clone() });
        let tx = self.job_tx.clone();
        let ctx = self.ctx.clone();
        std::thread::spawn(move || {
            let pc = progress.clone();
            let ctx2 = ctx.clone();
            let mut cb = move |f: f32, _l: &str| {
                if let Ok(mut p) = pc.lock() {
                    *p = f.clamp(0.0, 1.0);
                }
                ctx2.request_repaint();
            };
            let result = f(&mut cb);
            let _ = tx.send(JobDone { id, tab, title, result });
            ctx.request_repaint();
        });
        id
    }

    pub fn tab_mut(&mut self, id: u64) -> Option<&mut RepoTab> {
        self.tabs.iter_mut().find(|t| t.id == id).and_then(|t| match &mut t.kind {
            TabKind::Repo(r) => Some(r.as_mut()),
            _ => None,
        })
    }

    pub fn active_repo_id(&self) -> Option<u64> {
        self.tabs.get(self.active).and_then(|t| match t.kind {
            TabKind::Repo(_) => Some(t.id),
            _ => None,
        })
    }

    pub fn close_tab(&mut self, idx: usize) {
        if idx >= self.tabs.len() {
            return;
        }
        let tab = self.tabs.remove(idx);
        self.closed.push(tab.key());
        if self.tabs.is_empty() {
            self.add_tab(TabKind::Home, true);
        }
        if self.active >= self.tabs.len() {
            self.active = self.tabs.len() - 1;
        } else if self.active > idx {
            self.active -= 1;
        }
        self.save_settings();
    }

    pub fn reopen_closed(&mut self) {
        if let Some(k) = self.closed.pop() {
            match k.as_str() {
                "#launchpad" => {
                    self.add_tab(TabKind::Launchpad, true);
                }
                "#release" => {
                    self.add_tab(TabKind::ReleaseNotes, true);
                }
                "#home" => {
                    self.add_tab(TabKind::Home, true);
                }
                "#guide" => {
                    self.add_tab(TabKind::Guide, true);
                }
                p => {
                    self.open_repo_tab(PathBuf::from(p), true);
                }
            }
        }
    }

    pub fn new_tab(&mut self) {
        self.add_tab(TabKind::Home, true);
    }

    pub fn browse_open_repo(&mut self) {
        if self.selftest.is_some() || self.screenshot.is_some() {
            return;
        }
        if let Some(p) = rfd::FileDialog::new().set_title(t("menu.open_repo")).pick_folder() {
            self.open_repo(p);
        }
    }

    pub fn open_prefs(&mut self, page: PrefPage) {
        self.pref_page = page;
        self.screen = Screen::Prefs;
    }

    fn process_jobs(&mut self) {
        while let Ok(done) = self.job_rx.try_recv() {
            self.jobs.retain(|j| j.id != done.id);
            match done.result {
                Ok(JobOut::Msg(m)) => {
                    let text = if m.trim().is_empty() { tf("notify.done", &[&done.title]) } else { format!("{}: {}", done.title, first_line(&m)) };
                    self.toast(text, false);
                }
                Ok(JobOut::Cloned(p)) => {
                    self.toast(tf("notify.cloned", &[&p.display().to_string()]), false);
                    self.open_repo(p);
                }
                Ok(JobOut::Releases(r)) => {
                    self.gitdl.releases = r;
                    self.gitdl.loading = false;
                    self.gitdl.selected = 0;
                    self.gitdl.asset = 0;
                    self.gitdl.error = None;
                }
                Ok(JobOut::GitInstalled(m)) => {
                    self.gitdl.busy = false;
                    self.engine = make_engine(&self.settings);
                    self.git_version = self.engine.cli.as_ref().and_then(|p| git::git_version(p));
                    let key = match m.as_str() {
                        "installer_started" => "gitdl.installer_started",
                        "source_saved" => "gitdl.source_saved",
                        _ => "gitdl.installed",
                    };
                    self.toast(t(key), false);
                }
                Ok(JobOut::Models(m)) => {
                    self.ai_models = m;
                }
                Ok(JobOut::RepoList(r)) => {
                    self.clone_form.repos = r;
                    self.clone_form.loading = false;
                }
                Err(e) => {
                    self.gitdl.loading = false;
                    self.gitdl.busy = false;
                    self.clone_form.loading = false;
                    if done.title == t("gitdl.title") {
                        self.gitdl.error = Some(e.clone());
                    }
                    self.toast(format!("{}: {}", done.title, e), true);
                }
            }
            if let Some(id) = done.tab {
                let max = self.settings.graph_max_commits;
                if let Some(r) = self.tab_mut(id) {
                    r.reload(max);
                }
            }
        }
    }

    fn poll_languages(&mut self, now: f64) {
        if now - self.lang_poll < 2.0 {
            return;
        }
        self.lang_poll = now;
        let mut new_names = Vec::new();
        if let Ok(mut g) = i18n::global().write() {
            if g.poll(&self.settings.language) {
                new_names = std::mem::take(&mut g.last_new);
            }
        }
        for n in new_names {
            self.toast(tf("notify.new_language", &[&n]), false);
        }
    }

    fn auto_refresh(&mut self, now: f64) {
        let max = self.settings.graph_max_commits;
        let fetch_every = self.settings.auto_fetch_minutes as f64 * 60.0;
        let mut fetch_tab: Option<(u64, PathBuf)> = None;
        if self.active < self.tabs.len() {
            let id = self.tabs[self.active].id;
            if let TabKind::Repo(r) = &mut self.tabs[self.active].kind {
                if now - r.last_poll > 2.5 {
                    r.last_poll = now;
                    let fp = repo::fingerprint(&r.path);
                    if fp != r.fingerprint {
                        r.reload(max);
                    }
                }
                if fetch_every > 0.0 && r.last_fetch == 0.0 {
                    r.last_fetch = now;
                } else if fetch_every > 0.0 && now - r.last_fetch > fetch_every {
                    r.last_fetch = now;
                    fetch_tab = Some((id, r.path.clone()));
                }
            }
        }
        if let Some((id, path)) = fetch_tab {
            let e = self.engine.clone();
            self.spawn_job(t("jobs.auto_fetch"), Some(id), move |p| git::fetch(&e, &path, None, Some(p)).map(|_| JobOut::Msg(String::new())));
        }
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        use egui::{Key, KeyboardShortcut, Modifiers};
        let cmd = Modifiers::COMMAND;
        let sc = |m: Modifiers, k: Key| ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(m, k)));
        if sc(cmd | Modifiers::SHIFT, Key::T) {
            self.reopen_closed();
        }
        if sc(cmd, Key::T) {
            self.new_tab();
        }
        if sc(cmd, Key::W) {
            self.close_tab(self.active);
        }
        if sc(cmd, Key::N) {
            self.screen = Screen::Clone;
        }
        if sc(cmd, Key::I) {
            self.modal = Some(screens::new_init_form(&self.settings));
        }
        if sc(cmd | Modifiers::ALT, Key::O) {
            self.add_tab(TabKind::Launchpad, true);
        }
        if sc(cmd, Key::O) {
            self.browse_open_repo();
        }
        if sc(cmd, Key::Comma) {
            self.open_prefs(PrefPage::General);
        }
        if sc(cmd, Key::Plus) || sc(cmd, Key::Equals) {
            self.set_zoom(self.settings.zoom + 0.1);
        }
        if sc(cmd, Key::Minus) {
            self.set_zoom(self.settings.zoom - 0.1);
        }
        if sc(cmd, Key::Num0) {
            self.set_zoom(1.0);
        }
        if sc(Modifiers::ALT, Key::T) {
            if let Some(id) = self.active_repo_id() {
                actions::handle(self, id, repo::RepoAction::OpenTerminal);
            }
        }
        if sc(Modifiers::ALT, Key::O) {
            if let Some(id) = self.active_repo_id() {
                actions::handle(self, id, repo::RepoAction::OpenFolder);
            }
        }
        if sc(cmd, Key::Tab) && !self.tabs.is_empty() {
            self.active = (self.active + 1) % self.tabs.len();
        }
        let typing = ctx.memory(|m| m.focused().is_some());
        if !typing && self.modal.is_none() && self.screen == Screen::Main {
            let down = ctx.input(|i| i.key_pressed(Key::ArrowDown));
            let up = ctx.input(|i| i.key_pressed(Key::ArrowUp));
            let esc = ctx.input(|i| i.key_pressed(Key::Escape));
            if let Some(TabKind::Repo(r)) = self.tabs.get_mut(self.active).map(|t| &mut t.kind) {
                if down {
                    r.move_selection(1);
                }
                if up {
                    r.move_selection(-1);
                }
                if esc {
                    r.diff = None;
                }
            }
        }
    }

    pub fn set_zoom(&mut self, z: f32) {
        self.settings.zoom = ((z * 10.0).round() / 10.0).clamp(0.6, 2.0);
        self.ctx.set_zoom_factor(self.settings.zoom);
        self.save_settings();
    }

    fn central(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().frame(egui::Frame::none().fill(CONTENT)).show(ctx, |ui| {
            if self.screen == Screen::Prefs {
                screens_prefs::preferences(self, ui);
                return;
            }
            let idx = self.active.min(self.tabs.len().saturating_sub(1));
            let tab_id = self.tabs.get(idx).map(|t| t.id).unwrap_or(0);
            let recent: Vec<(String, String)> = self.settings.recent_repos.iter().map(|r| (r.name.clone(), r.path.clone())).collect();
            let ai_on = self.ai_on();
            let mut acts = Vec::new();
            let mut home_act = None;
            let settings = self.settings.clone();
            match self.tabs.get_mut(idx).map(|t| &mut t.kind) {
                Some(TabKind::Repo(r)) => r.show(ui, &settings, &recent, &mut acts, ai_on),
                Some(TabKind::Home) => home_act = home::home(ui, &recent),
                Some(TabKind::Launchpad) => home_act = home::launchpad(ui, &recent),
                Some(TabKind::ReleaseNotes) => home::release_notes(ui),
                Some(TabKind::Guide) => home::guide(ui),
                None => {}
            }
            for a in acts {
                actions::handle(self, tab_id, a);
            }
            if let Some(a) = home_act {
                home::handle(self, a);
            }
        });
    }
}

pub fn first_line(s: &str) -> String {
    s.lines().find(|l| !l.trim().is_empty()).unwrap_or("").chars().take(160).collect()
}

impl eframe::App for App {
    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw: &mut egui::RawInput) {
        // Automated runs ignore the real mouse and keyboard, so a person's cursor
        // over the window never changes the picture or the test.
        if self.screenshot.is_some() || self.selftest.is_some() {
            raw.events.retain(|e| {
                !matches!(
                    e,
                    egui::Event::PointerMoved(_) | egui::Event::PointerButton { .. } | egui::Event::MouseWheel { .. } | egui::Event::Text(_)
                ) && !matches!(e, egui::Event::Key { .. })
            });
            raw.events.push(egui::Event::PointerGone);
        }
        if let Some(st) = &mut self.selftest {
            // One key per frame, like a person pressing it.
            if !st.pending_keys.is_empty() {
                let k = st.pending_keys.remove(0);
                raw.events.push(egui::Event::Key { key: k, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() });
                raw.events.push(egui::Event::Key { key: k, physical_key: None, pressed: false, repeat: false, modifiers: Default::default() });
            }
        }
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frame += 1;
        self.status_drawn = false;
        let now = ctx.input(|i| i.time);
        self.process_jobs();
        actions::process_ai(self);
        self.poll_languages(now);
        if self.screen == Screen::Main {
            self.auto_refresh(now);
        }
        self.shortcuts(ctx);

        match self.screen {
            Screen::Onboarding(step) => screens::onboarding(self, ctx, step),
            Screen::Clone => screens::clone_page(self, ctx),
            Screen::Main | Screen::Prefs => {
                self.menu_bar(ctx);
                self.tab_bar(ctx);
                self.status_bar(ctx);
                self.central(ctx);
            }
        }
        screens::modals(self, ctx);
        self.toasts_ui(ctx);

        // --screenshot: draw ~120 frames, then save the picture and quit.
        if let Some(path) = self.screenshot.clone() {
            ctx.request_repaint();
            if self.frame >= 120 && !self.shot_requested {
                self.shot_requested = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
            }
            let shot = ctx.input(|i| {
                i.raw.events.iter().find_map(|e| match e {
                    egui::Event::Screenshot { image, .. } => Some(image.clone()),
                    _ => None,
                })
            });
            if let Some(img) = shot {
                save_png(&path, &img);
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            if self.frame > 900 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        if self.selftest.is_some() {
            ctx.request_repaint();
            selftest::step(self, ctx);
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.save_settings();
    }
}

fn save_png(path: &std::path::Path, img: &egui::ColorImage) {
    if let Some(d) = path.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let [w, h] = img.size;
    let mut buf = Vec::with_capacity(w * h * 4);
    for c in &img.pixels {
        buf.extend_from_slice(&[c.r(), c.g(), c.b(), 255]);
    }
    if let Some(im) = image::RgbaImage::from_raw(w as u32, h as u32, buf) {
        let _ = im.save(path);
    }
}
