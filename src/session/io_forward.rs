use std::os::fd::{AsFd, OwnedFd};
use std::sync::Arc;

use anyhow::{Context, Result};
use nix::errno::Errno;
use nix::fcntl::{FcntlArg, OFlag, fcntl};
use tokio::task::JoinHandle;

use nix::poll::{PollFd, PollFlags, PollTimeout, poll};

#[derive(Debug, Clone)]
pub struct ForwardStopper {
    stop_w: Arc<OwnedFd>,
}

#[derive(Debug, Clone, Copy)]
pub enum ForwardExitReason {
    PeerClosed,
    StopRequested,
}

struct ForwardInstance {
    bt_fd: OwnedFd,
    pty_fd: OwnedFd,
    stop_r: OwnedFd,
    buf_bt: [u8; 4096],
    buf_pty: [u8; 4096],
}

pub fn spawn_bidirectional_forwarding(
    session_id: u64,
    bt_fd: OwnedFd,
    pty_master_fd: OwnedFd,
) -> Result<(ForwardStopper, JoinHandle<ForwardExitReason>)> {
    let (stop_r, stop_w) = nix::unistd::pipe().context("create stop pipe failed")?;
    let stopper = ForwardStopper {
        stop_w: Arc::new(stop_w),
    };

    let mut forwarder = ForwardInstance::new(bt_fd, pty_master_fd, stop_r);

    let join = tokio::task::spawn_blocking(move || match forwarder.run() {
        Ok(reason) => {
            tracing::info!(session_id, ?reason, "forward loop exited");
            reason
        }
        Err(err) => {
            tracing::info!(session_id, error = %err, "forward loop treated as peer close");
            ForwardExitReason::PeerClosed
        }
    });

    Ok((stopper, join))
}

impl ForwardStopper {
    pub fn request_stop(&self) {
        let _ = nix::unistd::write(&*self.stop_w, &[1]);
    }
}

impl ForwardInstance {
    fn new(bt_fd: OwnedFd, pty_fd: OwnedFd, stop_r: OwnedFd) -> Self {
        Self {
            bt_fd,
            pty_fd,
            stop_r,
            buf_bt: [0_u8; 4096],
            buf_pty: [0_u8; 4096],
        }
    }

    fn run(&mut self) -> Result<ForwardExitReason> {
        set_blocking(&self.bt_fd).context("set bt fd blocking")?;
        set_blocking(&self.pty_fd).context("set pty fd blocking")?;

        loop {
            let mut poll_fds = [
                PollFd::new(self.bt_fd.as_fd(), PollFlags::POLLIN),
                PollFd::new(self.pty_fd.as_fd(), PollFlags::POLLIN),
                PollFd::new(self.stop_r.as_fd(), PollFlags::POLLIN),
            ];

            poll(&mut poll_fds, PollTimeout::NONE).context("poll failed")?;

            let bt_revents = poll_fds[0].revents().unwrap_or(PollFlags::empty());
            let pty_revents = poll_fds[1].revents().unwrap_or(PollFlags::empty());
            let stop_revents = poll_fds[2].revents().unwrap_or(PollFlags::empty());

            if stop_revents.contains(PollFlags::POLLIN) {
                return Ok(ForwardExitReason::StopRequested);
            }

            if bt_revents.intersects(PollFlags::POLLHUP | PollFlags::POLLERR | PollFlags::POLLNVAL)
                || pty_revents.intersects(PollFlags::POLLHUP | PollFlags::POLLERR | PollFlags::POLLNVAL)
            {
                return Ok(ForwardExitReason::PeerClosed);
            }

            if bt_revents.contains(PollFlags::POLLIN) {
                let n = match nix::unistd::read(&self.bt_fd, &mut self.buf_bt) {
                    Ok(n) => n,
                    Err(Errno::EINTR) => continue,
                    Err(Errno::EAGAIN) => continue,
                    Err(err) if is_disconnect_errno(err) => return Ok(ForwardExitReason::PeerClosed),
                    Err(err) => return Err(err).context("read bt fd failed"),
                };
                if n == 0 {
                    return Ok(ForwardExitReason::PeerClosed);
                }
                match write_all(&self.pty_fd, &self.buf_bt[..n]) {
                    Ok(()) => {}
                    Err(err) if is_disconnect_error(&err) => return Ok(ForwardExitReason::PeerClosed),
                    Err(err) => return Err(err).context("write to pty failed"),
                }
            }

            if pty_revents.contains(PollFlags::POLLIN) {
                let n = match nix::unistd::read(&self.pty_fd, &mut self.buf_pty) {
                    Ok(n) => n,
                    Err(Errno::EINTR) => continue,
                    Err(Errno::EAGAIN) => continue,
                    Err(err) if is_disconnect_errno(err) => return Ok(ForwardExitReason::PeerClosed),
                    Err(err) => return Err(err).context("read pty failed"),
                };
                if n == 0 {
                    return Ok(ForwardExitReason::PeerClosed);
                }
                match write_all(&self.bt_fd, &self.buf_pty[..n]) {
                    Ok(()) => {}
                    Err(err) if is_disconnect_error(&err) => return Ok(ForwardExitReason::PeerClosed),
                    Err(err) => return Err(err).context("write to bt fd failed"),
                }
            }
        }
    }
}

fn write_all<Fd: AsFd>(fd: &Fd, mut bytes: &[u8]) -> Result<()> {
    while !bytes.is_empty() {
        let n = match nix::unistd::write(fd, bytes) {
            Ok(n) => n,
            Err(Errno::EINTR) => continue,
            Err(Errno::EAGAIN) => continue,
            Err(err) => return Err(err).context("write failed"),
        };
        if n == 0 {
            anyhow::bail!("short write: wrote 0 bytes");
        }
        bytes = &bytes[n..];
    }
    Ok(())
}

fn set_blocking(fd: &OwnedFd) -> Result<()> {
    let bits = fcntl(fd, FcntlArg::F_GETFL).context("fcntl(F_GETFL) failed")?;
    let mut flags = OFlag::from_bits_truncate(bits);
    flags.remove(OFlag::O_NONBLOCK);
    fcntl(fd, FcntlArg::F_SETFL(flags)).context("fcntl(F_SETFL) failed")?;
    Ok(())
}

fn is_disconnect_errno(err: Errno) -> bool {
    matches!(err, Errno::EPIPE | Errno::ECONNRESET | Errno::ENOTCONN | Errno::EIO)
}

fn is_disconnect_error(err: &anyhow::Error) -> bool {
    err.chain().any(|cause| {
        cause
            .downcast_ref::<Errno>()
            .map(|errno| is_disconnect_errno(*errno))
            .unwrap_or(false)
    })
}
