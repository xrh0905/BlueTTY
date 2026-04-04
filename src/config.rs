use std::path::Path;

use anyhow::{Context, Result};
use ini::configparser::ini::Ini;

#[derive(Debug, Clone)]
pub struct Config {
    pub bluez: BluezConfig,
    pub session: SessionConfig,
}

#[derive(Debug, Clone)]
pub struct BluezConfig {
    pub profile_path: String,
    pub profile_uuid: String,
    pub profile_name: String,
    pub require_authentication: bool,
    pub require_authorization: bool,
}

#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub mode: SessionMode,
    pub subcommand_template: String,
    pub hup_fallback: bool,
    pub max_sessions: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionMode {
    None,
    Getty,
    Exec,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bluez: BluezConfig {
                profile_path: "/com/bluetty/spp_profile".to_string(),
                profile_uuid: "00001101-0000-1000-8000-00805f9b34fb".to_string(),
                profile_name: "SPP Getty".to_string(),
                require_authentication: false,
                require_authorization: false,
            },
            session: SessionConfig {
                mode: SessionMode::Getty,
                subcommand_template:
                    "/sbin/agetty -8 -s -L --noclear -H {host} --login-program /bin/login {tty} xterm-256color"
                        .to_string(),
                hup_fallback: false,
                max_sessions: 0,
            },
        }
    }
}

impl Config {
    pub fn load(path: Option<&Path>) -> Result<Self> {
        let mut cfg = Self::default();
        let resolved = resolve_config_path(path);

        let Some(config_path) = resolved else {
            tracing::info!("config file not found; using built-in defaults");
            return Ok(cfg);
        };

        let mut ini = Ini::new();
        ini.load(config_path.to_string_lossy().as_ref())
            .map_err(|e| anyhow::anyhow!(e))
            .with_context(|| format!("failed to load config {}", config_path.display()))?;

        if let Some(v) = ini.get("bluez", "ProfilePath") {
            cfg.bluez.profile_path = v;
        }
        if let Some(v) = ini.get("bluez", "Uuid") {
            cfg.bluez.profile_uuid = v;
        }
        if let Some(v) = ini.get("bluez", "Name") {
            cfg.bluez.profile_name = v;
        }
        if let Some(v) = ini.get("bluez", "RequireAuthentication") {
            cfg.bluez.require_authentication = parse_bool(&v)?;
        }
        if let Some(v) = ini.get("bluez", "RequireAuthorization") {
            cfg.bluez.require_authorization = parse_bool(&v)?;
        }

        if let Some(v) = ini.get("session", "Mode") {
            cfg.session.mode = parse_session_mode(&v)?;
        }
        if let Some(v) = ini.get("session", "SubcommandTemplate") {
            cfg.session.subcommand_template = v;
        }
        if let Some(v) = ini.get("session", "HupFallback") {
            cfg.session.hup_fallback = parse_bool(&v)?;
        }
        if let Some(v) = ini.get("session", "MaxSessions") {
            cfg.session.max_sessions = v
                .parse::<usize>()
                .with_context(|| format!("invalid session.MaxSessions: {v}"))?;
        }

        tracing::info!(path = %config_path.display(), "loaded configuration file");
        Ok(cfg)
    }
}

fn resolve_config_path(path: Option<&Path>) -> Option<std::path::PathBuf> {
    if let Some(p) = path {
        return Some(p.to_path_buf());
    }

    if let Ok(from_env) = std::env::var("BLUETTY_CONFIG") {
        let p = std::path::PathBuf::from(from_env);
        if p.exists() {
            return Some(p);
        }
    }

    let local = std::path::PathBuf::from("bluetty.conf");
    if local.exists() {
        return Some(local);
    }

    let etc = std::path::PathBuf::from("/etc/bluetty/bluetty.conf");
    if etc.exists() {
        return Some(etc);
    }

    None
}

fn parse_bool(v: &str) -> Result<bool> {
    match v.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => anyhow::bail!("invalid boolean value: {v}"),
    }
}

fn parse_session_mode(v: &str) -> Result<SessionMode> {
    match v.trim().to_ascii_lowercase().as_str() {
        "none" => Ok(SessionMode::None),
        "getty" => Ok(SessionMode::Getty),
        "exec" => Ok(SessionMode::Exec),
        _ => anyhow::bail!("invalid session.Mode: {v}"),
    }
}


