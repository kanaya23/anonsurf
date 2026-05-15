use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    pub config_file: PathBuf,
    pub runtime_dir: PathBuf,
    pub state_dir: PathBuf,
    pub snapshot_file: PathBuf,
    pub torrc_file: PathBuf,
    pub tor_data_dir: PathBuf,
    pub tor_pid_file: PathBuf,
    pub tor_control_cookie: PathBuf,
    pub nftables_file: PathBuf,
}

impl Paths {
    pub fn system() -> Self {
        Self::new(
            "/etc/anonsurf-rs/config.toml",
            "/run/anonsurf-rs",
            "/var/lib/anonsurf-rs",
        )
    }

    pub fn new(
        config_file: impl AsRef<Path>,
        runtime_dir: impl AsRef<Path>,
        state_dir: impl AsRef<Path>,
    ) -> Self {
        let runtime_dir = runtime_dir.as_ref().to_path_buf();
        let state_dir = state_dir.as_ref().to_path_buf();
        let tor_data_dir = state_dir.join("tor");

        Self {
            config_file: config_file.as_ref().to_path_buf(),
            runtime_dir: runtime_dir.clone(),
            state_dir: state_dir.clone(),
            snapshot_file: state_dir.join("snapshot.json"),
            torrc_file: runtime_dir.join("torrc"),
            tor_data_dir: tor_data_dir.clone(),
            tor_pid_file: runtime_dir.join("tor.pid"),
            tor_control_cookie: tor_data_dir.join("control_auth_cookie"),
            nftables_file: runtime_dir.join("nftables.conf"),
        }
    }
}
