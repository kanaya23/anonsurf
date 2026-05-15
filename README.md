# anonsurf-rs

AnonSurf is being rebuilt as a distro-agnostic Rust service instead of the old
Parrot-specific Nim/Vala/bash stack.

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

## Runtime Paths

- Config: `/etc/anonsurf-rs/config.toml`
- Runtime: `/run/anonsurf-rs/`
- State/snapshots: `/var/lib/anonsurf-rs/`

The daemon uses a private Tor instance and never rewrites `/etc/tor/torrc`.
Firewall changes are isolated to an `inet anonsurf_rs` nftables table or tagged
iptables chains. `anonsurf repair` restores the recorded DNS snapshot and removes
only anonsurf-rs managed firewall state.

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
