use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Condvar, Mutex},
    thread::{self, JoinHandle},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub const MAX_BODY_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_TITLE_BYTES: usize = 4096;
const MAX_NOTE_BYTES: u64 = (MAX_BODY_BYTES + MAX_TITLE_BYTES + 4) as u64;
#[derive(Clone, Debug)]
pub struct Note {
    pub path: PathBuf,
    pub title: Arc<str>,
    pub body: Arc<str>,
}
impl Note {
    pub fn load(path: &Path) -> Result<Self, String> {
        let mut body = String::new();
        fs::File::open(path)
            .and_then(|file| file.take(MAX_NOTE_BYTES + 1).read_to_string(&mut body))
            .map_err(|e| e.to_string())?;
        if body.len() as u64 > MAX_NOTE_BYTES {
            return Err("Notes are limited to 2 MiB in this prototype".into());
        }
        let (title, body) = if let Some(heading) = body.strip_prefix("# ") {
            let (title, body) = heading.split_once('\n').unwrap_or((heading, ""));
            (
                title.to_owned(),
                body.strip_prefix('\n').unwrap_or(body).to_owned(),
            )
        } else {
            (
                path.file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
                body,
            )
        };
        if body.len() > MAX_BODY_BYTES || title.len() > MAX_TITLE_BYTES {
            return Err("This prototype supports a 2 MiB body and a 4 KiB title".into());
        }
        Ok(Self {
            path: path.to_owned(),
            title: title.into(),
            body: body.into(),
        })
    }
    pub fn markdown(&self) -> String {
        format!(
            "# {}\n\n{}",
            self.title.replace(['\n', '\r'], " "),
            self.body
        )
    }
    pub fn bytes(&self) -> usize {
        self.title.len() + self.body.len() + self.path.as_os_str().len()
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    pub tabs: Vec<PathBuf>,
    pub active: Option<PathBuf>,
    pub width: f32,
    pub height: f32,
}
impl Default for Session {
    fn default() -> Self {
        Self {
            tabs: vec![],
            active: None,
            width: 1100.,
            height: 820.,
        }
    }
}
pub fn session(directory: &Path) -> Session {
    fs::read(directory.join("session.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}
pub fn scan(directory: &Path) -> Result<Vec<Note>, String> {
    fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    let mut paths = vec![];
    for entry in fs::read_dir(directory).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_type().map_err(|e| e.to_string())?.is_file()
            && entry
                .path()
                .extension()
                .is_some_and(|e| e == "md" || e == "txt")
        {
            paths.push(entry.path());
        }
    }
    paths.sort();
    paths
        .iter()
        .map(|path| {
            let mut prefix = vec![];
            fs::File::open(path)
                .and_then(|f| f.take(4096).read_to_end(&mut prefix))
                .map_err(|e| e.to_string())?;
            let prefix = String::from_utf8_lossy(&prefix);
            let title = prefix
                .lines()
                .next()
                .and_then(|l| l.strip_prefix("# "))
                .map(str::to_owned)
                .unwrap_or_else(|| {
                    path.file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned()
                });
            Ok(Note {
                path: path.clone(),
                title: title.into(),
                body: Arc::from(""),
            })
        })
        .collect()
}
pub fn new_note(directory: &Path) -> Note {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    Note {
        path: directory.join(format!("note-{stamp}.md")),
        title: Arc::from("Untitled"),
        body: Arc::from(""),
    }
}
pub fn atomic_write(path: &Path, content: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("Missing parent directory")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temporary = parent.join(format!(".fire-notes-{}-{stamp}.tmp", std::process::id()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        if let Ok(metadata) = fs::metadata(path) {
            file.set_permissions(metadata.permissions())?;
        }
        file.write_all(content)?;
        file.sync_all()?;
        fs::rename(&temporary, path)?;
        fs::File::open(parent)?.sync_all()
    })()
    .map_err(|e: std::io::Error| e.to_string());
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}
#[derive(Clone)]
pub struct Save {
    pub id: usize,
    pub revision: u64,
    pub path: PathBuf,
    pub content: Arc<str>,
}
struct Queue {
    pending: BTreeMap<usize, Save>,
    closing: bool,
}
/// One writer, one latest snapshot per open note. Closing drains pending writes and joins.
pub struct Writer {
    state: Arc<(Mutex<Queue>, Condvar)>,
    thread: Option<JoinHandle<()>>,
}
impl Writer {
    pub fn new(mut complete: impl FnMut(usize, u64, Result<(), String>) + Send + 'static) -> Self {
        let state = Arc::new((
            Mutex::new(Queue {
                pending: BTreeMap::new(),
                closing: false,
            }),
            Condvar::new(),
        ));
        let worker = state.clone();
        let thread = thread::spawn(move || loop {
            let (lock, wake) = &*worker;
            let mut queue = lock.lock().unwrap();
            while queue.pending.is_empty() && !queue.closing {
                queue = wake.wait(queue).unwrap();
            }
            if !queue.closing {
                let (q, _) = wake
                    .wait_timeout_while(queue, Duration::from_millis(300), |q| !q.closing)
                    .unwrap();
                queue = q;
            }
            let closing = queue.closing;
            let jobs = std::mem::take(&mut queue.pending);
            drop(queue);
            for (_, save) in jobs {
                let result = atomic_write(&save.path, save.content.as_bytes());
                complete(save.id, save.revision, result);
            }
            if closing {
                break;
            }
        });
        Self {
            state,
            thread: Some(thread),
        }
    }
    pub fn submit(&self, save: Save) {
        let (lock, wake) = &*self.state;
        lock.lock().unwrap().pending.insert(save.id, save);
        wake.notify_one();
    }
}
impl Drop for Writer {
    fn drop(&mut self) {
        let (lock, wake) = &*self.state;
        lock.lock().unwrap().closing = true;
        wake.notify_one();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn directory() -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "fire-notes-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&d).unwrap();
        d
    }
    #[test]
    fn markdown_round_trip_preserves_unicode_and_trailing_text() {
        let d = directory();
        let note = Note {
            path: d.join("note.md"),
            title: Arc::from("A thought"),
            body: Arc::from("Café\nПривет\n\n"),
        };
        atomic_write(&note.path, note.markdown().as_bytes()).unwrap();
        let loaded = Note::load(&note.path).unwrap();
        assert_eq!(loaded.title, note.title);
        assert_eq!(loaded.body, note.body);
        fs::remove_dir_all(d).unwrap();
    }
    #[test]
    fn writer_flushes_the_latest_snapshot_on_close() {
        let d = directory();
        let path = d.join("note.md");
        let writer = Writer::new(|_, _, result| result.unwrap());
        for revision in 0..100 {
            writer.submit(Save {
                id: 1,
                revision,
                path: path.clone(),
                content: Arc::from(revision.to_string()),
            });
        }
        drop(writer);
        assert_eq!(fs::read_to_string(path).unwrap(), "99");
        fs::remove_dir_all(d).unwrap();
    }
    #[test]
    fn failed_atomic_write_does_not_remove_existing_data() {
        let d = directory();
        let path = d.join("directory");
        fs::create_dir(&path).unwrap();
        assert!(atomic_write(&path, b"text").is_err());
        assert!(path.is_dir());
        assert_eq!(fs::read_dir(&d).unwrap().count(), 1);
        fs::remove_dir_all(d).unwrap();
    }
}
