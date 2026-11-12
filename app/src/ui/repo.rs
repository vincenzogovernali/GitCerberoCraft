//! A repository tab: state, layout of its panels and the toolbar.
//! The left panel, the graph and the side panels live in sibling files.

use super::theme::*;
use eframe::egui::{self, Align2, CursorIcon, Pos2, Rect, Rounding, Sense, Stroke, Ui, Vec2};
use egui_phosphor::regular as ic;
use gcc_core::ai::ChatMessage;
use gcc_core::config::{PullMode, Settings};
use gcc_core::git::{self, CommitDetail, DiffLine, Snapshot};
use gcc_core::graph::{self, Layout as GraphLayout};
use gcc_core::i18n::t;
use git2::Oid;
use std::collections::HashSet;
use std::path::PathBuf;

pub const ROW_H: f32 = 28.0;
pub const LANE_W: f32 = 22.0;
pub const NODE_R: f32 = 11.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sel {
    None,
    Wip,
    Commit(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeftTab {
    List,
    Agents,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RightView {
    Details,
    Ai,
}

#[derive(Clone, Debug)]
pub struct DiffView {
    pub title: String,
    pub file: String,
    pub lines: Vec<DiffLine>,
    /// Some(true) staged, Some(false) unstaged, None = from a commit.
    pub staged: Option<bool>,
}

#[derive(Clone, Debug)]
pub enum UndoEntry {
    Commit { before: Option<Oid>, after: Oid },
    Checkout { from: String, to: String },
    BranchCreate { name: String, oid: Oid },
    BranchDelete { name: String, oid: Oid },
    Reset { before: Oid, after: Oid },
}

#[derive(Clone, Debug)]
pub enum RepoAction {
    Refresh,
    Fetch,
    Pull(PullMode),
    Push(bool),
    Undo,
    Redo,
    NewBranch(Option<Oid>),
    Stash,
    Pop(usize),
    ApplyStash(usize),
    DropStash(usize),
    Checkout(String),
    CheckoutCommit(Oid),
    DeleteBranch(String),
    RenameBranch(String),
    Merge(String),
    CherryPick(Oid),
    Revert(Oid),
    Reset(Oid, &'static str),
    Tag(Oid),
    DeleteTag(String),
    Stage(Vec<String>),
    Unstage(Vec<String>),
    StageAll,
    UnstageAll,
    Discard(Vec<String>),
    DiscardAll,
    Commit,
    CommitPush,
    OpenTerminal,
    OpenFolder,
    OpenEditor,
    AiExplain(Oid),
    AiReview,
    AiCommitMessage,
    AiAsk(String),
    Copy(String),
    SwitchRepo(String),
    OpenRepo,
    AbortMerge,
    AddRemote,
    RemoveRemote(String),
    ShowDiffCommit(Oid, String),
    ShowDiffWip(String, bool),
    RunTerminal(String),
    ConfigureAi,
}

#[derive(Default, Clone, Copy)]
pub struct Hud {
    pub toolbar: bool,
    pub left_panel: bool,
    pub graph_header: bool,
    pub rows_drawn: usize,
}

pub struct RepoTab {
    pub path: PathBuf,
    pub snap: Option<Snapshot>,
    pub error: Option<String>,
    pub layout: GraphLayout,
    pub sel: Sel,
    pub detail: Option<CommitDetail>,
    pub diff: Option<DiffView>,
    pub filter: String,
    pub search: String,
    pub search_open: bool,
    pub left_open: bool,
    pub left_tab: LeftTab,
    pub closed_sections: HashSet<String>,
    pub closed_folders: HashSet<String>,
    pub summary: String,
    pub description: String,
    pub amend: bool,
    pub undo: Vec<UndoEntry>,
    pub redo: Vec<UndoEntry>,
    pub last_poll: f64,
    pub fingerprint: String,
    pub terminal_open: bool,
    pub term_input: String,
    pub term_output: String,
    pub right: RightView,
    pub ai_msgs: Vec<ChatMessage>,
    pub ai_input: String,
    pub ai_busy: bool,
    pub ai_status: String,
    pub scroll_to: Option<usize>,
    pub last_fetch: f64,
    pub hud: Hud,
}

impl RepoTab {
    pub fn new(path: PathBuf) -> Self {
        let mut closed = HashSet::new();
        for s in ["tags", "submodules", "worktrees"] {
            closed.insert(s.to_string());
        }
        Self {
            path,
            snap: None,
            error: None,
            layout: GraphLayout::default(),
            sel: Sel::None,
            detail: None,
            diff: None,
            filter: String::new(),
            search: String::new(),
            search_open: false,
            left_open: true,
            left_tab: LeftTab::List,
            closed_sections: closed,
            closed_folders: HashSet::new(),
            summary: String::new(),
            description: String::new(),
            amend: false,
            undo: vec![],
            redo: vec![],
            last_poll: 0.0,
            fingerprint: String::new(),
            terminal_open: false,
            term_input: String::new(),
            term_output: String::new(),
            right: RightView::Details,
            ai_msgs: vec![],
            ai_input: String::new(),
            ai_busy: false,
            ai_status: String::new(),
            scroll_to: None,
            last_fetch: 0.0,
            hud: Hud::default(),
        }
    }

    pub fn name(&self) -> String {
        self.snap.as_ref().map(|s| s.name.clone()).unwrap_or_else(|| git::repo_name(&self.path))
    }

    pub fn reload(&mut self, max: usize) {
        match git::load(&self.path, max) {
            Ok(s) => {
                let prev_sel_id = match self.sel {
                    Sel::Commit(i) => self.snap.as_ref().and_then(|o| o.commits.get(i)).map(|c| c.id),
                    _ => None,
                };
                self.layout = graph::layout(&s.commits);
                self.fingerprint = fingerprint(&self.path);
                if let Some(id) = prev_sel_id {
                    self.sel = s.row_of(id).map(Sel::Commit).unwrap_or(Sel::None);
                }
                if self.sel == Sel::Wip && s.wip.is_empty() {
                    self.sel = Sel::None;
                }
                self.snap = Some(s);
                self.error = None;
                self.refresh_detail();
            }
            Err(e) => self.error = Some(e),
        }
    }

    pub fn refresh_detail(&mut self) {
        self.detail = None;
        if let (Sel::Commit(i), Some(s)) = (self.sel, &self.snap) {
            if let Some(c) = s.commits.get(i) {
                self.detail = git::commit_detail(&self.path, c.id).ok();
            }
        }
    }

    pub fn select(&mut self, sel: Sel) {
        if self.sel != sel {
            self.sel = sel;
            self.diff = None;
            self.refresh_detail();
        }
    }

    pub fn has_wip(&self) -> bool {
        self.snap.as_ref().map(|s| !s.wip.is_empty()).unwrap_or(false)
    }

    pub fn row_count(&self) -> usize {
        self.snap.as_ref().map(|s| s.commits.len()).unwrap_or(0) + usize::from(self.has_wip())
    }

    /// Move the selection by `delta` rows (keyboard navigation).
    pub fn move_selection(&mut self, delta: i32) {
        let wip = self.has_wip();
        let n = self.row_count() as i32;
        if n == 0 {
            return;
        }
        let cur: i32 = match self.sel {
            Sel::None => -1,
            Sel::Wip => 0,
            Sel::Commit(i) => i as i32 + i32::from(wip),
        };
        let next = (cur + delta).clamp(0, n - 1);
        let sel = if wip && next == 0 { Sel::Wip } else { Sel::Commit((next - i32::from(wip)) as usize) };
        self.select(sel);
        self.scroll_to = Some(next as usize);
    }

    pub fn selected_oid(&self) -> Option<Oid> {
        match self.sel {
            Sel::Commit(i) => self.snap.as_ref().and_then(|s| s.commits.get(i)).map(|c| c.id),
            _ => None,
        }
    }

    pub fn show(&mut self, ui: &mut Ui, settings: &Settings, recent: &[(String, String)], acts: &mut Vec<RepoAction>, ai_on: bool) {
        self.hud = Hud::default();
        egui::TopBottomPanel::top("repo_toolbar")
            .exact_height(46.0)
            .frame(egui::Frame::none().fill(TOOLBAR))
            .show_inside(ui, |ui| self.toolbar(ui, recent, acts));

        if self.terminal_open {
            egui::TopBottomPanel::bottom("repo_terminal")
                .resizable(true)
                .default_height(200.0)
                .frame(egui::Frame::none().fill(hex(0x16181C)).inner_margin(egui::Margin::same(8.0)))
                .show_inside(ui, |ui| self.terminal(ui, acts));
        }

        if self.left_open {
            egui::SidePanel::left("repo_left")
                .exact_width(settings.left_panel_width.clamp(160.0, 400.0))
                .resizable(false)
                .frame(egui::Frame::none().fill(PANEL))
                .show_inside(ui, |ui| self.left_panel(ui, acts, ai_on));
        } else {
            egui::SidePanel::left("repo_left_closed")
                .exact_width(30.0)
                .resizable(false)
                .frame(egui::Frame::none().fill(PANEL))
                .show_inside(ui, |ui| {
                    let r = ui.max_rect();
                    let c = Pos2::new(r.center().x, r.top() + 18.0);
                    let resp = ui.interact(Rect::from_center_size(c, Vec2::splat(20.0)), ui.id().with("open_left"), Sense::click());
                    ui.painter().circle_filled(c, 9.0, hex(0xC9CCD1));
                    text_at(ui, c, Align2::CENTER_CENTER, ic::CARET_RIGHT, bold(12.0), PANEL);
                    if resp.clicked() {
                        self.left_open = true;
                    }
                });
        }

        let show_right = self.sel != Sel::None || self.right == RightView::Ai;
        if show_right {
            egui::SidePanel::right("repo_right")
                .default_width(380.0)
                .min_width(300.0)
                .max_width(620.0)
                .resizable(true)
                .frame(egui::Frame::none().fill(PANEL))
                .show_inside(ui, |ui| {
                    if self.right == RightView::Ai {
                        self.ai_panel(ui, acts, ai_on);
                    } else if self.sel == Sel::Wip {
                        self.wip_panel(ui, acts, ai_on);
                    } else {
                        self.commit_panel(ui, acts, ai_on);
                    }
                });
        }

        egui::CentralPanel::default().frame(egui::Frame::none().fill(CONTENT)).show_inside(ui, |ui| {
            if let Some(e) = &self.error {
                ui.add_space(40.0);
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new(ic::WARNING).size(36.0).color(MOD_YELLOW));
                    ui.label(egui::RichText::new(t("repo.open_error")).size(18.0));
                    ui.label(egui::RichText::new(e).color(TEXT_DIM));
                });
                return;
            }
            if self.diff.is_some() {
                self.diff_view(ui, acts);
            } else {
                self.graph_view(ui, acts);
            }
        });
    }

    fn toolbar(&mut self, ui: &mut Ui, recent: &[(String, String)], acts: &mut Vec<RepoAction>) {
        self.hud.toolbar = true;
        let rect = ui.max_rect();
        let p = ui.painter().clone();
        let top = rect.top();
        let x0 = rect.left() + 18.0;
        p.text(Pos2::new(x0, top + 9.0), Align2::LEFT_CENTER, t("toolbar.repository"), font(11.5), TEXT_DIM);
        let name = self.name();
        let nr = p.text(Pos2::new(x0, top + 28.0), Align2::LEFT_CENTER, &name, bold(16.5), TEXT_BRIGHT);
        p.text(Pos2::new(nr.right() + 6.0, top + 29.0), Align2::LEFT_CENTER, "▾", font(12.0), TEXT);
        let repo_rect = Rect::from_min_max(Pos2::new(x0 - 4.0, top + 2.0), Pos2::new(nr.right() + 18.0, top + 44.0));
        let repo_resp = ui.interact(repo_rect, ui.id().with("repo_pick"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
        let chev_x = nr.right() + 30.0;
        p.text(Pos2::new(chev_x, top + 28.0), Align2::LEFT_CENTER, ic::CARET_RIGHT, font(16.0), TEXT_DIM);
        let bx = chev_x + 25.0;
        p.text(Pos2::new(bx, top + 9.0), Align2::LEFT_CENTER, t("toolbar.branch_label"), font(11.5), TEXT_DIM);
        let branch = self.snap.as_ref().map(|s| s.head_name.clone()).unwrap_or_default();
        let br = p.text(Pos2::new(bx, top + 28.0), Align2::LEFT_CENTER, &branch, font(15.5), TEXT_BRIGHT);
        let caret_x = (br.right() + 30.0).max(bx + 78.0);
        p.text(Pos2::new(caret_x, top + 29.0), Align2::LEFT_CENTER, "▾", font(12.0), TEXT);
        let branch_rect = Rect::from_min_max(Pos2::new(bx - 4.0, top + 2.0), Pos2::new(caret_x + 14.0, top + 44.0));
        let branch_resp = ui.interact(branch_rect, ui.id().with("branch_pick"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);

        let repo_pop = ui.id().with("repo_pop");
        egui::popup::popup_below_widget(ui, repo_pop, &repo_resp, egui::PopupCloseBehavior::CloseOnClick, |ui| {
            ui.set_min_width(260.0);
            for (n, path) in recent {
                if ui.button(format!("{}  {}", ic::GIT_BRANCH, n)).on_hover_text(path).clicked() {
                    acts.push(RepoAction::SwitchRepo(path.clone()));
                }
            }
            ui.separator();
            if ui.button(format!("{}  {}", ic::FOLDER_OPEN, t("menu.open_repo"))).clicked() {
                acts.push(RepoAction::OpenRepo);
            }
        });
        if repo_resp.clicked() {
            ui.memory_mut(|m| m.toggle_popup(repo_pop));
        }
        let branch_pop = ui.id().with("branch_pop");
        egui::popup::popup_below_widget(ui, branch_pop, &branch_resp, egui::PopupCloseBehavior::CloseOnClick, |ui| {
            ui.set_min_width(240.0);
            if let Some(s) = &self.snap {
                for b in &s.local {
                    let label = if b.is_head { format!("{}  {}", ic::CHECK, b.name) } else { format!("      {}", b.name) };
                    if ui.button(label).clicked() && !b.is_head {
                        acts.push(RepoAction::Checkout(b.name.clone()));
                    }
                }
            }
            ui.separator();
            if ui.button(format!("{}  {}", ic::PLUS, t("toolbar.new_branch"))).clicked() {
                acts.push(RepoAction::NewBranch(None));
            }
        });
        if branch_resp.clicked() {
            ui.memory_mut(|m| m.toggle_popup(branch_pop));
        }

        let has_wip = self.has_wip();
        let has_stash = self.snap.as_ref().map(|s| !s.stashes.is_empty()).unwrap_or(false);
        let can_branch = self.snap.as_ref().map(|s| !s.unborn).unwrap_or(false);
        let start = (caret_x + 120.0).max(rect.left() + 385.0);
        let bw = 52.0;
        let mut x = start;
        let btn = |ui: &mut Ui, x: f32, label: &str, icon: &str, enabled: bool, id: &str| -> egui::Response {
            let r = Rect::from_min_size(Pos2::new(x - bw / 2.0, top + 1.0), Vec2::new(bw, 44.0));
            let resp = ui.interact(r, ui.id().with(id), if enabled { Sense::click() } else { Sense::hover() });
            if enabled && resp.hovered() {
                ui.painter().rect_filled(r.shrink(1.0), Rounding::same(3.0), hex(0x3D424C));
            }
            let (lc, icc) = if enabled { (hex(0xB9BDC4), TEXT_BRIGHT) } else { (TEXT_FAINT, TEXT_FAINT) };
            ui.painter().text(Pos2::new(x, top + 9.0), Align2::CENTER_CENTER, label, font(11.5), lc);
            ui.painter().text(Pos2::new(x, top + 30.0), Align2::CENTER_CENTER, icon, font(20.0), icc);
            if enabled {
                resp.on_hover_cursor(CursorIcon::PointingHand)
            } else {
                resp
            }
        };
        if btn(ui, x, &t("toolbar.undo"), ic::ARROW_COUNTER_CLOCKWISE, !self.undo.is_empty(), "undo").clicked() {
            acts.push(RepoAction::Undo);
        }
        x += bw;
        if btn(ui, x, &t("toolbar.redo"), ic::ARROW_CLOCKWISE, !self.redo.is_empty(), "redo").clicked() {
            acts.push(RepoAction::Redo);
        }
        x += bw;
        if btn(ui, x, &t("toolbar.pull"), ic::ARROW_LINE_DOWN, true, "pull").clicked() {
            acts.push(RepoAction::Pull(PullMode::FastForwardIfPossible));
        }
        let caret = Rect::from_center_size(Pos2::new(x + 32.0, top + 18.0), Vec2::new(14.0, 26.0));
        let caret_resp = ui.interact(caret, ui.id().with("pull_caret"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
        p.text(caret.center(), Align2::CENTER_CENTER, "▾", font(12.0), TEXT);
        let pull_pop = ui.id().with("pull_pop");
        egui::popup::popup_below_widget(ui, pull_pop, &caret_resp, egui::PopupCloseBehavior::CloseOnClick, |ui| {
            ui.set_min_width(230.0);
            for (m, k) in [
                (PullMode::FetchAll, "pull.fetch_all"),
                (PullMode::FastForwardIfPossible, "pull.ff_if_possible"),
                (PullMode::FastForwardOnly, "pull.ff_only"),
                (PullMode::Rebase, "pull.rebase"),
            ] {
                if ui.button(t(k)).clicked() {
                    acts.push(RepoAction::Pull(m));
                }
            }
        });
        if caret_resp.clicked() {
            ui.memory_mut(|m| m.toggle_popup(pull_pop));
        }
        x += bw + 15.0;
        let push = btn(ui, x, &t("toolbar.push"), ic::ARROW_LINE_UP, true, "push");
        if push.clicked() {
            acts.push(RepoAction::Push(false));
        }
        push.context_menu(|ui| {
            if ui.button(t("push.force")).clicked() {
                acts.push(RepoAction::Push(true));
                ui.close_menu();
            }
        });
        x += bw;
        if btn(ui, x, &t("toolbar.branch"), ic::GIT_BRANCH, can_branch, "branchbtn").clicked() {
            acts.push(RepoAction::NewBranch(self.selected_oid()));
        }
        x += bw;
        if btn(ui, x, &t("toolbar.stash"), ic::TRAY_ARROW_DOWN, has_wip, "stash").clicked() {
            acts.push(RepoAction::Stash);
        }
        x += bw;
        if btn(ui, x, &t("toolbar.pop"), ic::TRAY_ARROW_UP, has_stash, "pop").clicked() {
            acts.push(RepoAction::Pop(0));
        }
        x += bw / 2.0 + 4.0;
        p.line_segment([Pos2::new(x, top + 4.0), Pos2::new(x, top + 44.0)], Stroke::new(1.0, hex(0x4A4F59)));
        x += 31.0;
        let term = btn(ui, x, &t("toolbar.terminal"), ic::TERMINAL_WINDOW, true, "term");
        if term.clicked() {
            self.terminal_open = !self.terminal_open;
        }
        term.context_menu(|ui| {
            if ui.button(t("toolbar.open_external_terminal")).clicked() {
                acts.push(RepoAction::OpenTerminal);
                ui.close_menu();
            }
        });
        x += bw + 6.0;
        if btn(ui, x, &t("toolbar.ai"), ic::SPARKLE, true, "aibtn").clicked() {
            self.right = if self.right == RightView::Ai { RightView::Details } else { RightView::Ai };
        }

        let rx = rect.right() - 24.0;
        if btn(ui, rx, &t("toolbar.search"), ic::MAGNIFYING_GLASS, true, "search").clicked() {
            self.search_open = !self.search_open;
            if !self.search_open {
                self.search.clear();
            }
        }
        let actions = btn(ui, rx - 52.0, &t("toolbar.actions"), ic::SLIDERS, true, "actions");
        let actions_pop = ui.id().with("actions_pop");
        egui::popup::popup_below_widget(ui, actions_pop, &actions, egui::PopupCloseBehavior::CloseOnClick, |ui| {
            ui.set_min_width(250.0);
            let items: [(&str, &str, RepoAction); 8] = [
                (ic::ARROWS_CLOCKWISE, "actions.refresh", RepoAction::Refresh),
                (ic::CLOUD_ARROW_DOWN, "actions.fetch_all", RepoAction::Fetch),
                (ic::FOLDER_OPEN, "actions.open_folder", RepoAction::OpenFolder),
                (ic::FILE_TEXT, "actions.open_editor", RepoAction::OpenEditor),
                (ic::TERMINAL_WINDOW, "toolbar.open_external_terminal", RepoAction::OpenTerminal),
                (ic::LINK, "actions.add_remote", RepoAction::AddRemote),
                (ic::SPARKLE, "actions.ai_review", RepoAction::AiReview),
                (ic::COPY, "actions.copy_path", RepoAction::Copy(self.path.display().to_string())),
            ];
            for (icon, key, a) in items {
                if ui.button(format!("{icon}  {}", t(key))).clicked() {
                    acts.push(a);
                }
            }
        });
        if actions.clicked() {
            ui.memory_mut(|m| m.toggle_popup(actions_pop));
        }
    }
}

/// A short text that changes whenever HEAD, a ref or the working tree changes.
pub fn fingerprint(path: &std::path::Path) -> String {
    let Ok(repo) = git2::Repository::open(path) else { return String::new() };
    let mut s = String::new();
    if let Ok(h) = repo.head() {
        s.push_str(&format!("{:?}{:?}", h.name(), h.target()));
    }
    if let Ok(refs) = repo.references() {
        for r in refs.flatten() {
            s.push_str(r.name().unwrap_or(""));
            if let Some(t) = r.target() {
                s.push_str(&t.to_string()[..8]);
            }
        }
    }
    let mut opts = git2::StatusOptions::new();
    opts.include_untracked(true).recurse_untracked_dirs(true);
    if let Ok(st) = repo.statuses(Some(&mut opts)) {
        for e in st.iter() {
            s.push_str(e.path().unwrap_or(""));
            s.push_str(&format!("{}", e.status().bits()));
        }
    }
    if let Ok(m) = std::fs::metadata(repo.path().join("index")).and_then(|m| m.modified()) {
        s.push_str(&format!("{m:?}"));
    }
    s
}
