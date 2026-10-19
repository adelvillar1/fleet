//! An exclusive advisory lock on one file's writes, released on drop.
//!
//! The read-modify-write pattern — read a file, edit one thing, rename
//! the new copy over it — is only atomic across processes when the
//! whole cycle holds a lock. The temp-plus-rename discipline makes a
//! reader never see a half-written file, but it does nothing about two
//! writers that both read the old value and each write back one update:
//! the last rename wins and the other increment is silently lost.
//!
//! Unix only, and `flock` through libc's own symbol rather than a crate:
//! the binary carries no dependency that provides one and needs exactly
//! this one primitive. A lock that cannot be created leaves the caller
//! running unprotected rather than refusing it, because a lost
//! recall-count bump is cheaper than a refused supersession.
//!
//! The lock file is a sibling beside its subject — `<path>.lock` — never
//! inside a directory a walk would read: the fact loader reads `*.md`
//! and the session index reads `.json`, so a `.lock` sibling enters
//! neither. It is deliberately left on disk, because unlinking a
//! flocked file another process still holds is a race; the corpora that
//! use it gitignore the sibling instead.

use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

/// An exclusive lock on one file's writes, released on drop.
pub struct FileLock {
    file: Option<File>,
}

impl FileLock {
    /// The lock file beside `path`: `<path>.lock`.
    pub fn beside(path: &Path) -> PathBuf {
        let mut lock = path.to_path_buf().into_os_string();
        lock.push(".lock");
        PathBuf::from(lock)
    }

    /// Take the lock, or carry on without it: a lock that cannot be
    /// created leaves the write running unprotected rather than
    /// refusing it. The `Option` records whether it was really held.
    pub fn take(lock: &Path) -> FileLock {
        let Ok(file) = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock)
        else {
            return FileLock { file: None };
        };
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            unsafe extern "C" {
                fn flock(fd: i32, operation: i32) -> i32;
            }
            // LOCK_EX is 2 on every unix. A failure leaves the file
            // open and unlocked, which the drop releases normally.
            unsafe { flock(file.as_raw_fd(), 2) };
        }
        FileLock { file: Some(file) }
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        #[cfg(unix)]
        if let Some(file) = &self.file {
            use std::os::unix::io::AsRawFd;
            unsafe extern "C" {
                fn flock(fd: i32, operation: i32) -> i32;
            }
            // LOCK_UN is 8 on every unix.
            unsafe { flock(file.as_raw_fd(), 8) };
        }
    }
}
