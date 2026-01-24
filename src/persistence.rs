use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Get the data directory for storing notes
/// - If running from source (binary path contains "target") or FIRE_NOTES_DEV is set: ./tmp/fire-notes
/// - If installed (binary path elsewhere): ~/.local/share/fire-notes
pub fn get_data_dir() -> PathBuf {
    // Check if we should use local storage
    let use_local_storage = std::env::var("FIRE_NOTES_DEV").is_ok()
        || std::env::current_exe()
            .map(|p| p.iter().any(|c| c == "target"))
            .unwrap_or(false);

    if use_local_storage {
        // Local/Dev mode: use local tmp directory relative to current working directory
        // We use current_dir because when running via 'cargo run', it sets CWD to project root
        let mut path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        path.push("tmp");
        path.push("fire-notes");
        path
    } else {
        // Installed mode: use system data directory
        let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
        base.join("fire-notes")
    }
}

fn is_internal_state_file(path: &PathBuf) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some("window_state.txt")
            | Some("session_state.txt")
            | Some("window_state.json")
            | Some("session_state.json")
            | Some("note_metadata.json")
    )
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct NoteMetadata {
    titles: HashMap<String, String>,
}

fn note_metadata_path() -> PathBuf {
    get_data_dir().join("note_metadata.json")
}

fn load_note_metadata() -> NoteMetadata {
    let content = fs::read_to_string(note_metadata_path()).ok();
    content
        .and_then(|payload| serde_json::from_str::<NoteMetadata>(&payload).ok())
        .unwrap_or_default()
}

pub fn load_note_title(path: &PathBuf) -> Option<String> {
    let metadata = load_note_metadata();
    metadata.titles.get(&path.to_string_lossy().to_string()).cloned()
}

pub fn save_note_title(path: &PathBuf, title: &str) -> std::io::Result<()> {
    let mut metadata = load_note_metadata();
    metadata
        .titles
        .insert(path.to_string_lossy().to_string(), title.to_string());
    let payload = serde_json::to_string_pretty(&metadata)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;
    fs::write(note_metadata_path(), payload)
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct WindowState {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

fn window_state_path() -> PathBuf {
    get_data_dir().join("window_state.json")
}

fn window_state_legacy_path() -> PathBuf {
    get_data_dir().join("window_state.txt")
}

pub fn load_window_state() -> Option<WindowState> {
    if let Ok(content) = fs::read_to_string(window_state_path()) {
        if let Ok(state) = serde_json::from_str::<WindowState>(&content) {
            if state.width > 0 && state.height > 0 {
                return Some(state);
            }
        }
    }

    let content = fs::read_to_string(window_state_legacy_path()).ok()?;
    let mut parts = content.split_whitespace();
    let x = parts.next()?.parse().ok()?;
    let y = parts.next()?.parse().ok()?;
    let width: u32 = parts.next()?.parse().ok()?;
    let height: u32 = parts.next()?.parse().ok()?;
    if width == 0 || height == 0 {
        return None;
    }
    Some(WindowState {
        x,
        y,
        width,
        height,
    })
}

pub fn save_window_state(state: WindowState) -> std::io::Result<()> {
    let dir = ensure_data_dir()?;
    let path = dir.join("window_state.json");
    let payload = serde_json::to_string_pretty(&state)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;
    fs::write(path, payload)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabState {
    pub path: PathBuf,
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub scroll_offset: usize,
    pub scroll_offset_x: f32,
    pub word_wrap: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub active_path: Option<PathBuf>,
    pub tabs: Vec<TabState>,
}

fn session_state_path() -> PathBuf {
    get_data_dir().join("session_state.json")
}

fn session_state_legacy_path() -> PathBuf {
    get_data_dir().join("session_state.txt")
}

pub fn load_session_state() -> Option<SessionState> {
    if let Ok(content) = fs::read_to_string(session_state_path()) {
        if let Ok(state) = serde_json::from_str::<SessionState>(&content) {
            if !state.tabs.is_empty() || state.active_path.is_some() {
                return Some(state);
            }
        }
    }

    let content = fs::read_to_string(session_state_legacy_path()).ok()?;
    let mut active_path = None;
    let mut tabs = Vec::new();

    for line in content.lines() {
        let mut parts = line.split('\t');
        let tag = parts.next()?;
        match tag {
            "active" => {
                if let Some(path) = parts.next() {
                    if !path.is_empty() {
                        active_path = Some(PathBuf::from(path));
                    }
                }
            }
            "tab" => {
                let path = PathBuf::from(parts.next()?);
                let cursor_line = parts.next()?.parse().ok()?;
                let cursor_col = parts.next()?.parse().ok()?;
                let scroll_offset = parts.next()?.parse().ok()?;
                let scroll_offset_x = parts.next()?.parse().ok()?;
                let word_wrap = parts.next()?.parse().ok()?;
                tabs.push(TabState {
                    path,
                    cursor_line,
                    cursor_col,
                    scroll_offset,
                    scroll_offset_x,
                    word_wrap,
                });
            }
            _ => {}
        }
    }

    if active_path.is_none() && tabs.is_empty() {
        return None;
    }

    Some(SessionState { active_path, tabs })
}

pub fn save_session_state(state: &SessionState) -> std::io::Result<()> {
    let dir = ensure_data_dir()?;
    let path = dir.join("session_state.json");
    let payload = serde_json::to_string_pretty(state)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;
    fs::write(path, payload)
}

/// Ensure the data directory exists
pub fn ensure_data_dir() -> std::io::Result<PathBuf> {
    let dir = get_data_dir();
    if let Some(parent) = dir.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// List all note files in the data directory
pub fn list_notes() -> std::io::Result<Vec<PathBuf>> {
    let dir = get_data_dir();
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut notes = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file()
            && path.extension().map_or(false, |e| e == "md" || e == "txt")
            && !is_internal_state_file(&path)
        {
            notes.push(path);
        }
    }
    notes.sort();
    Ok(notes)
}

/// Save a note to the data directory
pub fn save_note(filename: &str, content: &str) -> std::io::Result<PathBuf> {
    let dir = ensure_data_dir()?;
    let path = dir.join(filename);
    fs::write(&path, content)?;
    Ok(path)
}

/// Load a note from the data directory
#[allow(dead_code)]
pub fn load_note(path: &PathBuf) -> std::io::Result<String> {
    fs::read_to_string(path)
}

/// Generate a unique filename for a new note
pub fn generate_note_filename() -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("note_{}.md", timestamp)
}
