use std::sync::Arc;

use anyhow::Result;

use crate::{bluez::BluezRuntime, config::Config, session::SessionManager};

pub struct App {
    config: Config,
}

impl App {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub async fn run(self) -> Result<()> {
        let sessions = Arc::new(SessionManager::new(self.config.session.clone()));
        let bluez = BluezRuntime::new(self.config.bluez.clone(), Arc::clone(&sessions));

        let bluez_handle = bluez.start().await?;
        tracing::info!("bluetty is running; waiting for signals");

        tokio::signal::ctrl_c().await?;
        tracing::info!("shutdown signal received");

        sessions.shutdown_all().await;
        bluez_handle.stop().await;
        Ok(())
    }
}
