# anonsurf-rs

AnonSurf is being rebuilt as a distro-agnostic Rust service instead of the old
Parrot-specific Nim/Vala/bash stack.

## Status

**Project status: finished (ship-ready for Debian/Ubuntu-style systems).**

The new control path is:

```text
GTK/libadwaita GUI or clap CLI
  -> unprivileged D-Bus client
  -> privileged anonsurfd
  -> private Tor + DNS snapshot/restore + dedicated firewall rules
```

## Components

- `anonsurf-core`: config, status, state machine, paths, snapshots.
- `anonsurf-daemon`: privileged D-Bus service.
- `anonsurf-cli`: Rust/clap terminal interface with legacy command aliases.
- `anonsurf-gui`: GTK4/libadwaita desktop app.
- `anonsurf-firewall`: nftables backend with iptables fallback.
- `anonsurf-dns`: resolver detection, snapshot, restore, repair.
- `anonsurf-tor`: private Tor config/process/control-port handling.

## Toolchain

Use Rust from `rustup` (the repository uses `Cargo.lock` v4 and distro-packaged
Cargo on older Ubuntu/Debian releases can be too old).

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --component rustfmt,clippy
. "$HOME/.cargo/env"
```

## Build

```sh
cargo build --workspace
cargo test --workspace
```

## Install

```sh
make build
sudo make install
```

`make install` installs prebuilt release binaries and fails if they are missing
or outdated, so build as your normal user first.

## Build and install a `.deb` package (Debian/Ubuntu)

```sh
. "$HOME/.cargo/env"
sudo apt-get install -y debhelper dpkg-dev pkg-config libgtk-4-dev libadwaita-1-dev
make deb
sudo dpkg -i ../anonsurf_*_amd64.deb
```

## Uninstall / purge

```sh
sudo apt remove anonsurf
sudo apt purge anonsurf
```

## Commands

Legacy commands are preserved:

```sh
anonsurf start
anonsurf stop
anonsurf restart
anonsurf status
anonsurf changeid
anonsurf myip
```

New commands:

```sh
anonsurf new-identity
anonsurf tor-check
anonsurf repair
anonsurf logs
anonsurf doctor
anonsurf completions bash
anonsurf config show-default
```

Mutating commands (`start`, `stop`, `restart`, `changeid`, `repair`, and
`config apply-*`) require root/Polkit authorization.

In restricted environments without firewall capabilities (for example containers
without `CAP_NET_ADMIN`), you can set `firewall.preferred_backend = "none"` in
`/etc/anonsurf-rs/config.toml` for non-anonymizing functional validation.

## Threat model

anonsurf-rs is designed to reduce IP/DNS leakage from host traffic by:

- routing traffic through a private Tor instance,
- applying dedicated firewall redirects/rules,
- switching resolver behavior and restoring snapshots on stop/repair.

## What it does not protect against

- endpoint compromise or malware already running as your user/root,
- deanonymization by browser fingerprinting, account login correlation, or unsafe app behavior,
- traffic outside enforced host networking policy (for example unsupported namespaces/network stacks),
- operator misconfiguration (for example disabling firewall policy while expecting full anonymity).

## Known limitations

- Full transparent-routing guarantees require firewall capabilities (`nftables` or `iptables`) and appropriate privileges.
- In containerized/restricted environments without `CAP_NET_ADMIN`, default `start` can fail on firewall apply.
- GUI requires GTK4/libadwaita runtime support compatible with this project floor (`v4_6` / `v1_1` APIs).

## Firewall backend behavior

- Preferred selection is configured in `firewall.preferred_backend`.
- Backend order is `nftables` first, then `iptables` fallback.
- `none` is an explicit opt-out mode: firewall rules are intentionally not applied (for constrained testing only).
- Repair removes only AnonSurf-managed table/chains and does not flush unrelated host firewall state.

## DNS backend behavior

- Backend detection supports `systemd-resolved`, `resolvconf`, and plain `/etc/resolv.conf`.
- Start snapshots resolver state and points DNS at local Tor DNSPort.
- Stop/repair restores DNS from snapshot (or reports no snapshot when absent).

## Runtime Paths

- Config: `/etc/anonsurf-rs/config.toml`
- Runtime: `/run/anonsurf-rs/`
- State/snapshots: `/var/lib/anonsurf-rs/`

The daemon uses a private Tor instance and never rewrites `/etc/tor/torrc`.
Firewall changes are isolated to an `inet anonsurf_rs` nftables table or tagged
iptables chains. `anonsurf repair` restores the recorded DNS snapshot and removes
only anonsurf-rs managed firewall state.

## Repair instructions

If networking is interrupted or an operation fails mid-way:

```sh
sudo anonsurf repair
anonsurf --json status
```

`repair` is safe to run repeatedly and is the primary rollback mechanism for AnonSurf-managed DNS/firewall state.

## Safe Testing

Local tests avoid mutating host networking:

```sh
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --release --workspace
make install DESTDIR=/tmp/anonsurf-install-check
```

For a fuller start/repair smoke test, use the container harness:

```sh
tools/sandbox-smoke.sh
```

That harness runs with `NET_ADMIN` inside Docker/Podman, so DNS/firewall/Tor
mutations stay inside the container network namespace.

## Container/testing mode warning

Containerized VPS/CI environments frequently block netfilter capabilities even with root.
In those environments, validate command flow with `firewall.preferred_backend = "none"` and run full leak/firewall verification on real VMs with `CAP_NET_ADMIN`.

## Bug report checklist

Include all items below in issues:

1. Distro/version/kernel (`cat /etc/os-release`, `uname -a`).
2. Install method (source build or `.deb`) and exact package version.
3. Active config file (`/etc/anonsurf-rs/config.toml`) with sensitive values redacted.
4. Command sequence that failed and full JSON output (`anonsurf --json ...`).
5. Daemon logs (`anonsurf logs` and journal output if systemd is PID 1).
6. Firewall tooling availability (`nft --version`, `iptables --version`).
7. DNS stack details (`resolvectl status` / resolvconf setup / `/etc/resolv.conf` mode).
