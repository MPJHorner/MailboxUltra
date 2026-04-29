//! Rasterise `icon/icon.svg` into the per-size PNGs an `iconutil` iconset
//! expects. After this binary runs, `make icon` (or the contributor) hands
//! the iconset directory to `iconutil -c icns` to produce
//! `icon/AppIcon.icns`.
//!
//! Build with `cargo run --bin icon-gen --features icon-tool`.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use resvg::usvg;
use tiny_skia::{Pixmap, Transform};

/// macOS .icns expects these names + sizes inside `<iconset>.iconset/`.
const ENTRIES: &[(&str, u32)] = &[
    ("icon_16x16.png", 16),
    ("icon_16x16@2x.png", 32),
    ("icon_32x32.png", 32),
    ("icon_32x32@2x.png", 64),
    ("icon_128x128.png", 128),
    ("icon_128x128@2x.png", 256),
    ("icon_256x256.png", 256),
    ("icon_256x256@2x.png", 512),
    ("icon_512x512.png", 512),
    ("icon_512x512@2x.png", 1024),
];

fn main() -> Result<()> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let svg_path = manifest.join("icon/icon.svg");
    let iconset = manifest.join("icon/AppIcon.iconset");
    let runtime_window_icon = manifest.join("assets/icon-512.png");
    fs::create_dir_all(&iconset)?;
    fs::create_dir_all(runtime_window_icon.parent().unwrap())?;

    let svg_data =
        fs::read(&svg_path).with_context(|| format!("reading {}", svg_path.display()))?;
    let opts = usvg::Options::default();
    let tree = usvg::Tree::from_data(&svg_data, &opts)
        .with_context(|| format!("parsing {}", svg_path.display()))?;
    let svg_size = tree.size();

    for (name, size) in ENTRIES {
        let path = iconset.join(name);
        rasterise(&tree, svg_size, *size, &path)?;
        println!("wrote {}", path.display());
    }

    rasterise(&tree, svg_size, 512, &runtime_window_icon)?;
    println!("wrote {}", runtime_window_icon.display());

    Ok(())
}

fn rasterise(tree: &usvg::Tree, src: usvg::Size, size: u32, out: &PathBuf) -> Result<()> {
    let scale_x = size as f32 / src.width();
    let scale_y = size as f32 / src.height();
    let mut pixmap = Pixmap::new(size, size).context("allocating pixmap")?;
    resvg::render(
        tree,
        Transform::from_scale(scale_x, scale_y),
        &mut pixmap.as_mut(),
    );
    let bytes = pixmap.encode_png().context("encoding PNG")?;
    fs::write(out, bytes).with_context(|| format!("writing {}", out.display()))?;
    Ok(())
}
