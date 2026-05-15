use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AnonStatus {
    #[default]
    Disabled,
    Transitioning,
    Enabled,
    Failed,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TorStatus {
    #[default]
    Stopped,
    Starting,
    Running,
    Failed,
    Unknown,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DnsBackend {
    SystemdResolved,
    Resolvconf,
    ResolvConfFile,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FirewallBackend {
    Nftables,
    Iptables,
    None,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BridgeMode {
    #[default]
    Off,
    Auto,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Status {
    pub status: AnonStatus,
    pub tor_status: TorStatus,
    pub current_exit_ip: Option<String>,
    pub is_tor: Option<bool>,
    pub dns_backend: DnsBackend,
    pub firewall_backend: FirewallBackend,
    pub bridge_mode: BridgeMode,
    pub last_error: Option<String>,
}

impl Default for Status {
    fn default() -> Self {
        Self {
            status: AnonStatus::Disabled,
            tor_status: TorStatus::Stopped,
            current_exit_ip: None,
            is_tor: None,
            dns_backend: DnsBackend::Unknown,
            firewall_backend: FirewallBackend::Unknown,
            bridge_mode: BridgeMode::Off,
            last_error: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TorCheck {
    pub ip: Option<String>,
    pub is_tor: bool,
    pub source: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandOutcome {
    pub ok: bool,
    pub message: String,
    pub changed: Vec<String>,
    pub status: Status,
}

impl CommandOutcome {
    pub fn ok(message: impl Into<String>, changed: Vec<String>, status: Status) -> Self {
        Self {
            ok: true,
            message: message.into(),
            changed,
            status,
        }
    }

    pub fn failed(message: impl Into<String>, status: Status) -> Self {
        Self {
            ok: false,
            message: message.into(),
            changed: Vec::new(),
            status,
        }
    }
}
