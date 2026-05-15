use anonsurf_core::{command_exists, DnsBackend, FileSnapshot, Paths, Snapshot};
use anyhow::{anyhow, Context, Result};
use std::{
    fs,
    io::ErrorKind,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

pub const RESOLV_CONF: &str = "/etc/resolv.conf";

#[derive(Debug, Clone)]
pub struct DnsPlan {
    pub backend: DnsBackend,
    pub changed: Vec<String>,
    pub snapshot: Snapshot,
}

pub fn detect_backend() -> DnsBackend {
    if command_exists("resolvectl") && Path::new("/run/systemd/resolve").exists() {
        DnsBackend::SystemdResolved
    } else if Path::new("/run/resolvconf/resolv.conf").exists()
        || Path::new("/etc/resolvconf/resolv.conf.d").exists()
    {
        DnsBackend::Resolvconf
    } else if Path::new(RESOLV_CONF).exists() {
        DnsBackend::ResolvConfFile
    } else {
        DnsBackend::Unknown
    }
}

pub fn apply_tor_dns(paths: &Paths, dns_port: u16) -> Result<DnsPlan> {
    apply_tor_dns_with_resolv_conf(paths, dns_port, Path::new(RESOLV_CONF))
}

pub fn apply_tor_dns_with_resolv_conf(
    paths: &Paths,
    dns_port: u16,
    resolv_conf_path: &Path,
) -> Result<DnsPlan> {
    fs::create_dir_all(&paths.state_dir)?;
    let backend = detect_backend();
    let mut snapshot = Snapshot::empty(now());
    let backup = paths.state_dir.join("resolv.conf.backup");

    if resolv_conf_path.exists() {
        fs::copy(resolv_conf_path, &backup)
            .with_context(|| format!("failed to snapshot {}", resolv_conf_path.display()))?;
        snapshot.dns_files.push(FileSnapshot {
            path: resolv_conf_path.to_string_lossy().into_owned(),
            backup_path: backup.to_string_lossy().into_owned(),
            existed: true,
        });
    } else {
        snapshot.dns_files.push(FileSnapshot {
            path: resolv_conf_path.to_string_lossy().into_owned(),
            backup_path: backup.to_string_lossy().into_owned(),
            existed: false,
        });
    }
    snapshot.store(&paths.snapshot_file)?;

    let resolv_conf = format!(
        "# Managed by anonsurf-rs. Use `anonsurf repair` to restore networking.\nnameserver 127.0.0.1\noptions edns0 trust-ad\n# Tor DNSPort: {dns_port}\n"
    );
    let mut changed = vec![format!("snapshotted {}", resolv_conf_path.display())];
    match fs::write(resolv_conf_path, resolv_conf) {
        Ok(()) => changed.push("set resolver to 127.0.0.1 for Tor DNSPort".to_string()),
        Err(err) if err.kind() == ErrorKind::PermissionDenied => changed.push(format!(
            "warning: could not update {} ({err}); continuing with firewall DNS redirect",
            resolv_conf_path.display()
        )),
        Err(err) => {
            return Err(err).with_context(|| {
                format!("failed to point {} at Tor DNS", resolv_conf_path.display())
            });
        }
    }

    Ok(DnsPlan {
        backend,
        changed,
        snapshot,
    })
}

pub fn repair(paths: &Paths) -> Result<Vec<String>> {
    let Some(snapshot) = Snapshot::load(&paths.snapshot_file)? else {
        return Ok(vec!["no DNS snapshot found; nothing to restore".to_string()]);
    };

    let mut changed = Vec::new();
    for file in &snapshot.dns_files {
        let target = Path::new(&file.path);
        let backup = Path::new(&file.backup_path);
        if file.existed {
            if backup.exists() {
                fs::copy(backup, target).with_context(|| {
                    format!("failed to restore {} from {}", file.path, file.backup_path)
                })?;
                changed.push(format!("restored {}", file.path));
            } else {
                return Err(anyhow!("missing DNS backup {}", file.backup_path));
            }
        } else if target.exists() {
            fs::remove_file(target).with_context(|| format!("failed to remove {}", file.path))?;
            changed.push(format!("removed managed {}", file.path));
        }
    }
    Ok(changed)
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_round_trip_without_filesystem_privilege() {
        let dir = std::env::temp_dir().join(format!("anonsurf-dns-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let snapshot = Snapshot::empty(42);
        let path = dir.join("snapshot.json");
        snapshot.store(&path).unwrap();
        assert_eq!(Snapshot::load(&path).unwrap(), Some(snapshot));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn apply_and_repair_restore_temp_resolv_conf() {
        let dir = std::env::temp_dir().join(format!(
            "anonsurf-dns-apply-repair-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let resolv_conf = dir.join("resolv.conf");
        fs::write(&resolv_conf, "nameserver 9.9.9.9\n").unwrap();
        let paths = Paths::new(dir.join("config.toml"), dir.join("run"), dir.join("state"));

        let plan = apply_tor_dns_with_resolv_conf(&paths, 9053, &resolv_conf).unwrap();
        assert!(plan.changed.iter().any(|line| line.contains("snapshotted")));
        let tor_dns = fs::read_to_string(&resolv_conf).unwrap();
        assert!(tor_dns.contains("nameserver 127.0.0.1"));
        assert!(tor_dns.contains("Tor DNSPort: 9053"));

        let changed = repair(&paths).unwrap();
        assert!(changed.iter().any(|line| line.contains("restored")));
        assert_eq!(
            fs::read_to_string(&resolv_conf).unwrap(),
            "nameserver 9.9.9.9\n"
        );

        let _ = fs::remove_dir_all(&dir);
    }
}
