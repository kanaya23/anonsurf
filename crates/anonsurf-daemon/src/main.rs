mod service;

use anonsurf_core::{DBUS_PATH, DBUS_SERVICE};
use anyhow::Result;
use service::AnonSurfService;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let service = AnonSurfService::new()?;
    let _connection = zbus::connection::Builder::system()?
        .name(DBUS_SERVICE)?
        .serve_at(DBUS_PATH, service)?
        .build()
        .await?;

    info!("anonsurfd is serving {DBUS_SERVICE} at {DBUS_PATH}");
    std::future::pending::<()>().await;
    Ok(())
}
