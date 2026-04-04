use std::os::fd::{AsFd, OwnedFd};
use std::path::PathBuf;

use anyhow::{Context, Result};
use nix::pty::openpty;
use nix::unistd::ttyname;

pub fn create_pty_pair() -> Result<(OwnedFd, OwnedFd, String)> {
    let pair = openpty(None, None).context("openpty failed")?;

    let slave_path = ttyname(pair.slave.as_fd())
        .context("resolve slave tty name")?
        .to_string_lossy()
        .to_string();

    let master_fd: OwnedFd = pair.master.into();
    let slave_fd: OwnedFd = pair.slave.into();

    Ok((master_fd, slave_fd, normalize_pts_path(slave_path)))
}

fn normalize_pts_path(path: String) -> String {
    let p = PathBuf::from(path);
    p.to_string_lossy().to_string()
}

