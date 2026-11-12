//! Left panel of a repository tab: List / Agents switch, filter and the
//! LOCAL, REMOTE, STASHES, TAGS, SUBMODULES and WORKTREES sections.

use super::repo::*;
use super::theme::*;
use eframe::egui::{self, Align2, Color32, CursorIcon, Pos2, Rect, Rounding, ScrollArea, Sense, Stroke, Ui, Vec2};
use egui_phosphor::regular as ic;
use gcc_core::i18n::{t, tf};

impl RepoTab {
    pub(super) fn left_panel(&mut self, ui: &mut Ui, acts: &mut Vec<RepoAction>, ai_on: bool) {
        self.hud.left_panel = true;
        let rect = ui.max_rect();
        let p = ui.painter().clone();
        p.line_segment([rect.right_top(), rect.right_bottom()], Stroke::new(1.0, CONTENT));
        let c = Pos2::new(rect.left() + 18.0, rect.top() + 18.0);
        let col_resp = ui.interact(Rect::from_center_size(c, Vec2::splat(20.0)), ui.id().with("collapse"), Sense::click());
        p.circle_filled(c, 9.0, hex(0xC9CCD1));
        p.text(c, Align2::CENTER_CENTER, ic::CARET_LEFT, bold(12.0), PANEL);
        if col_resp.clicked() {
            self.left_open = false;
        }
        let seg = Rect::from_min_max(Pos2::new(rect.left() + 36.0, rect.top() + 7.0), Pos2::new(rect.right() - 9.0, rect.top() + 30.0));
        p.rect(seg, Rounding::same(3.0), PANEL, Stroke::new(1.0, hex(0x4A4F59)));
        let half = seg.width() * 0.5;
        let lr = Rect::from_min_size(seg.min, Vec2::new(half, seg.height()));
        let ar = Rect::from_min_size(Pos2::new(seg.left() + half, seg.top()), Vec2::new(half, seg.height()));
        for (r, tab, icon, key) in [(lr, LeftTab::List, ic::LIST_BULLETS, "left.list"), (ar, LeftTab::Agents, ic::ROBOT, "left.agents")] {
            let resp = ui.interact(r, ui.id().with(key), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
            if self.left_tab == tab {
                p.rect(r.shrink(1.0), Rounding::same(2.0), SELECT, Stroke::new(1.0, hex(0x5A7BB5)));
            }
            p.text(r.center(), Align2::CENTER_CENTER, format!("{icon} {}", t(key)), font(13.0), TEXT_BRIGHT);
            if resp.clicked() {
                self.left_tab = tab;
                if tab == LeftTab::Agents {
                    self.right = RightView::Ai;
                }
            }
        }
        ui.add_space(38.0);
        if self.left_tab == LeftTab::Agents {
            self.agents_tab(ui, acts, ai_on);
            return;
        }
        let Some(s) = self.snap.clone() else { return };
        ui.horizontal(|ui| {
            ui.add_space(12.0);
            ui.label(egui::RichText::new(t("left.viewing")).size(13.5).color(TEXT));
            ui.label(egui::RichText::new(s.commits.len().to_string()).size(13.5).color(LINK_SOFT));
        });
        ui.add_space(2.0);
        let filter_rect = ui
            .horizontal(|ui| {
                ui.add_space(9.0);
                let w = ui.available_width() - 10.0;
                input(ui, &mut self.filter, &t("left.filter"), w, 22.0, INPUT);
            })
            .response
            .rect;
        p.text(Pos2::new(rect.right() - 22.0, filter_rect.center().y), Align2::CENTER_CENTER, ic::MAGNIFYING_GLASS, font(14.0), TEXT_DIM);
        ui.add_space(8.0);
        let filter = self.filter.to_lowercase();
        let matches = |n: &str| filter.is_empty() || n.to_lowercase().contains(&filter);
        let wip_off = usize::from(!s.wip.is_empty());

        ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let local: Vec<_> = s.local.iter().filter(|b| matches(&b.name)).collect();
            if self.section(ui, "local", ic::LAPTOP, &t("left.local"), local.len()) {
                for b in local {
                    let resp = self.item_row(ui, 1, ic::GIT_BRANCH, &b.name, b.is_head, b.ahead, b.behind);
                    if resp.double_clicked() && !b.is_head {
                        acts.push(RepoAction::Checkout(b.name.clone()));
                    }
                    if resp.clicked() {
                        if let Some(i) = b.oid.and_then(|o| s.row_of(o)) {
                            self.select(Sel::Commit(i));
                            self.scroll_to = Some(i + wip_off);
                        }
                    }
                    let name = b.name.clone();
                    let head = b.is_head;
                    let oid = b.oid;
                    resp.context_menu(|ui| {
                        if !head && ui.button(tf("ctx.checkout", &[&name])).clicked() {
                            acts.push(RepoAction::Checkout(name.clone()));
                            ui.close_menu();
                        }
                        if !head && ui.button(tf("ctx.merge_into_current", &[&name])).clicked() {
                            acts.push(RepoAction::Merge(name.clone()));
                            ui.close_menu();
                        }
                        if ui.button(t("ctx.create_branch_here")).clicked() {
                            acts.push(RepoAction::NewBranch(oid));
                            ui.close_menu();
                        }
                        if ui.button(t("ctx.rename_branch")).clicked() {
                            acts.push(RepoAction::RenameBranch(name.clone()));
                            ui.close_menu();
                        }
                        if !head && ui.button(tf("ctx.delete_branch", &[&name])).clicked() {
                            acts.push(RepoAction::DeleteBranch(name.clone()));
                            ui.close_menu();
                        }
                        if ui.button(t("ctx.copy_name")).clicked() {
                            acts.push(RepoAction::Copy(name.clone()));
                            ui.close_menu();
                        }
                    });
                }
            }
            let total_remote: usize = s.remotes.iter().map(|r| r.branches.len()).sum();
            if self.section(ui, "remote", ic::CLOUD, &t("left.remote"), total_remote) {
                for r in &s.remotes {
                    let resp = self.item_row(ui, 1, ic::GLOBE, &r.name, false, 0, 0).on_hover_text(&r.url);
                    let rn = r.name.clone();
                    resp.context_menu(|ui| {
                        if ui.button(tf("ctx.fetch_remote", &[&rn])).clicked() {
                            acts.push(RepoAction::Fetch);
                            ui.close_menu();
                        }
                        if ui.button(t("ctx.remove_remote")).clicked() {
                            acts.push(RepoAction::RemoveRemote(rn.clone()));
                            ui.close_menu();
                        }
                    });
                    let mut last_folder: Option<String> = None;
                    for b in r.branches.iter().filter(|b| matches(&b.name)) {
                        let (folder, leaf) = match b.name.split_once('/') {
                            Some((f, l)) => (Some(f.to_string()), l.to_string()),
                            None => (None, b.name.clone()),
                        };
                        let mut depth = 2;
                        if let Some(f) = &folder {
                            let key = format!("{}/{}", r.name, f);
                            if last_folder.as_ref() != Some(f) {
                                let open = !self.closed_folders.contains(&key);
                                let fr = self.item_row(ui, 2, if open { ic::FOLDER_OPEN } else { ic::FOLDER_SIMPLE }, f, false, 0, 0);
                                if fr.clicked() {
                                    if open {
                                        self.closed_folders.insert(key.clone());
                                    } else {
                                        self.closed_folders.remove(&key);
                                    }
                                }
                                last_folder = Some(f.clone());
                            }
                            if self.closed_folders.contains(&key) {
                                continue;
                            }
                            depth = 3;
                        }
                        let full = format!("{}/{}", r.name, b.name);
                        let resp = self.item_row(ui, depth, ic::GIT_BRANCH, &leaf, false, 0, 0).on_hover_text(&full);
                        if resp.double_clicked() {
                            acts.push(RepoAction::Checkout(full.clone()));
                        }
                        if resp.clicked() {
                            if let Some(i) = b.oid.and_then(|o| s.row_of(o)) {
                                self.select(Sel::Commit(i));
                                self.scroll_to = Some(i + wip_off);
                            }
                        }
                        resp.context_menu(|ui| {
                            if ui.button(tf("ctx.checkout", &[&full])).clicked() {
                                acts.push(RepoAction::Checkout(full.clone()));
                                ui.close_menu();
                            }
                            if ui.button(tf("ctx.merge_into_current", &[&full])).clicked() {
                                acts.push(RepoAction::Merge(full.clone()));
                                ui.close_menu();
                            }
                        });
                    }
                }
            }
            if self.section(ui, "stashes", ic::ARCHIVE, &t("left.stashes"), s.stashes.len()) {
                for st in &s.stashes {
                    let resp = self.item_row(ui, 1, ic::ARCHIVE, &st.message, false, 0, 0);
                    let i = st.index;
                    resp.context_menu(|ui| {
                        if ui.button(t("ctx.apply_stash")).clicked() {
                            acts.push(RepoAction::ApplyStash(i));
                            ui.close_menu();
                        }
                        if ui.button(t("ctx.pop_stash")).clicked() {
                            acts.push(RepoAction::Pop(i));
                            ui.close_menu();
                        }
                        if ui.button(t("ctx.drop_stash")).clicked() {
                            acts.push(RepoAction::DropStash(i));
                            ui.close_menu();
                        }
                    });
                }
            }
            let tags: Vec<_> = s.tags.iter().filter(|(n, _)| matches(n)).collect();
            if self.section(ui, "tags", ic::TAG, &t("left.tags"), tags.len()) {
                for (n, o) in tags {
                    let resp = self.item_row(ui, 1, ic::TAG, n, false, 0, 0);
                    if resp.clicked() {
                        if let Some(i) = s.row_of(*o) {
                            self.select(Sel::Commit(i));
                            self.scroll_to = Some(i + wip_off);
                        }
                    }
                    let nn = n.clone();
                    resp.context_menu(|ui| {
                        if ui.button(t("ctx.delete_tag")).clicked() {
                            acts.push(RepoAction::DeleteTag(nn.clone()));
                            ui.close_menu();
                        }
                    });
                }
            }
            if self.section(ui, "submodules", ic::CUBE, &t("left.submodules"), s.submodules.len()) {
                for (n, _) in &s.submodules {
                    self.item_row(ui, 1, ic::CUBE, n, false, 0, 0);
                }
            }
            if self.section(ui, "worktrees", ic::TREE_STRUCTURE, &t("left.worktrees"), s.worktrees.len()) {
                for n in &s.worktrees {
                    self.item_row(ui, 1, ic::TREE_STRUCTURE, n, false, 0, 0);
                }
            }
            let w = ui.available_width();
            let (r, _) = ui.allocate_exact_size(Vec2::new(w, 1.0), Sense::hover());
            ui.painter().line_segment([r.left_top(), r.right_top()], Stroke::new(1.0, SECTION_LINE));
        });
    }

    fn section(&mut self, ui: &mut Ui, key: &str, icon: &str, title: &str, count: usize) -> bool {
        let w = ui.available_width();
        let (r, resp) = ui.allocate_exact_size(Vec2::new(w, 30.0), Sense::click());
        let open = !self.closed_sections.contains(key);
        let p = ui.painter();
        p.line_segment([r.left_top(), r.right_top()], Stroke::new(1.0, SECTION_LINE));
        if resp.hovered() {
            p.rect_filled(r.shrink(1.0), Rounding::ZERO, hex(0x2D3038));
        }
        p.text(Pos2::new(r.left() + 12.0, r.center().y), Align2::CENTER_CENTER, if open { ic::CARET_DOWN } else { ic::CARET_RIGHT }, font(13.0), TEXT);
        p.text(Pos2::new(r.left() + 31.0, r.center().y), Align2::CENTER_CENTER, icon, font(14.0), TEXT_DIM);
        p.text(Pos2::new(r.left() + 43.0, r.center().y), Align2::LEFT_CENTER, title, bold(12.5), hex(0xC8CCD2));
        p.text(Pos2::new(r.right() - 12.0, r.center().y), Align2::RIGHT_CENTER, count.to_string(), bold(12.5), hex(0x8EA6D8));
        if resp.clicked() {
            if open {
                self.closed_sections.insert(key.to_string());
            } else {
                self.closed_sections.remove(key);
            }
        }
        if open {
            ui.add_space(2.0);
        }
        open
    }

    #[allow(clippy::too_many_arguments)]
    fn item_row(&self, ui: &mut Ui, depth: usize, icon: &str, label: &str, head: bool, ahead: usize, behind: usize) -> egui::Response {
        let w = ui.available_width();
        let (r, resp) = ui.allocate_exact_size(Vec2::new(w, 24.0), Sense::click());
        let p = ui.painter();
        if head {
            p.rect_filled(r, Rounding::ZERO, HEAD_GREEN);
            let cb = Rect::from_center_size(Pos2::new(r.left() + 11.0, r.center().y), Vec2::splat(13.0));
            p.rect_filled(cb, Rounding::same(2.0), CHECK_GREEN);
            p.text(cb.center(), Align2::CENTER_CENTER, ic::CHECK, bold(10.0), Color32::WHITE);
        } else if resp.hovered() {
            p.rect_filled(r, Rounding::ZERO, hex(0x323641));
        }
        let x = r.left() + 10.0 + depth as f32 * 14.0;
        p.text(Pos2::new(x, r.center().y), Align2::CENTER_CENTER, icon, font(13.0), if head { TEXT } else { TEXT_DIM });
        let mut right = r.right() - 8.0;
        if behind > 0 {
            let rr = p.text(Pos2::new(right, r.center().y), Align2::RIGHT_CENTER, format!("{}{behind}", ic::ARROW_LINE_DOWN), font(11.5), TEXT_DIM);
            right = rr.left() - 4.0;
        }
        if ahead > 0 {
            let rr = p.text(Pos2::new(right, r.center().y), Align2::RIGHT_CENTER, format!("{}{ahead}", ic::ARROW_LINE_UP), font(11.5), TEXT_DIM);
            right = rr.left() - 4.0;
        }
        let f = if head { bold(13.5) } else { font(13.5) };
        let max = right - (x + 12.0) - 4.0;
        let text = elide(ui, label, &f, max);
        p.text(Pos2::new(x + 12.0, r.center().y), Align2::LEFT_CENTER, text, f, if head { TEXT_BRIGHT } else { TEXT });
        resp.on_hover_cursor(CursorIcon::PointingHand)
    }

    fn agents_tab(&mut self, ui: &mut Ui, acts: &mut Vec<RepoAction>, ai_on: bool) {
        ui.add_space(4.0);
        egui::Frame::none().inner_margin(egui::Margin::symmetric(12.0, 4.0)).show(ui, |ui| {
            ui.label(egui::RichText::new(t("agents.title")).size(15.0).color(TEXT_BRIGHT));
            ui.add_space(4.0);
            let (c, k) = if ai_on { (CHECK_GREEN, "agents.ready") } else { (MOD_YELLOW, "agents.off") };
            ui.label(egui::RichText::new(format!("● {}", t(k))).color(c).size(12.5));
            ui.add_space(8.0);
            let w = ui.available_width();
            let b = |ui: &mut Ui, icon: &str, key: &str| ui.add_sized([w, 28.0], egui::Button::new(format!("{icon}  {}", t(key))));
            if b(ui, ic::CHAT_CIRCLE_TEXT, "agents.open_chat").clicked() {
                self.right = RightView::Ai;
            }
            if let Some(o) = self.selected_oid() {
                if b(ui, ic::SPARKLE, "agents.explain_selected").clicked() {
                    acts.push(RepoAction::AiExplain(o));
                }
            }
            if b(ui, ic::SHIELD_CHECK, "agents.review_wip").clicked() {
                acts.push(RepoAction::AiReview);
            }
            if b(ui, ic::PENCIL_SIMPLE, "agents.commit_message").clicked() {
                acts.push(RepoAction::AiCommitMessage);
            }
            ui.add_space(8.0);
            if b(ui, ic::GEAR, "agents.configure").clicked() {
                acts.push(RepoAction::ConfigureAi);
            }
            ui.add_space(10.0);
            ui.label(egui::RichText::new(t("agents.hint")).size(12.0).color(TEXT_DIM));
        });
    }
}
