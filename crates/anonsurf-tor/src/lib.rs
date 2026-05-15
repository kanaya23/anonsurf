use anonsurf_core::{BridgeMode, Config, Paths, TorCheck};
use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::{
    fs,
    io::{Read, Write},
    net::TcpStream,
    path::Path,
    process::{Command, Stdio},
    time::Duration,
};

pub fn generate_torrc(config: &Config, paths: &Paths, bridges: &[String]) -> String {
    let tor = &config.tor;
    let mut out = String::new();
    out.push_str("# Managed by anonsurf-rs. Do not edit by hand.\n");
    out.push_str(&format!("DataDirectory {}\n", paths.tor_data_dir.display()));
    out.push_str(&format!("PidFile {}\n", paths.tor_pid_file.display()));
    out.push_str(&format!(
        "VirtualAddrNetwork {}\n",
        tor.virtual_addr_network
    ));
    out.push_str("AutomapHostsOnResolve 1\n");
    out.push_str("AutomapHostsSuffixes .exit,.onion\n");
    out.push_str(&format!(
        "TransPort 127.0.0.1:{} IsolateClientAddr IsolateSOCKSAuth IsolateClientProtocol IsolateDestPort IsolateDestAddr\n",
        tor.trans_port
    ));
    out.push_str(&format!(
        "SocksPort 127.0.0.1:{} IsolateClientAddr IsolateSOCKSAuth IsolateClientProtocol IsolateDestPort IsolateDestAddr\n",
        tor.socks_port
    ));
    out.push_str(&format!("DNSPort 127.0.0.1:{}\n", tor.dns_port));
    out.push_str(&format!("ControlPort 127.0.0.1:{}\n", tor.control_port));
    out.push_str("CookieAuthentication 1\n");
    out.push_str("ClientRejectInternalAddresses 1\n");
    out.push_str("SafeSocks 1\n");
    out.push_str("TestSocks 1\n");
    out.push_str("HardwareAccel 1\n");
    out.push_str("RunAsDaemon 0\n");
    if let Some(user) = tor.user.as_deref() {
        out.push_str(&format!("User {user}\n"));
    }

    match tor.bridge_mode {
        BridgeMode::Off => {}
        BridgeMode::Auto => {
            if let Some(bridge) = bridges.first() {
                out.push_str("UseBridges 1\n");
                out.push_str("ClientTransportPlugin obfs4 exec /usr/bin/obfs4proxy managed\n");
                out.push_str(&format!("Bridge {bridge}\n"));
            }
        }
        BridgeMode::Manual => {
            if let Some(bridge) = tor.manual_bridge.as_deref() {
                out.push_str("UseBridges 1\n");
                out.push_str("ClientTransportPlugin obfs4 exec /usr/bin/obfs4proxy managed\n");
                out.push_str(&format!("Bridge {bridge}\n"));
            }
        }
    }
    out
}

pub fn start(config: &Config, paths: &Paths, bridges: &[String]) -> Result<u32> {
    fs::create_dir_all(&paths.runtime_dir)?;
    fs::create_dir_all(&paths.tor_data_dir)?;

    let mut effective_config = config.clone();
    if let Some(user) = config.tor.user.as_deref() {
        if user_exists(user) {
            let _ = Command::new("chown")
                .arg("-R")
                .arg(format!("{user}:{user}"))
                .arg(&paths.tor_data_dir)
                .status();
        } else {
            effective_config.tor.user = None;
        }
    }

    let torrc = generate_torrc(&effective_config, paths, bridges);
    fs::write(&paths.torrc_file, torrc)?;

    let child = Command::new(&effective_config.tor.binary)
        .arg("-f")
        .arg(&paths.torrc_file)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("failed to spawn {}", effective_config.tor.binary))?;

    let pid = child.id();
    fs::write(&paths.tor_pid_file, pid.to_string())?;
    Ok(pid)
}

pub fn stop(paths: &Paths) -> Result<Option<u32>> {
    let pid = match read_pid(&paths.tor_pid_file)? {
        Some(pid) => pid,
        None => return Ok(None),
    };
    let status = Command::new("kill")
        .arg("-TERM")
        .arg(pid.to_string())
        .status()
        .with_context(|| "failed to execute kill")?;
    if !status.success() {
        return Err(anyhow!("failed to stop Tor pid {pid}"));
    }
    let _ = fs::remove_file(&paths.tor_pid_file);
    Ok(Some(pid))
}

pub fn new_identity(config: &Config, paths: &Paths) -> Result<String> {
    let cookie = fs::read(&paths.tor_control_cookie).with_context(|| {
        format!(
            "failed to read Tor control cookie {}",
            paths.tor_control_cookie.display()
        )
    })?;
    let cookie_hex = hex(&cookie);
    let mut stream = TcpStream::connect(("127.0.0.1", config.tor.control_port))
        .with_context(|| "failed to connect to Tor control port")?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream
        .write_all(format!("AUTHENTICATE {cookie_hex}\r\nSIGNAL NEWNYM\r\nQUIT\r\n").as_bytes())?;
    let mut reply = String::new();
    stream.read_to_string(&mut reply)?;
    if reply.contains("250 OK") {
        Ok(reply)
    } else {
        Err(anyhow!("Tor control rejected new identity: {reply}"))
    }
}

pub fn tor_check(config: &Config) -> TorCheck {
    let proxy = format!("socks5h://127.0.0.1:{}", config.tor.socks_port);
    let output = Command::new("curl")
        .arg("-fsSL")
        .arg("--max-time")
        .arg("15")
        .arg("--proxy")
        .arg(&proxy)
        .arg("https://check.torproject.org/api/ip")
        .output();

    match output {
        Ok(output) if output.status.success() => {
            parse_tor_check(&String::from_utf8_lossy(&output.stdout))
        }
        Ok(output) => TorCheck {
            ip: None,
            is_tor: false,
            source: "check.torproject.org".to_string(),
            error: Some(String::from_utf8_lossy(&output.stderr).trim().to_string()),
        },
        Err(err) => TorCheck {
            ip: None,
            is_tor: false,
            source: "check.torproject.org".to_string(),
            error: Some(err.to_string()),
        },
    }
}

#[derive(Debug, Deserialize)]
struct TorProjectReply {
    #[serde(rename = "IsTor")]
    is_tor: bool,
    #[serde(rename = "IP")]
    ip: Option<String>,
}

fn parse_tor_check(raw: &str) -> TorCheck {
    match serde_json::from_str::<TorProjectReply>(raw) {
        Ok(reply) => TorCheck {
            ip: reply.ip,
            is_tor: reply.is_tor,
            source: "check.torproject.org".to_string(),
            error: None,
        },
        Err(err) => TorCheck {
            ip: None,
            is_tor: false,
            source: "check.torproject.org".to_string(),
            error: Some(format!("failed to parse Tor check response: {err}")),
        },
    }
}

fn read_pid(path: &Path) -> Result<Option<u32>> {
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(path)?;
    Ok(Some(raw.trim().parse()?))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn user_exists(user: &str) -> bool {
    Command::new("id")
        .arg("-u")
        .arg(user)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn torrc_uses_private_paths_and_not_system_torrc() {
        let config = Config::default();
        let paths = Paths::new(
            "/tmp/config.toml",
            "/tmp/run-anonsurf",
            "/tmp/state-anonsurf",
        );
        let torrc = generate_torrc(&config, &paths, &[]);
        assert!(torrc.contains("DataDirectory /tmp/state-anonsurf/tor"));
        assert!(torrc.contains("ControlPort 127.0.0.1:9051"));
        assert!(!torrc.contains("/etc/tor/torrc"));
    }

    #[test]
    fn parses_tor_project_reply() {
        let check = parse_tor_check(r#"{"IsTor":true,"IP":"1.2.3.4"}"#);
        assert!(check.is_tor);
        assert_eq!(check.ip.as_deref(), Some("1.2.3.4"));
    }
}
