//! Recoverable note removal. Entries contain their own title, original path and view state.
use crate::storage::{self, Note, NoteView};
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

pub const RETENTION: u64 = 30 * 24 * 60 * 60;
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entry {
    pub key: u64,
    pub original: PathBuf,
    pub title: String,
    pub deleted_at: u64,
    pub view: NoteView,
}
impl Entry {
    fn folder(&self, directory: &Path) -> PathBuf {
        directory.join("trash").join(self.key.to_string())
    }
    pub fn days_left(&self) -> u64 {
        self.deleted_at
            .saturating_add(RETENTION)
            .saturating_sub(now())
            .div_ceil(86400)
    }
}
pub enum Operation {
    Move {
        id: usize,
        path: PathBuf,
        title: String,
        view: NoteView,
    },
    Restore(Entry),
    List,
}
pub enum Change {
    Moved(usize, Entry),
    Restored(Entry, Note),
    Listed(Vec<Entry>),
}
impl Operation {
    pub fn run(self, directory: &Path) -> Result<Change, String> {
        match self {
            Self::Move {
                id,
                path,
                title,
                view,
            } => move_note(directory, &path, title, view, now()).map(|e| Change::Moved(id, e)),
            Self::Restore(entry) => {
                restore(directory, &entry, now()).map(|n| Change::Restored(entry, n))
            }
            Self::List => {
                purge(directory, now())?;
                list(directory).map(Change::Listed)
            }
        }
    }
    pub fn bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + match self {
                Self::Move { path, title, .. } => path.as_os_str().len() + title.len(),
                Self::Restore(e) => e.original.as_os_str().len() + e.title.len(),
                Self::List => 0,
            }
    }
}
impl Change {
    pub fn bytes(&self) -> usize {
        let entry =
            |e: &Entry| std::mem::size_of::<Entry>() + e.original.as_os_str().len() + e.title.len();
        match self {
            Self::Moved(_, e) => entry(e),
            Self::Restored(e, n) => entry(e) + n.bytes(),
            Self::Listed(v) => v.iter().map(entry).sum(),
        }
    }
}
pub fn list(directory: &Path) -> Result<Vec<Entry>, String> {
    let entries = match fs::read_dir(directory.join("trash")) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(vec![]),
        Err(e) => return Err(e.to_string()),
    };
    let mut result = vec![];
    for item in entries {
        let item = item.map_err(|e| e.to_string())?;
        if !item.file_type().map_err(|e| e.to_string())?.is_dir() {
            continue;
        }
        // Unrecognized or incomplete entries are retained, never purged.
        let Ok(bytes) = fs::read(item.path().join("entry.json")) else {
            continue;
        };
        let Ok(entry) = serde_json::from_slice::<Entry>(&bytes) else {
            continue;
        };
        if item.file_name() == entry.key.to_string().as_str()
            && item.path().join("note.md").is_file()
        {
            result.push(entry);
        }
    }
    result.sort_by_key(|e| std::cmp::Reverse((e.deleted_at, e.key)));
    Ok(result)
}
fn copy_new(source: &Path, target: &Path) -> io::Result<()> {
    let mut input = fs::File::open(source)?;
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)?;
    if let Err(error) = io::copy(&mut input, &mut output).and_then(|_| output.sync_all()) {
        let _ = fs::remove_file(target);
        return Err(error);
    }
    Ok(())
}
fn move_note(
    directory: &Path,
    source: &Path,
    title: String,
    view: NoteView,
    time: u64,
) -> Result<Entry, String> {
    let trash = directory.join("trash");
    fs::create_dir_all(&trash).map_err(|e| e.to_string())?;
    let mut key = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let folder = loop {
        let folder = trash.join(key.to_string());
        match fs::create_dir(&folder) {
            Ok(()) => break folder,
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => key = key.wrapping_add(1),
            Err(e) => return Err(e.to_string()),
        }
    };
    let entry = Entry {
        key,
        original: source.to_owned(),
        title,
        deleted_at: time,
        view,
    };
    let staged = folder.join("pending.md");
    // Commit a complete recovery copy before touching the source. This also works across disks.
    let result = (|| {
        let bytes = serde_json::to_vec(&entry).map_err(|e| e.to_string())?;
        storage::atomic_write(&folder.join("entry.json"), &bytes)?;
        copy_new(source, &staged).map_err(|e| e.to_string())?;
        fs::rename(&staged, folder.join("note.md")).map_err(|e| e.to_string())?;
        fs::File::open(&folder)
            .and_then(|f| f.sync_all())
            .map_err(|e| e.to_string())?;
        fs::remove_file(source).map_err(|e| e.to_string())
    })();
    if let Err(e) = result {
        // Keep the source and remove only files created by this attempt.
        for name in ["pending.md", "note.md", "entry.json"] {
            let _ = fs::remove_file(folder.join(name));
        }
        let _ = fs::remove_dir(folder);
        return Err(e);
    }
    Ok(entry)
}
fn restore(directory: &Path, entry: &Entry, time: u64) -> Result<Note, String> {
    if time.saturating_sub(entry.deleted_at) >= RETENTION {
        return Err("The 30-day recovery period has ended".into());
    }
    let folder = entry.folder(directory);
    let source = folder.join("note.md");
    let mut note = Note::load(&source)?;
    let parent = entry.original.parent().ok_or("Missing original folder")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let mut target = entry.original.clone();
    let mut suffix = 0;
    loop {
        match copy_new(&source, &target) {
            Ok(()) => break,
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                suffix += 1;
                let stem = entry
                    .original
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy();
                let ext = entry
                    .original
                    .extension()
                    .unwrap_or_default()
                    .to_string_lossy();
                target = parent.join(format!("{stem}-restored-{}-{suffix}.{ext}", entry.key));
            }
            Err(e) => return Err(e.to_string()),
        }
    }
    // A concurrent retention cleanup may have removed these after we opened the copy.
    for name in ["note.md", "entry.json"] {
        if let Err(e) = fs::remove_file(folder.join(name)) {
            if e.kind() != io::ErrorKind::NotFound {
                return Err(e.to_string());
            }
        }
    }
    let _ = fs::remove_dir(&folder);
    note.path = target;
    note.title = Arc::from(entry.title.as_str());
    Ok(note)
}
pub fn purge(directory: &Path, time: u64) -> Result<(), String> {
    for entry in list(directory)? {
        if time.saturating_sub(entry.deleted_at) < RETENTION {
            continue;
        }
        let folder = entry.folder(directory);
        for name in ["note.md", "entry.json"] {
            if let Err(e) = fs::remove_file(folder.join(name)) {
                if e.kind() != io::ErrorKind::NotFound {
                    return Err(e.to_string());
                }
            }
        }
        // Never recursively remove unrelated files placed in an entry folder.
        let _ = fs::remove_dir(folder);
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (PathBuf, PathBuf) {
        let directory = std::env::temp_dir().join(format!(
            "fire-trash-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&directory).unwrap();
        let source = directory.join("note.md");
        fs::write(&source, "Last keystroke. Привет.\n").unwrap();
        (directory, source)
    }
    #[test]
    fn trash_and_restore_preserve_content_title_and_view_without_overwriting() {
        let (d, source) = fixture();
        let entry = move_note(
            &d,
            &source,
            "My title".into(),
            NoteView {
                caret: 4,
                wrap: true,
                ..NoteView::default()
            },
            100,
        )
        .unwrap();
        assert!(!source.exists());
        assert_eq!(list(&d).unwrap().len(), 1);
        fs::write(&source, "Another note").unwrap();
        let restored = restore(&d, &entry, 101).unwrap();
        assert_ne!(restored.path, source);
        assert_eq!(&*restored.title, "My title");
        assert_eq!(&*restored.body, "Last keystroke. Привет.\n");
        assert_eq!(fs::read_to_string(source).unwrap(), "Another note");
        assert!(entry.view.wrap);
        assert_eq!(entry.view.caret, 4);
        assert!(list(&d).unwrap().is_empty());
        fs::remove_dir_all(d).unwrap();
    }
    #[test]
    fn retention_has_a_thirty_day_boundary_and_ignores_unrecognized_files() {
        let (d, source) = fixture();
        let entry = move_note(&d, &source, "Title".into(), NoteView::default(), 100).unwrap();
        fs::write(entry.folder(&d).join("keep.txt"), "unrelated").unwrap();
        purge(&d, 100 + RETENTION - 1).unwrap();
        assert_eq!(list(&d).unwrap().len(), 1);
        assert!(restore(&d, &entry, 100 + RETENTION).is_err());
        purge(&d, 100 + RETENTION).unwrap();
        assert!(list(&d).unwrap().is_empty());
        assert!(entry.folder(&d).join("keep.txt").exists());
        fs::remove_dir_all(d).unwrap();
    }
    #[test]
    fn failed_trash_does_not_publish_an_entry() {
        let (d, source) = fixture();
        assert!(move_note(
            &d,
            &d.join("missing.md"),
            "Missing".into(),
            NoteView::default(),
            100
        )
        .is_err());
        assert!(source.exists());
        assert!(list(&d).unwrap().is_empty());
        fs::remove_dir_all(d).unwrap();
    }
}
