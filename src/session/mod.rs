mod getty;
mod io_forward;
mod pty;

use std::collections::HashMap;
use std::time::Duration;
use std::os::fd::OwnedFd;
use std::sync::Arc;

use anyhow::{Context, Result};
use nix::errno::Errno;
use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;
use tokio::sync::Mutex;

use crate::config::{SessionConfig, SessionMode};
use io_forward::{ForwardExitReason, ForwardStopper};

#[derive(Debug, Clone)]
pub struct SessionManager {
    config: SessionConfig,
    inner: Arc<Mutex<SessionState>>,
}

#[derive(Debug, Clone)]
pub struct SessionPeerInfo {
    pub device_path: String,
    pub address: String,
    pub name: String,
}

#[derive(Debug)]
struct SessionState {
    next_id: u64,
    sessions: HashMap<u64, SessionEntry>,
    monitor_tasks: Vec<tokio::task::JoinHandle<()>>,
}

#[derive(Debug)]
struct SessionEntry {
    pub id: u64,
    pub device_path: String,
    pub pty_slave_path: String,
    pub _pty_slave_keepalive: OwnedFd,
    pub child_pid: Option<u32>,
    pub lifecycle: SessionLifecycle,
    pub stopper: ForwardStopper,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SessionHandle {
    pub id: u64,
    pub device_path: String,
    pub pty_slave_path: String,
    pub child_pid: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionLifecycle {
    Running,
    ShuttingDown,
}

impl SessionManager {
    pub fn new(config: SessionConfig) -> Self {
        Self {
            config,
            inner: Arc::new(Mutex::new(SessionState {
                next_id: 1,
                sessions: HashMap::new(),
                monitor_tasks: Vec::new(),
            })),
        }
    }

    pub async fn len(&self) -> usize {
        self.inner.lock().await.sessions.len()
    }

    pub async fn create_session(&self, peer: SessionPeerInfo, bt_fd: OwnedFd) -> Result<SessionHandle> {
        if self.config.max_sessions > 0 && self.len().await >= self.config.max_sessions {
            anyhow::bail!("max sessions reached");
        }

        let (pty_master, pty_slave_keepalive, pty_slave_path) =
            pty::create_pty_pair().context("create pty pair")?;
        let child =
            getty::spawn_getty(&self.config, &pty_slave_path, &peer).context("spawn session child")?;
        let child_pid = child.id();
        let lifecycle_child_pid = if matches!(self.config.mode, SessionMode::None) {
            None
        } else {
            Some(child_pid)
        };

        let managed_child = if matches!(self.config.mode, SessionMode::None) {
            tokio::task::spawn_blocking(move || {
                let mut detached = child;
                let _ = detached.wait();
            });
            None
        } else {
            Some(child)
        };

        let mut guard = self.inner.lock().await;
        let id = guard.next_id;
        guard.next_id += 1;

        let (stopper, forward_join) =
            io_forward::spawn_bidirectional_forwarding(id, bt_fd, pty_master)
                .context("spawn forwarding tasks")?;

        let handle = SessionHandle {
            id,
            device_path: peer.device_path.clone(),
            pty_slave_path: pty_slave_path.clone(),
            child_pid: lifecycle_child_pid,
        };

        guard.sessions.insert(
            id,
            SessionEntry {
                id,
                device_path: peer.device_path,
                pty_slave_path,
                _pty_slave_keepalive: pty_slave_keepalive,
                child_pid: lifecycle_child_pid,
                lifecycle: SessionLifecycle::Running,
                stopper,
            },
        );

        let inner = Arc::clone(&self.inner);
        let monitor_task = tokio::spawn(async move {
            monitor_session(inner, id, managed_child, forward_join).await;
        });
        guard.monitor_tasks.push(monitor_task);

        tracing::info!(
            session_id = id,
            child_pid = child_pid,
            pty = %handle.pty_slave_path,
            "session created"
        );

        Ok(handle)
    }

    pub async fn shutdown_device(&self, device_path: &str) {
        let mut guard = self.inner.lock().await;
        let mut matched = 0usize;
        let mut child_pids = Vec::new();

        for entry in guard.sessions.values_mut() {
            if entry.device_path == device_path {
                matched += 1;
                if let Some(child_pid) = request_session_shutdown(entry) {
                    child_pids.push(child_pid);
                }
            }
        }

        drop(guard);
        for child_pid in child_pids {
            terminate_session_child(child_pid, &self.config);
        }

        tracing::info!(device = %device_path, matched, "shutdown requested for device sessions");
    }

    pub async fn shutdown_all(&self) {
        let mut guard = self.inner.lock().await;
        let mut child_pids = Vec::new();
        for session in guard.sessions.values_mut() {
            if let Some(child_pid) = request_session_shutdown(session) {
                child_pids.push(child_pid);
            }
            tracing::info!(session_id = session.id, "shutting down session");
        }

        drop(guard);
        for child_pid in child_pids {
            terminate_session_child(child_pid, &self.config);
        }

        let settle_timeout = Duration::from_millis(self.config.process_group_term_timeout_ms);
        let settle_deadline = tokio::time::Instant::now() + settle_timeout;

        loop {
            let remaining = self.inner.lock().await.sessions.len();
            if remaining == 0 {
                break;
            }

            if tokio::time::Instant::now() >= settle_deadline {
                tracing::warn!(remaining, "session shutdown timed out; escalating to SIGKILL");
                self.escalate_shutdown_to_kill().await;
                break;
            }

            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        self.join_monitor_tasks().await;
    }

    async fn escalate_shutdown_to_kill(&self) {
        let mut guard = self.inner.lock().await;
        for entry in guard.sessions.values_mut() {
            if let Some(child_pid) = entry.child_pid {
                send_signal_to_process_group_or_pid(child_pid, Signal::SIGKILL);
            }
        }
    }

    async fn join_monitor_tasks(&self) {
        let tasks = {
            let mut guard = self.inner.lock().await;
            guard.monitor_tasks.drain(..).collect::<Vec<_>>()
        };

        for mut task in tasks {
            match tokio::time::timeout(Duration::from_secs(1), async { (&mut task).await }).await {
                Ok(Ok(())) => {}
                Ok(Err(err)) => {
                    tracing::warn!(error = %err, "monitor task join failed");
                }
                Err(_) => {
                    task.abort();
                    let _ = task.await;
                    tracing::warn!("monitor task join timed out and was aborted");
                }
            }
        }
    }
}

fn request_session_shutdown(entry: &mut SessionEntry) -> Option<u32> {
    if entry.lifecycle != SessionLifecycle::Running {
        return None;
    }

    entry.lifecycle = SessionLifecycle::ShuttingDown;
    entry.stopper.request_stop();
    entry.child_pid
}

fn terminate_session_child(child_pid: u32, config: &SessionConfig) {
    send_signal_to_process_group_or_pid(child_pid, Signal::SIGHUP);

    if config.hup_to_term_delay_ms > 0 {
        std::thread::sleep(Duration::from_millis(config.hup_to_term_delay_ms));
    }

    send_signal_to_process_group_or_pid(child_pid, Signal::SIGTERM);
}

fn send_signal_to_process_group_or_pid(child_pid: u32, signal: Signal) {
    let pgid = unsafe { nix::libc::getpgid(child_pid as nix::libc::pid_t) };
    if pgid > 0 {
        let group = Pid::from_raw(-pgid);
        match kill(group, signal) {
            Ok(()) => {
                tracing::info!(child_pid, pgid, ?signal, "sent signal to session process group");
                return;
            }
            Err(Errno::ESRCH) => {}
            Err(err) => {
                tracing::warn!(
                    child_pid,
                    pgid,
                    ?signal,
                    error = %err,
                    "failed to send signal to session process group"
                );
            }
        }
    }

    let pid = Pid::from_raw(child_pid as i32);
    match kill(pid, signal) {
        Ok(()) => tracing::info!(child_pid, ?signal, "sent signal to session child"),
        Err(Errno::ESRCH) => {}
        Err(err) => tracing::warn!(child_pid, ?signal, error = %err, "failed to send signal to session child"),
    }
}

async fn monitor_session(
    inner: Arc<Mutex<SessionState>>,
    session_id: u64,
    managed_child: Option<std::process::Child>,
    forward_join: tokio::task::JoinHandle<ForwardExitReason>,
) {
    let reason = match forward_join.await {
        Ok(reason) => reason,
        Err(err) => {
            tracing::warn!(session_id, error = %err, "forward task join failed");
            ForwardExitReason::PeerClosed
        }
    };

    if let Some(mut child) = managed_child {
        if let Err(err) = terminate_child_for_reason(&mut child, reason) {
            tracing::warn!(session_id, error = %err, ?reason, "failed to stop session child");
        }

        let wait_result = tokio::task::spawn_blocking(move || wait_child(child)).await;

        match wait_result {
            Ok(Ok(status)) => tracing::info!(session_id, ?status, ?reason, "session child exited"),
            Ok(Err(err)) => tracing::warn!(session_id, error = %err, ?reason, "wait child failed"),
            Err(err) => {
                tracing::warn!(session_id, error = %err, ?reason, "wait join failed")
            }
        }
    } else {
        tracing::info!(session_id, ?reason, "session child lifecycle unmanaged in mode=none");
    }

    let mut guard = inner.lock().await;
    if let Some(entry) = guard.sessions.remove(&session_id) {
        tracing::info!(
            session_id = entry.id,
            child_pid = ?entry.child_pid,
            pty = %entry.pty_slave_path,
            ?reason,
            "session closed"
        );
    }
}

fn wait_child(mut child: std::process::Child) -> std::io::Result<std::process::ExitStatus> {
    child.wait()
}

fn terminate_child_for_reason(
    child: &mut std::process::Child,
    reason: ForwardExitReason,
) -> std::io::Result<()> {
    match reason {
        ForwardExitReason::StopRequested | ForwardExitReason::PeerClosed => {
            if child.try_wait()?.is_none() {
                send_signal_to_process_group_or_pid(child.id(), Signal::SIGHUP);
                send_signal_to_process_group_or_pid(child.id(), Signal::SIGTERM);
            }
            Ok(())
        }
    }
}
