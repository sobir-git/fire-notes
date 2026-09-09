//! Fire Notes' visual language. Framework controls receive these through Theme.
use fire_ui::Color;
use fire_ui_widgets::{Palette, Scale, Theme};

pub const CANVAS: Color = Color::hex(0x000000);
pub const CHROME: Color = Color::hex(0x100909);
pub const PANEL: Color = Color::hex(0x201613);
pub const RAISED: Color = Color::hex(0x3b261d);
pub const BORDER: Color = Color::hex(0x62432f);
pub const TEXT: Color = Color::hex(0xffe6cc);
pub const MUTED: Color = Color::hex(0xb69b86);
pub const EMBER: Color = Color::hex(0xff8a36);
pub const ACTIVE_TAB: Color = Color::hex(0x291510);
pub const SELECTION: Color = Color::hex(0x422619);
pub const DANGER: Color = Color::hex(0x792c21);
pub const POPUP_RADIUS: f32 = 6.;
pub const CONTROL_TEXT: f32 = 14.;

pub fn editor_theme() -> Theme {
    Theme {
        color: Palette {
            background: CANVAS,
            surface: PANEL,
            raised: RAISED,
            sunken: CHROME,
            border: BORDER,
            border_strong: BORDER,
            foreground: TEXT,
            muted: MUTED,
            faint: MUTED,
            accent: EMBER,
            accent_light: EMBER,
            accent_hover: EMBER,
            on_accent: CANVAS,
            focus: EMBER,
            selection: SELECTION,
            danger: DANGER,
            success: EMBER,
        },
        scale: Scale {
            font_size: 16.,
            unit: 8.,
            inset: 0.,
            radius: 0.,
            panel_radius: 0.,
            border: 1.,
            control: 26.,
            halo: 0.,
        },
    }
}

pub fn popup_theme() -> Theme {
    let base = editor_theme();
    Theme {
        color: Palette {
            background: PANEL,
            selection: RAISED,
            ..base.color
        },
        scale: Scale {
            font_size: CONTROL_TEXT,
            inset: 8.,
            radius: POPUP_RADIUS,
            panel_radius: POPUP_RADIUS,
            ..base.scale
        },
    }
}

/// A copy of `theme` drawing text in `color`.
pub fn with_foreground(theme: Theme, color: Color) -> Theme {
    Theme {
        color: Palette {
            foreground: color,
            ..theme.color
        },
        ..theme
    }
}

/// A copy of `theme` with a different corner radius.
pub fn with_radius(theme: Theme, radius: f32) -> Theme {
    Theme {
        scale: Scale {
            radius,
            panel_radius: radius,
            ..theme.scale
        },
        ..theme
    }
}
