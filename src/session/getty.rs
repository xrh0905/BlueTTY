use std::process::{Child, Command, Stdio};
#[cfg(unix)]
use std::{io, os::unix::process::CommandExt};

use anyhow::{Context, Result};

use crate::config::{SessionConfig, SessionMode};
use crate::session::SessionPeerInfo;

pub fn spawn_getty(config: &SessionConfig, pty_slave_path: &str, peer: &SessionPeerInfo) -> Result<Child> {
    let tty_arg = pty_slave_path.trim_start_matches("/dev/");
    let argv = render_subcommand_args(config, tty_arg, peer)?;
    let (program, args) = argv
        .split_first()
        .ok_or_else(|| anyhow::anyhow!("session.SubcommandTemplate produced an empty command"))?;
    let setsid = matches!(config.mode, SessionMode::Getty);

    tracing::info!(
        mode = ?config.mode,
        program = %program,
        tty = tty_arg,
        addr = %peer.address,
        name = %peer.name,
        setsid,
        args = %quote_args(args),
        "starting session child from SubcommandTemplate"
    );

    let mut cmd = Command::new(program);
    configure_setsid(&mut cmd, setsid);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    cmd.spawn().context("failed to spawn session subcommand")
}

#[cfg(unix)]
fn configure_setsid(cmd: &mut Command, enabled: bool) {
    if !enabled {
        return;
    }

    unsafe {
        cmd.pre_exec(|| {
            if nix::libc::setsid() == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

#[cfg(not(unix))]
fn configure_setsid(_cmd: &mut Command, _enabled: bool) {}

fn render_subcommand_args(config: &SessionConfig, tty: &str, peer: &SessionPeerInfo) -> Result<Vec<String>> {
    let host = render_host_token(&peer.name);
    let args = shell_words::split(&config.subcommand_template).with_context(|| {
        format!(
            "invalid session.SubcommandTemplate: {}",
            config.subcommand_template
        )
    })?
        .into_iter()
        .map(|token| {
            token
                .replace("{tty}", tty)
                .replace("{addr}", &peer.address)
                .replace("{name}", &peer.name)
                .replace("{host}", &host)
        })
        .collect();

    Ok(args)
}

fn render_host_token(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
            out.push(ch);
        } else if ch.is_whitespace() {
            out.push('-');
        }
    }

    if out.is_empty() {
        "bluetooth-peer".to_string()
    } else {
        out
    }
}

fn quote_args(args: &[String]) -> String {
    args.iter().map(|a| format!("{:?}", a)).collect::<Vec<_>>().join(" ")
}
