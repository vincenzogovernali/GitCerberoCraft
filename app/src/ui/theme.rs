//! Colours (measured from the reference pictures), fonts and small drawing helpers.

use eframe::egui::{
    self, Align2, Color32, FontFamily, FontId, Pos2, Rect, Response, Rounding, Sense, Stroke, Ui, Vec2,
};

pub const fn hex(v: u32) -> Color32 {
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

pub const TITLE_BAR: Color32 = hex(0x221F21);
pub const MENU_BAR: Color32 = hex(0x1F1F1F);
pub const TAB_BAR: Color32 = hex(0x2A2D34);
pub const TAB_ACTIVE: Color32 = hex(0x33373F);
pub const TOOLBAR: Color32 = hex(0x33373F);
pub const CONTENT: Color32 = hex(0x1C1E23);
pub const PANEL: Color32 = hex(0x272A31);
pub const INPUT: Color32 = hex(0x1F2227);
pub const DIVIDER: Color32 = hex(0x2E3034);
pub const SECTION_LINE: Color32 = hex(0x33373F);
pub const SELECT: Color32 = hex(0x2E3D5A);
pub const SELECT_ROW: Color32 = hex(0x2B3446);
pub const HEAD_GREEN: Color32 = hex(0x37553E);
pub const CHECK_GREEN: Color32 = hex(0x5CB85C);
pub const MODAL_HEADER: Color32 = hex(0x3D424D);
pub const MODAL_LEFT: Color32 = hex(0x32363F);
pub const MODAL_SELECT: Color32 = hex(0x374665);
pub const MODAL_BODY: Color32 = hex(0x272A31);
pub const POPUP: Color32 = hex(0x3B3F48);
pub const ONBOARD_BG: Color32 = hex(0x0D1017);
pub const ONBOARD_INPUT: Color32 = hex(0x202329);
pub const PURPLE_BTN: Color32 = hex(0x5A13B8);
pub const PURPLE_DEEP: Color32 = hex(0x44088B);
pub const MAGENTA_BTN: Color32 = hex(0xC000F5);
pub const FOCUS_BLUE: Color32 = hex(0x6EB5FF);
pub const BROWSE_BLUE: Color32 = hex(0x2E3D5A);
pub const CREATE_GREEN: Color32 = hex(0x2C6E49);
pub const LINK: Color32 = hex(0x2FA6E8);
pub const LINK_SOFT: Color32 = hex(0x4FA8FF);
pub const TEXT: Color32 = hex(0xD5D8DD);
pub const TEXT_BRIGHT: Color32 = hex(0xF1F2F4);
pub const TEXT_DIM: Color32 = hex(0x9A9EA6);
pub const TEXT_FAINT: Color32 = hex(0x6B7079);
pub const BADGE_BLUE: Color32 = hex(0x3D8BFD);
pub const ADD_GREEN: Color32 = hex(0x7BD938);
pub const DEL_RED: Color32 = hex(0xF0524F);
pub const MOD_YELLOW: Color32 = hex(0xF2CA33);
pub const REN_BLUE: Color32 = hex(0x4FA8FF);
pub const TOAST_BLUE: Color32 = hex(0x3C7BF5);
pub const TOAST_BG: Color32 = hex(0x33373F);

pub fn lane_color(i: usize) -> Color32 {
    let l = gcc_core::graph::LANE_COLORS;
    hex(l[i % l.len()])
}

pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(f(a.r(), b.r()), f(a.g(), b.g()), f(a.b(), b.b()))
}

pub fn font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

pub fn bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("bold".into()))
}

pub fn mono(size: f32) -> FontId {
    FontId::new(size, FontFamily::Monospace)
}

fn load_font(paths: &[&str]) -> Option<Vec<u8>> {
    for p in paths {
        if let Ok(b) = std::fs::read(p) {
            return Some(b);
        }
    }
    None
}

pub fn setup(ctx: &egui::Context) {
    load_logo(ctx);
    let mut fonts = egui::FontDefinitions::default();
    let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
    let fdir = format!("{windir}\\Fonts\\");
    let regular = load_font(&[
        &format!("{fdir}segoeui.ttf"),
        "/System/Library/Fonts/SFNS.ttf",
        "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
        "/usr/share/fonts/noto/NotoSans-Regular.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    ]);
    let semibold = load_font(&[
        &format!("{fdir}seguisb.ttf"),
        &format!("{fdir}segoeuib.ttf"),
        "/usr/share/fonts/truetype/noto/NotoSans-SemiBold.ttf",
        "/usr/share/fonts/truetype/noto/NotoSans-Bold.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
    ]);
    let monofont = load_font(&[&format!("{fdir}consola.ttf"), "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf"]);
    let symbols = load_font(&[&format!("{fdir}seguisym.ttf")]);

    if let Some(b) = regular {
        fonts.font_data.insert("ui".into(), egui::FontData::from_owned(b));
        fonts.families.get_mut(&FontFamily::Proportional).unwrap().insert(0, "ui".into());
    }
    let mut bold_family = Vec::new();
    if let Some(b) = semibold {
        fonts.font_data.insert("ui_bold".into(), egui::FontData::from_owned(b));
        bold_family.push("ui_bold".to_string());
    }
    if let Some(b) = monofont {
        fonts.font_data.insert("ui_mono".into(), egui::FontData::from_owned(b));
        fonts.families.get_mut(&FontFamily::Monospace).unwrap().insert(0, "ui_mono".into());
    }
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
    if let Some(b) = symbols {
        fonts.font_data.insert("symbols".into(), egui::FontData::from_owned(b));
        fonts.families.get_mut(&FontFamily::Proportional).unwrap().push("symbols".into());
    }
    let prop = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    for f in prop {
        if !bold_family.contains(&f) {
            bold_family.push(f);
        }
    }
    fonts.families.insert(FontFamily::Name("bold".into()), bold_family);
    let mono_list = fonts.families.get_mut(&FontFamily::Monospace).unwrap();
    if !mono_list.contains(&"phosphor".to_string()) {
        mono_list.push("phosphor".into());
    }
    ctx.set_fonts(fonts);

    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (egui::TextStyle::Small, font(11.0)),
        (egui::TextStyle::Body, font(14.0)),
        (egui::TextStyle::Button, font(14.0)),
        (egui::TextStyle::Heading, font(20.0)),
        (egui::TextStyle::Monospace, mono(13.0)),
    ]
    .into();
    style.spacing.item_spacing = Vec2::new(8.0, 6.0);
    style.spacing.button_padding = Vec2::new(10.0, 4.0);
    style.spacing.interact_size.y = 26.0;
    style.spacing.scroll = egui::style::ScrollStyle::thin();
    let v = &mut style.visuals;
    *v = egui::Visuals::dark();
    v.panel_fill = CONTENT;
    v.window_fill = MODAL_BODY;
    v.window_stroke = Stroke::new(1.0, DIVIDER);
    v.extreme_bg_color = INPUT;
    v.faint_bg_color = PANEL;
    v.override_text_color = Some(TEXT);
    v.hyperlink_color = LINK;
    v.selection.bg_fill = SELECT;
    v.selection.stroke = Stroke::new(1.0, FOCUS_BLUE);
    v.window_rounding = Rounding::same(4.0);
    v.menu_rounding = Rounding::same(6.0);
    v.popup_shadow = egui::epaint::Shadow { offset: Vec2::new(0.0, 4.0), blur: 14.0, spread: 0.0, color: Color32::from_black_alpha(120) };
    v.window_shadow = v.popup_shadow;
    let w = &mut v.widgets;
    w.noninteractive.bg_fill = PANEL;
    w.noninteractive.weak_bg_fill = PANEL;
    w.noninteractive.bg_stroke = Stroke::new(1.0, DIVIDER);
    w.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
    for (st, bg) in [(&mut w.inactive, PANEL), (&mut w.hovered, hex(0x3A3F49)), (&mut w.active, hex(0x444A55)), (&mut w.open, hex(0x3A3F49))] {
        st.bg_fill = bg;
        st.weak_bg_fill = bg;
        st.rounding = Rounding::same(3.0);
        st.fg_stroke = Stroke::new(1.0, TEXT_BRIGHT);
        st.bg_stroke = Stroke::new(1.0, hex(0x3A3F49));
    }
    w.inactive.fg_stroke = Stroke::new(1.0, TEXT);
    ctx.set_style(style);
}

// --- drawing helpers -------------------------------------------------------

pub fn text_at(ui: &Ui, pos: Pos2, align: Align2, text: impl ToString, f: FontId, color: Color32) -> Rect {
    ui.painter().text(pos, align, text.to_string(), f, color)
}

pub fn text_width(ui: &Ui, text: &str, f: FontId) -> f32 {
    ui.fonts(|fo| fo.layout_no_wrap(text.to_string(), f, Color32::WHITE).size().x)
}

/// Clip text with an ellipsis so it fits in `max_w`.
pub fn elide(ui: &Ui, text: &str, f: &FontId, max_w: f32) -> String {
    if text_width(ui, text, f.clone()) <= max_w {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let (mut lo, mut hi) = (0usize, chars.len());
    while lo < hi {
        let mid = (lo + hi + 1) / 2;
        let s: String = chars[..mid].iter().collect::<String>() + "…";
        if text_width(ui, &s, f.clone()) <= max_w {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    chars[..lo].iter().collect::<String>() + "…"
}

/// Flat button with background (Open / Clone / Create style).
pub fn big_button(ui: &mut Ui, icon: &str, label: &str, size: Vec2) -> Response {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let bg = if resp.is_pointer_button_down_on() {
        hex(0x3A3F49)
    } else if resp.hovered() {
        hex(0x31353D)
    } else {
        PANEL
    };
    ui.painter().rect(rect, Rounding::same(3.0), bg, Stroke::new(1.0, hex(0x30343B)));
    let iw = text_width(ui, icon, font(17.0));
    let lw = text_width(ui, label, font(17.0));
    let total = iw + 10.0 + lw;
    let x = rect.center().x - total / 2.0;
    text_at(ui, Pos2::new(x, rect.center().y), Align2::LEFT_CENTER, icon, font(17.0), TEXT_BRIGHT);
    text_at(ui, Pos2::new(x + iw + 10.0, rect.center().y), Align2::LEFT_CENTER, label, font(17.0), TEXT_BRIGHT);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Solid coloured button (purple "Clone the repo!", green "Create Repository"...).
pub fn solid_button(ui: &mut Ui, label: &str, fill: Color32, size: Vec2, enabled: bool) -> Response {
    let (rect, resp) = ui.allocate_exact_size(size, if enabled { Sense::click() } else { Sense::hover() });
    let mut bg = fill;
    if !enabled {
        bg = mix(fill, PANEL, 0.6);
    } else if resp.hovered() {
        bg = mix(fill, Color32::WHITE, 0.12);
    }
    ui.painter().rect(rect, Rounding::same(3.0), bg, Stroke::new(1.0, mix(bg, Color32::WHITE, 0.08)));
    let col = if enabled { TEXT_BRIGHT } else { TEXT_DIM };
    text_at(ui, rect.center(), Align2::CENTER_CENTER, label, font(13.0), col);
    if enabled {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        resp
    }
}

/// Outline button used in forms ("Browse").
pub fn outline_button(ui: &mut Ui, label: &str, size: Vec2) -> Response {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let bg = if resp.hovered() { hex(0x36507A) } else { BROWSE_BLUE };
    ui.painter().rect(rect, Rounding::same(3.0), bg, Stroke::new(1.0, hex(0x4A6FA8)));
    text_at(ui, rect.center(), Align2::CENTER_CENTER, label, font(13.0), TEXT_BRIGHT);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn link(ui: &mut Ui, text: &str, color: Color32, f: FontId) -> Response {
    let w = text_width(ui, text, f.clone());
    let h = f.size + 6.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, h), Sense::click());
    let c = if resp.hovered() { mix(color, Color32::WHITE, 0.25) } else { color };
    text_at(ui, rect.left_center(), Align2::LEFT_CENTER, text, f, c);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A text field with the dark input look; returns the response.
pub fn input(ui: &mut Ui, value: &mut String, hint: &str, width: f32, height: f32, fill: Color32) -> Response {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let id = ui.next_auto_id();
    let has_focus = ui.memory(|m| m.has_focus(id));
    ui.painter().rect(
        rect,
        Rounding::same(3.0),
        fill,
        Stroke::new(1.0, if has_focus { FOCUS_BLUE } else { hex(0x2C3038) }),
    );
    let inner = rect.shrink2(Vec2::new(10.0, 0.0));
    let mut child = ui.child_ui(inner, egui::Layout::left_to_right(egui::Align::Center), None);
    child.set_clip_rect(inner);
    let te = egui::TextEdit::singleline(value)
        .id(id)
        .hint_text(egui::RichText::new(hint).color(TEXT_FAINT))
        .frame(false)
        .font(font(14.0))
        .text_color(TEXT_BRIGHT)
        .desired_width(inner.width());
    child.add(te)
}

pub fn checkbox(ui: &mut Ui, value: &mut bool) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::click());
    if resp.clicked() {
        *value = !*value;
    }
    let r = Rect::from_center_size(rect.center(), Vec2::splat(11.0));
    if *value {
        ui.painter().rect_filled(r, Rounding::same(2.0), FOCUS_BLUE);
        text_at(ui, r.center(), Align2::CENTER_CENTER, egui_phosphor::regular::CHECK, font(10.0), Color32::BLACK);
    } else {
        ui.painter().rect_stroke(r, Rounding::same(1.0), Stroke::new(1.0, TEXT_DIM));
    }
    resp
}

/// Circle with initials: our stand-in for a person's avatar.
pub fn avatar(painter: &egui::Painter, center: Pos2, radius: f32, name: &str) {
    let h = gcc_core::util::hash_str(name);
    let hue = (h % 360) as f32 / 360.0;
    let col: Color32 = egui::ecolor::Hsva::new(hue, 0.45, 0.55, 1.0).into();
    painter.circle_filled(center, radius, col);
    painter.text(
        center,
        Align2::CENTER_CENTER,
        gcc_core::util::initials(name),
        bold(radius * 0.85),
        Color32::WHITE,
    );
}

/// The person's icon, resized at build time (see build.rs).
pub static ICON_256: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/icon_256.rgba"));

static LOGO: std::sync::OnceLock<egui::TextureHandle> = std::sync::OnceLock::new();

fn load_logo(ctx: &egui::Context) {
    if LOGO.get().is_none() {
        let img = egui::ColorImage::from_rgba_unmultiplied([256, 256], ICON_256);
        let _ = LOGO.set(ctx.load_texture("app_logo", img, egui::TextureOptions::LINEAR));
    }
}

/// The app's mark: the person's icon; a drawn shield with three heads if missing.
pub fn logo(painter: &egui::Painter, center: Pos2, size: f32, color: Color32) {
    if let Some(tex) = LOGO.get() {
        let r = Rect::from_center_size(center, Vec2::splat(size * 1.1));
        painter.image(tex.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
        return;
    }
    let s = size / 2.0;
    let pts = vec![
        center + Vec2::new(-s * 0.8, -s * 0.75),
        center + Vec2::new(s * 0.8, -s * 0.75),
        center + Vec2::new(s * 0.8, s * 0.05),
        center + Vec2::new(0.0, s),
        center + Vec2::new(-s * 0.8, s * 0.05),
    ];
    painter.add(egui::Shape::convex_polygon(pts, color, Stroke::NONE));
    let dark = Color32::from_rgb(20, 24, 30);
    for dx in [-0.42f32, 0.0, 0.42] {
        let c = center + Vec2::new(dx * s, -s * 0.22 + if dx == 0.0 { -s * 0.12 } else { 0.0 });
        painter.circle_filled(c, s * 0.2, dark);
        painter.circle_filled(c + Vec2::new(-s * 0.07, -s * 0.03), s * 0.045, color);
        painter.circle_filled(c + Vec2::new(s * 0.07, -s * 0.03), s * 0.045, color);
    }
    painter.line_segment(
        [center + Vec2::new(-s * 0.3, s * 0.35), center + Vec2::new(s * 0.3, s * 0.35)],
        Stroke::new(s * 0.08, dark),
    );
}

/// Small pixel-art profile picture (our own design).
pub fn pixel_avatar(painter: &egui::Painter, rect: Rect) {
    const MAP: [&str; 6] = ["..X..X..", ".XXXXXX.", "XX.XX.XX", "XXXXXXXX", ".XXXXXX.", ".X.XX.X."];
    let cell = rect.width() / 8.0;
    let top = rect.top() + (rect.height() - cell * 6.0) / 2.0;
    for (y, row) in MAP.iter().enumerate() {
        for (x, ch) in row.chars().enumerate() {
            if ch == 'X' {
                let r = Rect::from_min_size(Pos2::new(rect.left() + x as f32 * cell, top + y as f32 * cell), Vec2::splat(cell));
                let c = if y < 2 { hex(0x8C7BFF) } else { hex(0x5B4BDB) };
                painter.rect_filled(r, Rounding::ZERO, c);
            }
        }
    }
}

pub fn status_color(c: char) -> Color32 {
    match c {
        'A' => ADD_GREEN,
        'D' => DEL_RED,
        'R' => REN_BLUE,
        'C' => hex(0xFF8A3D),
        _ => MOD_YELLOW,
    }
}

pub fn status_icon(c: char) -> &'static str {
    use egui_phosphor::regular as ic;
    match c {
        'A' => ic::PLUS,
        'D' => ic::MINUS,
        'R' => ic::ARROW_RIGHT,
        'C' => ic::WARNING,
        _ => ic::PENCIL_SIMPLE,
    }
}
