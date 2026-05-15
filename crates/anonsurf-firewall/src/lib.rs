use anonsurf_core::{command_exists, Config, FirewallBackend, Paths};
use anyhow::{anyhow, Context, Result};
use std::{ffi::OsStr, fs, path::Path, process::Command};

pub const NFT_TABLE: &str = "anonsurf_rs";
pub const IPTABLES_NAT_CHAIN: &str = "ANONSURF_RS_NAT";
pub const IPTABLES_FILTER_CHAIN: &str = "ANONSURF_RS_FILTER";

#[derive(Debug, Clone)]
pub struct FirewallPlan {
    pub backend: FirewallBackend,
    pub changed: Vec<String>,
}

pub fn detect_backend(preferred: FirewallBackend) -> FirewallBackend {
    match preferred {
        FirewallBackend::Nftables if command_exists("nft") => FirewallBackend::Nftables,
        FirewallBackend::Iptables if command_exists("iptables") => FirewallBackend::Iptables,
        FirewallBackend::Nftables | FirewallBackend::Unknown => {
            if command_exists("nft") {
                FirewallBackend::Nftables
            } else if command_exists("iptables") {
                FirewallBackend::Iptables
            } else {
                FirewallBackend::None
            }
        }
        FirewallBackend::Iptables => {
            if command_exists("iptables") {
                FirewallBackend::Iptables
            } else if command_exists("nft") {
                FirewallBackend::Nftables
            } else {
                FirewallBackend::None
            }
        }
        FirewallBackend::None => FirewallBackend::None,
    }
}

pub fn apply(config: &Config, paths: &Paths) -> Result<FirewallPlan> {
    let backend = detect_backend(config.firewall.preferred_backend);
    match backend {
        FirewallBackend::Nftables => {
            fs::create_dir_all(&paths.runtime_dir)?;
            let script = build_nft_script(config);
            fs::write(&paths.nftables_file, script)?;
            let _ = run("nft", ["delete", "table", "inet", NFT_TABLE]);
            run("nft", ["-f", path_arg(&paths.nftables_file).as_str()])?;
            Ok(FirewallPlan {
                backend,
                changed: vec![
                    format!("installed nftables inet {NFT_TABLE} table"),
                    format!("wrote {}", paths.nftables_file.display()),
                ],
            })
        }
        FirewallBackend::Iptables => apply_iptables(config),
        FirewallBackend::None | FirewallBackend::Unknown => Err(anyhow!(
            "neither nftables nor iptables is available; cannot enforce transparent Tor routing"
        )),
    }
}

pub fn repair(backend: FirewallBackend) -> Result<Vec<String>> {
    let mut changed = Vec::new();
    match backend {
        FirewallBackend::Nftables | FirewallBackend::Unknown => {
            if command_exists("nft") {
                let _ = run("nft", ["delete", "table", "inet", NFT_TABLE]);
                changed.push(format!(
                    "removed nftables inet {NFT_TABLE} table if present"
                ));
            }
        }
        FirewallBackend::Iptables => {
            if command_exists("iptables") {
                remove_iptables_chain_references("nat", "OUTPUT", IPTABLES_NAT_CHAIN);
                remove_iptables_chain_references("filter", "OUTPUT", IPTABLES_FILTER_CHAIN);
                let _ = run("iptables", ["-t", "nat", "-F", IPTABLES_NAT_CHAIN]);
                let _ = run("iptables", ["-t", "nat", "-X", IPTABLES_NAT_CHAIN]);
                let _ = run("iptables", ["-F", IPTABLES_FILTER_CHAIN]);
                let _ = run("iptables", ["-X", IPTABLES_FILTER_CHAIN]);
                changed.push("removed tagged anonsurf-rs iptables chains if present".to_string());
            }
        }
        FirewallBackend::None => {}
    }
    Ok(changed)
}

pub fn build_nft_script(config: &Config) -> String {
    let tor = &config.tor;
    let excludes = config
        .firewall
        .exclude_cidrs
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ");
    let tor_uid_return = tor
        .user
        .as_deref()
        .map(|user| format!("    meta skuid \"{user}\" return\n"))
        .unwrap_or_default();
    let tor_uid_accept = tor
        .user
        .as_deref()
        .map(|user| format!("    meta skuid \"{user}\" accept\n"))
        .unwrap_or_default();
    let input_policy = if config.firewall.block_inbound {
        "drop"
    } else {
        "accept"
    };

    format!(
        r#"table inet {NFT_TABLE} {{
  chain output_nat {{
    type nat hook output priority dstnat; policy accept;
{tor_uid_return}    udp dport 53 redirect to :{dns_port}
    ip daddr {{ {excludes} }} return
    tcp flags syn redirect to :{trans_port}
  }}

  chain output_filter {{
    type filter hook output priority filter; policy accept;
{tor_uid_accept}    ct state established,related accept
    ip daddr {{ {excludes} }} accept
    oifname "lo" accept
    reject with icmpx type admin-prohibited
  }}

  chain input_filter {{
    type filter hook input priority filter; policy {input_policy};
    ct state established,related accept
    iifname "lo" accept
  }}
}}
"#,
        dns_port = tor.dns_port,
        trans_port = tor.trans_port,
    )
}

fn apply_iptables(config: &Config) -> Result<FirewallPlan> {
    let tor = &config.tor;
    let _ = run("iptables", ["-t", "nat", "-N", IPTABLES_NAT_CHAIN]);
    let _ = run("iptables", ["-N", IPTABLES_FILTER_CHAIN]);
    let _ = run("iptables", ["-t", "nat", "-F", IPTABLES_NAT_CHAIN]);
    let _ = run("iptables", ["-F", IPTABLES_FILTER_CHAIN]);

    if let Some(user) = tor.user.as_deref() {
        run(
            "iptables",
            [
                "-t",
                "nat",
                "-A",
                IPTABLES_NAT_CHAIN,
                "-m",
                "owner",
                "--uid-owner",
                user,
                "-j",
                "RETURN",
            ],
        )?;
        run(
            "iptables",
            [
                "-A",
                IPTABLES_FILTER_CHAIN,
                "-m",
                "owner",
                "--uid-owner",
                user,
                "-j",
                "ACCEPT",
            ],
        )?;
    }

    run(
        "iptables",
        [
            "-t",
            "nat",
            "-A",
            IPTABLES_NAT_CHAIN,
            "-p",
            "udp",
            "--dport",
            "53",
            "-j",
            "REDIRECT",
            "--to-ports",
            &tor.dns_port.to_string(),
        ],
    )?;

    for cidr in &config.firewall.exclude_cidrs {
        run(
            "iptables",
            [
                "-t",
                "nat",
                "-A",
                IPTABLES_NAT_CHAIN,
                "-d",
                cidr,
                "-j",
                "RETURN",
            ],
        )?;
        run(
            "iptables",
            ["-A", IPTABLES_FILTER_CHAIN, "-d", cidr, "-j", "ACCEPT"],
        )?;
    }

    run(
        "iptables",
        [
            "-t",
            "nat",
            "-A",
            IPTABLES_NAT_CHAIN,
            "-p",
            "tcp",
            "--syn",
            "-j",
            "REDIRECT",
            "--to-ports",
            &tor.trans_port.to_string(),
        ],
    )?;
    run(
        "iptables",
        [
            "-A",
            IPTABLES_FILTER_CHAIN,
            "-m",
            "state",
            "--state",
            "ESTABLISHED,RELATED",
            "-j",
            "ACCEPT",
        ],
    )?;
    run(
        "iptables",
        ["-A", IPTABLES_FILTER_CHAIN, "-o", "lo", "-j", "ACCEPT"],
    )?;
    run("iptables", ["-A", IPTABLES_FILTER_CHAIN, "-j", "REJECT"])?;

    ensure_jump(
        "iptables",
        &["-t", "nat", "-C", "OUTPUT", "-j", IPTABLES_NAT_CHAIN],
        &["-t", "nat", "-A", "OUTPUT", "-j", IPTABLES_NAT_CHAIN],
    )?;
    ensure_jump(
        "iptables",
        &["-C", "OUTPUT", "-j", IPTABLES_FILTER_CHAIN],
        &["-A", "OUTPUT", "-j", IPTABLES_FILTER_CHAIN],
    )?;

    Ok(FirewallPlan {
        backend: FirewallBackend::Iptables,
        changed: vec!["installed tagged anonsurf-rs iptables chains".to_string()],
    })
}

fn ensure_jump(program: &str, check_args: &[&str], add_args: &[&str]) -> Result<()> {
    if run(program, check_args).is_err() {
        run(program, add_args)?;
    }
    Ok(())
}

fn remove_iptables_chain_references(table: &str, chain: &str, target: &str) {
    loop {
        let result = if table == "filter" {
            run("iptables", ["-D", chain, "-j", target])
        } else {
            run("iptables", ["-t", table, "-D", chain, "-j", target])
        };
        if result.is_err() {
            break;
        }
    }
}

fn path_arg(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn run<I, S>(program: &str, args: I) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("failed to execute {program}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(anyhow!(
            "{program} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nft_script_uses_dedicated_table_and_ports() {
        let config = Config::default();
        let script = build_nft_script(&config);
        assert!(script.contains("table inet anonsurf_rs"));
        assert!(script.contains("redirect to :9053"));
        assert!(script.contains("redirect to :9040"));
        assert!(!script.contains("flush ruleset"));
        let dns_redirect = script.find("udp dport 53 redirect").unwrap();
        let exclude_return = script.find("ip daddr {").unwrap();
        assert!(dns_redirect < exclude_return);
    }
}
