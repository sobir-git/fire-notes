use std::fs;
use std::path::PathBuf;

/// Get the data directory for storing notes
/// Returns ~/.local/share/fire-notes on Linux
pub fn get_data_dir() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("fire-notes")
}

/// Ensure the data directory exists
pub fn ensure_data_dir() -> std::io::Result<PathBuf> {
    let dir = get_data_dir();
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
        if path.is_file() && path.extension().map_or(false, |e| e == "md" || e == "txt") {
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
