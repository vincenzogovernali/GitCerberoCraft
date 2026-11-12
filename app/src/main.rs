//! GitCerberoCraft desktop app.
//!
//!   gitcerberocraft [repo-path]
//!   gitcerberocraft --screenshot shot.png   draw ~120 frames, save a PNG, quit
//!   gitcerberocraft --self-test result.json scripted check of the core loop

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod ui;

use eframe::egui;
use std::path::PathBuf;
use std::sync::Arc;

/// The person's icon (assets/icon.png), resized at build time.
fn icon() -> egui::IconData {
    egui::IconData { rgba: ui::theme::ICON_256.to_vec(), width: 256, height: 256 }
}

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut screenshot = None;
    let mut selftest = None;
    let mut open = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--screenshot" => {
                screenshot = args.get(i + 1).map(PathBuf::from);
                i += 1;
            }
            "--self-test" => {
                selftest = args.get(i + 1).map(PathBuf::from);
                i += 1;
            }
            a if !a.starts_with('-') => open = Some(PathBuf::from(a)),
            _ => {}
        }
        i += 1;
    }
    let automated = screenshot.is_some() || selftest.is_some();
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("GitCerberoCraft")
        .with_inner_size([1232.0, 760.0])
        .with_min_inner_size([820.0, 520.0])
        .with_icon(Arc::new(icon()));
    if automated {
        viewport = viewport.with_active(false);
        #[cfg(windows)]
        {
            viewport = viewport.with_window_level(egui::WindowLevel::AlwaysOnBottom);
        }
    }
    let options = eframe::NativeOptions { viewport, run_and_return: true, ..Default::default() };
    eframe::run_native(
        "GitCerberoCraft",
        options,
        Box::new(move |cc| Ok(Box::new(ui::App::new(cc, ui::Options { screenshot, selftest, open })))),
    )
}
