//! Home ("New Tab"), Launchpad, Release Notes and Guide pages.

use super::theme::*;
use super::*;
use eframe::egui::{self, Align2, Pos2, Rect, Rounding, Sense, Stroke, Ui, Vec2};
use egui_phosphor::regular as ic;
use gcc_core::i18n::{t, tf};
use std::sync::Arc;

pub enum HomeAction {
    Open,
    Clone,
    Create,
    OpenRecent(String),
    ConfigureAi,
    Tutorials,
    ReleaseNotes,
    Documentation,
    Fetch(String),
    Pull(String),
    Forget(String),
}

pub fn home(ui: &mut Ui, recent: &[(String, String)]) -> Option<HomeAction> {
    let mut act = None;
    let full = ui.max_rect();
    let w = full.width();
    let left_x = full.left() + (w * 0.085).max(30.0);
    let div_x = full.left() + (w * 0.597).max(470.0).min(w - 300.0);
    let right_x = div_x + 21.0;
    let top = full.top();
    let p = ui.painter().clone();

    // Left: Repositories
    p.text(Pos2::new(left_x, top + 51.0), Align2::LEFT_CENTER, t("home.repositories"), font(20.5), hex(0xD3D6DB));
    let mut x = left_x;
    let by = top + 82.0;
    for (icon, key, a) in [
        (ic::FOLDER_OPEN, "home.open", HomeAction::Open),
        (ic::CLOUD_ARROW_DOWN, "home.clone", HomeAction::Clone),
        (ic::PLUS_SQUARE, "home.create", HomeAction::Create),
    ] {
        let label = t(key);
        let bw = text_width(ui, &label, font(17.0)) + text_width(ui, icon, font(17.0)) + 38.0;
        let r = Rect::from_min_size(Pos2::new(x, by), Vec2::new(bw.max(91.0), 44.0));
        let mut child = ui.child_ui(r, egui::Layout::left_to_right(egui::Align::Center), None);
        if big_button(&mut child, icon, &label, r.size()).clicked() {
            act = Some(a);
        }
        x = r.right() + 10.0;
    }
    p.text(Pos2::new(left_x, top + 155.0), Align2::LEFT_CENTER, t("home.recent"), font(14.5), hex(0xC8CCD2));
    let mut y = top + 177.0;
    if recent.is_empty() {
        p.text(Pos2::new(left_x + 3.0, y), Align2::LEFT_CENTER, t("home.no_recent"), font(14.0), TEXT_FAINT);
    }
    for (name, path) in recent.iter().take(14) {
        let nr = p.text(Pos2::new(left_x + 3.0, y), Align2::LEFT_CENTER, name, bold(14.5), LINK);
        let pr = p.text(Pos2::new(nr.right() + 13.0, y), Align2::LEFT_CENTER, elide(ui, path, &font(14.0), div_x - nr.right() - 40.0), font(14.0), hex(0x7D828A));
        let hit = nr.union(pr).expand2(Vec2::new(4.0, 3.0));
        let resp = ui.interact(hit, ui.id().with(("recent", path)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
        if resp.hovered() {
            p.line_segment([Pos2::new(nr.left(), nr.bottom()), Pos2::new(nr.right(), nr.bottom())], Stroke::new(1.0, LINK));
        }
        if resp.clicked() {
            act = Some(HomeAction::OpenRecent(path.clone()));
        }
        let pc = path.clone();
        resp.context_menu(|ui| {
            if ui.button(t("home.forget")).clicked() {
                act = Some(HomeAction::Forget(pc.clone()));
                ui.close_menu();
            }
        });
        y += 22.0;
    }

    // Divider
    p.line_segment([Pos2::new(div_x, top + 40.0), Pos2::new(div_x, full.bottom() - 40.0)], Stroke::new(1.0, DIVIDER));

    // Right: AI + resources
    let rw = (full.right() - right_x - 40.0).min(390.0);
    p.text(Pos2::new(right_x, top + 49.0), Align2::LEFT_CENTER, t("home.ai_title"), font(18.5), hex(0xD3D6DB));
    let galley = ui.fonts(|f| f.layout(t("home.ai_text"), font(14.5), hex(0xC8CCD2), rw));
    let gh = galley.size().y;
    p.galley(Pos2::new(right_x, top + 79.0), galley, hex(0xC8CCD2));
    let br_top = top + 79.0 + gh + 11.0;
    let label = t("home.ai_button");
    let bw = text_width(ui, &label, font(17.0)) + 52.0;
    let r = Rect::from_min_size(Pos2::new(right_x, br_top), Vec2::new(bw, 44.0));
    let resp = ui.interact(r, ui.id().with("explore_ai"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
    p.rect(r, Rounding::same(3.0), if resp.hovered() { hex(0x31353D) } else { PANEL }, Stroke::new(1.0, hex(0x30343B)));
    p.text(Pos2::new(r.left() + 11.0, r.center().y), Align2::LEFT_CENTER, &label, font(17.0), TEXT_BRIGHT);
    p.text(Pos2::new(r.right() - 18.0, r.center().y), Align2::CENTER_CENTER, ic::SPARKLE, font(17.0), TEXT_DIM);
    if resp.clicked() {
        act = Some(HomeAction::ConfigureAi);
    }
    let ry = r.bottom() + 31.0;
    p.text(Pos2::new(right_x, ry), Align2::LEFT_CENTER, t("home.resources"), font(14.5), hex(0xC8CCD2));
    let mut ly = ry + 22.0;
    for (key, a, ext) in [
        ("home.tutorials", HomeAction::Tutorials, false),
        ("home.release_notes", HomeAction::ReleaseNotes, false),
        ("home.documentation", HomeAction::Documentation, true),
    ] {
        let text = if ext { format!("{} {}", t(key), ic::BOOK_OPEN) } else { t(key) };
        let lr = p.text(Pos2::new(right_x + 3.0, ly), Align2::LEFT_CENTER, &text, font(14.5), LINK);
        let resp = ui.interact(lr.expand(2.0), ui.id().with(key), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
        if resp.hovered() {
            p.line_segment([Pos2::new(lr.left(), lr.bottom()), Pos2::new(lr.right(), lr.bottom())], Stroke::new(1.0, LINK));
        }
        if resp.clicked() {
            act = Some(a);
        }
        ly += 22.0;
    }
    act
}

#[derive(Clone)]
struct RepoCard {
    name: String,
    path: String,
    branch: String,
    changes: usize,
    ahead: usize,
    behind: usize,
    last: String,
    when: i64,
    ok: bool,
}

fn card_info(name: &str, path: &str) -> RepoCard {
    let mut c = RepoCard { name: name.into(), path: path.into(), branch: String::new(), changes: 0, ahead: 0, behind: 0, last: String::new(), when: 0, ok: false };
    if let Ok(repo) = git2::Repository::open(path) {
        c.ok = true;
        if let Ok(h) = repo.head() {
            c.branch = h.shorthand().unwrap_or("").to_string();
            if let Ok(commit) = h.peel_to_commit() {
                c.last = commit.summary().unwrap_or("").to_string();
                c.when = commit.time().seconds();
            }
            if let Ok(b) = repo.find_branch(&c.branch, git2::BranchType::Local) {
                if let (Ok(up), Some(l)) = (b.upstream(), b.get().target()) {
                    if let Some(u) = up.get().target() {
                        if let Ok((a, bh)) = repo.graph_ahead_behind(l, u) {
                            c.ahead = a;
                            c.behind = bh;
                        }
                    }
                }
            }
        }
        if let Ok(w) = gcc_core::git::wip_status(&repo) {
            c.changes = w.count();
        }
    }
    c
}

pub fn launchpad(ui: &mut Ui, recent: &[(String, String)]) -> Option<HomeAction> {
    let mut act = None;
    let id = egui::Id::new("launchpad_cache");
    let now = ui.input(|i| i.time);
    let cached: Option<(f64, Arc<Vec<RepoCard>>)> = ui.data(|d| d.get_temp(id));
    let cards = match cached {
        Some((t0, c)) if now - t0 < 8.0 && c.len() == recent.len().min(20) => c,
        _ => {
            let c: Arc<Vec<RepoCard>> = Arc::new(recent.iter().take(20).map(|(n, p)| card_info(n, p)).collect());
            ui.data_mut(|d| d.insert_temp(id, (now, c.clone())));
            c
        }
    };
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        egui::Frame::none().inner_margin(egui::Margin::symmetric(60.0, 30.0)).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("{} {}", ic::ROCKET_LAUNCH, t("tabs.launchpad"))).size(22.0).color(TEXT_BRIGHT));
            });
            ui.label(egui::RichText::new(t("launchpad.subtitle")).color(TEXT_DIM));
            ui.add_space(14.0);
            if cards.is_empty() {
                ui.label(egui::RichText::new(t("home.no_recent")).color(TEXT_FAINT));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button(format!("{} {}", ic::FOLDER_OPEN, t("home.open"))).clicked() {
                        act = Some(HomeAction::Open);
                    }
                    if ui.button(format!("{} {}", ic::CLOUD_ARROW_DOWN, t("home.clone"))).clicked() {
                        act = Some(HomeAction::Clone);
                    }
                    if ui.button(format!("{} {}", ic::PLUS_SQUARE, t("home.create"))).clicked() {
                        act = Some(HomeAction::Create);
                    }
                });
            }
            for c in cards.iter() {
                egui::Frame::none().fill(PANEL).rounding(4.0).inner_margin(egui::Margin::symmetric(16.0, 12.0)).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(ic::GIT_BRANCH).size(18.0).color(lane_color(0)));
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(&c.name).font(bold(16.0)).color(TEXT_BRIGHT));
                            ui.label(egui::RichText::new(&c.path).size(12.0).color(TEXT_FAINT));
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(t("home.open")).clicked() {
                                act = Some(HomeAction::OpenRecent(c.path.clone()));
                            }
                            if c.ok && ui.button(format!("{} {}", ic::ARROW_LINE_DOWN, t("toolbar.pull"))).clicked() {
                                act = Some(HomeAction::Pull(c.path.clone()));
                            }
                            if c.ok && ui.button(format!("{} {}", ic::CLOUD_ARROW_DOWN, t("launchpad.fetch"))).clicked() {
                                act = Some(HomeAction::Fetch(c.path.clone()));
                            }
                        });
                    });
                    if c.ok {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(egui::RichText::new(format!("{} {}", ic::GIT_BRANCH, c.branch)).color(TEXT));
                            let (txt, col) = if c.changes == 0 { (t("launchpad.clean"), CHECK_GREEN) } else { (tf("launchpad.changes", &[&c.changes.to_string()]), MOD_YELLOW) };
                            ui.label(egui::RichText::new(txt).color(col));
                            if c.ahead > 0 {
                                ui.label(egui::RichText::new(format!("{} {}", ic::ARROW_LINE_UP, c.ahead)).color(TEXT));
                            }
                            if c.behind > 0 {
                                ui.label(egui::RichText::new(format!("{} {}", ic::ARROW_LINE_DOWN, c.behind)).color(LINK_SOFT));
                            }
                            if !c.last.is_empty() {
                                ui.label(egui::RichText::new(format!("· {} · {}", c.last, gcc_core::util::relative_time(c.when))).color(TEXT_DIM));
                            }
                        });
                    } else {
                        ui.label(egui::RichText::new(t("launchpad.missing")).color(DEL_RED));
                    }
                });
                ui.add_space(8.0);
            }
        });
    });
    act
}

pub fn release_notes(ui: &mut Ui) {
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        egui::Frame::none().inner_margin(egui::Margin::symmetric(80.0, 30.0)).show(ui, |ui| {
            ui.label(egui::RichText::new(t("release.title")).size(24.0).color(TEXT_BRIGHT));
            ui.label(egui::RichText::new(format!("GitCerberoCraft {}", gcc_core::config::APP_VERSION)).color(TEXT_DIM));
            ui.add_space(14.0);
            for i in 1..=12 {
                let key = format!("release.item{i}");
                let text = t(&key);
                if text == key {
                    break;
                }
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new("•").color(lane_color(0)));
                    ui.label(egui::RichText::new(text).color(TEXT).size(14.5));
                });
                ui.add_space(4.0);
            }
        });
    });
}

pub fn guide(ui: &mut Ui) {
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        egui::Frame::none().inner_margin(egui::Margin::symmetric(80.0, 30.0)).show(ui, |ui| {
            ui.label(egui::RichText::new(t("guide.title")).size(24.0).color(TEXT_BRIGHT));
            ui.add_space(10.0);
            for i in 1..=20 {
                let hk = format!("guide.h{i}");
                let h = t(&hk);
                if h == hk {
                    break;
                }
                ui.add_space(8.0);
                ui.label(egui::RichText::new(h).font(bold(16.0)).color(TEXT_BRIGHT));
                ui.label(egui::RichText::new(t(&format!("guide.p{i}"))).color(TEXT).size(14.0));
            }
        });
    });
}

pub fn handle(app: &mut App, a: HomeAction) {
    match a {
        HomeAction::Open => app.browse_open_repo(),
        HomeAction::Clone => app.screen = Screen::Clone,
        HomeAction::Create => app.modal = Some(screens::new_init_form(&app.settings)),
        HomeAction::OpenRecent(p) => {
            app.open_repo(PathBuf::from(p));
        }
        HomeAction::ConfigureAi => app.open_prefs(PrefPage::Ai),
        HomeAction::Tutorials | HomeAction::Documentation => {
            app.add_tab(TabKind::Guide, true);
        }
        HomeAction::ReleaseNotes => {
            if let Some(i) = app.tabs.iter().position(|t| matches!(t.kind, TabKind::ReleaseNotes)) {
                app.active = i;
            } else {
                app.add_tab(TabKind::ReleaseNotes, true);
            }
        }
        HomeAction::Fetch(p) => {
            let e = app.engine.clone();
            let path = PathBuf::from(p);
            app.spawn_job(t("jobs.fetch"), None, move |pr| gcc_core::git::fetch(&e, &path, None, Some(pr)).map(JobOut::Msg));
        }
        HomeAction::Pull(p) => {
            let e = app.engine.clone();
            let path = PathBuf::from(p);
            let mode = app.settings.pull_mode;
            app.spawn_job(t("jobs.pull"), None, move |pr| gcc_core::git::pull(&e, &path, mode, Some(pr)).map(JobOut::Msg));
        }
        HomeAction::Forget(p) => {
            app.settings.recent_repos.retain(|r| r.path != p);
            app.save_settings();
        }
    }
}
