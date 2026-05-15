use anonsurf_core::{
    AnonStatus, CommandOutcome, Config, DnsBackend, FirewallBackend, OperationError, Paths,
    ServiceState, Status, TorStatus, DBUS_INTERFACE,
};
use anyhow::{Context, Result};
use std::{
    collections::HashMap,
    fs,
    sync::{Arc, Mutex},
};
use tracing::error;
use zbus::{fdo, interface, message::Header, zvariant::Value, Connection, Proxy};

const POLKIT_ACTION_MANAGE: &str = "org.anonsurf.rs1.manage";

#[derive(Clone)]
pub struct AnonSurfService {
    inner: Arc<Mutex<Runtime>>,
}

struct Runtime {
    config: Config,
    paths: Paths,
    state: ServiceState,
}

#[derive(Clone)]
struct RuntimeContext {
    config: Config,
    paths: Paths,
    status: Status,
}

struct StartResult {
    changed: Vec<String>,
    dns_backend: DnsBackend,
    firewall_backend: FirewallBackend,
    bridge_mode: anonsurf_core::BridgeMode,
}

impl AnonSurfService {
    pub fn new() -> Result<Self> {
        let paths = Paths::system();
        let config = Config::load_or_default(&paths.config_file)
            .with_context(|| format!("failed to load {}", paths.config_file.display()))?;
        Ok(Self {
            inner: Arc::new(Mutex::new(Runtime {
                config,
                paths,
                state: ServiceState::default(),
            })),
        })
    }

    fn with_runtime<T>(&self, op: impl FnOnce(&mut Runtime) -> Result<T>) -> fdo::Result<T> {
        let mut runtime = self
            .inner
            .lock()
            .map_err(|_| fdo::Error::Failed("daemon state lock poisoned".to_string()))?;
        op(&mut runtime).map_err(|err| fdo::Error::Failed(err.to_string()))
    }

    fn begin_operation(&self, message: &'static str) -> fdo::Result<RuntimeContext> {
        self.with_runtime(|runtime| {
            runtime.state.begin_transition(message).map_err(map_busy)?;
            Ok(RuntimeContext {
                config: runtime.config.clone(),
                paths: runtime.paths.clone(),
                status: runtime.state.status(),
            })
        })
    }
}

#[interface(name = "org.anonsurf.rs1")]
impl AnonSurfService {
    async fn get_status(&self) -> fdo::Result<String> {
        self.with_runtime(|runtime| Ok(serde_json::to_string_pretty(&runtime.state.status())?))
    }

    async fn start(
        &self,
        #[zbus(connection)] connection: &Connection,
        #[zbus(header)] header: Header<'_>,
    ) -> fdo::Result<String> {
        authorize_manage(connection, &header).await?;
        if self.with_runtime(|runtime| Ok(runtime.state.status().status == AnonStatus::Enabled))? {
            return self.with_runtime(|runtime| {
                let outcome = CommandOutcome::ok(
                    "AnonSurf already enabled",
                    vec!["already enabled".to_string()],
                    runtime.state.status(),
                );
                Ok(serde_json::to_string_pretty(&outcome)?)
            });
        }

        let ctx = self.begin_operation("start requested")?;
        let result = start_operation(&ctx.config, &ctx.paths);
        self.with_runtime(|runtime| {
            let outcome = match result {
                Ok(result) => {
                    for change in &result.changed {
                        runtime.state.log(change);
                    }
                    runtime.state.mark_enabled(
                        TorStatus::Running,
                        result.dns_backend,
                        result.firewall_backend,
                        result.bridge_mode,
                    );
                    runtime.state.log("start completed");
                    CommandOutcome::ok("AnonSurf enabled", result.changed, runtime.state.status())
                }
                Err(err) => {
                    error!("start failed: {err:?}");
                    let repair =
                        repair_operation(&ctx.config, &ctx.paths, ctx.status.firewall_backend)
                            .unwrap_or_else(|repair_err| {
                                vec![format!("rollback warning: {repair_err}")]
                            });
                    for change in repair {
                        runtime.state.log(change);
                    }
                    runtime.state.mark_failed(err.to_string());
                    CommandOutcome::failed("AnonSurf failed to start", runtime.state.status())
                }
            };
            Ok(serde_json::to_string_pretty(&outcome)?)
        })
    }

    async fn stop(
        &self,
        #[zbus(connection)] connection: &Connection,
        #[zbus(header)] header: Header<'_>,
    ) -> fdo::Result<String> {
        authorize_manage(connection, &header).await?;
        let ctx = self.begin_operation("stop requested")?;
        let result = repair_operation(&ctx.config, &ctx.paths, ctx.status.firewall_backend);
        self.with_runtime(|runtime| {
            let outcome = match result {
                Ok(changed) => {
                    for change in &changed {
                        runtime.state.log(change);
                    }
                    runtime.state.mark_disabled();
                    CommandOutcome::ok("AnonSurf disabled", changed, runtime.state.status())
                }
                Err(err) => {
                    runtime.state.mark_failed(err.to_string());
                    CommandOutcome::failed(
                        "AnonSurf failed to stop cleanly",
                        runtime.state.status(),
                    )
                }
            };
            Ok(serde_json::to_string_pretty(&outcome)?)
        })
    }

    async fn restart(
        &self,
        #[zbus(connection)] connection: &Connection,
        #[zbus(header)] header: Header<'_>,
    ) -> fdo::Result<String> {
        authorize_manage(connection, &header).await?;
        let ctx = self.begin_operation("restart requested")?;
        let result = repair_operation(&ctx.config, &ctx.paths, ctx.status.firewall_backend)
            .and_then(|mut changed| {
                let start = start_operation(&ctx.config, &ctx.paths)?;
                changed.extend(start.changed.clone());
                Ok((changed, start))
            });
        self.with_runtime(|runtime| {
            let outcome = match result {
                Ok((changed, start)) => {
                    for change in &changed {
                        runtime.state.log(change);
                    }
                    runtime.state.mark_enabled(
                        TorStatus::Running,
                        start.dns_backend,
                        start.firewall_backend,
                        start.bridge_mode,
                    );
                    CommandOutcome::ok("AnonSurf restarted", changed, runtime.state.status())
                }
                Err(err) => {
                    runtime.state.mark_failed(err.to_string());
                    CommandOutcome::failed("AnonSurf failed to restart", runtime.state.status())
                }
            };
            Ok(serde_json::to_string_pretty(&outcome)?)
        })
    }

    async fn new_identity(
        &self,
        #[zbus(connection)] connection: &Connection,
        #[zbus(header)] header: Header<'_>,
    ) -> fdo::Result<String> {
        authorize_manage(connection, &header).await?;
        let ctx = self.with_runtime(|runtime| {
            Ok(RuntimeContext {
                config: runtime.config.clone(),
                paths: runtime.paths.clone(),
                status: runtime.state.status(),
            })
        })?;
        let result = anonsurf_tor::new_identity(&ctx.config, &ctx.paths);
        self.with_runtime(|runtime| {
            let outcome = match result {
                Ok(reply) => {
                    runtime.state.log("requested Tor NEWNYM");
                    CommandOutcome::ok(
                        "Tor identity rotation requested",
                        vec![reply.trim().to_string()],
                        runtime.state.status(),
                    )
                }
                Err(err) => {
                    runtime.state.log(format!("new identity failed: {err}"));
                    CommandOutcome::failed("Tor identity rotation failed", runtime.state.status())
                }
            };
            Ok(serde_json::to_string_pretty(&outcome)?)
        })
    }

    async fn repair_networking(
        &self,
        #[zbus(connection)] connection: &Connection,
        #[zbus(header)] header: Header<'_>,
    ) -> fdo::Result<String> {
        authorize_manage(connection, &header).await?;
        let ctx = self.begin_operation("repair requested")?;
        let result = repair_operation(&ctx.config, &ctx.paths, ctx.status.firewall_backend);
        self.with_runtime(|runtime| {
            let outcome = match result {
                Ok(changed) => {
                    for change in &changed {
                        runtime.state.log(change);
                    }
                    runtime.state.mark_disabled();
                    CommandOutcome::ok(
                        "Networking repair completed",
                        changed,
                        runtime.state.status(),
                    )
                }
                Err(err) => {
                    runtime.state.mark_failed(err.to_string());
                    CommandOutcome::failed("Networking repair failed", runtime.state.status())
                }
            };
            Ok(serde_json::to_string_pretty(&outcome)?)
        })
    }

    async fn tor_check(&self) -> fdo::Result<String> {
        let config = self.with_runtime(|runtime| Ok(runtime.config.clone()))?;
        let check = anonsurf_tor::tor_check(&config);
        self.with_runtime(|runtime| {
            runtime
                .state
                .update_tor_check(check.ip.clone(), check.is_tor);
            Ok(serde_json::to_string_pretty(&check)?)
        })
    }

    async fn get_logs(&self, limit: u32) -> fdo::Result<String> {
        self.with_runtime(|runtime| {
            let logs = runtime.state.logs(limit as usize);
            Ok(serde_json::to_string_pretty(&logs)?)
        })
    }

    async fn set_config(
        &self,
        config_toml: &str,
        #[zbus(connection)] connection: &Connection,
        #[zbus(header)] header: Header<'_>,
    ) -> fdo::Result<String> {
        authorize_manage(connection, &header).await?;
        let config: Config = toml::from_str(config_toml).map_err(|err| {
            fdo::Error::InvalidArgs(format!("failed to parse config TOML: {err}"))
        })?;
        let path = self.with_runtime(|runtime| Ok(runtime.paths.config_file.clone()))?;
        config
            .store(&path)
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        self.with_runtime(|runtime| {
            runtime.config = config;
            runtime.state.log("config updated");
            let outcome = CommandOutcome::ok(
                "Configuration updated",
                vec![path.display().to_string()],
                runtime.state.status(),
            );
            Ok(serde_json::to_string_pretty(&outcome)?)
        })
    }
}

fn start_operation(config: &Config, paths: &Paths) -> Result<StartResult> {
    fs::create_dir_all(&paths.runtime_dir)?;
    fs::create_dir_all(&paths.state_dir)?;

    let bridges = load_bridges();
    let tor_pid = anonsurf_tor::start(config, paths, &bridges)?;

    let dns_plan = anonsurf_dns::apply_tor_dns(paths, config.tor.dns_port)?;

    let firewall_plan = anonsurf_firewall::apply(config, paths)?;

    let mut snapshot = dns_plan.snapshot;
    snapshot.firewall_backend = firewall_plan.backend;
    snapshot.tor_pid = Some(tor_pid);
    snapshot.store(&paths.snapshot_file)?;

    let mut changed = Vec::new();
    changed.push(format!("started private Tor pid {tor_pid}"));
    changed.extend(dns_plan.changed);
    changed.extend(firewall_plan.changed);
    Ok(StartResult {
        changed,
        dns_backend: dns_plan.backend,
        firewall_backend: firewall_plan.backend,
        bridge_mode: config.tor.bridge_mode,
    })
}

fn repair_operation(
    config: &Config,
    paths: &Paths,
    fallback_firewall_backend: FirewallBackend,
) -> Result<Vec<String>> {
    let mut changed = Vec::new();

    if config.repair.stop_private_tor {
        match anonsurf_tor::stop(paths) {
            Ok(Some(pid)) => changed.push(format!("stopped private Tor pid {pid}")),
            Ok(None) => changed.push("private Tor was not running".to_string()),
            Err(err) => changed.push(format!("private Tor stop warning: {err}")),
        }
    }

    let snapshot_backend = anonsurf_core::Snapshot::load(&paths.snapshot_file)?
        .map(|snapshot| snapshot.firewall_backend)
        .unwrap_or(fallback_firewall_backend);

    if config.repair.remove_managed_firewall_rules {
        changed.extend(anonsurf_firewall::repair(snapshot_backend)?);
    }

    if config.repair.restore_dns_snapshot {
        changed.extend(anonsurf_dns::repair(paths)?);
    }

    let _ = fs::remove_file(&paths.snapshot_file);
    changed.push("repair completed".to_string());
    Ok(changed)
}

fn load_bridges() -> Vec<String> {
    ["/etc/anonsurf-rs/bridges.txt", "configs/bridges.txt"]
        .into_iter()
        .find_map(|path| fs::read_to_string(path).ok())
        .map(|raw| {
            raw.lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

async fn authorize_manage(connection: &Connection, header: &Header<'_>) -> fdo::Result<()> {
    let sender = header
        .sender()
        .ok_or_else(|| fdo::Error::AccessDenied("missing D-Bus sender".to_string()))?;

    let dbus = Proxy::new(
        connection,
        "org.freedesktop.DBus",
        "/org/freedesktop/DBus",
        "org.freedesktop.DBus",
    )
    .await
    .map_err(|err| fdo::Error::Failed(err.to_string()))?;

    let uid: u32 = dbus
        .call("GetConnectionUnixUser", &(sender.as_str()))
        .await
        .map_err(|err| fdo::Error::AccessDenied(err.to_string()))?;
    if uid == 0 {
        return Ok(());
    }

    let mut subject_details = HashMap::new();
    subject_details.insert("name", Value::from(sender.as_str()));
    let subject = ("system-bus-name", subject_details);
    let details: HashMap<&str, &str> = HashMap::new();
    let flags = 1_u32; // AllowUserInteraction.

    let polkit = Proxy::new(
        connection,
        "org.freedesktop.PolicyKit1",
        "/org/freedesktop/PolicyKit1/Authority",
        "org.freedesktop.PolicyKit1.Authority",
    )
    .await
    .map_err(|err| fdo::Error::AccessDenied(format!("Polkit unavailable: {err}")))?;

    let (authorized, challenge, _details): (bool, bool, HashMap<String, String>) = polkit
        .call(
            "CheckAuthorization",
            &(subject, POLKIT_ACTION_MANAGE, details, flags, ""),
        )
        .await
        .map_err(|err| fdo::Error::AccessDenied(format!("Polkit denied request: {err}")))?;

    if authorized {
        Ok(())
    } else if challenge {
        Err(fdo::Error::AuthFailed(
            "authentication was required but not completed".to_string(),
        ))
    } else {
        Err(fdo::Error::AccessDenied(
            "not authorized to manage AnonSurf networking".to_string(),
        ))
    }
}

fn map_busy(err: OperationError) -> anyhow::Error {
    anyhow::anyhow!(err)
}

#[allow(dead_code)]
const _: &str = DBUS_INTERFACE;
