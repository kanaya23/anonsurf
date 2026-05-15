use crate::FirewallBackend;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, ErrorKind},
    path::Path,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileSnapshot {
    pub path: String,
    pub backup_path: String,
    pub existed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub version: u32,
    pub created_unix_secs: u64,
    pub dns_files: Vec<FileSnapshot>,
    pub firewall_backend: FirewallBackend,
    pub tor_pid: Option<u32>,
}

impl Snapshot {
    pub fn empty(now_unix_secs: u64) -> Self {
        Self {
            version: 1,
            created_unix_secs: now_unix_secs,
            dns_files: Vec::new(),
            firewall_backend: FirewallBackend::Unknown,
            tor_pid: None,
        }
    }

    pub fn load(path: &Path) -> io::Result<Option<Self>> {
        match fs::read_to_string(path) {
            Ok(raw) => serde_json::from_str(&raw)
                .map(Some)
                .map_err(|err| io::Error::new(ErrorKind::InvalidData, err)),
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(None),
            Err(err) => Err(err),
        }
    }

    pub fn store(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let raw = serde_json::to_string_pretty(self)
            .map_err(|err| io::Error::new(ErrorKind::InvalidData, err))?;
        fs::write(path, raw)
    }
}
