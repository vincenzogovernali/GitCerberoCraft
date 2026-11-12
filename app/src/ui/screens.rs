//! Full-window pages (welcome steps, clone) and the dialogs.

use super::theme::*;
use super::*;
use eframe::egui::{self, Align2, Color32, CursorIcon, Pos2, Rect, Rounding, Sense, Stroke, Ui, Vec2};
use egui_phosphor::regular as ic;
use gcc_core::i18n::{t, tf};
use gcc_core::templates;

fn back_link(ui: &mut Ui, pos: Pos2) -> bool {
    let r = Rect::from_min_size(pos, Vec2::new(80.0, 22.0));
    let resp = ui.interact(r, ui.id().with("back"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
    let c = if resp.hovered() { LINK_SOFT } else { hex(0x4FA3F7) };
    ui.painter().text(Pos2::new(r.left(), r.center().y), Align2::LEFT_CENTER, ic::CARET_LEFT, font(14.0), c);
    ui.painter().text(Pos2::new(r.left() + 16.0, r.center().y), Align2::LEFT_CENTER, t("common.back"), font(14.5), c);
    resp.clicked()
}

fn glow(p: &egui::Painter, c: Pos2, radius: f32, color: Color32) {
    for i in (1..=24).rev() {
        let f = i as f32 / 24.0;
        let a = ((1.0 - f) * 22.0) as u8;
        p.circle_filled(c, radius * f, Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), a));
    }
}

fn rocket(p: &egui::Painter, c: Pos2, s: f32) {
    let body = vec![c + Vec2::new(-s * 0.35, s * 0.6), c + Vec2::new(-s * 0.2, -s * 0.5), c + Vec2::new(s * 0.45, -s * 0.9), c + Vec2::new(s * 0.2, -s * 0.1), c + Vec2::new(-s * 0.1, s * 0.75)];
    p.add(egui::Shape::convex_polygon(body, hex(0x1D1F27), Stroke::new(1.5, hex(0xB9BCC6))));
    p.circle(c + Vec2::new(s * 0.05, -s * 0.35), s * 0.13, hex(0x8C52FF), Stroke::new(1.5, hex(0xB9BCC6)));
    glow(p, c + Vec2::new(-s * 0.35, s * 0.75), s * 0.5, hex(0x7FB2FF));
}

/// Our own mascot: a friendly three-headed guardian made of soft shapes.
fn mascot(p: &egui::Painter, c: Pos2, s: f32) {
    glow(p, c, s * 1.6, hex(0x1FD1B8));
    let body = hex(0x24CDB5);
    let dark = hex(0x0E5E58);
    p.add(egui::Shape::ellipse_filled(c + Vec2::new(0.0, s * 0.55), Vec2::new(s * 0.75, s * 0.5), mix(body, dark, 0.35)));
    for (dx, dy, r) in [(-0.62f32, -0.05f32, 0.36f32), (0.0, -0.35, 0.42), (0.62, -0.05, 0.36)] {
        let hc = c + Vec2::new(dx * s, dy * s);
        p.circle_filled(hc, r * s, body);
        p.circle_filled(hc + Vec2::new(-r * s * 0.35, -r * s * 0.1), r * s * 0.16, Color32::WHITE);
        p.circle_filled(hc + Vec2::new(r * s * 0.35, -r * s * 0.1), r * s * 0.16, Color32::WHITE);
        p.circle_filled(hc + Vec2::new(-r * s * 0.33, -r * s * 0.08), r * s * 0.08, dark);
        p.circle_filled(hc + Vec2::new(r * s * 0.37, -r * s * 0.08), r * s * 0.08, dark);
        p.circle_filled(hc + Vec2::new(0.0, r * s * 0.3), r * s * 0.12, dark);
        // ears
        let e1 = vec![hc + Vec2::new(-r * s * 0.8, -r * s * 0.4), hc + Vec2::new(-r * s * 0.55, -r * s * 1.15), hc + Vec2::new(-r * s * 0.15, -r * s * 0.8)];
        let e2 = vec![hc + Vec2::new(r * s * 0.8, -r * s * 0.4), hc + Vec2::new(r * s * 0.55, -r * s * 1.15), hc + Vec2::new(r * s * 0.15, -r * s * 0.8)];
        p.add(egui::Shape::convex_polygon(e1, body, Stroke::NONE));
        p.add(egui::Shape::convex_polygon(e2, body, Stroke::NONE));
    }
    p.circle_stroke(c + Vec2::new(0.0, -s * 0.1), s * 1.15, Stroke::new(1.5, Color32::from_rgba_unmultiplied(200, 255, 250, 60)));
}

pub fn onboarding(app: &mut App, ctx: &egui::Context, step: u8) {
    egui::CentralPanel::default().frame(egui::Frame::none().fill(ONBOARD_BG)).show(ctx, |ui| {
        let full = ui.max_rect();
        let p = ui.painter().clone();
        let cx = full.center().x;
        match step {
            0 => {
                // Left column
                let lx = full.left() + full.width() * 0.253;
                let top = full.top() + full.height() * 0.22;
                logo(&p, Pos2::new(lx, top - 2.0), 56.0, hex(0x1D1F27));
                p.text(Pos2::new(lx, top + 61.0), Align2::CENTER_CENTER, t("onboard.welcome"), bold(17.5), TEXT_BRIGHT);
                let g = ui.fonts(|f| f.layout(t("onboard.welcome_text"), font(14.0), hex(0xB0B4BC), 340.0));
                let gw = g.size().x;
                p.galley(Pos2::new(lx - gw / 2.0, top + 82.0), g, hex(0xB0B4BC));
                let col = Rect::from_min_size(Pos2::new(lx - 168.0, top + 140.0), Vec2::new(337.0, 220.0));
                let mut cui = ui.child_ui(col, egui::Layout::top_down(egui::Align::Min), None);
                cui.spacing_mut().item_spacing.y = 8.0;
                // Language
                egui::Frame::none().fill(ONBOARD_INPUT).rounding(2.0).inner_margin(egui::Margin::symmetric(10.0, 4.0)).show(&mut cui, |ui| {
                    ui.set_width(317.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("{}  {}", ic::TRANSLATE, t("prefs.language"))).color(TEXT));
                        language_combo(app, ui, "onb_lang");
                    });
                });
                // Git status
                egui::Frame::none().fill(ONBOARD_INPUT).rounding(2.0).inner_margin(egui::Margin::symmetric(10.0, 6.0)).show(&mut cui, |ui| {
                    ui.set_width(317.0);
                    match &app.git_version {
                        Some(v) => {
                            ui.label(egui::RichText::new(format!("{}  {}", ic::CHECK, tf("onboard.git_found", &[v]))).color(CHECK_GREEN));
                        }
                        None => {
                            ui.label(egui::RichText::new(format!("{}  {}", ic::INFO, t("onboard.git_missing"))).color(MOD_YELLOW));
                            if ui.button(format!("{}  {}", ic::DOWNLOAD_SIMPLE, t("profile.download_git"))).clicked() {
                                app.settings.onboarding_done = true;
                                app.open_prefs(PrefPage::Git);
                            }
                        }
                    }
                });
                cui.add_space(6.0);
                if solid_button(&mut cui, &t("onboard.continue"), PURPLE_BTN, Vec2::new(337.0, 32.0), true).clicked() {
                    app.screen = Screen::Onboarding(1);
                }
                // Right card
                let card = Rect::from_min_size(Pos2::new(full.left() + full.width() * 0.503, full.top() + full.height() * 0.212), Vec2::new(full.width() * 0.413, 326.0));
                glow(&p, card.right_top() + Vec2::new(-90.0, 10.0), 260.0, hex(0x6A2BA8));
                p.rect(card, Rounding::same(8.0), Color32::from_rgba_unmultiplied(23, 23, 33, 235), Stroke::new(1.0, hex(0x2C2F3A)));
                rocket(&p, card.right_top() + Vec2::new(-120.0, -10.0), 40.0);
                let x = card.left() + 40.0;
                let g = ui.fonts(|f| f.layout(t("onboard.card_title"), bold(15.5), TEXT_BRIGHT, 220.0));
                p.galley(Pos2::new(x, card.top() + 68.0), g, TEXT_BRIGHT);
                p.text(Pos2::new(x, card.top() + 146.0), Align2::LEFT_CENTER, t("onboard.card_lead"), font(14.0), hex(0xB0B4BC));
                let mut y = card.top() + 176.0;
                for i in 1..=4 {
                    p.text(Pos2::new(x + 24.0, y), Align2::LEFT_CENTER, format!("•  {}", t(&format!("onboard.card_b{i}"))), font(14.0), hex(0xB0B4BC));
                    y += 20.0;
                }
                p.text(Pos2::new(x, y + 16.0), Align2::LEFT_CENTER, t("onboard.card_more"), font(14.0), hex(0xB0B4BC));
                // Skip link
                let sy = full.top() + full.height() * 0.77;
                let pre = t("onboard.skip_q");
                let link = t("onboard.skip");
                let w1 = text_width(ui, &pre, font(14.0));
                let w2 = text_width(ui, &link, font(14.0));
                let x0 = cx - (w1 + w2 + 4.0) / 2.0;
                p.text(Pos2::new(x0, sy), Align2::LEFT_CENTER, &pre, font(14.0), TEXT);
                let lr = p.text(Pos2::new(x0 + w1 + 4.0, sy), Align2::LEFT_CENTER, &link, font(14.0), hex(0x4FA3F7));
                if ui.interact(lr, ui.id().with("skip"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand).clicked() {
                    app.settings.onboarding_done = true;
                    app.screen = Screen::Main;
                    app.save_settings();
                }
                p.text(Pos2::new(cx, full.bottom() - 26.0), Align2::CENTER_CENTER, t("onboard.free_note"), font(13.5), hex(0x9DA1A9));
            }
            1 => {
                if back_link(ui, full.min + Vec2::new(18.0, 44.0)) {
                    app.screen = Screen::Onboarding(0);
                }
                let lx = full.left() + full.width() * 0.125;
                let top = full.top() + full.height() * 0.345;
                p.text(Pos2::new(lx, top), Align2::LEFT_CENTER, t("onboard.details_title"), bold(17.5), TEXT_BRIGHT);
                let g = ui.fonts(|f| f.layout(t("onboard.details_text"), font(14.0), hex(0xB0B4BC), 410.0));
                p.galley(Pos2::new(lx, top + 20.0), g, hex(0xB0B4BC));
                let form = Rect::from_min_size(Pos2::new(lx, top + 88.0), Vec2::new(408.0, 200.0));
                let mut f = ui.child_ui(form, egui::Layout::top_down(egui::Align::Min), None);
                f.label(egui::RichText::new(t("onboard.name")).color(hex(0xC8CCD2)).size(14.5));
                input(&mut f, &mut app.onboard_name, "", 408.0, 34.0, ONBOARD_INPUT);
                f.add_space(14.0);
                f.label(egui::RichText::new(t("onboard.email")).color(hex(0xC8CCD2)).size(14.5));
                input(&mut f, &mut app.onboard_email, "", 408.0, 34.0, ONBOARD_INPUT);
                f.add_space(18.0);
                f.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                    let label = t("onboard.use_details");
                    let w = text_width(ui, &label, font(13.0)) + 26.0;
                    let ok = !app.onboard_name.trim().is_empty() && app.onboard_email.contains('@');
                    if solid_button(ui, &label, PURPLE_BTN, Vec2::new(w, 28.0), ok).clicked() {
                        app.settings.profile_mut().user_name = app.onboard_name.trim().to_string();
                        app.settings.profile_mut().user_email = app.onboard_email.trim().to_string();
                        if git::read_config(None, "user.name").is_none() {
                            let _ = git::write_global_identity(app.onboard_name.trim(), app.onboard_email.trim());
                        }
                        app.engine = make_engine(&app.settings);
                        app.save_settings();
                        app.screen = Screen::Onboarding(2);
                    }
                });
                mascot(&p, Pos2::new(full.left() + full.width() * 0.71, full.top() + full.height() * 0.5), 105.0);
                rocket(&p, Pos2::new(full.left() + full.width() * 0.84, full.top() + full.height() * 0.36), 34.0);
            }
            _ => {
                p.rect_filled(full, Rounding::ZERO, hex(0x1D2028));
                if back_link(ui, full.min + Vec2::new(18.0, 44.0)) {
                    app.screen = Screen::Onboarding(1);
                }
                let top = full.top() + full.height() * 0.2;
                p.text(Pos2::new(cx, top), Align2::CENTER_CENTER, t("onboard.learn_title"), bold(17.5), TEXT_BRIGHT);
                let banner = Rect::from_center_size(Pos2::new(cx, top + 126.0), Vec2::new(full.width() * 0.676, 194.0));
                p.rect_filled(banner, Rounding::same(3.0), hex(0x272334));
                glow(&p, banner.left_center(), 160.0, hex(0xA020F0));
                glow(&p, banner.right_center(), 160.0, hex(0xA020F0));
                p.rect_stroke(banner, Rounding::same(3.0), Stroke::new(2.0, hex(0x8B5BD6)));
                // mini mock of the toolbar
                let mock = Rect::from_center_size(banner.center() + Vec2::new(0.0, 12.0), Vec2::new(banner.width() * 0.7, banner.height() - 24.0));
                p.rect_filled(mock, Rounding::ZERO, hex(0x3A3D46));
                let tb = Rect::from_min_size(mock.min + Vec2::new(0.0, 32.0), Vec2::new(mock.width(), 40.0));
                p.rect_filled(tb, Rounding::ZERO, hex(0x50535C));
                let labels = ["toolbar.undo", "toolbar.redo", "toolbar.pull", "toolbar.push", "toolbar.branch", "toolbar.stash", "toolbar.pop"];
                let icons = [ic::ARROW_COUNTER_CLOCKWISE, ic::ARROW_CLOCKWISE, ic::ARROW_LINE_DOWN, ic::ARROW_LINE_UP, ic::GIT_BRANCH, ic::TRAY_ARROW_DOWN, ic::TRAY_ARROW_UP];
                for (i, (l, icn)) in labels.iter().zip(icons.iter()).enumerate() {
                    let x = tb.left() + 50.0 + i as f32 * 48.0;
                    p.text(Pos2::new(x, tb.top() + 10.0), Align2::CENTER_CENTER, t(l), font(10.5), TEXT);
                    p.text(Pos2::new(x, tb.top() + 27.0), Align2::CENTER_CENTER, *icn, font(15.0), TEXT_BRIGHT);
                }
                for k in 0..3 {
                    let y = tb.bottom() + 20.0 + k as f32 * 26.0;
                    p.circle_filled(Pos2::new(mock.left() + 70.0 + k as f32 * 8.0, y), 9.0, lane_color(k * 2));
                    p.rect_filled(Rect::from_min_size(Pos2::new(mock.left() + 150.0, y - 9.0), Vec2::new(mock.width() * 0.5, 18.0)), Rounding::same(2.0), hex(0x2F323A));
                }
                let lens = banner.center() + Vec2::new(30.0, -30.0);
                p.circle_filled(lens, 56.0, hex(0x4A4D56));
                p.circle_stroke(lens, 56.0, Stroke::new(3.0, hex(0xF0B8F5)));
                p.text(lens + Vec2::new(0.0, -14.0), Align2::CENTER_CENTER, t("toolbar.branch"), font(17.0), TEXT_BRIGHT);
                p.text(lens + Vec2::new(0.0, 14.0), Align2::CENTER_CENTER, ic::GIT_BRANCH, font(24.0), TEXT_BRIGHT);
                let br = Rect::from_center_size(Pos2::new(cx, banner.bottom() + 41.0), Vec2::new(300.0, 30.0));
                let mut bu = ui.child_ui(br, egui::Layout::left_to_right(egui::Align::Center), None);
                if solid_button(&mut bu, &t("onboard.start_guide"), MAGENTA_BTN, br.size(), true).clicked() {
                    app.settings.onboarding_done = true;
                    app.screen = Screen::Main;
                    app.add_tab(TabKind::Guide, true);
                    app.save_settings();
                }
                let oy = br.bottom() + 50.0;
                p.line_segment([Pos2::new(full.left() + full.width() * 0.14, oy), Pos2::new(cx - 20.0, oy)], Stroke::new(1.0, hex(0x5A5E6A)));
                p.line_segment([Pos2::new(cx + 20.0, oy), Pos2::new(full.right() - full.width() * 0.145, oy)], Stroke::new(1.0, hex(0x5A5E6A)));
                p.text(Pos2::new(cx, oy), Align2::CENTER_CENTER, t("onboard.or"), font(14.0), TEXT);
                p.text(Pos2::new(cx, oy + 41.0), Align2::CENTER_CENTER, t("onboard.already"), bold(17.5), TEXT_BRIGHT);
                let row = Rect::from_center_size(Pos2::new(cx, oy + 105.0), Vec2::new(610.0, 28.0));
                let mut ru = ui.child_ui(row, egui::Layout::left_to_right(egui::Align::Center), None);
                ru.spacing_mut().item_spacing.x = 10.0;
                if solid_button(&mut ru, &t("onboard.open_repo"), hex(0x3A3D46), Vec2::new(300.0, 28.0), true).clicked() {
                    app.settings.onboarding_done = true;
                    app.screen = Screen::Main;
                    app.save_settings();
                    app.browse_open_repo();
                }
                if solid_button(&mut ru, &t("onboard.clone_repo"), hex(0x3A3D46), Vec2::new(300.0, 28.0), true).clicked() {
                    app.settings.onboarding_done = true;
                    app.screen = Screen::Clone;
                    app.save_settings();
                }
            }
        }
    });
}

pub fn language_combo(app: &mut App, ui: &mut Ui, id: &str) {
    let langs = gcc_core::i18n::global().read().map(|g| g.languages.clone()).unwrap_or_default();
    let current = app.settings.language.clone();
    let label = if current == "auto" {
        t("prefs.lang_auto")
    } else {
        langs.iter().find(|l| l.code == current).map(|l| l.name.clone()).unwrap_or(current.clone())
    };
    let mut chosen: Option<String> = None;
    egui::ComboBox::from_id_source(id).selected_text(label).width(170.0).show_ui(ui, |ui| {
        if ui.selectable_label(current == "auto", t("prefs.lang_auto")).clicked() {
            chosen = Some("auto".into());
        }
        for l in &langs {
            if ui.selectable_label(current == l.code, format!("{} ({})", l.name, l.code)).clicked() {
                chosen = Some(l.code.clone());
            }
        }
    });
    if let Some(c) = chosen {
        app.settings.language = c.clone();
        if let Ok(mut g) = gcc_core::i18n::global().write() {
            g.set_language(&c);
        }
        app.save_settings();
    }
}

const PROVIDERS: [(&str, &str); 6] = [
    ("clone.with", "URL"),
    ("clone.from", "GitHub.com"),
    ("clone.from", "GitLab.com"),
    ("clone.from", "Bitbucket.org"),
    ("clone.from", "Codeberg.org"),
    ("clone.from", "Azure DevOps"),
];

fn provider_url(provider: usize, owner: &str, repo: &str) -> String {
    match provider {
        1 => format!("https://github.com/{owner}/{repo}.git"),
        2 => format!("https://gitlab.com/{owner}/{repo}.git"),
        3 => format!("https://bitbucket.org/{owner}/{repo}.git"),
        4 => format!("https://codeberg.org/{owner}/{repo}.git"),
        5 => format!("https://dev.azure.com/{owner}/_git/{repo}"),
        _ => String::new(),
    }
}

fn list_repos(provider: usize, owner: &str, token: &str) -> Result<Vec<(String, String)>, String> {
    let agent = ureq::AgentBuilder::new().timeout(std::time::Duration::from_secs(20)).user_agent("GitCerberoCraft").build();
    let v: serde_json::Value = match provider {
        1 => {
            let mut r = agent.get(&format!("https://api.github.com/users/{owner}/repos?per_page=100&sort=updated"));
            if !token.is_empty() {
                r = r.set("Authorization", &format!("Bearer {token}"));
            }
            r.call().map_err(|e| e.to_string())?.into_json().map_err(|e| e.to_string())?
        }
        2 => {
            let mut r = agent.get(&format!("https://gitlab.com/api/v4/users/{owner}/projects?per_page=100&order_by=last_activity_at"));
            if !token.is_empty() {
                r = r.set("PRIVATE-TOKEN", token);
            }
            r.call().map_err(|e| e.to_string())?.into_json().map_err(|e| e.to_string())?
        }
        4 => agent
            .get(&format!("https://codeberg.org/api/v1/users/{owner}/repos?limit=50"))
            .call()
            .map_err(|e| e.to_string())?
            .into_json()
            .map_err(|e| e.to_string())?,
        _ => return Err("listing is available for GitHub, GitLab and Codeberg".into()),
    };
    let mut out = Vec::new();
    for r in v.as_array().cloned().unwrap_or_default() {
        let name = r["full_name"].as_str().or(r["path_with_namespace"].as_str()).or(r["name"].as_str()).unwrap_or("").to_string();
        let url = r["clone_url"].as_str().or(r["http_url_to_repo"].as_str()).unwrap_or("").to_string();
        if !url.is_empty() {
            out.push((name, url));
        }
    }
    Ok(out)
}

pub fn clone_page(app: &mut App, ctx: &egui::Context) {
    egui::CentralPanel::default().frame(egui::Frame::none().fill(ONBOARD_BG)).show(ctx, |ui| {
        let full = ui.max_rect();
        let p = ui.painter().clone();
        if back_link(ui, full.min + Vec2::new(18.0, 40.0)) {
            app.screen = Screen::Main;
        }
        let lx = full.left() + 106.0;
        let top = full.top() + 91.0;
        p.text(Pos2::new(lx, top), Align2::LEFT_CENTER, t("clone.step1"), bold(18.0), TEXT_BRIGHT);
        let mut y = top + 34.0;
        for (i, (pre, name)) in PROVIDERS.iter().enumerate() {
            let r = Rect::from_min_size(Pos2::new(lx, y), Vec2::new(270.0, 48.0));
            let resp = ui.interact(r, ui.id().with(("prov", i)), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
            let sel = app.clone_form.provider == i;
            let (bg, border) = if sel { (CONTENT, FOCUS_BLUE) } else if resp.hovered() { (hex(0x151821), hex(0x3A3F4C)) } else { (ONBOARD_BG, hex(0x2E3340)) };
            p.rect(r, Rounding::same(4.0), bg, Stroke::new(1.0, border));
            let pr = p.text(Pos2::new(r.left() + 16.0, r.center().y), Align2::LEFT_CENTER, t(pre), font(13.0), hex(0x8A8E96));
            p.text(Pos2::new(pr.right() + 5.0, r.center().y), Align2::LEFT_CENTER, *name, bold(13.0), hex(0xD9DCE1));
            if resp.clicked() {
                app.clone_form.provider = i;
                app.clone_form.repos.clear();
            }
            y += 57.0;
        }
        let rx = full.left() + 416.0;
        let rw = (full.right() - rx - 60.0).clamp(300.0, 460.0);
        let prov = app.clone_form.provider;
        let title = if prov == 0 { t("clone.step2_url") } else { tf("clone.step2_from", &[PROVIDERS[prov].1]) };
        p.text(Pos2::new(rx, top), Align2::LEFT_CENTER, title, bold(18.0), TEXT_BRIGHT);
        let form = Rect::from_min_size(Pos2::new(rx, top + 32.0), Vec2::new(rw, full.bottom() - top - 60.0));
        let mut f = ui.child_ui(form, egui::Layout::top_down(egui::Align::Min), None);
        f.spacing_mut().item_spacing.y = 6.0;
        if prov != 0 {
            f.label(egui::RichText::new(t("clone.owner")).color(hex(0xC8CCD2)).size(14.5));
            f.horizontal(|ui| {
                input(ui, &mut app.clone_form.owner, "octocat", rw - 150.0, 34.0, ONBOARD_INPUT);
                let can_list = matches!(prov, 1 | 2 | 4);
                if can_list {
                    let b = ui.add_sized([140.0, 34.0], egui::Button::new(t("clone.list")).fill(hex(0x2B2E36)));
                    if b.clicked() && !app.clone_form.owner.trim().is_empty() {
                        let owner = app.clone_form.owner.trim().to_string();
                        let token = match prov {
                            1 => app.settings.github_token.clone(),
                            2 => app.settings.gitlab_token.clone(),
                            _ => String::new(),
                        };
                        app.clone_form.loading = true;
                        app.spawn_job(t("clone.list"), None, move |_| list_repos(prov, &owner, &token).map(JobOut::RepoList));
                    }
                }
            });
            if app.clone_form.loading {
                f.spinner();
            }
            if !app.clone_form.repos.is_empty() {
                egui::ScrollArea::vertical().max_height(150.0).show(&mut f, |ui| {
                    for (n, u) in app.clone_form.repos.clone() {
                        if ui.selectable_label(app.clone_form.url == u, n).clicked() {
                            app.clone_form.url = u;
                        }
                    }
                });
            } else {
                f.label(egui::RichText::new(t("clone.owner_hint")).color(TEXT_FAINT).size(12.5));
            }
            f.add_space(6.0);
        }
        f.label(egui::RichText::new(t("clone.url")).color(hex(0xC8CCD2)).size(14.5));
        let before = app.clone_form.url.clone();
        input(&mut f, &mut app.clone_form.url, "https://…", rw, 34.0, ONBOARD_INPUT);
        if prov != 0 && app.clone_form.url == before && app.clone_form.url.is_empty() && app.clone_form.owner.contains('/') {
            let (o, r) = app.clone_form.owner.split_once('/').unwrap();
            app.clone_form.url = provider_url(prov, o, r);
        }
        f.add_space(8.0);
        f.label(egui::RichText::new(t("clone.location")).color(hex(0xC8CCD2)).size(14.5));
        f.horizontal(|ui| {
            input(ui, &mut app.clone_form.parent, "", rw - 76.0, 34.0, ONBOARD_INPUT);
            if ui.add_sized([66.0, 34.0], egui::Button::new(t("common.browse")).fill(hex(0x2B2E36))).clicked() && app.screenshot.is_none() {
                if let Some(d) = rfd::FileDialog::new().pick_folder() {
                    app.clone_form.parent = d.display().to_string();
                }
            }
        });
        let repo_name = app
            .clone_form
            .url
            .trim()
            .trim_end_matches('/')
            .trim_end_matches(".git")
            .rsplit(['/', ':'])
            .next()
            .unwrap_or("")
            .to_string();
        let dest = PathBuf::from(&app.clone_form.parent).join(&repo_name);
        if !repo_name.is_empty() {
            f.label(egui::RichText::new(tf("clone.full_path", &[&dest.display().to_string()])).color(TEXT_DIM).size(12.5));
        }
        f.add_space(6.0);
        f.horizontal(|ui| {
            ui.label(egui::RichText::new(t("clone.shallow")).color(hex(0xC8CCD2)).size(14.5));
            checkbox(ui, &mut app.clone_form.shallow);
        });
        f.horizontal(|ui| {
            ui.label(egui::RichText::new(t("clone.sparse")).color(hex(0xC8CCD2)).size(14.5));
            checkbox(ui, &mut app.clone_form.sparse);
        });
        f.add_space(16.0);
        f.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            let label = t("clone.button");
            let w = text_width(ui, &label, font(13.0)) + 24.0;
            let ok = !app.clone_form.url.trim().is_empty() && !repo_name.is_empty() && app.jobs.is_empty();
            if solid_button(ui, &label, PURPLE_DEEP, Vec2::new(w, 28.0), ok).clicked() {
                let e = app.engine.clone();
                let url = app.clone_form.url.trim().to_string();
                let shallow = app.clone_form.shallow;
                let sparse = app.clone_form.sparse;
                let d = dest.clone();
                app.screen = Screen::Main;
                app.spawn_job(tf("jobs.cloning", &[&dest.display().to_string()]), None, move |p| {
                    if sparse && e.cli.is_some() {
                        let ds = d.display().to_string();
                        let mut args = vec!["clone", "--progress", "--sparse"];
                        if shallow {
                            args.extend(["--depth", "1"]);
                        }
                        args.push(&url);
                        args.push(&ds);
                        let cwd = d.parent().map(|x| x.to_path_buf()).unwrap_or_else(std::env::temp_dir);
                        let _ = std::fs::create_dir_all(&cwd);
                        gcc_core::git::run_git(&e, &cwd, &args, Some(p)).map(|_| JobOut::Cloned(d))
                    } else {
                        gcc_core::git::clone(&e, &url, &d, shallow, Some(p)).map(JobOut::Cloned)
                    }
                });
            }
        });
    });
}

pub fn new_init_form(s: &Settings) -> Modal {
    Modal::Init(InitForm {
        provider: 0,
        name: String::new(),
        parent: s.default_clone_dir.clone(),
        branch: s.default_branch.clone(),
        gitignore: String::new(),
        license: String::new(),
        lfs: false,
        remote: String::new(),
    })
}

/// Dim the window and return the rect of a centred dialog.
fn dialog_shell(ctx: &egui::Context, id: &str, size: Vec2, title: &str, add: impl FnOnce(&mut Ui)) -> bool {
    let screen = ctx.screen_rect();
    egui::Area::new(egui::Id::new(("dim", id))).order(egui::Order::Middle).fixed_pos(screen.min).show(ctx, |ui| {
        ui.painter().rect_filled(screen, Rounding::ZERO, Color32::from_black_alpha(150));
        ui.allocate_rect(screen, Sense::click());
    });
    let size = Vec2::new(size.x.min(screen.width() - 40.0), size.y.min(screen.height() - 40.0));
    let r = Rect::from_center_size(screen.center() + Vec2::new(0.0, 12.0), size);
    let mut close = false;
    egui::Area::new(egui::Id::new(("dlg", id))).order(egui::Order::Foreground).fixed_pos(r.min).show(ctx, |ui| {
        let p = ui.painter();
        p.rect_filled(r, Rounding::same(3.0), MODAL_BODY);
        let hdr = Rect::from_min_size(r.min, Vec2::new(r.width(), 52.0));
        p.rect_filled(hdr, Rounding { nw: 3.0, ne: 3.0, sw: 0.0, se: 0.0 }, MODAL_HEADER);
        p.text(Pos2::new(hdr.left() + 20.0, hdr.center().y), Align2::LEFT_CENTER, title, font(18.5), TEXT_BRIGHT);
        let xr = Rect::from_center_size(Pos2::new(hdr.right() - 30.0, hdr.center().y), Vec2::splat(26.0));
        let xresp = ui.interact(xr, ui.id().with("x"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
        ui.painter().text(xr.center(), Align2::CENTER_CENTER, ic::X, bold(19.0), if xresp.hovered() { TEXT_BRIGHT } else { hex(0xCED0D2) });
        if xresp.clicked() {
            close = true;
        }
        let body = Rect::from_min_max(Pos2::new(r.left(), hdr.bottom()), r.max);
        let mut child = ui.child_ui(body, egui::Layout::top_down(egui::Align::Min), None);
        child.set_clip_rect(body);
        add(&mut child);
    });
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        close = true;
    }
    close
}

pub fn modals(app: &mut App, ctx: &egui::Context) {
    let Some(modal) = app.modal.take() else { return };
    let mut keep = true;
    let mut next: Option<Modal> = None;
    match modal {
        Modal::Init(mut f) => {
            let mut create = false;
            let close = dialog_shell(ctx, "init", Vec2::new(998.0, 640.0), &t("init.title"), |ui| {
                let body = ui.max_rect();
                let left = Rect::from_min_size(body.min, Vec2::new(250.0, body.height()));
                ui.painter().rect_filled(left, Rounding::ZERO, MODAL_LEFT);
                for (i, (icon, key)) in [(ic::LAPTOP, "init.local_only"), (ic::LINK, "init.with_remote")].iter().enumerate() {
                    let r = Rect::from_min_size(Pos2::new(left.left(), left.top() + i as f32 * 40.0), Vec2::new(250.0, 40.0));
                    let resp = ui.interact(r, ui.id().with(("initp", i)), Sense::click());
                    if f.provider == i {
                        ui.painter().rect_filled(r, Rounding::ZERO, MODAL_SELECT);
                    } else if resp.hovered() {
                        ui.painter().rect_filled(r, Rounding::ZERO, hex(0x383C46));
                    }
                    ui.painter().text(Pos2::new(r.left() + 19.0, r.center().y), Align2::CENTER_CENTER, *icon, font(15.0), TEXT_BRIGHT);
                    ui.painter().text(Pos2::new(r.left() + 37.0, r.center().y), Align2::LEFT_CENTER, t(key), font(14.5), TEXT_BRIGHT);
                    if resp.clicked() {
                        f.provider = i;
                    }
                }
                let form = Rect::from_min_max(Pos2::new(left.right() + 10.0, body.top() + 10.0), Pos2::new(body.right() - 20.0, body.bottom() - 10.0));
                let mut ui2 = ui.child_ui(form, egui::Layout::top_down(egui::Align::Min), None);
                ui2.label(egui::RichText::new(t("init.subtitle")).size(18.5).color(TEXT_BRIGHT));
                ui2.add_space(10.0);
                let label_w = 150.0;
                let field_w = form.width() - label_w - 16.0;
                let row = |ui: &mut Ui, label: &str, h: f32, add: &mut dyn FnMut(&mut Ui)| {
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(Vec2::new(label_w, h), Sense::hover());
                        let g = ui.fonts(|fo| fo.layout(label.to_string(), font(14.5), hex(0xD5D8DD), label_w));
                        let gs = g.size();
                        ui.painter().galley(Pos2::new(r.right() - gs.x, r.center().y - gs.y / 2.0), g, TEXT);
                        ui.add_space(8.0);
                        add(ui);
                    });
                    ui.add_space(8.0);
                };
                row(&mut ui2, &t("init.name"), 32.0, &mut |ui| {
                    input(ui, &mut f.name, "", field_w, 32.0, INPUT);
                });
                row(&mut ui2, &t("init.in"), 32.0, &mut |ui| {
                    input(ui, &mut f.parent, "", field_w - 78.0, 32.0, INPUT);
                    if outline_button(ui, &t("common.browse"), Vec2::new(68.0, 32.0)).clicked() {
                        if let Some(d) = rfd::FileDialog::new().pick_folder() {
                            f.parent = d.display().to_string();
                        }
                    }
                });
                let full = PathBuf::from(&f.parent).join(f.name.trim());
                row(&mut ui2, &t("init.full_path"), 22.0, &mut |ui| {
                    ui.label(egui::RichText::new(full.display().to_string()).color(TEXT).size(14.5));
                });
                row(&mut ui2, &t("init.branch"), 32.0, &mut |ui| {
                    input(ui, &mut f.branch, "main", field_w, 32.0, INPUT);
                });
                row(&mut ui2, &t("init.gitignore"), 32.0, &mut |ui| {
                    let sel = if f.gitignore.is_empty() { t("common.select") } else { f.gitignore.clone() };
                    egui::ComboBox::from_id_source("gi").selected_text(sel).width(field_w - 8.0).show_ui(ui, |ui| {
                        ui.selectable_value(&mut f.gitignore, String::new(), t("common.none"));
                        for (n, _) in templates::GITIGNORES {
                            ui.selectable_value(&mut f.gitignore, n.to_string(), *n);
                        }
                    });
                });
                row(&mut ui2, &t("init.license"), 32.0, &mut |ui| {
                    let sel = if f.license.is_empty() { t("common.select") } else { f.license.clone() };
                    egui::ComboBox::from_id_source("lic").selected_text(sel).width(field_w - 8.0).show_ui(ui, |ui| {
                        ui.selectable_value(&mut f.license, String::new(), t("common.none"));
                        for (n, _) in templates::LICENSES {
                            ui.selectable_value(&mut f.license, n.to_string(), *n);
                        }
                    });
                });
                row(&mut ui2, &t("init.lfs"), 20.0, &mut |ui| {
                    checkbox(ui, &mut f.lfs);
                });
                if f.provider == 1 {
                    row(&mut ui2, &t("init.remote"), 32.0, &mut |ui| {
                        input(ui, &mut f.remote, "https://github.com/me/project.git", field_w, 32.0, INPUT);
                    });
                }
                ui2.add_space(4.0);
                ui2.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                    let ok = !f.name.trim().is_empty() && !f.parent.trim().is_empty();
                    let (r, resp) = ui.allocate_exact_size(Vec2::new(126.0, 31.0), if ok { Sense::click() } else { Sense::hover() });
                    let border = if ok { hex(0x3E9E5F) } else { hex(0x3A5A45) };
                    let bg = if ok && resp.hovered() { hex(0x2D5A3D) } else { hex(0x2C3935) };
                    ui.painter().rect(r, Rounding::same(2.0), bg, Stroke::new(1.0, border));
                    ui.painter().text(r.center(), Align2::CENTER_CENTER, t("init.create"), font(13.0), if ok { TEXT_BRIGHT } else { hex(0x787C7E) });
                    if ok && resp.clicked() {
                        create = true;
                    }
                });
            });
            if create {
                let dir = PathBuf::from(&f.parent).join(f.name.trim());
                let owner = app.settings.profile().user_name.clone();
                let o = git::InitOptions {
                    default_branch: f.branch.clone(),
                    gitignore: templates::gitignore(&f.gitignore),
                    license: templates::license_text(&f.license, &owner),
                    lfs: f.lfs,
                    remote_url: if f.provider == 1 { f.remote.clone() } else { String::new() },
                    initial_commit: !app.settings.profile().user_email.is_empty(),
                };
                match git::init(&app.engine, &dir, &o) {
                    Ok(note) => {
                        app.toast(tf("notify.initialized", &[&dir.display().to_string()]), false);
                        if !note.is_empty() {
                            app.toast(note, false);
                        }
                        app.open_repo(dir);
                        keep = false;
                    }
                    Err(e) => app.toast(e, true),
                }
            }
            if close {
                keep = false;
            }
            if keep {
                next = Some(Modal::Init(f));
            }
        }
        Modal::Input { title, label, mut value, label2, mut value2, kind } => {
            let mut submit = false;
            let mut cancel = false;
            let h = if label2.is_empty() { 220.0 } else { 300.0 };
            let close = dialog_shell(ctx, "input", Vec2::new(480.0, h), &title, |ui| {
                egui::Frame::none().inner_margin(egui::Margin::same(20.0)).show(ui, |ui| {
                    ui.label(egui::RichText::new(&label).color(TEXT));
                    let w = ui.available_width();
                    let r = input(ui, &mut value, "", w, 32.0, INPUT);
                    if ui.memory(|m| m.focused().is_none()) {
                        r.request_focus();
                    }
                    if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && label2.is_empty() {
                        submit = true;
                    }
                    if !label2.is_empty() {
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new(&label2).color(TEXT));
                        input(ui, &mut value2, "", w, 32.0, INPUT);
                    }
                    ui.add_space(16.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                        if solid_button(ui, &t("common.ok"), CREATE_GREEN, Vec2::new(100.0, 30.0), true).clicked() {
                            submit = true;
                        }
                        if solid_button(ui, &t("common.cancel"), hex(0x3A3D46), Vec2::new(100.0, 30.0), true).clicked() {
                            cancel = true;
                        }
                    });
                });
            });
            if submit {
                keep = !actions::submit_input(app, kind.clone(), value.clone(), value2.clone());
            }
            if close || cancel {
                keep = false;
            }
            if keep {
                next = Some(Modal::Input { title, label, value, label2, value2, kind });
            }
        }
        Modal::Confirm { title, text, action } => {
            let mut yes = false;
            let mut no = false;
            let close = dialog_shell(ctx, "confirm", Vec2::new(480.0, 220.0), &title, |ui| {
                egui::Frame::none().inner_margin(egui::Margin::same(20.0)).show(ui, |ui| {
                    ui.label(egui::RichText::new(&text).color(TEXT).size(14.5));
                    ui.add_space(24.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                        if solid_button(ui, &t("common.confirm"), hex(0xA33A3A), Vec2::new(110.0, 30.0), true).clicked() {
                            yes = true;
                        }
                        if solid_button(ui, &t("common.cancel"), hex(0x3A3D46), Vec2::new(100.0, 30.0), true).clicked() {
                            no = true;
                        }
                    });
                });
            });
            if yes {
                actions::confirm(app, action.clone());
                keep = false;
            }
            if close || no {
                keep = false;
            }
            if keep {
                next = Some(Modal::Confirm { title, text, action });
            }
        }
        Modal::About => {
            let gv = app.git_version.clone().unwrap_or_else(|| t("status.builtin_engine"));
            let close = dialog_shell(ctx, "about", Vec2::new(520.0, 330.0), &t("menu.about"), |ui| {
                let r = ui.max_rect();
                logo(ui.painter(), Pos2::new(r.left() + 60.0, r.top() + 70.0), 60.0, hex(0x6FD3C3));
                let tr = Rect::from_min_max(Pos2::new(r.left() + 110.0, r.top() + 30.0), r.max - Vec2::new(20.0, 20.0));
                let mut c = ui.child_ui(tr, egui::Layout::top_down(egui::Align::Min), None);
                c.label(egui::RichText::new("GitCerberoCraft").font(bold(22.0)).color(TEXT_BRIGHT));
                c.label(egui::RichText::new(tf("about.version", &[gcc_core::config::APP_VERSION])).color(TEXT_DIM));
                c.add_space(8.0);
                c.label(egui::RichText::new(t("about.text")).color(TEXT));
                c.add_space(8.0);
                c.label(egui::RichText::new(format!("libgit2 {} · Git: {}", git::engine_version(), gv)).color(TEXT_DIM).size(12.5));
            });
            keep = !close;
            if keep {
                next = Some(Modal::About);
            }
        }
        Modal::Shortcuts => {
            let close = dialog_shell(ctx, "keys", Vec2::new(560.0, 520.0), &t("menu.shortcuts"), |ui| {
                egui::Frame::none().inner_margin(egui::Margin::same(20.0)).show(ui, |ui| {
                    shortcuts_table(ui);
                });
            });
            keep = !close;
            if keep {
                next = Some(Modal::Shortcuts);
            }
        }
    }
    if app.modal.is_none() {
        app.modal = next;
    }
}

pub fn shortcuts_table(ui: &mut Ui) {
    egui::Grid::new("keys_grid").num_columns(2).spacing([40.0, 8.0]).show(ui, |ui| {
        for (k, key) in [
            ("Ctrl+T", "menu.new_tab"),
            ("Ctrl+W", "menu.close_tab"),
            ("Ctrl+Shift+T", "menu.reopen_tab"),
            ("Ctrl+Tab", "keys.next_tab"),
            ("Ctrl+N", "menu.clone_repo"),
            ("Ctrl+I", "menu.init_repo"),
            ("Ctrl+O", "menu.open_repo"),
            ("Alt+Ctrl+O", "menu.repo_management"),
            ("Alt+T", "menu.open_terminal"),
            ("Alt+O", "menu.open_file_manager"),
            ("Ctrl+,", "menu.preferences"),
            ("Ctrl+= / Ctrl+- / Ctrl+0", "keys.zoom"),
            ("↑ / ↓", "keys.move_selection"),
            ("Esc", "keys.close_diff"),
        ] {
            ui.label(egui::RichText::new(k).font(mono(13.0)).color(LINK_SOFT));
            ui.label(egui::RichText::new(t(key)).color(TEXT));
            ui.end_row();
        }
    });
}
