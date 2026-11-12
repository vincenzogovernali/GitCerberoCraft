//! Prepares the app icon (assets/icon.png) once at build time:
//! - small RGBA copies for the window/taskbar icon and the in-app logo,
//! - a Windows .ico embedded in both executables.

use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=assets/icon.png");
    println!("cargo:rerun-if-changed=build.rs");
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let img = image::open("assets/icon.png").expect("assets/icon.png").to_rgba8();
    let resize = |s: u32| image::imageops::resize(&img, s, s, image::imageops::FilterType::Lanczos3);

    // Raw RGBA (no decoding needed at start-up).
    std::fs::write(out.join("icon_256.rgba"), resize(256).into_raw()).unwrap();
    std::fs::write(out.join("icon_64.rgba"), resize(64).into_raw()).unwrap();

    // Windows icon with several sizes.
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "windows" {
        let ico_path = out.join("app.ico");
        let frames: Vec<image::codecs::ico::IcoFrame> = [16u32, 24, 32, 48, 64, 128, 256]
            .iter()
            .map(|&s| {
                let r = resize(s);
                image::codecs::ico::IcoFrame::as_png(r.as_raw(), s, s, image::ExtendedColorType::Rgba8).unwrap()
            })
            .collect();
        let f = std::fs::File::create(&ico_path).unwrap();
        image::codecs::ico::IcoEncoder::new(f).encode_images(&frames).unwrap();
        let rc = out.join("app.rc");
        let ico = ico_path.display().to_string().replace('\\', "\\\\");
        std::fs::write(&rc, format!("1 ICON \"{ico}\"\n")).unwrap();
        let _ = embed_resource::compile(&rc, embed_resource::NONE);
    }
}
