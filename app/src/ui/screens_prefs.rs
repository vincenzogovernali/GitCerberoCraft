//! Preferences: general, profiles, language, AI, Git (and Git download),
//! integrations, MCP server and keyboard shortcuts.

use super::theme::*;
use super::*;
use eframe::egui::{self, Align2, Pos2, Rect, Rounding, Sense, Ui, Vec2};
use egui_phosphor::regular as ic;
use gcc_core::config::{AiMode, ApiKind, Profile, PullMode};
use gcc_core::gitdl;
use gcc_core::i18n::{t, tf};

const PAGES: [(PrefPage, &str, &str); 8] = [
    (PrefPage::General, ic::GEAR, "prefs.general"),
    (PrefPage::Profiles, ic::USER_GEAR, "prefs.profiles"),
    (PrefPage::Language, ic::TRANSLATE, "prefs.language"),
    (PrefPage::Ai, ic::SPARKLE, "prefs.ai"),
    (PrefPage::Git, ic::GIT_BRANCH, "prefs.git"),
    (PrefPage::Integrations, ic::LINK, "prefs.integrations"),
    (PrefPage::Mcp, ic::PUZZLE_PIECE, "prefs.mcp"),
    (PrefPage::Shortcuts, ic::KEYBOARD, "prefs.shortcuts"),
];

fn field(ui: &mut Ui, label: &str, value: &mut String, hint: &str) -> bool {
    ui.label(egui::RichText::new(label).color(TEXT).size(13.5));
    let w = ui.available_width().min(560.0);
    let r = input(ui, value, hint, w, 30.0, INPUT);
    ui.add_space(6.0);
    r.changed()
}

fn heading(ui: &mut Ui, text: &str) {
    ui.add_space(6.0);
    ui.label(egui::RichText::new(text).font(bold(18.0)).color(TEXT_BRIGHT));
    ui.add_space(6.0);
}

pub fn preferences(app: &mut App, ui: &mut Ui) {
    let full = ui.max_rect();
    let left = Rect::from_min_size(full.min, Vec2::new(230.0, full.height()));
    ui.painter().rect_filled(left, Rounding::ZERO, MODAL_LEFT);
    let mut y = left.top() + 12.0;
    let back = Rect::from_min_size(Pos2::new(left.left(), y), Vec2::new(230.0, 34.0));
    let bresp = ui.interact(back, ui.id().with("prefs_back"), Sense::click());
    ui.painter().text(Pos2::new(back.left() + 18.0, back.center().y), Align2::LEFT_CENTER, format!("{} {}", ic::CARET_LEFT, t("common.back")), font(14.5), hex(0x4FA3F7));
    if bresp.clicked() {
        app.screen = Screen::Main;
    }
    y += 46.0;
    for (page, icon, key) in PAGES {
        let r = Rect::from_min_size(Pos2::new(left.left(), y), Vec2::new(230.0, 40.0));
        let resp = ui.interact(r, ui.id().with(key), Sense::click());
        if app.pref_page == page {
            ui.painter().rect_filled(r, Rounding::ZERO, MODAL_SELECT);
        } else if resp.hovered() {
            ui.painter().rect_filled(r, Rounding::ZERO, hex(0x383C46));
        }
        ui.painter().text(Pos2::new(r.left() + 20.0, r.center().y), Align2::CENTER_CENTER, icon, font(15.0), TEXT_BRIGHT);
        ui.painter().text(Pos2::new(r.left() + 40.0, r.center().y), Align2::LEFT_CENTER, t(key), font(14.5), TEXT_BRIGHT);
        if resp.clicked() {
            app.pref_page = page;
        }
        y += 40.0;
    }
    let body = Rect::from_min_max(Pos2::new(left.right(), full.top()), full.max);
    let mut child = ui.child_ui(body, egui::Layout::top_down(egui::Align::Min), None);
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(&mut child, |ui| {
        egui::Frame::none().inner_margin(egui::Margin::symmetric(36.0, 20.0)).show(ui, |ui| {
            ui.set_max_width(720.0);
            let before = app.settings.clone_for_compare();
            match app.pref_page {
                PrefPage::General => general(app, ui),
                PrefPage::Profiles => profiles(app, ui),
                PrefPage::Language => language(app, ui),
                PrefPage::Ai => ai(app, ui),
                PrefPage::Git => git_page(app, ui),
                PrefPage::Integrations => integrations(app, ui),
                PrefPage::Mcp => mcp(app, ui),
                PrefPage::Shortcuts => {
                    heading(ui, &t("prefs.shortcuts"));
                    screens::shortcuts_table(ui);
                }
            }
            if app.settings.clone_for_compare() != before {
                app.engine = make_engine(&app.settings);
                app.save_settings();
            }
        });
    });
}

trait CompareSettings {
    fn clone_for_compare(&self) -> String;
}
impl CompareSettings for Settings {
    fn clone_for_compare(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

fn general(app: &mut App, ui: &mut Ui) {
    heading(ui, &t("prefs.general"));
    let s = &mut app.settings;
    field(ui, &t("prefs.clone_dir"), &mut s.default_clone_dir, "");
    field(ui, &t("prefs.default_branch"), &mut s.default_branch, "main");
    field(ui, &t("prefs.editor"), &mut s.external_editor, "code");
    field(ui, &t("prefs.terminal"), &mut s.external_terminal, "wt");
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(t("prefs.auto_fetch")).color(TEXT));
        ui.add(egui::DragValue::new(&mut s.auto_fetch_minutes).range(0..=120).suffix(" min"));
    });
    ui.label(egui::RichText::new(t("prefs.auto_fetch_hint")).color(TEXT_DIM).size(12.0));
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(t("prefs.max_commits")).color(TEXT));
        ui.add(egui::DragValue::new(&mut s.graph_max_commits).range(100..=50_000).speed(50));
    });
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(t("prefs.pull_default")).color(TEXT));
        egui::ComboBox::from_id_source("pull_mode")
            .selected_text(match s.pull_mode {
                PullMode::FetchAll => t("pull.fetch_all"),
                PullMode::FastForwardIfPossible => t("pull.ff_if_possible"),
                PullMode::FastForwardOnly => t("pull.ff_only"),
                PullMode::Rebase => t("pull.rebase"),
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut s.pull_mode, PullMode::FetchAll, t("pull.fetch_all"));
                ui.selectable_value(&mut s.pull_mode, PullMode::FastForwardIfPossible, t("pull.ff_if_possible"));
                ui.selectable_value(&mut s.pull_mode, PullMode::FastForwardOnly, t("pull.ff_only"));
                ui.selectable_value(&mut s.pull_mode, PullMode::Rebase, t("pull.rebase"));
            });
    });
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(t("prefs.left_width")).color(TEXT));
        ui.add(egui::Slider::new(&mut s.left_panel_width, 160.0..=400.0));
    });
    ui.add_space(10.0);
    if ui.button(t("prefs.show_onboarding")).clicked() {
        app.screen = Screen::Onboarding(0);
    }
}

fn profiles(app: &mut App, ui: &mut Ui) {
    heading(ui, &t("prefs.profiles"));
    ui.label(egui::RichText::new(t("prefs.profiles_hint")).color(TEXT_DIM));
    ui.add_space(8.0);
    let mut remove = None;
    let n = app.settings.profiles.len();
    let active = app.settings.active_profile;
    for (i, p) in app.settings.profiles.iter_mut().enumerate() {
        egui::Frame::none().fill(PANEL).rounding(4.0).inner_margin(12.0).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(Vec2::splat(26.0), Sense::hover());
                pixel_avatar(ui.painter(), r);
                ui.label(egui::RichText::new(&p.name).font(bold(15.0)).color(TEXT_BRIGHT));
                if i == active {
                    ui.label(egui::RichText::new(t("prefs.active")).color(CHECK_GREEN));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if n > 1 && ui.button(ic::TRASH).clicked() {
                        remove = Some(i);
                    }
                });
            });
            field(ui, &t("prefs.profile_name"), &mut p.name, "");
            field(ui, &t("onboard.name"), &mut p.user_name, "");
            field(ui, &t("onboard.email"), &mut p.user_email, "");
        });
        ui.add_space(8.0);
    }
    if let Some(i) = remove {
        app.settings.profiles.remove(i);
        if app.settings.active_profile >= app.settings.profiles.len() {
            app.settings.active_profile = 0;
        }
    }
    ui.horizontal(|ui| {
        if ui.button(format!("{} {}", ic::PLUS, t("prefs.add_profile"))).clicked() {
            app.settings.profiles.push(Profile { name: format!("Profile {}", n + 1), ..Default::default() });
        }
        if ui.button(t("prefs.save_global_identity")).clicked() {
            let p = app.settings.profile().clone();
            match git::write_global_identity(&p.user_name, &p.user_email) {
                Ok(()) => app.toast(t("notify.identity_saved"), false),
                Err(e) => app.toast(e, true),
            }
        }
    });
}

fn language(app: &mut App, ui: &mut Ui) {
    heading(ui, &t("prefs.language"));
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(t("prefs.language")).color(TEXT));
        screens::language_combo(app, ui, "pref_lang");
    });
    ui.add_space(10.0);
    ui.label(egui::RichText::new(t("prefs.lang_how")).color(TEXT));
    ui.add_space(6.0);
    for d in gcc_core::i18n::lang_dirs() {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(ic::FOLDER_SIMPLE).color(TEXT_DIM));
            if ui.link(d.display().to_string()).clicked() {
                let _ = std::fs::create_dir_all(&d);
                gcc_core::util::open_path(&d);
            }
        });
    }
    ui.add_space(10.0);
    let langs = gcc_core::i18n::global().read().map(|g| (g.languages.clone(), g.current.clone())).unwrap_or_default();
    egui::Grid::new("langs").num_columns(3).spacing([24.0, 6.0]).show(ui, |ui| {
        ui.label(egui::RichText::new(t("prefs.lang_code")).strong());
        ui.label(egui::RichText::new(t("prefs.lang_name")).strong());
        ui.label(egui::RichText::new(t("prefs.lang_missing")).strong());
        ui.end_row();
        for l in &langs.0 {
            let missing = gcc_core::i18n::global().read().map(|g| g.missing_keys(&l.code).len()).unwrap_or(0);
            let c = if l.code == langs.1 { CHECK_GREEN } else { TEXT };
            ui.label(egui::RichText::new(&l.code).color(c));
            ui.label(egui::RichText::new(&l.name).color(c));
            ui.label(if l.path.is_none() { "—".to_string() } else { missing.to_string() });
            ui.end_row();
        }
    });
    ui.add_space(12.0);
    ui.label(egui::RichText::new(t("prefs.lang_new")).color(TEXT));
    let id = egui::Id::new("new_lang_code");
    let mut code: String = ui.data(|d| d.get_temp(id)).unwrap_or_default();
    ui.horizontal(|ui| {
        input(ui, &mut code, "fr", 120.0, 28.0, INPUT);
        if ui.button(t("prefs.lang_create")).clicked() && !code.trim().is_empty() {
            let dir = gcc_core::i18n::user_lang_dir();
            let _ = std::fs::create_dir_all(&dir);
            let dest = dir.join(format!("{}.json", code.trim().to_lowercase()));
            let src = gcc_core::i18n::project_lang_dir().map(|d| d.join("en.json"));
            let text = src.and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_else(|| include_str!("../../lang/en.json").to_string());
            let text = text.replacen("\"English\"", &format!("\"{}\"", code.trim()), 1);
            match std::fs::write(&dest, text) {
                Ok(()) => {
                    app.toast(tf("notify.lang_file_created", &[&dest.display().to_string()]), false);
                    gcc_core::util::open_path(&dir);
                }
                Err(e) => app.toast(e.to_string(), true),
            }
        }
    });
    ui.data_mut(|d| d.insert_temp(id, code));
}

fn ai(app: &mut App, ui: &mut Ui) {
    heading(ui, &t("prefs.ai"));
    ui.label(egui::RichText::new(t("prefs.ai_intro")).color(TEXT_DIM));
    ui.add_space(8.0);
    let a = &mut app.settings.ai;
    ui.horizontal(|ui| {
        ui.radio_value(&mut a.mode, AiMode::Off, t("prefs.ai_off"));
        ui.radio_value(&mut a.mode, AiMode::Api, t("prefs.ai_api"));
        ui.radio_value(&mut a.mode, AiMode::Acp, t("prefs.ai_acp"));
    });
    ui.add_space(8.0);
    let mut list_models = false;
    let mut test = false;
    match a.mode {
        AiMode::Off => {
            ui.label(egui::RichText::new(t("prefs.ai_off_hint")).color(TEXT_DIM));
        }
        AiMode::Api => {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(t("prefs.ai_provider")).color(TEXT));
                let prev = a.api_kind;
                egui::ComboBox::from_id_source("api_kind").selected_text(a.api_kind.label()).show_ui(ui, |ui| {
                    for k in [ApiKind::Anthropic, ApiKind::OpenAiCompatible, ApiKind::Ollama] {
                        ui.selectable_value(&mut a.api_kind, k, k.label());
                    }
                });
                if prev != a.api_kind {
                    a.base_url = a.api_kind.default_url().into();
                    a.model = a.api_kind.default_model().into();
                }
            });
            ui.label(egui::RichText::new(t("prefs.ai_openai_hint")).color(TEXT_DIM).size(12.0));
            ui.add_space(4.0);
            field(ui, &t("prefs.ai_url"), &mut a.base_url, "");
            ui.label(egui::RichText::new(t("prefs.ai_key")).color(TEXT).size(13.5));
            let w = ui.available_width().min(560.0);
            ui.add_sized([w, 28.0], egui::TextEdit::singleline(&mut a.api_key).password(true).hint_text("sk-…"));
            ui.add_space(6.0);
            field(ui, &t("prefs.ai_model"), &mut a.model, "");
            ui.horizontal(|ui| {
                if ui.button(t("prefs.ai_list_models")).clicked() {
                    list_models = true;
                }
                if ui.button(t("prefs.ai_test")).clicked() {
                    test = true;
                }
            });
            if !app.ai_models.is_empty() {
                egui::ScrollArea::vertical().max_height(140.0).id_source("models").show(ui, |ui| {
                    for m in &app.ai_models {
                        if ui.selectable_label(a.model == *m, m).clicked() {
                            a.model = m.clone();
                        }
                    }
                });
            }
        }
        AiMode::Acp => {
            ui.label(egui::RichText::new(t("prefs.ai_acp_hint")).color(TEXT_DIM));
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new(t("prefs.ai_presets")).color(TEXT));
                for (name, cmd, args) in [
                    ("Claude Code", "npx", "@zed-industries/claude-code-acp"),
                    ("Gemini CLI", "gemini", "--experimental-acp"),
                    ("Codex", "npx", "@zed-industries/codex-acp"),
                ] {
                    if ui.button(name).clicked() {
                        a.acp_command = cmd.into();
                        a.acp_args = args.into();
                    }
                }
            });
            field(ui, &t("prefs.ai_command"), &mut a.acp_command, "npx");
            field(ui, &t("prefs.ai_args"), &mut a.acp_args, "");
            if ui.button(t("prefs.ai_test")).clicked() {
                test = true;
            }
        }
    }
    if a.mode != AiMode::Off {
        ui.add_space(8.0);
        ui.label(egui::RichText::new(t("prefs.ai_system")).color(TEXT).size(13.5));
        let w = ui.available_width().min(560.0);
        ui.add_sized([w, 70.0], egui::TextEdit::multiline(&mut a.system_prompt));
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(t("prefs.ai_max_diff")).color(TEXT));
            ui.add(egui::DragValue::new(&mut a.max_diff_chars).range(2_000..=400_000).speed(1000));
        });
    }
    if list_models {
        let s = app.settings.ai.clone();
        app.spawn_job(t("prefs.ai_list_models"), None, move |_| gcc_core::ai::list_models(&s).map(JobOut::Models));
    }
    if test {
        let s = app.settings.ai.clone();
        let cwd = std::env::temp_dir();
        app.spawn_job(t("prefs.ai_test"), None, move |_| gcc_core::ai::ask_once(&s, "Reply with one short friendly sentence to confirm you are connected.", &cwd).map(JobOut::Msg));
    }
}

fn git_page(app: &mut App, ui: &mut Ui) {
    heading(ui, &t("prefs.git"));
    match (&app.engine.cli, &app.git_version) {
        (Some(p), Some(v)) => {
            ui.label(egui::RichText::new(format!("{} {}", ic::CHECK, tf("prefs.git_found", &[v, &p.display().to_string()]))).color(CHECK_GREEN));
        }
        _ => {
            ui.label(egui::RichText::new(format!("{} {}", ic::INFO, t("prefs.git_missing"))).color(MOD_YELLOW));
        }
    }
    ui.label(egui::RichText::new(tf("prefs.engine", &[&git::engine_version()])).color(TEXT_DIM).size(12.5));
    ui.add_space(6.0);
    let mut changed = false;
    ui.horizontal(|ui| {
        changed |= checkbox(ui, &mut app.settings.prefer_git_cli).clicked();
        ui.label(egui::RichText::new(t("prefs.prefer_cli")).color(TEXT));
    });
    changed |= field(ui, &t("prefs.git_path"), &mut app.settings.git_path_override, if cfg!(windows) { "C:\\Program Files\\Git\\cmd\\git.exe" } else { "/usr/bin/git" });
    if changed {
        app.engine = make_engine(&app.settings);
        app.git_version = app.engine.cli.as_ref().and_then(|p| git::git_version(p));
    }

    ui.add_space(14.0);
    ui.label(egui::RichText::new(format!("{} {}", ic::DOWNLOAD_SIMPLE, t("gitdl.title"))).font(bold(16.5)).color(TEXT_BRIGHT));
    ui.label(egui::RichText::new(tf("gitdl.system", &[gitdl::os_name(), gitdl::arch()])).color(TEXT_DIM));
    ui.add_space(6.0);
    if app.gitdl.releases.is_empty() {
        ui.horizontal(|ui| {
            if ui.add_enabled(!app.gitdl.loading, egui::Button::new(t("gitdl.load"))).clicked() {
                app.gitdl.loading = true;
                app.spawn_job(t("gitdl.title"), None, |_| gitdl::list_releases().map(JobOut::Releases));
            }
            if app.gitdl.loading {
                ui.spinner();
            }
        });
        if let Some(e) = &app.gitdl.error {
            ui.label(egui::RichText::new(e).color(DEL_RED));
        }
    } else {
        ui.horizontal(|ui| {
            checkbox(ui, &mut app.gitdl.show_pre);
            ui.label(egui::RichText::new(t("gitdl.show_pre")).color(TEXT));
        });
        let visible: Vec<usize> = app
            .gitdl
            .releases
            .iter()
            .enumerate()
            .filter(|(_, r)| app.gitdl.show_pre || !r.prerelease)
            .map(|(i, _)| i)
            .collect();
        if !visible.contains(&app.gitdl.selected) {
            app.gitdl.selected = visible.first().copied().unwrap_or(0);
            app.gitdl.asset = 0;
        }
        let rel = &app.gitdl.releases[app.gitdl.selected];
        let mut sel = app.gitdl.selected;
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(t("gitdl.version")).color(TEXT));
            egui::ComboBox::from_id_source("git_ver")
                .selected_text(format!("{} {}", rel.version, rel.date))
                .width(220.0)
                .show_ui(ui, |ui| {
                    for &i in &visible {
                        let r = &app.gitdl.releases[i];
                        let label = format!("{} {}{}", r.version, r.date, if r.prerelease { " (pre)" } else { "" });
                        ui.selectable_value(&mut sel, i, label);
                    }
                });
        });
        if sel != app.gitdl.selected {
            app.gitdl.selected = sel;
            app.gitdl.asset = 0;
        }
        let rel = app.gitdl.releases[app.gitdl.selected].clone();
        let mut asset = app.gitdl.asset.min(rel.assets.len().saturating_sub(1));
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(t("gitdl.package")).color(TEXT));
            let cur = &rel.assets[asset];
            egui::ComboBox::from_id_source("git_asset")
                .selected_text(format!("{} — {}", t(cur.kind.key()), cur.name))
                .width(380.0)
                .show_ui(ui, |ui| {
                    for (i, a) in rel.assets.iter().enumerate() {
                        let size = if a.size > 0 { format!(" ({})", gcc_core::util::human_size(a.size)) } else { String::new() };
                        ui.selectable_value(&mut asset, i, format!("{} — {}{}", t(a.kind.key()), a.name, size));
                    }
                });
        });
        app.gitdl.asset = asset;
        let a = rel.assets[asset].clone();
        ui.label(egui::RichText::new(t(&format!("{}_hint", a.kind.key()))).color(TEXT_DIM).size(12.5));
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let label = format!("{} {}", ic::DOWNLOAD_SIMPLE, t("gitdl.download_install"));
            if ui.add_enabled(!app.gitdl.busy, egui::Button::new(label).fill(CREATE_GREEN)).clicked() {
                app.gitdl.busy = true;
                let a2 = a.clone();
                app.spawn_job(tf("gitdl.downloading", &[&a.name]), None, move |p| {
                    let file = gitdl::download(&a2, &mut |f| p(f, ""))?;
                    gitdl::install(&a2, &file).map(JobOut::GitInstalled)
                });
            }
            if ui.button(t("gitdl.reload")).clicked() {
                app.gitdl.releases.clear();
                app.gitdl.loading = true;
                app.spawn_job(t("gitdl.title"), None, |_| gitdl::list_releases().map(JobOut::Releases));
            }
        });
    }
    if let Some(cmd) = gitdl::package_command() {
        ui.add_space(10.0);
        ui.label(egui::RichText::new(t("gitdl.package_manager")).color(TEXT));
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(&cmd).font(mono(13.0)).color(LINK_SOFT));
            if ui.small_button(format!("{} {}", ic::COPY, t("ai.copy"))).clicked() {
                ui.ctx().output_mut(|o| o.copied_text = cmd.clone());
            }
        });
    }
    if git::managed_git_dir().exists() {
        ui.add_space(10.0);
        ui.label(egui::RichText::new(tf("gitdl.managed_at", &[&git::managed_git_dir().display().to_string()])).color(TEXT_DIM));
        if ui.button(t("gitdl.remove")).clicked() {
            match gitdl::remove_managed() {
                Ok(()) => {
                    app.engine = make_engine(&app.settings);
                    app.git_version = app.engine.cli.as_ref().and_then(|p| git::git_version(p));
                    app.toast(t("gitdl.removed"), false);
                }
                Err(e) => app.toast(e, true),
            }
        }
    }
}

fn integrations(app: &mut App, ui: &mut Ui) {
    heading(ui, &t("prefs.integrations"));
    ui.label(egui::RichText::new(t("prefs.tokens_hint")).color(TEXT_DIM));
    ui.add_space(8.0);
    let w = ui.available_width().min(560.0);
    let mut changed = false;
    ui.label(egui::RichText::new("GitHub token").color(TEXT));
    ui.horizontal(|ui| {
        changed |= ui.add_sized([w, 28.0], egui::TextEdit::singleline(&mut app.settings.github_token).password(true).hint_text("ghp_… / github_pat_…")).changed();
        if ui.button(t("prefs.test_token")).clicked() {
            let tok = app.settings.github_token.clone();
            app.spawn_job(t("prefs.test_token"), None, move |_| {
                gcc_core::auth::github_whoami(&tok).map(|i| JobOut::Msg(gcc_core::auth::describe(&i)))
            });
        }
    });
    ui.add_space(6.0);
    ui.label(egui::RichText::new("GitLab token").color(TEXT));
    ui.horizontal(|ui| {
        changed |= ui.add_sized([w, 28.0], egui::TextEdit::singleline(&mut app.settings.gitlab_token).password(true).hint_text("glpat-…")).changed();
        if ui.button(t("prefs.test_token")).clicked() {
            let tok = app.settings.gitlab_token.clone();
            app.spawn_job(t("prefs.test_token"), None, move |_| {
                gcc_core::auth::gitlab_whoami(&tok, "https://gitlab.com/").map(|i| JobOut::Msg(gcc_core::auth::describe(&i)))
            });
        }
    });
    if changed {
        // Use the new token right away for push, pull and fetch.
        app.engine.token_github = app.settings.github_token.trim().to_string();
        app.engine.token_gitlab = app.settings.gitlab_token.trim().to_string();
    }
}

fn mcp(app: &mut App, ui: &mut Ui) {
    heading(ui, &t("prefs.mcp"));
    ui.label(egui::RichText::new(t("prefs.mcp_intro")).color(TEXT));
    ui.add_space(8.0);
    let exe = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join(if cfg!(windows) { "gitcerberocraft-cli.exe" } else { "gitcerberocraft-cli" })))
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "gitcerberocraft-cli".into());
    let repo = app
        .active_repo_id()
        .and_then(|id| app.tabs.iter().find(|t| t.id == id))
        .and_then(|t| match &t.kind {
            TabKind::Repo(r) => Some(r.path.display().to_string()),
            _ => None,
        })
        .unwrap_or_else(|| "C:\\path\\to\\repo".into());
    let cfg = serde_json::to_string_pretty(&serde_json::json!({
        "mcpServers": {"gitcerberocraft": {"command": exe, "args": ["mcp", "--repo", repo]}}
    }))
    .unwrap_or_default();
    egui::Frame::none().fill(INPUT).rounding(3.0).inner_margin(10.0).show(ui, |ui| {
        ui.label(egui::RichText::new(&cfg).font(mono(12.5)).color(hex(0xC8E1C8)));
    });
    if ui.button(format!("{} {}", ic::COPY, t("prefs.mcp_copy"))).clicked() {
        ui.ctx().output_mut(|o| o.copied_text = cfg.clone());
    }
    ui.add_space(10.0);
    ui.label(egui::RichText::new(t("prefs.mcp_tools")).font(bold(15.0)).color(TEXT_BRIGHT));
    egui::Grid::new("mcp_tools").num_columns(2).spacing([20.0, 4.0]).show(ui, |ui| {
        for tl in gcc_core::mcp::tools() {
            ui.label(egui::RichText::new(tl["name"].as_str().unwrap_or("")).font(mono(12.5)).color(LINK_SOFT));
            ui.label(egui::RichText::new(tl["description"].as_str().unwrap_or("")).color(TEXT).size(13.0));
            ui.end_row();
        }
    });
    ui.add_space(10.0);
    ui.label(egui::RichText::new(t("prefs.mcp_cli")).color(TEXT_DIM));
}
