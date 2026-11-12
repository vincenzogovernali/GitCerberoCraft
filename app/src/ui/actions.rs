//! What the buttons of a repository tab do.

use super::repo::{DiffView, RepoAction, RightView, Sel, UndoEntry};
use super::*;
use gcc_core::ai::{self, AiEvent, AiRequest, ChatMessage};
use gcc_core::git;
use gcc_core::i18n::{t, tf};

fn result_toast(app: &mut App, ok_key: &str, r: Result<String, String>) {
    match r {
        Ok(m) if m.is_empty() => app.toast(t(ok_key), false),
        Ok(m) => app.toast(format!("{}: {}", t(ok_key), first_line(&m)), false),
        Err(e) => app.toast(e, true),
    }
}

fn reload(app: &mut App, id: u64) {
    let max = app.settings.graph_max_commits;
    if let Some(r) = app.tab_mut(id) {
        r.reload(max);
    }
}

pub fn handle(app: &mut App, id: u64, action: RepoAction) {
    let Some(path) = app.tab_mut(id).map(|r| r.path.clone()) else { return };
    let e = app.engine.clone();
    match action {
        RepoAction::Refresh => {
            reload(app, id);
            app.toast(t("notify.refreshed"), false);
        }
        RepoAction::Fetch => {
            app.spawn_job(t("jobs.fetch"), Some(id), move |p| git::fetch(&e, &path, None, Some(p)).map(JobOut::Msg));
        }
        RepoAction::Pull(mode) => {
            app.settings.pull_mode = mode;
            let title = if mode == gcc_core::config::PullMode::FetchAll { t("jobs.fetch") } else { t("jobs.pull") };
            app.spawn_job(title, Some(id), move |p| git::pull(&e, &path, mode, Some(p)).map(JobOut::Msg));
        }
        RepoAction::Push(force) => {
            if force {
                app.modal = Some(Modal::Confirm { title: t("push.force"), text: t("confirm.force_push"), action: ConfirmAction::ForcePush(id) });
            } else {
                app.spawn_job(t("jobs.push"), Some(id), move |p| git::push(&e, &path, false, Some(p)).map(JobOut::Msg));
            }
        }
        RepoAction::Undo => undo(app, id, true),
        RepoAction::Redo => undo(app, id, false),
        RepoAction::NewBranch(at) => {
            app.modal = Some(Modal::Input {
                title: t("branch.create_title"),
                label: t("branch.name"),
                value: String::new(),
                label2: String::new(),
                value2: String::new(),
                kind: InputKind::NewBranch { tab: id, at },
            })
        }
        RepoAction::RenameBranch(old) => {
            app.modal = Some(Modal::Input {
                title: t("ctx.rename_branch"),
                label: t("branch.name"),
                value: old.clone(),
                label2: String::new(),
                value2: String::new(),
                kind: InputKind::RenameBranch { tab: id, old },
            })
        }
        RepoAction::Tag(oid) => {
            app.modal = Some(Modal::Input {
                title: t("ctx.create_tag_here"),
                label: t("tag.name"),
                value: String::new(),
                label2: t("tag.message"),
                value2: String::new(),
                kind: InputKind::Tag { tab: id, oid },
            })
        }
        RepoAction::Stash => {
            app.modal = Some(Modal::Input {
                title: t("toolbar.stash"),
                label: t("stash.message"),
                value: String::new(),
                label2: String::new(),
                value2: String::new(),
                kind: InputKind::Stash { tab: id },
            })
        }
        RepoAction::AddRemote => {
            app.modal = Some(Modal::Input {
                title: t("actions.add_remote"),
                label: t("remote.name"),
                value: "origin".into(),
                label2: t("remote.url"),
                value2: String::new(),
                kind: InputKind::AddRemote { tab: id },
            })
        }
        RepoAction::Pop(i) => {
            let r = git::stash_pop(&path, i).map(|_| String::new());
            result_toast(app, "notify.stash_popped", r);
            reload(app, id);
        }
        RepoAction::ApplyStash(i) => {
            let r = git::stash_apply(&path, i).map(|_| String::new());
            result_toast(app, "notify.stash_applied", r);
            reload(app, id);
        }
        RepoAction::DropStash(i) => {
            app.modal = Some(Modal::Confirm { title: t("ctx.drop_stash"), text: t("confirm.drop_stash"), action: ConfirmAction::DropStash(id, i) });
        }
        RepoAction::Checkout(name) => {
            let from = app.tab_mut(id).and_then(|r| r.snap.as_ref().map(|s| s.head_name.clone())).unwrap_or_default();
            match git::checkout(&path, &name) {
                Ok(()) => {
                    let to = name.split_once('/').filter(|_| !git::checkout_is_local(&path, &name)).map(|x| x.1.to_string()).unwrap_or(name.clone());
                    if let Some(r) = app.tab_mut(id) {
                        r.undo.push(UndoEntry::Checkout { from, to: to.clone() });
                        r.redo.clear();
                    }
                    app.toast(tf("notify.checked_out", &[&to]), false);
                }
                Err(err) => app.toast(err, true),
            }
            reload(app, id);
        }
        RepoAction::CheckoutCommit(oid) => {
            let r = git::checkout_commit(&path, oid).map(|_| String::new());
            result_toast(app, "notify.detached", r);
            reload(app, id);
        }
        RepoAction::DeleteBranch(name) => {
            app.modal = Some(Modal::Confirm {
                title: tf("ctx.delete_branch", &[&name]),
                text: tf("confirm.delete_branch", &[&name]),
                action: ConfirmAction::DeleteBranch(id, name),
            });
        }
        RepoAction::Merge(name) => {
            app.spawn_job(tf("jobs.merge", &[&name]), Some(id), move |_| git::merge(&e, &path, &name, false).map(JobOut::Msg));
        }
        RepoAction::CherryPick(oid) => {
            let r = git::cherry_pick(&e, &path, oid);
            result_toast(app, "notify.cherry_picked", r);
            reload(app, id);
        }
        RepoAction::Revert(oid) => {
            let r = git::revert(&e, &path, oid);
            result_toast(app, "notify.reverted", r);
            reload(app, id);
        }
        RepoAction::Reset(oid, mode) => {
            if mode == "hard" {
                app.modal = Some(Modal::Confirm { title: t("ctx.reset_hard"), text: t("confirm.reset_hard"), action: ConfirmAction::ResetHard(id, oid) });
            } else {
                do_reset(app, id, oid, mode);
            }
        }
        RepoAction::DeleteTag(name) => {
            let r = git::delete_tag(&path, &name).map(|_| String::new());
            result_toast(app, "notify.tag_deleted", r);
            reload(app, id);
        }
        RepoAction::Stage(files) => {
            if let Err(err) = git::stage(&path, &files) {
                app.toast(err, true);
            }
            after_wip_change(app, id);
        }
        RepoAction::Unstage(files) => {
            if let Err(err) = git::unstage(&path, &files) {
                app.toast(err, true);
            }
            after_wip_change(app, id);
        }
        RepoAction::StageAll => {
            if let Err(err) = git::stage_all(&path) {
                app.toast(err, true);
            }
            after_wip_change(app, id);
        }
        RepoAction::UnstageAll => {
            if let Err(err) = git::unstage_all(&path) {
                app.toast(err, true);
            }
            after_wip_change(app, id);
        }
        RepoAction::Discard(files) => {
            app.modal = Some(Modal::Confirm { title: t("wip.discard_file"), text: t("confirm.discard"), action: ConfirmAction::Discard(id, files) });
        }
        RepoAction::DiscardAll => {
            app.modal = Some(Modal::Confirm { title: t("wip.discard_all"), text: t("confirm.discard_all"), action: ConfirmAction::DiscardAll(id) });
        }
        RepoAction::Commit => commit(app, id),
        RepoAction::OpenTerminal => {
            if let Err(err) = gcc_core::util::open_terminal(&path, &app.settings.external_terminal) {
                app.toast(err, true);
            }
        }
        RepoAction::OpenFolder => gcc_core::util::open_path(&path),
        RepoAction::OpenEditor => {
            if let Err(err) = gcc_core::util::open_editor(&path, &app.settings.external_editor) {
                app.toast(err, true);
            }
        }
        RepoAction::AiExplain(oid) => {
            let summary = app
                .tab_mut(id)
                .and_then(|r| r.snap.as_ref().and_then(|s| s.row_of(oid).map(|i| s.commits[i].summary.clone())))
                .unwrap_or_default();
            let diff = git::commit_diff(&path, oid, None).map(|l| git::diff_text(&l)).unwrap_or_default();
            let diff = clip(&diff, app.settings.ai.max_diff_chars);
            let shown = tf("ai.q_explain", &[&oid.to_string()[..7], &summary]);
            ask_ai(app, id, shown, ai::prompt_explain_commit(&summary, &diff), AiPurpose::Chat);
        }
        RepoAction::AiReview => {
            let staged = git::wip_diff(&path, None, true).map(|l| git::diff_text(&l)).unwrap_or_default();
            let unstaged = git::wip_diff(&path, None, false).map(|l| git::diff_text(&l)).unwrap_or_default();
            let diff = clip(&format!("{staged}\n{unstaged}"), app.settings.ai.max_diff_chars);
            if diff.trim().is_empty() {
                app.toast(t("ai.nothing_to_review"), true);
                return;
            }
            ask_ai(app, id, t("ai.q_review"), ai::prompt_review(&diff), AiPurpose::Chat);
        }
        RepoAction::AiCommitMessage => {
            let mut diff = git::wip_diff(&path, None, true).map(|l| git::diff_text(&l)).unwrap_or_default();
            if diff.trim().is_empty() {
                diff = git::wip_diff(&path, None, false).map(|l| git::diff_text(&l)).unwrap_or_default();
            }
            let diff = clip(&diff, app.settings.ai.max_diff_chars);
            if diff.trim().is_empty() {
                app.toast(t("ai.nothing_to_review"), true);
                return;
            }
            ask_ai(app, id, t("ai.q_message"), ai::prompt_commit_message(&diff), AiPurpose::CommitMessage);
        }
        RepoAction::AiAsk(q) => {
            let ctx = git::status_text(&path).unwrap_or_default();
            let sel = app
                .tab_mut(id)
                .and_then(|r| r.selected_oid().map(|o| (o, r.snap.as_ref().and_then(|s| s.row_of(o)).map(|i| s_summary(r, i)))))
                .map(|(o, s)| format!("\nSelected commit: {} {}", o, s.unwrap_or_default()))
                .unwrap_or_default();
            let full = format!("{q}\n\n---\nRepository context:\n{ctx}{sel}");
            ask_ai(app, id, q, full, AiPurpose::Chat);
        }
        RepoAction::Copy(text) => {
            app.ctx.output_mut(|o| o.copied_text = text);
            app.toast(t("notify.copied"), false);
        }
        RepoAction::SwitchRepo(p) => {
            app.open_repo(PathBuf::from(p));
        }
        RepoAction::OpenRepo => app.browse_open_repo(),
        RepoAction::AbortMerge => {
            let r = git::abort_merge(&e, &path).map(|_| String::new());
            result_toast(app, "notify.merge_aborted", r);
            reload(app, id);
        }
        RepoAction::RemoveRemote(name) => {
            app.modal = Some(Modal::Confirm {
                title: t("ctx.remove_remote"),
                text: tf("confirm.remove_remote", &[&name]),
                action: ConfirmAction::RemoveRemote(id, name),
            });
        }
        RepoAction::ShowDiffCommit(oid, file) => {
            match git::commit_diff(&path, oid, Some(&file)) {
                Ok(lines) => {
                    if let Some(r) = app.tab_mut(id) {
                        r.diff = Some(DiffView { title: file.clone(), file, lines, staged: None });
                    }
                }
                Err(err) => app.toast(err, true),
            }
        }
        RepoAction::ShowDiffWip(file, staged) => match git::wip_diff(&path, Some(&file), staged) {
            Ok(lines) => {
                if let Some(r) = app.tab_mut(id) {
                    let title = format!("{} ({})", file, if staged { t("wip.staged_short") } else { t("wip.unstaged_short") });
                    r.diff = Some(DiffView { title, file, lines, staged: Some(staged) });
                }
            }
            Err(err) => app.toast(err, true),
        },
        RepoAction::RunTerminal(line) => {
            let extra = app.engine.cli.as_ref().and_then(|p| p.parent().map(|d| d.to_path_buf()));
            let out = gcc_core::util::run_shell(&path, &line, extra.as_deref());
            if let Some(r) = app.tab_mut(id) {
                r.term_output.push_str(&format!("$ {line}\n{out}\n"));
                if r.term_output.len() > 200_000 {
                    let cut = r.term_output.len() - 150_000;
                    r.term_output = r.term_output[cut..].to_string();
                }
            }
            reload(app, id);
        }
        RepoAction::ConfigureAi => app.open_prefs(PrefPage::Ai),
    }
}

fn s_summary(r: &super::repo::RepoTab, i: usize) -> String {
    r.snap.as_ref().map(|s| s.commits[i].summary.clone()).unwrap_or_default()
}

fn clip(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut end = max;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}\n… (diff truncated)", &s[..end])
}

fn after_wip_change(app: &mut App, id: u64) {
    let max = app.settings.graph_max_commits;
    if let Some(r) = app.tab_mut(id) {
        let file = r.diff.as_ref().and_then(|d| d.staged.map(|_| d.file.clone()));
        r.reload(max);
        if r.has_wip() {
            r.sel = Sel::Wip;
        }
        if file.is_some() {
            r.diff = None;
        }
    }
}

pub fn commit(app: &mut App, id: u64) {
    let e = app.engine.clone();
    let Some(r) = app.tab_mut(id) else { return };
    let path = r.path.clone();
    let mut msg = r.summary.trim().to_string();
    if !r.description.trim().is_empty() {
        msg = format!("{msg}\n\n{}", r.description.trim());
    }
    let amend = r.amend;
    let before = r.snap.as_ref().and_then(|s| s.head_oid);
    match git::commit(&e, &path, &msg, amend) {
        Ok(oid) => {
            if let Some(r) = app.tab_mut(id) {
                r.summary.clear();
                r.description.clear();
                r.amend = false;
                if !amend {
                    r.undo.push(UndoEntry::Commit { before, after: oid });
                    r.redo.clear();
                }
                r.sel = Sel::None;
            }
            app.toast(tf("notify.committed", &[&oid.to_string()[..7]]), false);
        }
        Err(err) => app.toast(err, true),
    }
    reload(app, id);
}

fn do_reset(app: &mut App, id: u64, oid: git2::Oid, mode: &str) {
    let Some(r) = app.tab_mut(id) else { return };
    let path = r.path.clone();
    let before = r.snap.as_ref().and_then(|s| s.head_oid);
    match git::reset(&path, oid, mode) {
        Ok(()) => {
            if let (Some(b), Some(r)) = (before, app.tab_mut(id)) {
                r.undo.push(UndoEntry::Reset { before: b, after: oid });
                r.redo.clear();
            }
            app.toast(t("notify.reset_done"), false);
        }
        Err(e) => app.toast(e, true),
    }
    reload(app, id);
}

fn undo(app: &mut App, id: u64, is_undo: bool) {
    let Some(r) = app.tab_mut(id) else { return };
    let path = r.path.clone();
    let entry = if is_undo { r.undo.pop() } else { r.redo.pop() };
    let Some(entry) = entry else { return };
    let res: Result<(), String> = match (&entry, is_undo) {
        (UndoEntry::Commit { before, .. }, true) => match before {
            Some(b) => git::reset(&path, *b, "soft"),
            None => Err(t("undo.first_commit")),
        },
        (UndoEntry::Commit { after, .. }, false) => git::reset(&path, *after, "soft"),
        (UndoEntry::Checkout { from, .. }, true) => git::checkout(&path, from),
        (UndoEntry::Checkout { to, .. }, false) => git::checkout(&path, to),
        (UndoEntry::BranchCreate { name, .. }, true) => git::delete_branch(&path, name).map(|_| ()),
        (UndoEntry::BranchCreate { name, oid }, false) => git::create_branch(&path, name, Some(*oid), false),
        (UndoEntry::BranchDelete { name, oid }, true) => git::create_branch(&path, name, Some(*oid), false),
        (UndoEntry::BranchDelete { name, .. }, false) => git::delete_branch(&path, name).map(|_| ()),
        (UndoEntry::Reset { before, .. }, true) => git::reset(&path, *before, "mixed"),
        (UndoEntry::Reset { after, .. }, false) => git::reset(&path, *after, "mixed"),
    };
    match res {
        Ok(()) => {
            if let Some(r) = app.tab_mut(id) {
                if is_undo {
                    r.redo.push(entry);
                } else {
                    r.undo.push(entry);
                }
            }
            app.toast(if is_undo { t("notify.undone") } else { t("notify.redone") }, false);
        }
        Err(e) => app.toast(e, true),
    }
    reload(app, id);
}

fn ask_ai(app: &mut App, id: u64, shown: String, prompt: String, purpose: AiPurpose) {
    if !app.ai_on() {
        app.toast(t("ai.not_configured"), true);
        if let Some(r) = app.tab_mut(id) {
            r.right = RightView::Ai;
        }
        return;
    }
    let settings = app.settings.ai.clone();
    let Some(r) = app.tab_mut(id) else { return };
    if r.ai_busy {
        return;
    }
    r.right = RightView::Ai;
    r.ai_busy = true;
    r.ai_status.clear();
    r.ai_msgs.push(ChatMessage { role: "user".into(), text: shown });
    // The model sees the full prompt for the last message.
    let mut history: Vec<ChatMessage> = r.ai_msgs.iter().rev().skip(1).take(8).rev().cloned().collect();
    history.push(ChatMessage { role: "user".into(), text: prompt });
    r.ai_msgs.push(ChatMessage { role: "assistant".into(), text: String::new() });
    let cwd = r.path.clone();
    app.ai_tab = Some(id);
    app.ai_purpose = purpose;
    app.ai.ask(AiRequest { settings, history, cwd });
}

pub fn process_ai(app: &mut App) {
    let mut events = Vec::new();
    while let Ok(e) = app.ai.rx.try_recv() {
        events.push(e);
    }
    if events.is_empty() {
        return;
    }
    let Some(id) = app.ai_tab else { return };
    let purpose = app.ai_purpose;
    let mut error = None;
    let mut finished_text = None;
    if let Some(r) = app.tab_mut(id) {
        for e in events {
            match e {
                AiEvent::Chunk(t) => {
                    if let Some(last) = r.ai_msgs.last_mut() {
                        last.text.push_str(&t);
                    }
                }
                AiEvent::Status(s) => r.ai_status = s,
                AiEvent::Done => {
                    r.ai_busy = false;
                    finished_text = r.ai_msgs.last().map(|m| m.text.clone());
                }
                AiEvent::Error(e) => {
                    r.ai_busy = false;
                    if let Some(last) = r.ai_msgs.last_mut() {
                        if last.text.is_empty() {
                            last.text = format!("⚠ {e}");
                        }
                    }
                    error = Some(e);
                }
            }
        }
        if let (Some(text), AiPurpose::CommitMessage) = (&finished_text, purpose) {
            let clean = text.trim().trim_matches('`').trim().to_string();
            let mut parts = clean.splitn(2, '\n');
            r.summary = parts.next().unwrap_or("").trim().to_string();
            r.description = parts.next().unwrap_or("").trim().to_string();
            if r.has_wip() {
                r.sel = Sel::Wip;
                r.right = RightView::Details;
            }
        }
    }
    if let Some(e) = error {
        app.toast(e, true);
    }
    app.ctx.request_repaint();
}

/// Carry out a confirmed dangerous action.
pub fn confirm(app: &mut App, action: ConfirmAction) {
    let e = app.engine.clone();
    match action {
        ConfirmAction::DiscardAll(id) => {
            if let Some(p) = app.tab_mut(id).map(|r| r.path.clone()) {
                let r = git::discard_all(&p).map(|_| String::new());
                result_toast(app, "notify.discarded", r);
                after_wip_change(app, id);
            }
        }
        ConfirmAction::Discard(id, files) => {
            if let Some(p) = app.tab_mut(id).map(|r| r.path.clone()) {
                let r = git::discard(&p, &files).map(|_| String::new());
                result_toast(app, "notify.discarded", r);
                after_wip_change(app, id);
            }
        }
        ConfirmAction::ResetHard(id, oid) => do_reset(app, id, oid, "hard"),
        ConfirmAction::DeleteBranch(id, name) => {
            if let Some(p) = app.tab_mut(id).map(|r| r.path.clone()) {
                match git::delete_branch(&p, &name) {
                    Ok(oid) => {
                        if let (Some(o), Some(r)) = (oid, app.tab_mut(id)) {
                            r.undo.push(UndoEntry::BranchDelete { name: name.clone(), oid: o });
                            r.redo.clear();
                        }
                        app.toast(tf("notify.branch_deleted", &[&name]), false);
                    }
                    Err(err) => app.toast(err, true),
                }
                reload(app, id);
            }
        }
        ConfirmAction::ForcePush(id) => {
            if let Some(p) = app.tab_mut(id).map(|r| r.path.clone()) {
                app.spawn_job(t("jobs.push"), Some(id), move |pr| git::push(&e, &p, true, Some(pr)).map(JobOut::Msg));
            }
        }
        ConfirmAction::DropStash(id, i) => {
            if let Some(p) = app.tab_mut(id).map(|r| r.path.clone()) {
                let r = git::stash_drop(&p, i).map(|_| String::new());
                result_toast(app, "notify.stash_dropped", r);
                reload(app, id);
            }
        }
        ConfirmAction::RemoveRemote(id, name) => {
            if let Some(p) = app.tab_mut(id).map(|r| r.path.clone()) {
                let r = git::remove_remote(&p, &name).map(|_| String::new());
                result_toast(app, "notify.remote_removed", r);
                reload(app, id);
            }
        }
    }
}

/// Carry out a filled-in input dialog. Returns false to keep the dialog open.
pub fn submit_input(app: &mut App, kind: InputKind, value: String, value2: String) -> bool {
    let e = app.engine.clone();
    let tab = match &kind {
        InputKind::NewBranch { tab, .. }
        | InputKind::RenameBranch { tab, .. }
        | InputKind::Tag { tab, .. }
        | InputKind::Stash { tab }
        | InputKind::AddRemote { tab } => *tab,
    };
    let Some(path) = app.tab_mut(tab).map(|r| r.path.clone()) else { return true };
    let v = value.trim().replace(' ', "-");
    let res: Result<String, String> = match kind {
        InputKind::NewBranch { at, .. } => {
            if v.is_empty() {
                return false;
            }
            let from = app.tab_mut(tab).and_then(|r| r.snap.as_ref().map(|s| s.head_name.clone())).unwrap_or_default();
            match git::create_branch(&path, &v, at, true) {
                Ok(()) => {
                    let oid = git::parse_oid(&path, &v).ok();
                    if let (Some(o), Some(r)) = (oid, app.tab_mut(tab)) {
                        r.undo.push(UndoEntry::BranchCreate { name: v.clone(), oid: o });
                        let _ = from;
                        r.redo.clear();
                    }
                    Ok(tf("notify.branch_created", &[&v]))
                }
                Err(e) => Err(e),
            }
        }
        InputKind::RenameBranch { old, .. } => git::rename_branch(&path, &old, &v).map(|_| tf("notify.branch_renamed", &[&v])),
        InputKind::Tag { oid, .. } => {
            if v.is_empty() {
                return false;
            }
            git::create_tag(&path, &v, oid, &value2, &e).map(|_| tf("notify.tag_created", &[&v]))
        }
        InputKind::Stash { .. } => git::stash_save(&e, &path, value.trim()).map(|_| t("notify.stashed")),
        InputKind::AddRemote { .. } => {
            if v.is_empty() || value2.trim().is_empty() {
                return false;
            }
            git::add_remote(&path, &v, value2.trim()).map(|_| tf("notify.remote_added", &[&v]))
        }
    };
    match res {
        Ok(m) => app.toast(m, false),
        Err(err) => app.toast(err, true),
    }
    reload(app, tab);
    true
}
