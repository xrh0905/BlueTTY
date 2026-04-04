use std::collections::HashMap;
use std::os::fd::OwnedFd;
use std::sync::Arc;

use anyhow::Result;
use zbus::{Connection, Proxy, interface};
use zbus::zvariant::{ObjectPath, OwnedFd as ZOwnedFd, OwnedValue, Value};

use crate::{
    config::BluezConfig,
    session::{SessionManager, SessionPeerInfo},
};

use super::device;

pub struct BluezRuntime {
    bluez: BluezConfig,
    sessions: Arc<SessionManager>,
}

pub struct BluezHandle {
    conn: Connection,
    profile_path: String,
}

impl BluezRuntime {
    pub fn new(bluez: BluezConfig, sessions: Arc<SessionManager>) -> Self {
        Self { bluez, sessions }
    }

    pub async fn start(&self) -> Result<BluezHandle> {
        let conn = Connection::system().await?;
        let profile_path = ObjectPath::try_from(self.bluez.profile_path.as_str())?;
        let profile = Profile1Impl {
            conn: conn.clone(),
            sessions: Arc::clone(&self.sessions),
        };

        conn.object_server()
            .at(self.bluez.profile_path.as_str(), profile)
            .await?;

        let proxy = Proxy::new(
            &conn,
            "org.bluez",
            "/org/bluez",
            "org.bluez.ProfileManager1",
        )
        .await?;

        let mut options: HashMap<&str, Value<'_>> = HashMap::new();
        options.insert("Name", self.bluez.profile_name.as_str().into());
        options.insert(
            "RequireAuthentication",
            self.bluez.require_authentication.into(),
        );
        options.insert(
            "RequireAuthorization",
            self.bluez.require_authorization.into(),
        );

        proxy
            .call_method(
                "RegisterProfile",
                &(
                    profile_path,
                    self.bluez.profile_uuid.as_str(),
                    options,
                ),
            )
            .await?;

        let active_sessions = self.sessions.len().await;
        tracing::info!(
            profile_path = %self.bluez.profile_path,
            profile_uuid = %self.bluez.profile_uuid,
            profile_name = %self.bluez.profile_name,
            require_authentication = self.bluez.require_authentication,
            require_authorization = self.bluez.require_authorization,
            active_sessions,
            "Profile1 registered on system bus"
        );

        Ok(BluezHandle {
            conn,
            profile_path: self.bluez.profile_path.clone(),
        })
    }
}

impl BluezHandle {
    pub async fn stop(self) {
        let path = match ObjectPath::try_from(self.profile_path.as_str()) {
            Ok(p) => p,
            Err(err) => {
                tracing::warn!(error = %err, "invalid profile path on stop");
                return;
            }
        };

        let proxy = match Proxy::new(
            &self.conn,
            "org.bluez",
            "/org/bluez",
            "org.bluez.ProfileManager1",
        )
        .await
        {
            Ok(p) => p,
            Err(err) => {
                tracing::warn!(error = %err, "failed to create ProfileManager1 proxy during stop");
                return;
            }
        };

        if let Err(err) = proxy.call_method("UnregisterProfile", &(path)).await {
            tracing::warn!(error = %err, "UnregisterProfile failed");
        } else {
            tracing::info!("Profile1 unregistered from BlueZ");
        }
    }
}

struct Profile1Impl {
    conn: Connection,
    sessions: Arc<SessionManager>,
}

#[interface(name = "org.bluez.Profile1")]
impl Profile1Impl {
    async fn release(&self) -> zbus::fdo::Result<()> {
        tracing::info!("Profile1 release requested by BlueZ");
        Ok(())
    }

    async fn request_disconnection(&self, device: ObjectPath<'_>) -> zbus::fdo::Result<()> {
        let device_path = device.to_string();
        self.sessions.shutdown_device(&device_path).await;
        tracing::info!(device = %device_path, "request disconnection handled");
        Ok(())
    }

    async fn new_connection(
        &self,
        device: ObjectPath<'_>,
        fd: ZOwnedFd,
        _fd_properties: HashMap<String, OwnedValue>,
    ) -> zbus::fdo::Result<()> {
        let device_path = device.to_string();
        let device_info = device::read_device_info(&self.conn, &device_path)
            .await
            .map_err(|err| zbus::fdo::Error::Failed(err.to_string()))?;

        let bt_fd: OwnedFd = fd.into();
        let peer = SessionPeerInfo {
            device_path: device_path.clone(),
            address: device_info.address.clone(),
            name: device_info.name.clone(),
        };

        self.sessions
            .create_session(peer, bt_fd)
            .await
            .map_err(|err| zbus::fdo::Error::Failed(err.to_string()))?;

        tracing::info!(
            device = %device_path,
            addr = %device_info.address,
            name = %device_info.name,
            paired = device_info.paired,
            trusted = device_info.trusted,
            "new bluetooth session accepted"
        );
        Ok(())
    }
}
