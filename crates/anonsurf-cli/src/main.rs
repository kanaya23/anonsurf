use anonsurf_core::{
    command_exists, CommandOutcome, Config, Status, TorCheck, DBUS_INTERFACE, DBUS_PATH,
    DBUS_SERVICE,
};
use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use zbus::{Connection, Proxy};

#[derive(Debug, Parser)]
#[command(
    name = "anonsurf",
    version,
    about = "Distro-agnostic Tor transparent proxy manager"
)]
struct Args {
    #[arg(
        long,
        global = true,
        help = "Print daemon JSON without human formatting"
    )]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Start,
    Stop,
    Restart,
    Status,
    #[command(name = "changeid")]
    ChangeId,
    #[command(name = "new-identity", alias = "newnym")]
    NewIdentity,
    #[command(name = "myip")]
    MyIp,
    #[command(name = "tor-check")]
    TorCheck,
    Repair,
    Logs {
        #[arg(default_value_t = 100)]
        limit: u32,
    },
    Doctor,
    Completions {
        shell: CompletionShell,
    },
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CompletionShell {
    Bash,
    Zsh,
    Fish,
}

#[derive(Debug, Subcommand)]
enum ConfigCommand {
    ShowDefault,
    ApplyDefault,
    ApplyFile { path: PathBuf },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    match args.command {
        Command::Start => print_outcome(call_no_args("Start").await?, args.json)?,
        Command::Stop => print_outcome(call_no_args("Stop").await?, args.json)?,
        Command::Restart => print_outcome(call_no_args("Restart").await?, args.json)?,
        Command::Status => print_status(call_no_args("GetStatus").await?, args.json)?,
        Command::ChangeId | Command::NewIdentity => {
            print_outcome(call_no_args("NewIdentity").await?, args.json)?
        }
        Command::MyIp | Command::TorCheck => {
            print_tor_check(call_no_args("TorCheck").await?, args.json)?
        }
        Command::Repair => print_outcome(call_no_args("RepairNetworking").await?, args.json)?,
        Command::Logs { limit } => print_logs(call_with_u32("GetLogs", limit).await?, args.json)?,
        Command::Doctor => doctor(),
        Command::Completions { shell } => print_completions(shell),
        Command::Config { command } => handle_config(command, args.json).await?,
    }
    Ok(())
}

async fn handle_config(command: ConfigCommand, json: bool) -> Result<()> {
    match command {
        ConfigCommand::ShowDefault => {
            let config = Config::default();
            println!("{}", config.to_toml_string()?);
        }
        ConfigCommand::ApplyDefault => {
            let config = Config::default().to_toml_string()?;
            print_outcome(call_with_str("SetConfig", &config).await?, json)?;
        }
        ConfigCommand::ApplyFile { path } => {
            let config = std::fs::read_to_string(&path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            print_outcome(call_with_str("SetConfig", &config).await?, json)?;
        }
    }
    Ok(())
}

async fn proxy<'a>(connection: &'a Connection) -> Result<Proxy<'a>> {
    Ok(Proxy::new(connection, DBUS_SERVICE, DBUS_PATH, DBUS_INTERFACE).await?)
}

async fn call_no_args(method: &str) -> Result<String> {
    let connection = Connection::system().await?;
    let proxy = proxy(&connection).await?;
    Ok(proxy.call(method, &()).await?)
}

async fn call_with_u32(method: &str, value: u32) -> Result<String> {
    let connection = Connection::system().await?;
    let proxy = proxy(&connection).await?;
    Ok(proxy.call(method, &(value)).await?)
}

async fn call_with_str(method: &str, value: &str) -> Result<String> {
    let connection = Connection::system().await?;
    let proxy = proxy(&connection).await?;
    Ok(proxy.call(method, &(value)).await?)
}

fn print_status(raw: String, json: bool) -> Result<()> {
    if json {
        println!("{raw}");
        return Ok(());
    }
    let status: Status = serde_json::from_str(&raw)?;
    println!("Status: {:?}", status.status);
    println!("Tor: {:?}", status.tor_status);
    println!(
        "Exit IP: {}",
        status
            .current_exit_ip
            .unwrap_or_else(|| "unknown".to_string())
    );
    println!(
        "Tor check: {}",
        status
            .is_tor
            .map(|value| if value { "tor" } else { "not tor" })
            .unwrap_or("unknown")
    );
    println!("DNS backend: {:?}", status.dns_backend);
    println!("Firewall backend: {:?}", status.firewall_backend);
    println!("Bridge mode: {:?}", status.bridge_mode);
    if let Some(error) = status.last_error {
        println!("Last error: {error}");
    }
    Ok(())
}

fn print_outcome(raw: String, json: bool) -> Result<()> {
    if json {
        println!("{raw}");
        return Ok(());
    }
    let outcome: CommandOutcome = serde_json::from_str(&raw)?;
    println!("{}", outcome.message);
    for change in outcome.changed {
        println!(" - {change}");
    }
    if let Some(error) = outcome.status.last_error {
        println!("Last error: {error}");
    }
    Ok(())
}

fn print_tor_check(raw: String, json: bool) -> Result<()> {
    if json {
        println!("{raw}");
        return Ok(());
    }
    let check: TorCheck = serde_json::from_str(&raw)?;
    println!(
        "Exit IP: {}",
        check.ip.unwrap_or_else(|| "unknown".to_string())
    );
    println!("Tor: {}", if check.is_tor { "yes" } else { "no" });
    if let Some(error) = check.error {
        println!("Error: {error}");
    }
    Ok(())
}

fn print_logs(raw: String, json: bool) -> Result<()> {
    if json {
        println!("{raw}");
        return Ok(());
    }
    let logs: Vec<String> = serde_json::from_str(&raw)?;
    for line in logs {
        println!("{line}");
    }
    Ok(())
}

fn doctor() {
    println!("anonsurf-rs doctor");
    for command in ["tor", "curl", "nft", "iptables"] {
        let found = command_exists(command);
        println!("{command}: {}", if found { "found" } else { "missing" });
    }
    println!(
        "resolvectl: {}",
        if command_exists("resolvectl") {
            "available (optional systemd-resolved backend)"
        } else {
            "not found (optional; resolvconf/resolv.conf backends still supported)"
        }
    );
    println!(
        "system bus: {}",
        if PathBuf::from("/run/dbus/system_bus_socket").exists() {
            "found"
        } else {
            "missing"
        }
    );
}

fn print_completions(shell: CompletionShell) {
    match shell {
        CompletionShell::Bash => print!(
            "{}",
            include_str!("../../../packaging/completions/anonsurf.bash")
        ),
        CompletionShell::Zsh => print!(
            "{}",
            include_str!("../../../packaging/completions/_anonsurf")
        ),
        CompletionShell::Fish => print!(
            "{}",
            include_str!("../../../packaging/completions/anonsurf.fish")
        ),
    }
}
