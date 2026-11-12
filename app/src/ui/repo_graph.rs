//! The commit graph: BRANCH / TAG, GRAPH and COMMIT MESSAGE columns.

use super::repo::*;
use super::theme::*;
use eframe::egui::{self, Align, Align2, Color32, Layout, Pos2, Rect, Rounding, ScrollArea, Sense, Stroke, Ui, Vec2};
use egui_phosphor::regular as ic;
use gcc_core::git::{self, RefKind};
use gcc_core::i18n::{t, tf};

impl RepoTab {
    pub(super) fn graph_view(&mut self, ui: &mut Ui, acts: &mut Vec<RepoAction>) {
        let Some(s) = self.snap.clone() else {
            ui.centered_and_justified(|ui| ui.spinner());
            return;
        };
        let full = ui.max_rect();
        let wip = !s.wip.is_empty();
        let branch_w = 140.0;
        let lanes = self.layout.max_cols.max(1) as f32;
        let graph_w = (36.0 + (lanes - 1.0) * LANE_W + 48.0).clamp(84.0, 300.0);
        let x_branch = full.left();
        let x_graph = x_branch + branch_w;
        let x_msg = x_graph + graph_w;

        self.hud.graph_header = true;
        let hdr = Rect::from_min_size(full.min, Vec2::new(full.width(), 22.0));
        let p = ui.painter().clone();
        p.rect_filled(hdr, Rounding::ZERO, PANEL);
        p.text(Pos2::new(x_branch + 16.0, hdr.center().y), Align2::LEFT_CENTER, t("graph.branch_tag"), mono(10.5), TEXT_DIM);
        p.text(Pos2::new(x_graph + 6.0, hdr.center().y), Align2::LEFT_CENTER, t("graph.graph"), mono(10.5), TEXT_DIM);
        p.text(Pos2::new(x_msg + 6.0, hdr.center().y), Align2::LEFT_CENTER, t("graph.message"), mono(10.5), TEXT_DIM);
        for x in [x_graph, x_msg] {
            p.line_segment([Pos2::new(x, hdr.top()), Pos2::new(x, hdr.bottom())], Stroke::new(1.0, CONTENT));
        }
        p.text(Pos2::new(hdr.right() - 10.0, hdr.center().y), Align2::CENTER_CENTER, ic::GEAR, font(12.0), TEXT_DIM);
        let mut y_top = hdr.bottom() + 2.0;

        if self.search_open {
            let r = Rect::from_min_size(Pos2::new(x_msg, y_top + 2.0), Vec2::new((full.right() - x_msg - 12.0).clamp(100.0, 420.0), 26.0));
            let mut child = ui.child_ui(r, Layout::left_to_right(Align::Center), None);
            let resp = input(&mut child, &mut self.search, &t("graph.search_hint"), r.width(), 26.0, INPUT);
            if resp.changed() {
                let q = self.search.to_lowercase();
                if let Some(i) = s.commits.iter().position(|c| {
                    !q.is_empty()
                        && (c.summary.to_lowercase().contains(&q) || c.author.to_lowercase().contains(&q) || c.id.to_string().starts_with(&q))
                }) {
                    self.select(Sel::Commit(i));
                    self.scroll_to = Some(i + usize::from(wip));
                }
            }
            y_top += 32.0;
        }

        let rows_rect = Rect::from_min_max(Pos2::new(full.left(), y_top), full.max);
        let mut child = ui.child_ui(rows_rect, Layout::top_down(Align::Min), None);
        let n = s.commits.len() + usize::from(wip);
        if n == 0 {
            child.add_space(30.0);
            child.vertical_centered(|ui| {
                ui.label(egui::RichText::new(t("graph.empty")).color(TEXT_DIM).size(15.0));
            });
            return;
        }
        let mut area = ScrollArea::vertical().auto_shrink([false, false]).id_source("graph_scroll");
        if let Some(target) = self.scroll_to.take() {
            let visible = rows_rect.height();
            let y = target as f32 * ROW_H;
            area = area.vertical_scroll_offset((y - visible / 2.0).max(0.0));
        }
        let search = self.search.to_lowercase();
        area.show_rows(&mut child, ROW_H, n, |ui, range| {
            let origin = ui.min_rect().min;
            let left = rows_rect.left();
            let width = rows_rect.width();
            let painter = ui.painter().clone();
            let lane_x = |k: usize| x_graph + 36.0 + k as f32 * LANE_W;
            let head_row = s.head_oid.and_then(|h| s.row_of(h));
            let mx = x_msg;
            // Pass 1: row backgrounds and lane tints, so the lines are drawn on top.
            let pointer = ui.input(|i| i.pointer.hover_pos());
            for vi in range.clone() {
                let y0 = origin.y + (vi - range.start) as f32 * ROW_H;
                let row_rect = Rect::from_min_size(Pos2::new(left, y0), Vec2::new(width, ROW_H));
                let cy = row_rect.center().y;
                let is_sel = match self.sel {
                    Sel::Wip => wip && vi == 0,
                    Sel::Commit(i) => vi == i + usize::from(wip),
                    Sel::None => false,
                };
                let hovered = pointer.map(|p| row_rect.contains(p)).unwrap_or(false) && ui.rect_contains_pointer(row_rect);
                if is_sel {
                    painter.rect_filled(row_rect, Rounding::ZERO, SELECT_ROW);
                } else if hovered {
                    painter.rect_filled(row_rect, Rounding::ZERO, hex(0x22252B));
                }
                let (col, color, amount) = if wip && vi == 0 {
                    let col = head_row.map(|r| self.layout.rows[r].col).unwrap_or(0);
                    let color = head_row.map(|r| lane_color(self.layout.rows[r].color)).unwrap_or(lane_color(0));
                    (col, color, 0.12)
                } else {
                    let rl = &self.layout.rows[vi - usize::from(wip)];
                    (rl.col, lane_color(rl.color), 0.14)
                };
                let nx = lane_x(col);
                painter.rect_filled(Rect::from_min_max(Pos2::new(nx, cy - 10.0), Pos2::new(mx - 3.0, cy + 10.0)), Rounding::ZERO, mix(CONTENT, color, amount));
                painter.rect_filled(Rect::from_min_size(Pos2::new(mx - 3.0, cy - 10.0), Vec2::new(2.0, 20.0)), Rounding::ZERO, color);
            }
            for vi in range.clone() {
                let y0 = origin.y + (vi - range.start) as f32 * ROW_H;
                let row_rect = Rect::from_min_size(Pos2::new(left, y0), Vec2::new(width, ROW_H));
                let cy = row_rect.center().y;
                let resp = ui.interact(row_rect, ui.id().with(("row", vi)), Sense::click());
                self.hud.rows_drawn += 1;
                if wip && vi == 0 {
                    let col = head_row.map(|r| self.layout.rows[r].col).unwrap_or(0);
                    let color = head_row.map(|r| lane_color(self.layout.rows[r].color)).unwrap_or(lane_color(0));
                    let nx = lane_x(col);
                    dashed_circle(&painter, Pos2::new(nx, cy), NODE_R - 1.0, color);
                    let end_y = match head_row {
                        Some(r) => y0 + (r + 1) as f32 * ROW_H + ROW_H / 2.0,
                        None => cy,
                    };
                    let mut yy = cy + NODE_R;
                    while yy < end_y - NODE_R {
                        painter.line_segment([Pos2::new(nx, yy), Pos2::new(nx, (yy + 3.0).min(end_y - NODE_R))], Stroke::new(2.0, color));
                        yy += 6.0;
                    }
                    let tr = painter.text(Pos2::new(mx + 6.0, cy), Align2::LEFT_CENTER, "// WIP", font(14.0), TEXT_DIM);
                    let mut x = tr.right() + 12.0;
                    let all: Vec<&git::FileChange> = s.wip.unstaged.iter().chain(s.wip.staged.iter()).collect();
                    for (icon, st, c) in [(ic::PENCIL_SIMPLE, 'M', MOD_YELLOW), (ic::PLUS, 'A', ADD_GREEN), (ic::MINUS, 'D', DEL_RED)] {
                        let nn = all.iter().filter(|f| f.status == st).count();
                        if nn > 0 {
                            let r = painter.text(Pos2::new(x, cy), Align2::LEFT_CENTER, format!("{icon} {nn}"), font(12.5), c);
                            x = r.right() + 10.0;
                        }
                    }
                    if resp.clicked() {
                        self.select(Sel::Wip);
                        self.right = RightView::Details;
                    }
                    continue;
                }
                let ci = vi - usize::from(wip);
                let c = &s.commits[ci];
                let rl = &self.layout.rows[ci];
                let color = lane_color(rl.color);
                let nx = lane_x(rl.col);
                for seg in &rl.down {
                    let a = Pos2::new(lane_x(seg.from), cy);
                    let b = Pos2::new(lane_x(seg.to), cy + ROW_H);
                    let st = Stroke::new(2.0, lane_color(seg.color));
                    if (a.x - b.x).abs() < 0.5 {
                        painter.line_segment([a, b], st);
                    } else {
                        let bez = egui::epaint::CubicBezierShape::from_points_stroke(
                            [a, Pos2::new(a.x, cy + ROW_H * 0.6), Pos2::new(b.x, cy + ROW_H * 0.4), b],
                            false,
                            Color32::TRANSPARENT,
                            st,
                        );
                        painter.add(bez);
                    }
                }
                if rl.is_merge {
                    painter.circle_filled(Pos2::new(nx, cy), 6.0, color);
                } else {
                    painter.circle_filled(Pos2::new(nx, cy), NODE_R, color);
                    avatar(&painter, Pos2::new(nx, cy), NODE_R - 2.0, &c.author);
                }
                if let Some(refs) = s.refs.get(&c.id) {
                    let mut refs = refs.clone();
                    refs.sort_by_key(|r| (!r.is_head, r.kind != RefKind::Local, r.kind == RefKind::Tag));
                    let first = &refs[0];
                    let pill = Rect::from_min_size(Pos2::new(left + 10.0, cy - 11.0), Vec2::new(branch_w - 46.0, 22.0));
                    let pill_col = if first.kind == RefKind::Tag { hex(0x4A4F59) } else { mix(CONTENT, color, 0.45) };
                    painter.line_segment([Pos2::new(pill.right(), cy), Pos2::new(nx - NODE_R, cy)], Stroke::new(2.0, mix(CONTENT, color, 0.8)));
                    painter.rect(pill, Rounding::same(2.0), pill_col, Stroke::new(1.0, mix(pill_col, color, 0.5)));
                    let mut tx = pill.left() + 6.0;
                    if first.is_head {
                        let r = painter.text(Pos2::new(tx, cy), Align2::LEFT_CENTER, ic::CHECK, bold(12.0), TEXT_BRIGHT);
                        tx = r.right() + 4.0;
                    }
                    let icon = match first.kind {
                        RefKind::Local => ic::LAPTOP,
                        RefKind::Remote => ic::CLOUD,
                        RefKind::Tag => ic::TAG,
                    };
                    let both = first.kind == RefKind::Local
                        && refs.iter().any(|r| r.kind == RefKind::Remote && r.name.ends_with(&format!("/{}", first.name)));
                    let icons = if both { format!("{icon} {}", ic::CLOUD) } else { icon.to_string() };
                    let iw = text_width(ui, &icons, font(12.0)) + 6.0;
                    let extra_n = refs.len().saturating_sub(1 + usize::from(both));
                    let extra = if extra_n > 0 { format!(" +{extra_n}") } else { String::new() };
                    let ew = text_width(ui, &extra, font(11.0));
                    let name = elide(ui, &first.name, &bold(12.5), pill.right() - tx - iw - ew - 6.0);
                    let r = painter.text(Pos2::new(tx, cy), Align2::LEFT_CENTER, &name, bold(12.5), TEXT_BRIGHT);
                    let r2 = painter.text(Pos2::new(r.right() + 5.0, cy), Align2::LEFT_CENTER, icons, font(12.0), TEXT);
                    if !extra.is_empty() {
                        painter.text(Pos2::new(r2.right() + 2.0, cy), Align2::LEFT_CENTER, extra, font(11.0), TEXT_DIM);
                    }
                    let all: Vec<String> = refs.iter().map(|r| r.name.clone()).collect();
                    ui.interact(pill, ui.id().with(("pill", vi)), Sense::hover()).on_hover_text(all.join("\n"));
                }
                let max_w = left + width - mx - 110.0;
                let dim = !search.is_empty() && !(c.summary.to_lowercase().contains(&search) || c.author.to_lowercase().contains(&search));
                let sc = if dim { TEXT_FAINT } else { TEXT_BRIGHT };
                let summary = elide(ui, &c.summary, &font(14.0), max_w.max(40.0));
                let r = painter.text(Pos2::new(mx + 6.0, cy), Align2::LEFT_CENTER, &summary, font(14.0), sc);
                if !c.body.is_empty() && r.right() + 40.0 < mx + 6.0 + max_w {
                    let body = elide(ui, &c.body, &font(13.5), mx + 6.0 + max_w - r.right() - 8.0);
                    painter.text(Pos2::new(r.right() + 8.0, cy), Align2::LEFT_CENTER, body, font(13.5), TEXT_DIM);
                }
                let rel = gcc_core::util::relative_time(c.time);
                let next_rel = s.commits.get(ci + 1).map(|n| gcc_core::util::relative_time(n.time));
                if ci == 0 || (next_rel.is_some() && next_rel.as_ref() != Some(&rel)) {
                    if next_rel.is_some() {
                        painter.line_segment([Pos2::new(left, row_rect.bottom()), Pos2::new(left + width, row_rect.bottom())], Stroke::new(1.0, DIVIDER));
                    }
                    let tw = text_width(ui, &rel, font(11.0));
                    let pr = Rect::from_min_size(Pos2::new(left + width - tw - 22.0, row_rect.bottom() - 7.0), Vec2::new(tw + 12.0, 15.0));
                    painter.rect_filled(pr, Rounding::same(2.0), hex(0x32363F));
                    painter.text(pr.center(), Align2::CENTER_CENTER, rel, font(11.0), hex(0x999B9F));
                }
                if resp.clicked() {
                    self.select(Sel::Commit(ci));
                    self.right = RightView::Details;
                }
                if resp.double_clicked() {
                    if let Some(l) = s.refs.get(&c.id).and_then(|v| v.iter().find(|r| r.kind != RefKind::Tag)) {
                        acts.push(RepoAction::Checkout(l.name.clone()));
                    }
                }
                let oid = c.id;
                let sha = c.id.to_string();
                let msg = c.summary.clone();
                resp.on_hover_text_at_pointer(format!("{} · {} · {}", c.author, gcc_core::util::format_date(c.time), c.short))
                    .context_menu(|ui| {
                        if ui.button(format!("{}  {}", ic::GIT_BRANCH, t("ctx.create_branch_here"))).clicked() {
                            acts.push(RepoAction::NewBranch(Some(oid)));
                            ui.close_menu();
                        }
                        if ui.button(format!("{}  {}", ic::TAG, t("ctx.create_tag_here"))).clicked() {
                            acts.push(RepoAction::Tag(oid));
                            ui.close_menu();
                        }
                        if ui.button(t("ctx.checkout_commit")).clicked() {
                            acts.push(RepoAction::CheckoutCommit(oid));
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button(t("ctx.cherry_pick")).clicked() {
                            acts.push(RepoAction::CherryPick(oid));
                            ui.close_menu();
                        }
                        if ui.button(t("ctx.revert")).clicked() {
                            acts.push(RepoAction::Revert(oid));
                            ui.close_menu();
                        }
                        ui.menu_button(t("ctx.reset_to"), |ui| {
                            for (k, m) in [("ctx.reset_soft", "soft"), ("ctx.reset_mixed", "mixed"), ("ctx.reset_hard", "hard")] {
                                if ui.button(t(k)).clicked() {
                                    acts.push(RepoAction::Reset(oid, m));
                                    ui.close_menu();
                                }
                            }
                        });
                        ui.separator();
                        if ui.button(format!("{}  {}", ic::SPARKLE, t("ctx.ai_explain"))).clicked() {
                            acts.push(RepoAction::AiExplain(oid));
                            ui.close_menu();
                        }
                        if ui.button(format!("{}  {}", ic::COPY, t("ctx.copy_sha"))).clicked() {
                            acts.push(RepoAction::Copy(sha.clone()));
                            ui.close_menu();
                        }
                        if ui.button(t("ctx.copy_message")).clicked() {
                            acts.push(RepoAction::Copy(msg.clone()));
                            ui.close_menu();
                        }
                    });
            }
        });
        if s.truncated {
            let r = Rect::from_min_size(Pos2::new(full.right() - 260.0, full.bottom() - 26.0), Vec2::new(250.0, 20.0));
            ui.painter().text(r.center(), Align2::CENTER_CENTER, tf("graph.truncated", &[&s.commits.len().to_string()]), font(11.5), TEXT_DIM);
        }
    }
}

fn dashed_circle(p: &egui::Painter, c: Pos2, r: f32, color: Color32) {
    let n = 16;
    for i in (0..n).step_by(2) {
        let a0 = i as f32 / n as f32 * std::f32::consts::TAU;
        let a1 = (i + 1) as f32 / n as f32 * std::f32::consts::TAU;
        p.line_segment([c + Vec2::angled(a0) * r, c + Vec2::angled(a1) * r], Stroke::new(2.0, color));
    }
}
