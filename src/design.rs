//! Fire Notes' visual language. Framework controls receive these through Theme.
use fire_ui::Color;
use fire_ui_widgets::Theme;

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
        background: CANVAS,
        panel: PANEL,
        raised: RAISED,
        border: BORDER,
        foreground: TEXT,
        muted: MUTED,
        accent: EMBER,
        selection: SELECTION,
        radius: 0.,
        font_size: 16.,
        inset: 0.,
    }
}

pub fn popup_theme() -> Theme {
    Theme {
        background: PANEL,
        selection: RAISED,
        radius: POPUP_RADIUS,
        font_size: CONTROL_TEXT,
        inset: 8.,
        ..editor_theme()
    }
}
