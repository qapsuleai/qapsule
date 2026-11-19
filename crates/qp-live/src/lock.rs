//! Single-connection locks.
//!
//! (a) In-process: a global registry keyed by endpoint (`<mode>-<path>`); a second claim in
//! the same process fails. (b) Machine: an operating-system file lock on
//! `<dir>/live-<key>.lock` (`std::fs::File::try_lock`); a second process fails and is told the
//! holder's PID (written into the file). The OS drops the lock when the holder dies.
//!
//! Also here: the multi-connect halt marker `<dir>/live-<key>.halted`, which keeps an
//! endpoint closed across restarts until a person resets it.

use std::collections::BTreeSet;
use std::fmt;
use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Optional override for the lock directory.
pub const VAR_LOCK_DIR: &str = "QP_LIVE_LOCK_DIR";

/// Endpoint identity used by locks and bans: lower-case `[a-z0-9-]`, e.g. `delayed-market`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EndpointKey(String);

impl EndpointKey {
    pub fn new(raw: &str) -> Self {
        let mut s = String::new();
        for c in raw.chars() {
            let c = c.to_ascii_lowercase();
            if c.is_ascii_alphanumeric() {
                s.push(c);
            } else if !s.ends_with('-') {
                s.push('-');
            }
        }
        let s = s.trim_matches('-').to_string();
        Self(if s.is_empty() { "endpoint".into() } else { s })
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn lock_file(&self, dir: &Path) -> PathBuf {
        dir.join(format!("live-{}.lock", self.0))
    }

    pub fn halt_marker(&self, dir: &Path) -> PathBuf {
        dir.join(format!("live-{}.halted", self.0))
    }
}

impl fmt::Display for EndpointKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Lock failure. Contains only the key, a PID and the lock file path (none are secret).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LockError {
    /// This process already has a connection (or a pending one) for the endpoint.
    InProcess { key: EndpointKey },
    /// Another process holds the machine lock.
    HeldByOtherProcess {
        key: EndpointKey,
        pid: Option<u32>,
        path: PathBuf,
    },
    /// Lock directory or file could not be used.
    Io { key: EndpointKey, detail: String },
}

impl fmt::Display for LockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LockError::InProcess { key } => write!(
                f,
                "endpoint {key} already has a live connection in this process (one connection per endpoint)"
            ),
            LockError::HeldByOtherProcess { key, pid, path } => {
                write!(f, "endpoint {key} is locked by another process")?;
                match pid {
                    Some(p) => write!(f, " (pid {p})")?,
                    None => write!(f, " (pid unknown)")?,
                }
                write!(
                    f,
                    "; lock file {}; only one Qapsule process may connect to an endpoint",
                    path.display()
                )
            }
            LockError::Io { key, detail } => write!(f, "endpoint {key} lock failed: {detail}"),
        }
    }
}

impl std::error::Error for LockError {}

static IN_PROCESS: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());

/// In-process claim; released on drop.
#[derive(Debug)]
pub struct ProcessSlot {
    key: EndpointKey,
}

impl Drop for ProcessSlot {
    fn drop(&mut self) {
        IN_PROCESS
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(self.key.as_str());
    }
}

pub fn claim_process_slot(key: &EndpointKey) -> Result<ProcessSlot, LockError> {
    let mut g = IN_PROCESS.lock().unwrap_or_else(|e| e.into_inner());
    if !g.insert(key.as_str().to_string()) {
        return Err(LockError::InProcess { key: key.clone() });
    }
    Ok(ProcessSlot { key: key.clone() })
}

/// Machine-wide lock file; the OS lock is released when this is dropped or the process dies.
#[derive(Debug)]
pub struct MachineLock {
    file: File,
    path: PathBuf,
}

impl MachineLock {
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for MachineLock {
    fn drop(&mut self) {
        let _ = self.file.set_len(0);
        let _ = self.file.unlock();
    }
}

pub fn acquire_machine_lock(dir: &Path, key: &EndpointKey) -> Result<MachineLock, LockError> {
    let io = |e: std::io::Error| LockError::Io {
        key: key.clone(),
        detail: e.kind().to_string(),
    };
    fs::create_dir_all(dir).map_err(io)?;
    let path = key.lock_file(dir);
    // No truncate on open: the holder's PID must survive a failed attempt.
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .map_err(io)?;
    match file.try_lock() {
        Ok(()) => {
            file.set_len(0).map_err(io)?;
            file.seek(SeekFrom::Start(0)).map_err(io)?;
            writeln!(file, "pid={}", std::process::id()).map_err(io)?;
            file.sync_all().map_err(io)?;
            Ok(MachineLock { file, path })
        }
        Err(TryLockError::WouldBlock) => {
            // The holder writes its PID right after locking; give it a moment.
            let mut pid = None;
            for _ in 0..25 {
                let mut text = String::new();
                let _ = file.seek(SeekFrom::Start(0));
                let _ = file.read_to_string(&mut text);
                pid = text
                    .lines()
                    .find_map(|l| l.strip_prefix("pid="))
                    .and_then(|p| p.trim().parse().ok());
                if pid.is_some() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            Err(LockError::HeldByOtherProcess {
                key: key.clone(),
                pid,
                path,
            })
        }
        Err(TryLockError::Error(e)) => Err(io(e)),
    }
}

/// Both locks for one endpoint; held for the whole life of a connection object.
#[derive(Debug)]
pub struct EndpointLock {
    // Field order = drop order: machine lock first, then the process slot.
    machine: MachineLock,
    _slot: ProcessSlot,
}

impl EndpointLock {
    pub fn lock_file(&self) -> &Path {
        self.machine.path()
    }
}

/// Claims the in-process slot, then the machine lock.
pub fn lock_endpoint(dir: &Path, key: &EndpointKey) -> Result<EndpointLock, LockError> {
    let slot = claim_process_slot(key)?;
    let machine = acquire_machine_lock(dir, key)?;
    Ok(EndpointLock {
        machine,
        _slot: slot,
    })
}

/// `QP_LIVE_LOCK_DIR`, else `<repo root>/fixtures/local` (the main tree's, also from inside a
/// git worktree, so worktrees share one lock), else `./fixtures/local`.
pub fn default_lock_dir() -> PathBuf {
    if let Some(d) = std::env::var_os(VAR_LOCK_DIR) {
        if !d.is_empty() {
            return PathBuf::from(d);
        }
    }
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    repo_root_from(&cwd)
        .unwrap_or(cwd)
        .join("fixtures")
        .join("local")
}

/// Walks up from `start` to the repository root; resolves a worktree to its main tree by
/// reading the `.git` file (no git command is run).
pub fn repo_root_from(start: &Path) -> Option<PathBuf> {
    for dir in start.ancestors() {
        let git = dir.join(".git");
        if git.is_dir() {
            return Some(dir.to_path_buf());
        }
        if git.is_file() {
            let text = fs::read_to_string(&git).ok()?;
            let gitdir = text.trim().strip_prefix("gitdir:")?.trim();
            let gitdir = if Path::new(gitdir).is_absolute() {
                PathBuf::from(gitdir)
            } else {
                dir.join(gitdir)
            };
            // <main>/.git/worktrees/<name>
            let parent = gitdir.parent()?;
            if parent.file_name().is_some_and(|n| n == "worktrees") {
                return parent.parent()?.parent().map(Path::to_path_buf);
            }
            return Some(dir.to_path_buf());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_is_sanitized() {
        assert_eq!(
            EndpointKey::new("Delayed-/Market").as_str(),
            "delayed-market"
        );
        assert_eq!(EndpointKey::new("///").as_str(), "endpoint");
    }

    #[test]
    fn in_process_second_claim_fails_and_is_released_on_drop() {
        let key = EndpointKey::new("unit-inproc");
        let a = claim_process_slot(&key).unwrap();
        let err = claim_process_slot(&key).unwrap_err();
        assert!(err.to_string().contains("already has a live connection"));
        drop(a);
        assert!(claim_process_slot(&key).is_ok());
    }

    #[test]
    fn machine_lock_reports_holder_pid() {
        let dir = tempfile::tempdir().unwrap();
        let key = EndpointKey::new("unit-machine");
        let held = acquire_machine_lock(dir.path(), &key).unwrap();
        // A second open file description conflicts even inside one process (flock / LockFileEx).
        match acquire_machine_lock(dir.path(), &key).unwrap_err() {
            LockError::HeldByOtherProcess { pid, .. } => {
                assert_eq!(pid, Some(std::process::id()))
            }
            other => panic!("unexpected {other:?}"),
        }
        drop(held);
        assert!(acquire_machine_lock(dir.path(), &key).is_ok());
    }

    #[test]
    fn worktree_resolves_to_main_tree() {
        let root = tempfile::tempdir().unwrap();
        let main = root.path().join("repo");
        let wt = main.join(".claude/worktrees/x");
        fs::create_dir_all(main.join(".git/worktrees/x")).unwrap();
        fs::create_dir_all(wt.join("crates")).unwrap();
        fs::write(
            wt.join(".git"),
            format!("gitdir: {}\n", main.join(".git/worktrees/x").display()),
        )
        .unwrap();
        assert_eq!(repo_root_from(&wt.join("crates")).unwrap(), main);
        assert_eq!(repo_root_from(&main).unwrap(), main);
    }
}
