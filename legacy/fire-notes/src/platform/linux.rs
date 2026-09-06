//! Linux-specific platform initialization.

/// Populate XCURSOR_THEME and XCURSOR_SIZE from gsettings if not already set.
///
/// On GNOME Wayland (and Plasma 6+) these env vars are not exported by the
/// session, so winit/SCTK falls back to theme "default" at size 24, ignoring
/// the user's cursor settings. Reading them from gsettings and setting them
/// before the event loop starts ensures the correct theme and size are used.
pub fn init_xcursor_env() {
    use std::env;
    use std::process::Command;

    if env::var("XCURSOR_THEME").is_err() {
        if let Ok(out) = Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", "cursor-theme"])
            .output()
        {
            let theme = String::from_utf8_lossy(&out.stdout)
                .trim()
                .trim_matches('\'')
                .to_string();
            if !theme.is_empty() {
                env::set_var("XCURSOR_THEME", &theme);
            }
        }
    }

    if env::var("XCURSOR_SIZE").is_err() {
        if let Ok(out) = Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", "cursor-size"])
            .output()
        {
            let size = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !size.is_empty() {
                env::set_var("XCURSOR_SIZE", &size);
            }
        }
    }
}
