//! Clipboard abstraction — keeps the clipboard out of app code entirely.

/// Clipboard operations needed by text widgets.
/// Implemented by the platform (arboard) and injectable for tests (noop).
pub trait ClipboardPort: Send {
    fn get_text(&mut self) -> Option<String>;
    fn set_text(&mut self, text: String);
}

/// Real clipboard backed by `arboard`.
pub struct ArboardClipboard(arboard::Clipboard);

impl ArboardClipboard {
    pub fn new() -> Option<Self> {
        arboard::Clipboard::new().ok().map(Self)
    }
}

impl ClipboardPort for ArboardClipboard {
    fn get_text(&mut self) -> Option<String> { self.0.get_text().ok() }
    fn set_text(&mut self, text: String)      { let _ = self.0.set_text(text); }
}

/// No-op clipboard for headless tests / environments without a display.
pub struct NoopClipboard;

impl ClipboardPort for NoopClipboard {
    fn get_text(&mut self) -> Option<String> { None }
    fn set_text(&mut self, _: String) {}
}
