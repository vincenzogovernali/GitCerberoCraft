//! Window chrome: menu bar, tab bar with its right-hand buttons, the profile,
//! notification and tab-list popups, the status bar and the toasts.

use super::theme::*;
use super::*;
use eframe::egui::{self, Align2, CursorIcon, Pos2, Rect, Rounding, Sense, Stroke, Vec2};
use egui_phosphor::regular as ic;
use gcc_core::config::AiMode;
use gcc_core::i18n::{t, tf};

impl App {
    pub(super) fn menu_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu")
            .exact_height(24.0)
            .frame(egui::Frame::none().fill(MENU_BAR).inner_margin(egui::Margin::symmetric(2.0, 0.0)))
            .show(ctx, |ui| {
                ui.style_mut().visuals.widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
                ui.style_mut().spacing.button_padding = Vec2::new(6.0, 2.0);
                ui.style_mut().spacing.interact_size.y = 22.0;
                egui::menu::bar(ui, |ui| {
                    ui.style_mut().override_font_id = Some(font(13.0));
                    ui.menu_button(egui::RichText::new(t("menu.file")).color(TEXT_BRIGHT), |ui| {
                        ui.set_min_width(380.0);
                        self.menu_item(ui, "menu.new_tab", "Ctrl+T", |a| a.new_tab());
                        self.menu_item(ui, "menu.close_tab", "Ctrl+W", |a| a.close_tab(a.active));
                        self.menu_item(ui, "menu.reopen_tab", "Ctrl+Shift+T", |a| a.reopen_closed());
                        ui.separator();
                        self.menu_item(ui, "menu.clone_repo", "Ctrl+N", |a| a.screen = Screen::Clone);
                        self.menu_item(ui, "menu.init_repo", "Ctrl+I", |a| a.modal = Some(screens::new_init_form(&a.settings)));
                        self.menu_item(ui, "menu.open_repo", "Ctrl+O", |a| a.browse_open_repo());
                        self.menu_item(ui, "menu.repo_management", "Alt+Ctrl+O", |a| {
                            a.add_tab(TabKind::Launchpad, true);
                        });
                        self.menu_item(ui, "menu.open_editor", "Ctrl+Shift+E", |a| {
                            if let Some(id) = a.active_repo_id() {
                                actions::handle(a, id, repo::RepoAction::OpenEditor);
                            }
                        });
                        ui.separator();
                        self.menu_item(ui, "menu.open_terminal", "Alt+T", |a| {
                            if let Some(id) = a.active_repo_id() {
                                actions::handle(a, id, repo::RepoAction::OpenTerminal);
                            }
                        });
                        self.menu_item(ui, "menu.open_file_manager", "Alt+O", |a| {
                            if let Some(id) = a.active_repo_id() {
                                actions::handle(a, id, repo::RepoAction::OpenFolder);
                            }
                        });
                        ui.separator();
                        self.menu_item(ui, "menu.preferences", "Ctrl+,", |a| a.open_prefs(PrefPage::General));
                        ui.separator();
                        self.menu_item(ui, "menu.check_git", "", |a| a.open_prefs(PrefPage::Git));
                        self.menu_item(ui, "menu.exit", "Alt+F4", |a| a.ctx.send_viewport_cmd(egui::ViewportCommand::Close));
                    });
                    ui.menu_button(egui::RichText::new(t("menu.edit")).color(TEXT_BRIGHT), |ui| {
                        ui.set_min_width(150.0);
                        self.menu_item(ui, "menu.undo", "Ctrl+Z", |a| {
                            if let Some(id) = a.active_repo_id() {
                                actions::handle(a, id, repo::RepoAction::Undo);
                            }
                        });
                        self.menu_item(ui, "menu.redo", "Ctrl+Y", |a| {
                            if let Some(id) = a.active_repo_id() {
                                actions::handle(a, id, repo::RepoAction::Redo);
                            }
                        });
                        ui.separator();
                        self.menu_item(ui, "menu.cut", "Ctrl+X", |a| a.ctx.send_viewport_cmd(egui::ViewportCommand::RequestCut));
                        self.menu_item(ui, "menu.copy", "Ctrl+C", |a| a.ctx.send_viewport_cmd(egui::ViewportCommand::RequestCopy));
                        self.menu_item(ui, "menu.paste", "Ctrl+V", |a| a.ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste));
                        self.menu_item(ui, "menu.select_all", "Ctrl+A", |_| {});
                    });
                    ui.menu_button(egui::RichText::new(t("menu.view")).color(TEXT_BRIGHT), |ui| {
                        ui.set_min_width(260.0);
                        self.menu_item(ui, "menu.toggle_left", "", |a| {
                            if let Some(TabKind::Repo(r)) = a.tabs.get_mut(a.active).map(|t| &mut t.kind) {
                                r.left_open = !r.left_open;
                            }
                        });
                        self.menu_item(ui, "menu.toggle_terminal", "", |a| {
                            if let Some(TabKind::Repo(r)) = a.tabs.get_mut(a.active).map(|t| &mut t.kind) {
                                r.terminal_open = !r.terminal_open;
                            }
                        });
                        self.menu_item(ui, "menu.toggle_ai", "", |a| {
                            if let Some(TabKind::Repo(r)) = a.tabs.get_mut(a.active).map(|t| &mut t.kind) {
                                r.right = if r.right == repo::RightView::Ai { repo::RightView::Details } else { repo::RightView::Ai };
                            }
                        });
                        ui.separator();
                        self.menu_item(ui, "menu.zoom_in", "Ctrl+=", |a| a.set_zoom(a.settings.zoom + 0.1));
                        self.menu_item(ui, "menu.zoom_out", "Ctrl+-", |a| a.set_zoom(a.settings.zoom - 0.1));
                        self.menu_item(ui, "menu.zoom_reset", "Ctrl+0", |a| a.set_zoom(1.0));
                        ui.separator();
                        self.menu_item(ui, "menu.fullscreen", "", |a| {
                            let fs = a.ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
                            a.ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!fs));
                        });
                    });
                    ui.menu_button(egui::RichText::new(t("menu.help")).color(TEXT_BRIGHT), |ui| {
                        ui.set_min_width(240.0);
                        self.menu_item(ui, "menu.guide", "", |a| {
                            a.add_tab(TabKind::Guide, true);
                        });
                        self.menu_item(ui, "menu.shortcuts", "", |a| a.modal = Some(Modal::Shortcuts));
                        self.menu_item(ui, "menu.release_notes", "", |a| {
                            a.add_tab(TabKind::ReleaseNotes, true);
                        });
                        self.menu_item(ui, "menu.languages_folder", "", |_| {
                            let d = gcc_core::i18n::user_lang_dir();
                            let _ = std::fs::create_dir_all(&d);
                            gcc_core::util::open_path(&d);
                        });
                        ui.separator();
                        self.menu_item(ui, "menu.about", "", |a| a.modal = Some(Modal::About));
                    });
                });
            });
    }

    fn menu_item(&mut self, ui: &mut egui::Ui, key: &str, shortcut: &str, f: impl FnOnce(&mut Self)) {
        if ui.add(egui::Button::new(t(key)).shortcut_text(shortcut)).clicked() {
            f(self);
            ui.close_menu();
        }
    }

    pub(super) fn tab_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("tabs")
            .exact_height(36.0)
            .frame(egui::Frame::none().fill(TAB_BAR))
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                let p = ui.painter().clone();
                let fr = Rect::from_min_size(rect.min, Vec2::new(36.0, rect.height()));
                let fresp = ui.interact(fr, ui.id().with("folder"), Sense::click()).on_hover_text(t("menu.repo_management"));
                if fresp.hovered() {
                    p.rect_filled(fr, Rounding::ZERO, hex(0x33373F));
                }
                p.text(fr.center(), Align2::CENTER_CENTER, ic::FOLDER_SIMPLE, font(17.0), TEXT);
                if fresp.clicked() {
                    self.new_tab();
                }
                let mut x = fr.right() + 4.0;
                let mut close: Option<usize> = None;
                let mut activate: Option<usize> = None;
                let right_limit = rect.right() - 260.0;
                for (i, tab) in self.tabs.iter().enumerate() {
                    let title = tab.title();
                    let icon = tab.icon();
                    let active = i == self.active && self.screen != Screen::Prefs;
                    let f = bold(15.0);
                    let tw = text_width(ui, &title, f.clone());
                    let iw = if icon.is_empty() { 0.0 } else { 18.0 };
                    let cw = if active { 20.0 } else { 0.0 };
                    let w = (tw + iw + cw + 40.0).clamp(80.0, 230.0);
                    if x + w > right_limit {
                        break;
                    }
                    let r = Rect::from_min_size(Pos2::new(x, rect.top()), Vec2::new(w, rect.height()));
                    let resp = ui.interact(r, ui.id().with(("tab", tab.id)), Sense::click());
                    if active {
                        p.rect_filled(r, Rounding::ZERO, TAB_ACTIVE);
                    } else if resp.hovered() {
                        p.rect_filled(r, Rounding::ZERO, hex(0x30333B));
                    }
                    let col = if active { TEXT_BRIGHT } else { hex(0xA3A7AE) };
                    let mut tx = r.left() + 20.0;
                    if !icon.is_empty() {
                        p.text(Pos2::new(tx, r.center().y), Align2::LEFT_CENTER, icon, font(13.0), col);
                        tx += iw;
                    }
                    let name = elide(ui, &title, &f, r.right() - tx - cw - 14.0);
                    p.text(Pos2::new(tx, r.center().y), Align2::LEFT_CENTER, name, f, col);
                    if active {
                        let cr = Rect::from_center_size(Pos2::new(r.right() - 16.0, r.center().y), Vec2::splat(16.0));
                        let cresp = ui.interact(cr, ui.id().with(("close", tab.id)), Sense::click());
                        p.text(cr.center(), Align2::CENTER_CENTER, ic::X, font(13.0), if cresp.hovered() { TEXT_BRIGHT } else { TEXT });
                        if cresp.clicked() {
                            close = Some(i);
                        }
                    }
                    if resp.clicked() {
                        activate = Some(i);
                    }
                    if resp.middle_clicked() {
                        close = Some(i);
                    }
                    let next_active = i + 1 == self.active;
                    if !active && !next_active {
                        p.line_segment(
                            [Pos2::new(r.right(), r.top() + 8.0), Pos2::new(r.right(), r.bottom() - 8.0)],
                            Stroke::new(1.0, hex(0x40444C)),
                        );
                    }
                    x += w;
                }
                let pr = Rect::from_min_size(Pos2::new(x + 2.0, rect.top()), Vec2::new(30.0, rect.height()));
                let presp = ui.interact(pr, ui.id().with("plus"), Sense::click()).on_hover_text(t("menu.new_tab"));
                p.text(pr.center(), Align2::CENTER_CENTER, ic::PLUS, font(16.0), if presp.hovered() { TEXT_BRIGHT } else { TEXT });
                if presp.clicked() {
                    self.new_tab();
                }
                if let Some(i) = activate {
                    self.active = i;
                    if self.screen == Screen::Prefs {
                        self.screen = Screen::Main;
                    }
                }
                if let Some(i) = close {
                    self.close_tab(i);
                }

                let mut rx = rect.right() - 10.0;
                let prof_label = self.settings.profile().name.clone();
                let lw = text_width(ui, &prof_label, font(13.5));
                let pw = lw + 46.0;
                let prr = Rect::from_min_size(Pos2::new(rx - pw, rect.top() + 4.0), Vec2::new(pw, rect.height() - 8.0));
                let prresp = ui.interact(prr, ui.id().with("profile"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
                p.rect_filled(prr, Rounding::same(3.0), if prresp.hovered() || self.profile_open { hex(0x3A4252) } else { hex(0x2F3644) });
                pixel_avatar(&p, Rect::from_center_size(Pos2::new(prr.left() + 14.0, prr.center().y), Vec2::splat(20.0)));
                p.text(Pos2::new(prr.left() + 28.0, prr.center().y), Align2::LEFT_CENTER, &prof_label, font(13.5), TEXT_BRIGHT);
                p.text(Pos2::new(prr.right() - 8.0, prr.center().y), Align2::CENTER_CENTER, "▾", font(11.0), TEXT_BRIGHT);
                if prresp.clicked() {
                    self.profile_open = !self.profile_open;
                    self.bell_open = false;
                    self.tabs_open = false;
                }
                rx = prr.left() - 8.0;
                let gr = Rect::from_center_size(Pos2::new(rx - 12.0, rect.center().y), Vec2::splat(26.0));
                let gresp = ui.interact(gr, ui.id().with("gear"), Sense::click()).on_hover_text(t("menu.preferences"));
                p.text(gr.center(), Align2::CENTER_CENTER, ic::GEAR, font(19.0), if gresp.hovered() { TEXT_BRIGHT } else { TEXT });
                if gresp.clicked() {
                    self.open_prefs(PrefPage::General);
                }
                rx = gr.left() - 6.0;
                let br = Rect::from_center_size(Pos2::new(rx - 12.0, rect.center().y), Vec2::splat(26.0));
                let bresp = ui.interact(br, ui.id().with("bell"), Sense::click()).on_hover_text(t("notify.title"));
                p.text(br.center(), Align2::CENTER_CENTER, ic::BELL, font(19.0), if bresp.hovered() { TEXT_BRIGHT } else { TEXT });
                if self.unread > 0 {
                    let bc = br.center() + Vec2::new(7.0, -7.0);
                    p.circle_filled(bc, 6.5, BADGE_BLUE);
                    p.text(bc, Align2::CENTER_CENTER, self.unread.min(9).to_string(), bold(9.0), egui::Color32::WHITE);
                }
                if bresp.clicked() {
                    self.bell_open = !self.bell_open;
                    self.profile_open = false;
                    self.tabs_open = false;
                    self.unread = 0;
                }
                rx = br.left() - 8.0;
                let dr = Rect::from_center_size(Pos2::new(rx - 12.0, rect.center().y), Vec2::splat(26.0));
                let dresp = ui.interact(dr, ui.id().with("tablist"), Sense::click()).on_hover_text(t("tabs.all"));
                p.text(dr.center(), Align2::CENTER_CENTER, ic::CARET_DOWN, font(18.0), if dresp.hovered() { TEXT_BRIGHT } else { TEXT });
                if dresp.clicked() {
                    self.tabs_open = !self.tabs_open;
                    self.bell_open = false;
                    self.profile_open = false;
                }
                let bottom = rect.bottom();
                self.popups(ctx, Pos2::new(dr.right(), bottom), Pos2::new(br.right() + 40.0, bottom), Pos2::new(prr.right(), bottom), bottom);
            });
    }

    fn popups(&mut self, ctx: &egui::Context, tabs_at: Pos2, bell_at: Pos2, prof_at: Pos2, bar_bottom: f32) {
        // A press below the tab bar and outside the popup closes it.
        let outside = |ctx: &egui::Context, r: Rect| {
            ctx.input(|i| {
                i.pointer.any_pressed() && i.pointer.interact_pos().map(|p| !r.contains(p) && p.y > bar_bottom).unwrap_or(false)
            })
        };
        if self.tabs_open {
            let area = egui::Area::new(egui::Id::new("tabs_popup"))
                .order(egui::Order::Foreground)
                .pivot(Align2::RIGHT_TOP)
                .fixed_pos(tabs_at)
                .show(ctx, |ui| {
                    egui::Frame::popup(ui.style()).fill(POPUP).show(ui, |ui| {
                        ui.set_min_width(240.0);
                        let mut go = None;
                        for (i, tab) in self.tabs.iter().enumerate() {
                            if ui.selectable_label(i == self.active, format!("{} {}", tab.icon(), tab.title())).clicked() {
                                go = Some(i);
                            }
                        }
                        if let Some(i) = go {
                            self.active = i;
                            self.tabs_open = false;
                            self.screen = Screen::Main;
                        }
                    });
                });
            if outside(ctx, area.response.rect) {
                self.tabs_open = false;
            }
        }
        if self.bell_open {
            let area = egui::Area::new(egui::Id::new("bell_popup"))
                .order(egui::Order::Foreground)
                .pivot(Align2::RIGHT_TOP)
                .fixed_pos(bell_at)
                .show(ctx, |ui| {
                    egui::Frame::popup(ui.style()).fill(POPUP).show(ui, |ui| {
                        ui.set_width(360.0);
                        ui.label(egui::RichText::new(t("notify.title")).font(bold(15.0)).color(TEXT_BRIGHT));
                        ui.separator();
                        if self.notifications.is_empty() {
                            ui.label(egui::RichText::new(t("notify.none")).color(TEXT_DIM));
                        }
                        egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                            for (n, err) in &self.notifications {
                                let c = if *err { DEL_RED } else { TEXT };
                                ui.label(egui::RichText::new(format!("{} {}", if *err { ic::WARNING } else { ic::INFO }, n)).color(c));
                                ui.add_space(2.0);
                            }
                        });
                        if !self.notifications.is_empty() && ui.small_button(t("notify.clear")).clicked() {
                            self.notifications.clear();
                        }
                    });
                });
            if outside(ctx, area.response.rect) {
                self.bell_open = false;
            }
        }
        if self.profile_open {
            let area = egui::Area::new(egui::Id::new("profile_popup"))
                .order(egui::Order::Foreground)
                .pivot(Align2::RIGHT_TOP)
                .fixed_pos(prof_at + Vec2::new(10.0, 0.0))
                .show(ctx, |ui| self.profile_popup(ui));
            if outside(ctx, area.response.rect) {
                self.profile_open = false;
            }
        }
    }

    fn profile_popup(&mut self, ui: &mut egui::Ui) {
        egui::Frame::none()
            .fill(POPUP)
            .rounding(Rounding::same(3.0))
            .shadow(ui.style().visuals.popup_shadow)
            .inner_margin(egui::Margin::symmetric(0.0, 8.0))
            .show(ui, |ui| {
                ui.set_width(330.0);
                egui::Frame::none().inner_margin(egui::Margin::symmetric(20.0, 4.0)).show(ui, |ui| {
                    ui.label(egui::RichText::new(t("profile.current")).font(bold(14.5)).color(TEXT_BRIGHT));
                });
                ui.separator();
                let mut switch_to = None;
                for (i, pr) in self.settings.profiles.iter().enumerate() {
                    egui::Frame::none().inner_margin(egui::Margin::symmetric(20.0, 2.0)).show(ui, |ui| {
                        let w = ui.available_width();
                        let (r, resp) = ui.allocate_exact_size(Vec2::new(w, 38.0), Sense::click());
                        let sel = i == self.settings.active_profile;
                        if sel {
                            ui.painter().rect_filled(r, Rounding::ZERO, hex(0x464B56));
                        } else if resp.hovered() {
                            ui.painter().rect_filled(r, Rounding::ZERO, hex(0x434852));
                        }
                        pixel_avatar(ui.painter(), Rect::from_center_size(Pos2::new(r.left() + 24.0, r.center().y), Vec2::splat(26.0)));
                        ui.painter().text(Pos2::new(r.left() + 48.0, r.center().y), Align2::LEFT_CENTER, &pr.name, font(14.5), TEXT_BRIGHT);
                        if resp.clicked() {
                            switch_to = Some(i);
                        }
                    });
                }
                if let Some(i) = switch_to {
                    self.settings.active_profile = i;
                    self.engine = make_engine(&self.settings);
                    self.save_settings();
                }
                egui::Frame::none().inner_margin(egui::Margin::symmetric(20.0, 2.0)).show(ui, |ui| {
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(t("profile.ai_assistant")).size(12.5).color(TEXT_DIM));
                    let ai = &self.settings.ai;
                    let status = match ai.mode {
                        AiMode::Off => t("profile.ai_off"),
                        AiMode::Api => format!("{} · {}", ai.api_kind.label(), ai.model),
                        AiMode::Acp => format!("ACP · {} {}", ai.acp_command, ai.acp_args),
                    };
                    ui.label(egui::RichText::new(status).size(13.0).color(TEXT));
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new(t("profile.signature")).size(12.5).color(TEXT_DIM));
                    let p = self.settings.profile();
                    let name = if p.user_name.is_empty() { t("profile.not_set") } else { p.user_name.clone() };
                    ui.label(egui::RichText::new(name).size(14.0).color(TEXT_BRIGHT));
                    ui.label(egui::RichText::new(&p.user_email).size(12.0).color(TEXT));
                    ui.add_space(6.0);
                });
                let mut go: Option<PrefPage> = None;
                for (icon, key, page) in [
                    (ic::USER_GEAR, "profile.manage", PrefPage::Profiles),
                    (ic::SPARKLE, "profile.configure_ai", PrefPage::Ai),
                    (ic::TRANSLATE, "profile.language", PrefPage::Language),
                    (ic::DOWNLOAD_SIMPLE, "profile.download_git", PrefPage::Git),
                ] {
                    egui::Frame::none().inner_margin(egui::Margin::symmetric(20.0, 0.0)).show(ui, |ui| {
                        let w = ui.available_width();
                        let (r, resp) = ui.allocate_exact_size(Vec2::new(w, 34.0), Sense::click());
                        if resp.hovered() {
                            ui.painter().rect_filled(r, Rounding::ZERO, hex(0x434852));
                        }
                        ui.painter().text(Pos2::new(r.left() + 10.0, r.center().y), Align2::CENTER_CENTER, icon, font(16.0), TEXT_DIM);
                        ui.painter().text(Pos2::new(r.left() + 30.0, r.center().y), Align2::LEFT_CENTER, t(key), font(14.5), TEXT_BRIGHT);
                        if resp.clicked() {
                            go = Some(page);
                        }
                    });
                }
                if let Some(p) = go {
                    self.profile_open = false;
                    self.open_prefs(p);
                }
            });
    }

    pub(super) fn status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status")
            .exact_height(24.0)
            .frame(egui::Frame::none().fill(PANEL))
            .show(ctx, |ui| {
                self.status_drawn = true;
                let rect = ui.max_rect();
                let p = ui.painter().clone();
                p.line_segment([rect.left_top(), rect.right_top()], Stroke::new(1.0, hex(0x1A1C20)));
                let cy = rect.center().y;
                let (left_icon, left_text, col) = if let Some(j) = self.jobs.first() {
                    let pr = j.progress.lock().map(|v| *v).unwrap_or(0.0);
                    (ic::CIRCLE_NOTCH, format!("{} {:.0}%", j.title, pr * 100.0), TEXT)
                } else if self.engine.cli.is_none() {
                    (ic::WARNING, t("status.builtin_engine"), MOD_YELLOW)
                } else if let Some((n, e)) = self.notifications.first() {
                    (if *e { ic::WARNING } else { ic::INFO }, n.clone(), if *e { MOD_YELLOW } else { TEXT })
                } else {
                    (ic::CHECK, t("status.ready"), TEXT)
                };
                p.text(Pos2::new(rect.left() + 26.0, cy), Align2::CENTER_CENTER, left_icon, font(14.0), col);
                let lt = elide(ui, &left_text, &font(13.0), rect.width() * 0.33);
                p.text(Pos2::new(rect.left() + 42.0, cy), Align2::LEFT_CENTER, lt, font(13.0), TEXT);
                let center_text = match &self.git_version {
                    Some(v) => tf("status.git_version", &[v]),
                    None => t("status.download_git"),
                };
                let cw = text_width(ui, &center_text, font(13.0));
                let cr = Rect::from_center_size(Pos2::new(rect.center().x, cy), Vec2::new(cw, 20.0));
                let cresp = ui.interact(cr, ui.id().with("gitver"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
                p.text(cr.center(), Align2::CENTER_CENTER, &center_text, font(13.0), if cresp.hovered() { LINK_SOFT } else { LINK });
                if cresp.clicked() {
                    self.open_prefs(PrefPage::Git);
                }
                let mut x = rect.right() - 8.0;
                let ver = gcc_core::config::APP_VERSION.to_string();
                let vw = text_width(ui, &ver, mono(12.0)) + 12.0;
                let vr = Rect::from_min_size(Pos2::new(x - vw, rect.top() + 3.0), Vec2::new(vw, 18.0));
                p.rect(vr, Rounding::same(2.0), hex(0x1F2227), Stroke::new(1.0, hex(0x3A3F49)));
                p.text(vr.center(), Align2::CENTER_CENTER, ver, mono(12.0), TEXT);
                x = vr.left() - 14.0;
                logo(&p, Pos2::new(x, cy), 15.0, hex(0x6FD3C3));
                x -= 18.0;
                let help = t("status.help");
                let hw = text_width(ui, &help, font(13.0));
                let hr = Rect::from_min_size(Pos2::new(x - hw, rect.top()), Vec2::new(hw, rect.height()));
                let hresp = ui.interact(hr, ui.id().with("help"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
                p.text(Pos2::new(hr.right(), cy), Align2::RIGHT_CENTER, &help, font(13.0), if hresp.hovered() { LINK_SOFT } else { LINK });
                if hresp.clicked() {
                    self.add_tab(TabKind::Guide, true);
                }
                x = hr.left() - 18.0;
                let z = format!("{:.0}%", self.settings.zoom * 100.0);
                let zw = text_width(ui, &z, font(13.0));
                let zr = Rect::from_min_size(Pos2::new(x - zw - 20.0, rect.top()), Vec2::new(zw + 20.0, rect.height()));
                let zresp = ui.interact(zr, ui.id().with("zoom"), Sense::click()).on_hover_text(t("status.zoom_hint"));
                p.text(Pos2::new(zr.left() + 7.0, cy), Align2::CENTER_CENTER, ic::MAGNIFYING_GLASS_PLUS, font(14.0), TEXT);
                p.text(Pos2::new(zr.right(), cy), Align2::RIGHT_CENTER, z, font(13.0), TEXT_BRIGHT);
                if zresp.clicked() {
                    self.set_zoom(1.0);
                }
                x = zr.left() - 22.0;
                let kr = Rect::from_center_size(Pos2::new(x, cy), Vec2::splat(20.0));
                let kresp = ui.interact(kr, ui.id().with("keys"), Sense::click()).on_hover_text(t("menu.shortcuts"));
                p.text(kr.center(), Align2::CENTER_CENTER, ic::KEYBOARD, font(16.0), TEXT);
                if kresp.clicked() {
                    self.modal = Some(Modal::Shortcuts);
                }
                x -= 30.0;
                let lr = Rect::from_center_size(Pos2::new(x, cy), Vec2::splat(20.0));
                let lresp = ui.interact(lr, ui.id().with("activity"), Sense::click()).on_hover_text(t("notify.title"));
                p.text(lr.center(), Align2::CENTER_CENTER, ic::LIST_BULLETS, font(16.0), TEXT);
                if lresp.clicked() {
                    self.bell_open = !self.bell_open;
                }
            });
    }

    pub(super) fn toasts_ui(&mut self, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        self.toasts.retain(|t| t.until > now);
        let screen = ctx.screen_rect();
        let mut y = screen.bottom() - 34.0;
        let jobs: Vec<(String, f32)> = self.jobs.iter().map(|j| (j.title.clone(), j.progress.lock().map(|v| *v).unwrap_or(0.0))).collect();
        for (i, (title, pr)) in jobs.iter().enumerate() {
            let h = 108.0;
            let r = Rect::from_min_size(Pos2::new(screen.left() + 14.0, y - h), Vec2::new(352.0, h));
            egui::Area::new(egui::Id::new(("job", i))).order(egui::Order::Foreground).fixed_pos(r.min).show(ctx, |ui| {
                let p = ui.painter();
                p.rect_filled(r, Rounding::same(2.0), TOAST_BG);
                p.rect_filled(Rect::from_min_size(r.min, Vec2::new(36.0, h)), Rounding::same(2.0), TOAST_BLUE);
                let c = Pos2::new(r.left() + 18.0, r.center().y);
                let ang = now as f32 * 4.0;
                for k in 0..8 {
                    let a = ang + k as f32 * 0.6;
                    let al = (k as f32 / 8.0 * 255.0) as u8;
                    p.circle_filled(c + Vec2::angled(a) * 8.0, 1.6, egui::Color32::from_white_alpha(al));
                }
                let galley = ui.fonts(|f| f.layout(title.clone(), font(16.5), TEXT_BRIGHT, 270.0));
                p.galley(Pos2::new(r.left() + 52.0, r.top() + 16.0), galley, TEXT_BRIGHT);
                let bar = Rect::from_min_size(Pos2::new(r.left() + 52.0, r.bottom() - 30.0), Vec2::new(284.0, 18.0));
                p.rect_filled(bar, Rounding::same(2.0), hex(0xF2F2F2));
                let fill_w = (bar.width() * pr.max(0.05)).min(bar.width());
                p.rect_filled(Rect::from_min_size(bar.min, Vec2::new(fill_w, bar.height())), Rounding::same(2.0), hex(0x5B8BD8));
                let _ = ui.allocate_exact_size(r.size(), Sense::hover());
            });
            y -= h + 8.0;
            ctx.request_repaint();
        }
        let toasts: Vec<(String, bool)> = self.toasts.iter().map(|t| (t.text.clone(), t.error)).collect();
        let mut ty = screen.bottom() - 34.0;
        let mut dismiss: Option<String> = None;
        for (i, (text, err)) in toasts.iter().enumerate().rev() {
            let galley = ctx.fonts(|f| f.layout(text.clone(), font(13.5), TEXT_BRIGHT, 360.0));
            let h = galley.size().y + 22.0;
            let r = Rect::from_min_size(Pos2::new(screen.right() - 420.0, ty - h), Vec2::new(400.0, h));
            egui::Area::new(egui::Id::new(("toast", i))).order(egui::Order::Foreground).fixed_pos(r.min).show(ctx, |ui| {
                let p = ui.painter();
                p.rect_filled(r, Rounding::same(3.0), TOAST_BG);
                p.rect_filled(Rect::from_min_size(r.min, Vec2::new(5.0, h)), Rounding::same(2.0), if *err { DEL_RED } else { CHECK_GREEN });
                p.galley(Pos2::new(r.left() + 20.0, r.top() + 11.0), galley.clone(), TEXT_BRIGHT);
                let resp = ui.allocate_exact_size(r.size(), Sense::click()).1;
                if resp.clicked() {
                    dismiss = Some(text.clone());
                }
            });
            ty -= h + 8.0;
        }
        if let Some(d) = dismiss {
            self.toasts.retain(|t| t.text != d);
        }
        if !self.toasts.is_empty() {
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }
    }
}
