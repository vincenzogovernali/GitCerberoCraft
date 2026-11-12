//! `--self-test <file.json>`: opens a throw-away repository the way a person
//! would, moves through the graph with the keyboard, stages and commits a
//! change through the same actions as the buttons, checks the on-screen
//! information and writes the results.

use super::repo::{RepoAction, Sel};
use super::*;
use serde_json::json;

pub struct SelfTest {
    pub out: PathBuf,
    pub phase: u32,
    pub wait: u32,
    pub repo: PathBuf,
    pub tab: Option<u64>,
    pub steps: Vec<(String, bool, String)>,
    pub pending_keys: Vec<egui::Key>,
    pub commits_before: usize,
    pub frames: u32,
}

impl SelfTest {
    pub fn new(out: PathBuf) -> Self {
        Self {
            out,
            phase: 0,
            wait: 0,
            repo: std::env::temp_dir().join(format!("gcc-selftest-{}", std::process::id())),
            tab: None,
            steps: vec![],
            pending_keys: vec![],
            commits_before: 0,
            frames: 0,
        }
    }
}

const MSG: &str = "Self-test commit";

fn setup_repo(dir: &std::path::Path, e: &gcc_core::git::Engine) -> Result<(), String> {
    let _ = std::fs::remove_dir_all(dir);
    let o = git::InitOptions {
        default_branch: "main".into(),
        gitignore: String::new(),
        license: String::new(),
        lfs: false,
        remote_url: String::new(),
        initial_commit: false,
    };
    git::init(e, dir, &o)?;
    for (i, text) in ["first commit", "add feature", "fix a bug"].iter().enumerate() {
        std::fs::write(dir.join("notes.txt"), format!("line {i}\n")).map_err(|e| e.to_string())?;
        git::stage_all(dir)?;
        git::commit(e, dir, text, false)?;
    }
    git::create_branch(dir, "feature/test", None, false)?;
    Ok(())
}

pub fn step(app: &mut App, ctx: &egui::Context) {
    let Some(mut st) = app.selftest.take() else { return };
    st.frames += 1;
    st.wait += 1;
    match st.phase {
        0 => {
            app.settings.profile_mut().user_name = "Self Test".into();
            app.settings.profile_mut().user_email = "selftest@example.com".into();
            app.engine = make_engine(&app.settings);
            app.engine.cli = None; // exercise the built-in engine, works without Git installed
            match setup_repo(&st.repo, &app.engine) {
                Ok(()) => {
                    st.tab = app.open_repo(st.repo.clone());
                    st.commits_before = st.tab.and_then(|id| app.tab_mut(id)).and_then(|r| r.snap.as_ref().map(|s| s.commits.len())).unwrap_or(0);
                }
                Err(e) => st.steps.push(("setup".into(), false, e)),
            }
            st.phase = 1;
            st.wait = 0;
        }
        1 if st.wait > 15 => {
            ctx.memory_mut(|m| {
                if let Some(f) = m.focused() {
                    m.surrender_focus(f);
                }
            });
            st.pending_keys.extend([egui::Key::ArrowDown, egui::Key::ArrowDown]);
            st.phase = 2;
            st.wait = 0;
        }
        2 if st.wait > 10 => {
            let (sel, summary) = st
                .tab
                .and_then(|id| app.tab_mut(id))
                .map(|r| {
                    let s = match r.sel {
                        Sel::Commit(i) => r.snap.as_ref().map(|s| s.commits[i].summary.clone()).unwrap_or_default(),
                        _ => String::new(),
                    };
                    (r.sel, s)
                })
                .unwrap_or((Sel::None, String::new()));
            let moved = matches!(sel, Sel::Commit(i) if i >= 1);
            st.steps.push((
                "move".into(),
                moved,
                format!("pressed Down twice in the commit graph: selection {:?} on \"{}\"", sel, summary),
            ));
            // Act: change a file, stage it and commit through the button actions.
            let _ = std::fs::write(st.repo.join("notes.txt"), "changed by the self-test\n");
            let _ = std::fs::write(st.repo.join("new-file.md"), "# hello\n");
            if let Some(id) = st.tab {
                actions::handle(app, id, RepoAction::Refresh);
                if let Some(r) = app.tab_mut(id) {
                    r.select(Sel::Wip);
                }
                actions::handle(app, id, RepoAction::StageAll);
                if let Some(r) = app.tab_mut(id) {
                    r.summary = MSG.into();
                }
                actions::handle(app, id, RepoAction::Commit);
            }
            st.phase = 3;
            st.wait = 0;
        }
        3 if st.wait > 12 => {
            let info = st.tab.and_then(|id| app.tab_mut(id)).map(|r| {
                let n = r.snap.as_ref().map(|s| s.commits.len()).unwrap_or(0);
                let top = r.snap.as_ref().and_then(|s| s.commits.first().map(|c| c.summary.clone())).unwrap_or_default();
                let clean = r.snap.as_ref().map(|s| s.wip.is_empty()).unwrap_or(false);
                (n, top, clean, r.hud, r.undo.len())
            });
            if let Some((n, top, clean, hud, undo)) = info {
                let ok = n == st.commits_before + 1 && top == MSG && clean;
                st.steps.push((
                    "act".into(),
                    ok,
                    format!(
                        "staged 2 files and committed: {} -> {} commits, newest \"{}\", working tree clean: {}, undo entries: {}",
                        st.commits_before, n, top, clean, undo
                    ),
                ));
                let hud_ok = hud.toolbar && hud.left_panel && hud.graph_header && hud.rows_drawn > 0 && app.status_drawn;
                st.steps.push((
                    "hud".into(),
                    hud_ok,
                    format!(
                        "toolbar {}, left panel {}, graph header {}, rows drawn {}, status bar {}",
                        hud.toolbar, hud.left_panel, hud.graph_header, hud.rows_drawn, app.status_drawn
                    ),
                ));
            } else {
                st.steps.push(("act".into(), false, "the repository tab did not open".into()));
                st.steps.push(("hud".into(), false, "no repository view".into()));
            }
            finish(&st, ctx);
            st.phase = 4;
        }
        _ => {}
    }
    if st.frames > 3000 && st.phase < 4 {
        st.steps.push(("timeout".into(), false, "self-test did not finish in time".into()));
        finish(&st, ctx);
        st.phase = 4;
    }
    app.selftest = Some(st);
}

fn finish(st: &SelfTest, ctx: &egui::Context) {
    let steps: Vec<serde_json::Value> = st.steps.iter().map(|(s, p, d)| json!({"step": s, "passed": p, "detail": d})).collect();
    let v = json!({"version": 1, "steps": steps});
    if let Some(d) = st.out.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let _ = std::fs::write(&st.out, serde_json::to_string_pretty(&v).unwrap_or_default());
    let _ = std::fs::remove_dir_all(&st.repo);
    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
}
