//! Font loading and discovery

use femtovg::{Canvas, FontId, renderer::OpenGl};

/// Load fonts with fallbacks for the editor
pub fn load_fonts(canvas: &mut Canvas<OpenGl>) -> Vec<FontId> {
    let mut fonts = Vec::new();

    // 1. Try common monospace font paths on Linux
    let mono_paths = [
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf",
        "/usr/share/fonts/truetype/ubuntu/UbuntuMono-R.ttf",
        "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
        "/usr/share/fonts/dejavu/DejaVuSansMono.ttf",
    ];

    for path in &mono_paths {
        if let Ok(font) = canvas.add_font(path) {
            fonts.push(font);
            break; // Use the first available monospace font
        }
    }

    // 2. Add fallback fonts for extended coverage (Cyrillic, CJK, etc.)
    // These might not be monospace, but better than a box.
    let fallback_paths = [
        "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf", // Excellent fallback
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",           // Good generic coverage
        "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        "/usr/share/fonts/TTF/DejaVuSans.ttf",
    ];

    for path in &fallback_paths {
        if let Ok(font) = canvas.add_font(path) {
            // Avoid adding duplicates if we somehow loaded the same file?
            // FontId is unique per add_font call usually.
            fonts.push(font);
        }
    }

    // 3. Fallback: if no fonts loaded at all, try to find any TTF
    if fonts.is_empty() {
        if let Ok(entries) = std::fs::read_dir("/usr/share/fonts/truetype") {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    if let Ok(sub_entries) = std::fs::read_dir(entry.path()) {
                        for sub_entry in sub_entries.flatten() {
                            let path = sub_entry.path();
                            if path.extension().map(|e| e == "ttf").unwrap_or(false) {
                                if let Ok(font) = canvas.add_font(path) {
                                    fonts.push(font);
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if fonts.is_empty() {
        panic!(
            "No suitable font found! Please install dejavu-fonts, liberation-fonts, or fonts-droid-fallback."
        );
    }

    fonts
}
