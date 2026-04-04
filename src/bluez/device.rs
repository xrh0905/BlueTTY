use anyhow::{Context, Result};
use zbus::Connection;
use zbus::fdo::PropertiesProxy;
use zbus::names::InterfaceName;
use zbus::zvariant::OwnedObjectPath;

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub address: String,
    pub name: String,
    pub paired: bool,
    pub trusted: bool,
}

pub async fn read_device_info(conn: &Connection, device_path: &str) -> Result<DeviceInfo> {
    let path = OwnedObjectPath::try_from(device_path)
        .with_context(|| format!("invalid device object path: {device_path}"))?;

    let proxy = PropertiesProxy::builder(conn)
        .destination("org.bluez")?
        .path(path)?
        .build()
        .await
        .context("build Properties proxy failed")?;
    let device_iface = InterfaceName::try_from("org.bluez.Device1")
        .context("invalid bluez interface name")?;

    let address = owned_to_string(
        proxy
            .get(device_iface.clone(), "Address")
            .await
            .context("get Device1.Address failed")?,
    )?;
    let name = owned_to_string(
        proxy
            .get(device_iface.clone(), "Name")
            .await
            .context("get Device1.Name failed")?,
    )
    .unwrap_or_else(|_| "unknown".to_string());
    let paired = owned_to_bool(
        proxy
            .get(device_iface.clone(), "Paired")
            .await
            .context("get Device1.Paired failed")?,
    )?;
    let trusted = owned_to_bool(
        proxy
            .get(device_iface, "Trusted")
            .await
            .context("get Device1.Trusted failed")?,
    )?;

    Ok(DeviceInfo {
        address,
        name,
        paired,
        trusted,
    })
}

fn owned_to_bool(v: zbus::zvariant::OwnedValue) -> Result<bool> {
    bool::try_from(v).context("owned value is not bool")
}

fn owned_to_string(v: zbus::zvariant::OwnedValue) -> Result<String> {
    String::try_from(v).context("owned value is not string")
}
