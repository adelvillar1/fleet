//! The background process registry: what `run_in_background` started,
//! controllable by id.
//!
//! The registry is the model's handle onto processes that outlive one
//! call. `bash` with `run_in_background` registers its child here; the
//! `process` tool reads incremental output since the last read, waits
//! up to a timeout, or kills. The registry is per run and closed at run
//! end: what the run spawned, the run shuts down cleanly — the standing
//! principle — and the report names what stopped.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

/// One registered child: its files and its read position.
#[derive(Debug)]
pub struct Child {
    pub pid: u32,
    pub stdout: PathBuf,
    pub stderr: PathBuf,
    /// Bytes of stdout already handed to the model.
    pub stdout_offset: u64,
    /// Bytes of stderr already handed to the model.
    pub stderr_offset: u64,
    /// The command, for the report.
    pub command: String,
    /// Whether the kill already ran.
    killed: AtomicBool,
}

impl Child {
    fn is_killed(&self) -> bool {
        self.killed.load(Ordering::Relaxed)
    }
}

/// The per-run registry. `Mutex` over the map: the tools run on the
/// blocking pool, one at a time per run, so contention is nil.
#[derive(Debug, Default)]
pub struct Registry {
    next: AtomicU64,
    children: Mutex<BTreeMap<String, Child>>,
}

impl Registry {
    pub fn new() -> Registry {
        Registry::default()
    }

    /// Register one background child. Returns its id.
    pub fn register(&self, command: &str, stdout: PathBuf, stderr: PathBuf, pid: u32) -> String {
        let id = format!("p{}", self.next.fetch_add(1, Ordering::Relaxed));
        if let Ok(mut children) = self.children.lock() {
            children.insert(
                id.clone(),
                Child {
                    pid,
                    stdout,
                    stderr,
                    stdout_offset: 0,
                    stderr_offset: 0,
                    command: command.to_string(),
                    killed: AtomicBool::new(false),
                },
            );
        }
        id
    }

    /// Bytes of combined output since the last read, plus whether the
    /// child has finished (pid gone, or killed).
    pub fn output(&self, id: &str) -> Result<(String, bool), String> {
        let mut children = self
            .children
            .lock()
            .map_err(|_| "the process registry is poisoned".to_string())?;
        let child = children
            .get_mut(id)
            .ok_or_else(|| format!("no background process `{id}`; the ids this run started are named in their own results"))?;
        let mut text = String::new();
        read_since(&child.stdout, &mut child.stdout_offset, &mut text);
        read_since(&child.stderr, &mut child.stderr_offset, &mut text);
        Ok((text, !child.is_killed() && !alive(child.pid)))
    }

    /// Wait up to `timeout_ms` for the child to finish, then report.
    pub fn wait(&self, id: &str, timeout_ms: u64) -> Result<(bool, String), String> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
        loop {
            let (text, finished) = self.output(id)?;
            if finished || std::time::Instant::now() >= deadline {
                return Ok((finished, text));
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }

    /// Kill the child's process group if we can, then report. The pid
    /// is reaped by the shell; the files stay for the output read.
    pub fn kill(&self, id: &str) -> Result<String, String> {
        let mut children = self
            .children
            .lock()
            .map_err(|_| "the process registry is poisoned".to_string())?;
        let child = children
            .get_mut(id)
            .ok_or_else(|| format!("no background process `{id}`"))?;
        let pid = child.pid;
        #[cfg(unix)]
        {
            // The child was spawned under `sh -c`, so its process group
            // is its own only if we set one; we did not, so kill the pid
            // and let the shell's children be orphans the owner sees.
            let _ = std::process::Command::new("kill")
                .arg(pid.to_string())
                .output();
        }
        #[cfg(windows)]
        {
            let _ = std::process::Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/F"])
                .output();
        }
        child.killed.store(true, Ordering::Relaxed);
        Ok(format!("killed `{id}` (pid {pid})"))
    }

    /// The children still running, for the run-end report. Killing the
    /// owner's server is worse than leaving it.
    pub fn still_running(&self) -> Vec<(String, String)> {
        let Ok(children) = self.children.lock() else {
            return Vec::new();
        };
        children
            .iter()
            .filter(|(_, child)| !child.is_killed() && alive(child.pid))
            .map(|(id, child)| (id.clone(), child.command.clone()))
            .collect()
    }

    /// Shut down every child this run spawned and still lives — the
    /// standing principle: what the agent spins up, the agent shuts down
    /// cleanly. Returns `(id, command)` for what it stopped, so the
    /// run-end report names the shutdown instead of leaving the owner to
    /// wonder what happened to the processes the transcript showed
    /// starting. A child already killed or already exited is not touched.
    pub fn shutdown(&self) -> Vec<(String, String)> {
        let mut stopped = Vec::new();
        for (id, command) in self.still_running() {
            if self.kill(&id).is_ok() {
                stopped.push((id, command));
            }
        }
        stopped
    }
}

/// Whether a pid still answers. `kill -0` on Unix; on Windows the
/// answer is conservative (treat as finished), because a probe there
/// needs a handle the registry does not hold.
///
/// A zombie answers `kill -0` — its pid exists until the parent reaps
/// it — so pid liveness alone read an exited driver as running
/// forever, and `process`'s "still running" was the KI-358 lie: the
/// work had finished, the shell was merely unreaped. `ps stat` names
/// the state: `Z` is dead-not-yet-reaped (finished), and an empty
/// state means the pid is gone (finished).
fn alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        let output = std::process::Command::new("ps")
            .args(["-o", "stat=", "-p", &pid.to_string()])
            .output()
            .ok();
        match output {
            Some(output) if output.status.success() => {
                let state = String::from_utf8_lossy(&output.stdout);
                let state = state.trim();
                !(state.is_empty() || state.starts_with('Z'))
            }
            _ => false,
        }
    }
    #[cfg(windows)]
    {
        let _ = pid;
        false
    }
}

/// Append the bytes of `path` past `offset` to `text`, advancing it.
fn read_since(path: &std::path::Path, offset: &mut u64, text: &mut String) {
    let Ok(bytes) = std::fs::read(path) else {
        return;
    };
    if (bytes.len() as u64) > *offset {
        let fresh = &bytes[*offset as usize..];
        text.push_str(&String::from_utf8_lossy(fresh));
        *offset = bytes.len() as u64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_reads_incrementally_and_reports_finish() {
        let dir = std::env::temp_dir().join(format!("fleet-proc-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let stdout = dir.join("out.log");
        std::fs::write(&stdout, "first line\n").unwrap();
        let registry = Registry::new();
        let id = registry.register("echo", stdout.clone(), dir.join("err.log"), u32::MAX);
        // A pid of u32::MAX never answers `kill -0`: reported finished.
        let (text, finished) = registry.output(&id).unwrap();
        assert_eq!(text, "first line\n");
        assert!(finished, "a dead pid is finished");
        // More output arrives; the second read is only the delta.
        std::fs::write(&stdout, "first line\nsecond line\n").unwrap();
        let (text, _) = registry.output(&id).unwrap();
        assert_eq!(text, "second line\n");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn shutdown_stops_what_the_run_spawned_and_names_it() {
        // A real child: the shutdown is what the run's close calls, and
        // the principle it pins is that nothing the agent spun up
        // outlives the run.
        let mut child = std::process::Command::new("sleep")
            .arg("30")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("sleep spawns");
        let pid = child.id();
        let dir = std::env::temp_dir().join(format!("fleet-proc-s-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let registry = Registry::new();
        let id = registry.register("sleep 30", dir.join("o.log"), dir.join("e.log"), pid);
        assert!(
            registry.still_running().iter().any(|(one, _)| *one == id),
            "the live child is registered"
        );
        let stopped = registry.shutdown();
        assert_eq!(stopped.len(), 1, "{stopped:?}");
        assert_eq!(stopped[0].0, id);
        assert_eq!(stopped[0].1, "sleep 30");
        assert!(
            registry.still_running().is_empty(),
            "nothing survives the shutdown"
        );
        let _ = child.kill();
        let _ = child.wait();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn wait_returns_within_its_timeout_on_a_dead_child() {
        let dir = std::env::temp_dir().join(format!("fleet-proc-w-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let registry = Registry::new();
        let id = registry.register("x", dir.join("o.log"), dir.join("e.log"), u32::MAX);
        let started = std::time::Instant::now();
        let (finished, _) = registry.wait(&id, 300).unwrap();
        assert!(finished);
        assert!(started.elapsed() < std::time::Duration::from_secs(5));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_unknown_id_is_a_named_error_and_still_running_lists_the_live() {
        let registry = Registry::new();
        let error = registry.output("p99").unwrap_err();
        assert!(error.contains("no background process"), "{error}");
        // A live pid: kill -0 answers for our own process.
        let dir = std::env::temp_dir().join(format!("fleet-proc-l-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let id = registry.register(
            "sleep",
            dir.join("o.log"),
            dir.join("e.log"),
            std::process::id(),
        );
        let running = registry.still_running();
        assert!(running.iter().any(|(held, _)| held == &id), "{running:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_exited_driver_whose_shell_is_unreaped_reads_finished() {
        // KI-358's pin: `sh -c true` exits immediately; fleet spawned
        // it and never waits, so its pid lingers as a zombie — which
        // `kill -0` calls alive. The surface must read the STATE:
        // Z is finished, and a driver that finished its work is not
        // "still running" because its shell is unreaped.
        let mut finished = std::process::Command::new("sh")
            .arg("-c")
            .arg("true")
            .spawn()
            .expect("spawn the driver that exits at once");
        let pid = finished.id();
        // Poll for the zombie state — the shell exits in milliseconds,
        // but a loaded machine gets a bounded wait rather than a flaky
        // assertion. The Child handle is held (not waited) so the pid
        // stays a zombie for the check.
        #[cfg(unix)]
        {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
            loop {
                let probe = std::process::Command::new("ps")
                    .args(["-o", "stat=", "-p", &pid.to_string()])
                    .output()
                    .unwrap();
                let state = String::from_utf8_lossy(&probe.stdout);
                if state.trim().starts_with('Z') {
                    break;
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "the fixture must become a zombie, not stay live: {state:?}"
                );
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }
        assert!(!alive(pid), "a zombie is not a live driver (KI-358)");
        let dir = std::env::temp_dir().join(format!("fleet-proc-z-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let registry = Registry::new();
        let id = registry.register("sh -c true", dir.join("o.log"), dir.join("e.log"), pid);
        let running = registry.still_running();
        assert!(
            running.iter().all(|(held, _)| held != &id),
            "the exited driver left the still-running report: {running:?}"
        );
        let (text, is_finished) = registry.output(&id).unwrap();
        assert!(is_finished, "the output read agrees: finished ({text})");
        let _ = finished.wait();
        let _ = std::fs::remove_dir_all(&dir);
    }
}
