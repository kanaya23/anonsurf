use crate::{BridgeMode, FirewallBackend};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, ErrorKind},
    path::Path,
};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub tor: TorConfig,
    pub dns: DnsConfig,
    pub firewall: FirewallConfig,
    pub repair: RepairConfig,
}

impl Config {
    pub fn load_or_default(path: &Path) -> io::Result<Self> {
        match fs::read_to_string(path) {
            Ok(raw) => toml::from_str(&raw).map_err(|err| {
                io::Error::new(
                    ErrorKind::InvalidData,
                    format!("failed to parse {}: {err}", path.display()),
                )
            }),
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(Self::default()),
            Err(err) => Err(err),
        }
    }

    pub fn to_toml_string(&self) -> io::Result<String> {
        toml::to_string_pretty(self).map_err(|err| io::Error::new(ErrorKind::InvalidData, err))
    }

    pub fn store(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, self.to_toml_string()?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TorConfig {
    pub binary: String,
    pub trans_port: u16,
    pub socks_port: u16,
    pub dns_port: u16,
    pub control_port: u16,
    pub virtual_addr_network: String,
    pub user: Option<String>,
    pub bridge_mode: BridgeMode,
    pub manual_bridge: Option<String>,
    pub bootstrap_timeout_secs: u64,
}

impl Default for TorConfig {
    fn default() -> Self {
        Self {
            binary: "tor".to_string(),
            trans_port: 9040,
            socks_port: 9050,
            dns_port: 9053,
            control_port: 9051,
            virtual_addr_network: "10.192.0.0/10".to_string(),
            user: Some("debian-tor".to_string()),
            bridge_mode: BridgeMode::Off,
            manual_bridge: None,
            bootstrap_timeout_secs: 30,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DnsConfig {
    pub prefer_systemd_resolved: bool,
    pub fallback_to_resolv_conf: bool,
}

impl Default for DnsConfig {
    fn default() -> Self {
        Self {
            prefer_systemd_resolved: true,
            fallback_to_resolv_conf: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirewallConfig {
    pub preferred_backend: FirewallBackend,
    pub exclude_cidrs: Vec<String>,
    pub block_inbound: bool,
}

impl Default for FirewallConfig {
    fn default() -> Self {
        Self {
            preferred_backend: FirewallBackend::Nftables,
            exclude_cidrs: vec![
                "127.0.0.0/8".to_string(),
                "10.0.0.0/8".to_string(),
                "172.16.0.0/12".to_string(),
                "192.168.0.0/16".to_string(),
            ],
            block_inbound: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairConfig {
    pub conservative_only: bool,
    pub restore_dns_snapshot: bool,
    pub remove_managed_firewall_rules: bool,
    pub stop_private_tor: bool,
}

impl Default for RepairConfig {
    fn default() -> Self {
        Self {
            conservative_only: true,
            restore_dns_snapshot: true,
            remove_managed_firewall_rules: true,
            stop_private_tor: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_round_trips_as_toml() {
        let config = Config::default();
        let encoded = config.to_toml_string().unwrap();
        let decoded: Config = toml::from_str(&encoded).unwrap();
        assert_eq!(decoded, config);
    }
}
