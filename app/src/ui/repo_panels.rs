//! Right-hand panels of a repository tab (commit details, work in progress,
//! AI chat), the diff view and the built-in terminal.

use super::repo::*;
use super::theme::*;
use eframe::egui::{self, Align, Align2, Color32, CursorIcon, Layout, Pos2, Rect, Rounding, ScrollArea, Sense, Ui, Vec2};
use egui_phosphor::regular as ic;
use gcc_core::git::{self, DiffLineKind};
use gcc_core::i18n::{t, tf};
use git2::Oid;

const UNSTAGE_RED: Color32 = hex(0x7A2E2E);

impl RepoTab {
    fn panel_header(ui: &mut Ui, title: &str) -> bool {
        let w = ui.available_width();
        let (r, _) = ui.allocate_exact_size(Vec2::new(w, 40.0), Sense::hover());
        ui.painter().rect_filled(r, Rounding::ZERO, hex(0x2E3138));
        ui.painter().text(Pos2::new(r.left() + 14.0, r.center().y), Align2::LEFT_CENTER, title, bold(14.0), TEXT_BRIGHT);
        let xr = Rect::from_center_size(Pos2::new(r.right() - 18.0, r.center().y), Vec2::splat(22.0));
        let resp = ui.interact(xr, ui.id().with(("close", title)), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
        ui.painter().text(xr.center(), Align2::CENTER_CENTER, ic::X, font(15.0), if resp.hovered() { TEXT_BRIGHT } else { TEXT_DIM });
        resp.clicked()
    }

    pub(super) fn commit_panel(&mut self, ui: &mut Ui, acts: &mut Vec<RepoAction>, ai_on: bool) {
        let Some(d) = self.detail.clone() else { return };
        if Self::panel_header(ui, &format!("{} {}", t("detail.commit"), &d.id[..7])) {
            self.select(Sel::None);
            return;
        }
        ScrollArea::vertical().auto_shrink([false, false]).id_source("commit_detail").show(ui, |ui| {
            egui::Frame::none().inner_margin(egui::Margin::symmetric(14.0, 10.0)).show(ui, |ui| {
                let mut lines = d.message.trim().splitn(2, '\n');
                let summary = lines.next().unwrap_or("");
                let body = lines.next().unwrap_or("").trim();
                ui.label(egui::RichText::new(summary).font(bold(15.5)).color(TEXT_BRIGHT));
                if !body.is_empty() {
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(body).color(TEXT).size(13.0));
                }
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(32.0), Sense::hover());
                    avatar(ui.painter(), r.center(), 15.0, &d.author);
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new(&d.author).color(TEXT_BRIGHT).strong());
                        ui.label(
                            egui::RichText::new(format!("{} {}", t("detail.authored"), gcc_core::util::format_date(d.time)))
                                .color(TEXT_DIM)
                                .size(12.0),
                        );
                    });
                });
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new(t("detail.sha")).color(TEXT_DIM).size(12.0));
                    if ui.link(egui::RichText::new(&d.id[..12]).monospace().size(12.0)).on_hover_text(t("ctx.copy_sha")).clicked() {
                        acts.push(RepoAction::Copy(d.id.clone()));
                    }
                    if !d.parents.is_empty() {
                        ui.label(egui::RichText::new(t("detail.parent")).color(TEXT_DIM).size(12.0));
                        ui.label(egui::RichText::new(d.parents.join(", ")).monospace().size(12.0));
                    }
                });
                ui.add_space(8.0);
                let oid = Oid::from_str(&d.id).ok();
                let b = ui.add(egui::Button::new(format!("{}  {}", ic::SPARKLE, t("detail.explain_ai"))).fill(PURPLE_DEEP));
                if b.clicked() {
                    if let Some(o) = oid {
                        acts.push(RepoAction::AiExplain(o));
                    }
                }
                if !ai_on {
                    b.on_hover_text(t("agents.off"));
                }
                ui.add_space(10.0);
                let adds: usize = d.files.iter().map(|f| f.additions).sum();
                let dels: usize = d.files.iter().map(|f| f.deletions).sum();
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(tf("detail.files_changed", &[&d.files.len().to_string()])).color(TEXT).strong());
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(egui::RichText::new(format!("-{dels}")).color(DEL_RED));
                        ui.label(egui::RichText::new(format!("+{adds}")).color(ADD_GREEN));
                    });
                });
                ui.add_space(4.0);
                for f in &d.files {
                    if file_row(ui, f, None).clicked() {
                        if let Some(o) = oid {
                            acts.push(RepoAction::ShowDiffCommit(o, f.path.clone()));
                        }
                    }
                }
            });
        });
    }

    pub(super) fn wip_panel(&mut self, ui: &mut Ui, acts: &mut Vec<RepoAction>, ai_on: bool) {
        let Some(s) = self.snap.clone() else { return };
        if Self::panel_header(ui, &tf("wip.header", &[&s.wip.count().to_string(), &s.head_name])) {
            self.select(Sel::None);
            return;
        }
        let avail_h = ui.available_height();
        let list_h = (avail_h - 280.0).max(120.0);
        egui::Frame::none().inner_margin(egui::Margin::symmetric(10.0, 6.0)).show(ui, |ui| {
            if s.state != "Clean" {
                egui::Frame::none().fill(hex(0x4A3A1A)).rounding(3.0).inner_margin(6.0).show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(egui::RichText::new(format!("{} {}", ic::WARNING, tf("wip.state", &[&s.state]))).color(MOD_YELLOW));
                        if ui.button(t("wip.abort")).clicked() {
                            acts.push(RepoAction::AbortMerge);
                        }
                    });
                });
            }
            ScrollArea::vertical().max_height(list_h).auto_shrink([false, false]).id_source("wip_lists").show(ui, |ui| {
                let mut unstaged = s.wip.unstaged.clone();
                for c in &s.wip.conflicted {
                    unstaged.push(git::FileChange { path: c.clone(), old_path: None, status: 'C', additions: 0, deletions: 0 });
                }
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(tf("wip.unstaged", &[&unstaged.len().to_string()])).strong().color(TEXT_BRIGHT));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if !unstaged.is_empty() {
                            if ui.add(egui::Button::new(t("wip.stage_all")).fill(CREATE_GREEN)).clicked() {
                                acts.push(RepoAction::StageAll);
                            }
                            if ui.button(ic::TRASH).on_hover_text(t("wip.discard_all")).clicked() {
                                acts.push(RepoAction::DiscardAll);
                            }
                        }
                    });
                });
                for f in &unstaged {
                    let resp = file_row(ui, f, Some(("wip.stage_file", CREATE_GREEN)));
                    if row_action_clicked(ui, &resp) {
                        acts.push(RepoAction::Stage(vec![f.path.clone()]));
                    } else if resp.clicked() {
                        acts.push(RepoAction::ShowDiffWip(f.path.clone(), false));
                    }
                    let p = f.path.clone();
                    resp.context_menu(|ui| {
                        if ui.button(t("wip.stage_file")).clicked() {
                            acts.push(RepoAction::Stage(vec![p.clone()]));
                            ui.close_menu();
                        }
                        if ui.button(t("wip.discard_file")).clicked() {
                            acts.push(RepoAction::Discard(vec![p.clone()]));
                            ui.close_menu();
                        }
                        if ui.button(t("ctx.copy_name")).clicked() {
                            acts.push(RepoAction::Copy(p.clone()));
                            ui.close_menu();
                        }
                    });
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(tf("wip.staged", &[&s.wip.staged.len().to_string()])).strong().color(TEXT_BRIGHT));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if !s.wip.staged.is_empty() && ui.add(egui::Button::new(t("wip.unstage_all")).fill(UNSTAGE_RED)).clicked() {
                            acts.push(RepoAction::UnstageAll);
                        }
                    });
                });
                for f in &s.wip.staged {
                    let resp = file_row(ui, f, Some(("wip.unstage_file", UNSTAGE_RED)));
                    if row_action_clicked(ui, &resp) {
                        acts.push(RepoAction::Unstage(vec![f.path.clone()]));
                    } else if resp.clicked() {
                        acts.push(RepoAction::ShowDiffWip(f.path.clone(), true));
                    }
                    let p = f.path.clone();
                    resp.context_menu(|ui| {
                        if ui.button(t("wip.unstage_file")).clicked() {
                            acts.push(RepoAction::Unstage(vec![p.clone()]));
                            ui.close_menu();
                        }
                    });
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("{} {}", ic::GIT_COMMIT, t("wip.commit"))).strong().color(TEXT_BRIGHT));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let r = ui.add(egui::Button::new(format!("{} {}", ic::SPARKLE, t("wip.ai_message"))).fill(PURPLE_DEEP));
                    if r.clicked() {
                        acts.push(RepoAction::AiCommitMessage);
                    }
                    if !ai_on {
                        r.on_hover_text(t("agents.off"));
                    }
                });
            });
            ui.horizontal(|ui| {
                checkbox(ui, &mut self.amend);
                ui.label(egui::RichText::new(t("wip.amend")).size(12.5));
            });
            let w = ui.available_width();
            let resp = ui.add(
                egui::TextEdit::singleline(&mut self.summary)
                    .hint_text(t("wip.summary"))
                    .desired_width(w)
                    .margin(egui::Margin::symmetric(8.0, 6.0)),
            );
            let len = self.summary.chars().count();
            ui.painter().text(
                Pos2::new(resp.rect.right() - 6.0, resp.rect.center().y),
                Align2::RIGHT_CENTER,
                (72i64 - len as i64).to_string(),
                font(11.0),
                if len > 72 { DEL_RED } else { TEXT_FAINT },
            );
            ui.add(
                egui::TextEdit::multiline(&mut self.description)
                    .hint_text(t("wip.description"))
                    .desired_width(w)
                    .desired_rows(4)
                    .margin(egui::Margin::symmetric(8.0, 6.0)),
            );
            ui.add_space(4.0);
            let n = s.wip.staged.len();
            let can = (n > 0 || self.amend || s.state == "Merge") && !self.summary.trim().is_empty();
            let label = if n == 0 && !self.amend { t("wip.stage_first") } else { tf("wip.commit_n", &[&n.to_string()]) };
            if solid_button(ui, &label, CREATE_GREEN, Vec2::new(w, 34.0), can).clicked() {
                acts.push(RepoAction::Commit);
            }
        });
    }

    pub(super) fn ai_panel(&mut self, ui: &mut Ui, acts: &mut Vec<RepoAction>, ai_on: bool) {
        if Self::panel_header(ui, &format!("{} {}", ic::SPARKLE, t("ai.title"))) {
            self.right = RightView::Details;
            return;
        }
        let h = ui.available_height();
        egui::Frame::none().inner_margin(egui::Margin::symmetric(10.0, 6.0)).show(ui, |ui| {
            if !ai_on {
                egui::Frame::none().fill(hex(0x3A3320)).rounding(3.0).inner_margin(8.0).show(ui, |ui| {
                    ui.label(egui::RichText::new(t("ai.not_configured")).color(MOD_YELLOW));
                    if ui.button(t("agents.configure")).clicked() {
                        acts.push(RepoAction::ConfigureAi);
                    }
                });
            }
            ui.horizontal_wrapped(|ui| {
                if let Some(o) = self.selected_oid() {
                    if ui.small_button(t("agents.explain_selected")).clicked() {
                        acts.push(RepoAction::AiExplain(o));
                    }
                }
                if ui.small_button(t("agents.review_wip")).clicked() {
                    acts.push(RepoAction::AiReview);
                }
                if ui.small_button(t("ai.clear")).clicked() {
                    self.ai_msgs.clear();
                }
            });
            ScrollArea::vertical()
                .max_height((h - 160.0).max(80.0))
                .auto_shrink([false, false])
                .stick_to_bottom(true)
                .id_source("ai_chat")
                .show(ui, |ui| {
                    if self.ai_msgs.is_empty() {
                        ui.label(egui::RichText::new(t("ai.empty")).color(TEXT_DIM));
                    }
                    for m in &self.ai_msgs {
                        let user = m.role == "user";
                        let fill = if user { hex(0x2E3D5A) } else { hex(0x30333B) };
                        egui::Frame::none().fill(fill).rounding(4.0).inner_margin(8.0).show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.label(egui::RichText::new(if user { t("ai.you") } else { t("ai.assistant") }).size(11.0).color(TEXT_DIM));
                            let shown: String = if user && m.text.chars().count() > 600 {
                                format!("{}…", m.text.chars().take(600).collect::<String>())
                            } else {
                                m.text.clone()
                            };
                            ui.add(egui::Label::new(egui::RichText::new(shown).color(TEXT_BRIGHT).size(13.0)).selectable(true));
                            if !user && ui.small_button(format!("{} {}", ic::COPY, t("ai.copy"))).clicked() {
                                acts.push(RepoAction::Copy(m.text.clone()));
                            }
                        });
                        ui.add_space(6.0);
                    }
                    if self.ai_busy {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            let st = if self.ai_status.is_empty() { t("ai.thinking") } else { self.ai_status.clone() };
                            ui.label(egui::RichText::new(st).color(TEXT_DIM));
                        });
                    }
                });
            ui.separator();
            let w = ui.available_width();
            let resp = ui.add(egui::TextEdit::multiline(&mut self.ai_input).hint_text(t("ai.ask_hint")).desired_width(w).desired_rows(2));
            let send_key = resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter) && !i.modifiers.shift);
            let send = ui
                .add_enabled(!self.ai_busy, egui::Button::new(format!("{} {}", ic::PAPER_PLANE_RIGHT, t("ai.send"))).fill(PURPLE_DEEP))
                .clicked();
            if (send || send_key) && !self.ai_input.trim().is_empty() && !self.ai_busy {
                let q = self.ai_input.trim().to_string();
                self.ai_input.clear();
                acts.push(RepoAction::AiAsk(q));
            }
        });
    }

    pub(super) fn diff_view(&mut self, ui: &mut Ui, acts: &mut Vec<RepoAction>) {
        let Some(d) = self.diff.clone() else { return };
        let full = ui.max_rect();
        let hdr = Rect::from_min_size(full.min, Vec2::new(full.width(), 36.0));
        ui.painter().rect_filled(hdr, Rounding::ZERO, PANEL);
        ui.painter().text(Pos2::new(hdr.left() + 12.0, hdr.center().y), Align2::LEFT_CENTER, &d.title, bold(14.0), TEXT_BRIGHT);
        let mut bar = ui.child_ui(hdr.shrink2(Vec2::new(10.0, 4.0)), Layout::right_to_left(Align::Center), None);
        if bar.button(format!("{} {}", ic::X, t("diff.close"))).clicked() {
            self.diff = None;
            return;
        }
        match d.staged {
            Some(false) => {
                if bar.add(egui::Button::new(t("wip.stage_file")).fill(CREATE_GREEN)).clicked() {
                    acts.push(RepoAction::Stage(vec![d.file.clone()]));
                }
            }
            Some(true) => {
                if bar.add(egui::Button::new(t("wip.unstage_file")).fill(UNSTAGE_RED)).clicked() {
                    acts.push(RepoAction::Unstage(vec![d.file.clone()]));
                }
            }
            None => {}
        }
        let adds = d.lines.iter().filter(|l| l.kind == DiffLineKind::Add).count();
        let dels = d.lines.iter().filter(|l| l.kind == DiffLineKind::Del).count();
        bar.label(egui::RichText::new(format!("-{dels}")).color(DEL_RED));
        bar.label(egui::RichText::new(format!("+{adds}")).color(ADD_GREEN));

        let body = Rect::from_min_max(Pos2::new(full.left(), hdr.bottom()), full.max);
        let mut child = ui.child_ui(body, Layout::top_down(Align::Min), None);
        let lh = 18.0;
        if d.lines.is_empty() {
            child.add_space(20.0);
            child.label(egui::RichText::new(t("diff.binary_or_empty")).color(TEXT_DIM));
            return;
        }
        ScrollArea::both().auto_shrink([false, false]).id_source("diff_scroll").show_rows(&mut child, lh, d.lines.len(), |ui, range| {
            let origin = ui.min_rect().min;
            let w = body.width();
            let p = ui.painter().clone();
            let mut maxw: f32 = w;
            for i in range.clone() {
                let l = &d.lines[i];
                let y = origin.y + (i - range.start) as f32 * lh;
                let r = Rect::from_min_size(Pos2::new(origin.x, y), Vec2::new(w.max(2000.0), lh));
                let (bg, fg, sign) = match l.kind {
                    DiffLineKind::Add => (hex(0x1E3A2A), hex(0xBFE9C9), "+"),
                    DiffLineKind::Del => (hex(0x45232A), hex(0xF2C0C4), "-"),
                    DiffLineKind::Hunk => (hex(0x262B38), hex(0x8EA6D8), ""),
                    _ => (CONTENT, TEXT, " "),
                };
                p.rect_filled(r, Rounding::ZERO, bg);
                let num = |n: Option<u32>| n.map(|x| x.to_string()).unwrap_or_default();
                if l.kind != DiffLineKind::Hunk {
                    p.text(Pos2::new(r.left() + 38.0, r.center().y), Align2::RIGHT_CENTER, num(l.old_no), mono(11.5), TEXT_FAINT);
                    p.text(Pos2::new(r.left() + 78.0, r.center().y), Align2::RIGHT_CENTER, num(l.new_no), mono(11.5), TEXT_FAINT);
                    p.text(Pos2::new(r.left() + 88.0, r.center().y), Align2::LEFT_CENTER, sign, mono(12.5), fg);
                }
                let text = l.text.replace('\t', "    ");
                let tx = r.left() + if l.kind == DiffLineKind::Hunk { 10.0 } else { 102.0 };
                let tr = p.text(Pos2::new(tx, r.center().y), Align2::LEFT_CENTER, text, mono(12.5), fg);
                maxw = maxw.max(tr.right() - origin.x + 20.0);
            }
            ui.allocate_space(Vec2::new(maxw, 1.0));
        });
    }

    pub(super) fn terminal(&mut self, ui: &mut Ui, acts: &mut Vec<RepoAction>) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(format!("{} {}", ic::TERMINAL_WINDOW, t("terminal.title"))).strong());
            ui.label(egui::RichText::new(self.path.display().to_string()).color(TEXT_DIM).size(12.0));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.small_button(ic::X).clicked() {
                    self.terminal_open = false;
                }
                if ui.small_button(t("terminal.clear")).clicked() {
                    self.term_output.clear();
                }
                if ui.small_button(t("toolbar.open_external_terminal")).clicked() {
                    acts.push(RepoAction::OpenTerminal);
                }
            });
        });
        let h = ui.available_height() - 34.0;
        ScrollArea::vertical()
            .max_height(h.max(40.0))
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .id_source("term_out")
            .show(ui, |ui| {
                ui.add(egui::Label::new(egui::RichText::new(&self.term_output).font(mono(12.5)).color(hex(0xC8E1C8))).selectable(true));
            });
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("$").font(mono(13.0)).color(CHECK_GREEN));
            let w = ui.available_width();
            let resp = ui.add(egui::TextEdit::singleline(&mut self.term_input).font(mono(13.0)).desired_width(w).hint_text(t("terminal.hint")));
            if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                let line = std::mem::take(&mut self.term_input);
                if !line.trim().is_empty() {
                    acts.push(RepoAction::RunTerminal(line));
                }
                resp.request_focus();
            }
        });
    }
}

/// One file in a list: status icon, folder (dim) and name. With `action`, a small
/// button appears on hover (stage / unstage).
fn file_row(ui: &mut Ui, f: &git::FileChange, action: Option<(&str, Color32)>) -> egui::Response {
    let w = ui.available_width();
    let (r, resp) = ui.allocate_exact_size(Vec2::new(w, 24.0), Sense::click());
    let p = ui.painter();
    if resp.hovered() {
        p.rect_filled(r, Rounding::same(2.0), hex(0x323641));
    }
    let col = status_color(f.status);
    p.text(Pos2::new(r.left() + 10.0, r.center().y), Align2::CENTER_CENTER, status_icon(f.status), font(13.0), col);
    let (dir, name) = match f.path.rsplit_once('/') {
        Some((d, n)) => (format!("{d}/"), n.to_string()),
        None => (String::new(), f.path.clone()),
    };
    let mut right = r.right() - 6.0;
    if f.additions + f.deletions > 0 {
        let rr = p.text(Pos2::new(right, r.center().y), Align2::RIGHT_CENTER, format!("+{} -{}", f.additions, f.deletions), font(11.0), TEXT_DIM);
        right = rr.left() - 6.0;
    }
    if let (Some((key, fill)), true) = (action, resp.hovered()) {
        let label = t(key);
        let bw = text_width(ui, &label, font(11.5)) + 14.0;
        let br = Rect::from_min_size(Pos2::new(right - bw, r.top() + 3.0), Vec2::new(bw, 18.0));
        p.rect_filled(br, Rounding::same(2.0), fill);
        p.text(br.center(), Align2::CENTER_CENTER, label, font(11.5), TEXT_BRIGHT);
        right = br.left() - 6.0;
    }
    let x = r.left() + 22.0;
    let nw = text_width(ui, &name, font(13.0));
    let avail = right - x;
    if nw >= avail || dir.is_empty() {
        let n = elide(ui, &name, &font(13.0), avail);
        p.text(Pos2::new(x, r.center().y), Align2::LEFT_CENTER, n, font(13.0), TEXT_BRIGHT);
    } else {
        let d = elide(ui, &dir, &font(13.0), avail - nw);
        let dr = p.text(Pos2::new(x, r.center().y), Align2::LEFT_CENTER, d, font(13.0), TEXT_DIM);
        p.text(Pos2::new(dr.right(), r.center().y), Align2::LEFT_CENTER, name, font(13.0), TEXT_BRIGHT);
    }
    resp.on_hover_text(f.old_path.as_ref().map(|o| format!("{o} → {}", f.path)).unwrap_or_else(|| f.path.clone()))
}

/// True when the click landed on the hover button at the right of a file row.
fn row_action_clicked(ui: &Ui, resp: &egui::Response) -> bool {
    if !resp.clicked() {
        return false;
    }
    match ui.input(|i| i.pointer.interact_pos()) {
        Some(p) => p.x > resp.rect.right() - 110.0,
        None => false,
    }
}
