# Copilot instructions for `anonsurf-rs`

## Build, test, and lint commands

- Toolchain bootstrap (recommended on Ubuntu/Debian): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --component rustfmt,clippy`
- Load toolchain in shell: `. "$HOME/.cargo/env"`
- Build workspace (debug): `cargo build --workspace`
- Build workspace (release): `cargo build --release --workspace` (also `make build`)
- Full test suite: `cargo test --workspace` (also `make test`)
- Lint (warnings are errors): `cargo clippy --workspace --all-targets -- -D warnings`
- Format check: `cargo fmt --all -- --check`
- Install smoke check without touching host system paths: `make install DESTDIR=/tmp/anonsurf-install-check`
- Containerized end-to-end smoke (Tor/firewall/DNS in container namespace): `tools/sandbox-smoke.sh`

Run a single test:

- By crate + test name: `cargo test -p anonsurf-core transition_rejects_nested_operation -- --exact`
- Pattern-based in one crate: `cargo test -p anonsurf-dns apply_and_repair`

## High-level architecture

This is a Rust workspace split by responsibility:

- `anonsurf-core`: shared domain model and contracts (status/state/config/path/snapshot types, D-Bus constants).
- `anonsurf-daemon` (`anonsurfd`): privileged system D-Bus service orchestrating start/stop/restart/repair/config updates.
- `anonsurf-cli` (`anonsurf`): unprivileged clap client calling daemon D-Bus methods and rendering JSON/human output.
- `anonsurf-gui` (`anonsurf-gui`): GTK/libadwaita D-Bus client for the same daemon operations.
- `anonsurf-tor`, `anonsurf-dns`, `anonsurf-firewall`: side-effect modules that apply/repair Tor, DNS, and firewall state.

Operational flow:

1. CLI/GUI calls daemon methods over system D-Bus (`org.anonsurf.rs1`).
2. Daemon authorizes mutating actions via Polkit action `org.anonsurf.rs1.manage`.
3. Start path creates runtime/state dirs, starts a **private Tor instance** from generated torrc, applies DNS + firewall changes, then stores a snapshot.
4. Repair/stop path restores from snapshot and removes only AnonSurf-managed networking state.

Runtime path model (from `anonsurf-core::Paths`):

- Config: `/etc/anonsurf-rs/config.toml`
- Runtime: `/run/anonsurf-rs/`
- State/snapshot: `/var/lib/anonsurf-rs/` (`snapshot.json`)

## Key codebase conventions

- Keep privileged networking logic in daemon + subsystem crates; CLI/GUI should remain D-Bus clients.
- Keep GUI API usage compatible with GTK4 `v4_6` and libadwaita `v1_1` (Ubuntu 22.04 / Debian-family compatibility floor).
- D-Bus methods currently return JSON `String` payloads (typed structs serialized/deserialized at boundaries), not typed zbus interfaces.
- Reuse `anonsurf-core` shared types/constants (`Status`, `CommandOutcome`, `Config`, `Paths`, `DBUS_*`) across crates instead of duplicating schemas.
- Preserve isolation guarantees:
  - do not mutate `/etc/tor/torrc` (use generated private torrc under runtime dir),
  - firewall rules must stay scoped to AnonSurf-owned nftables table / tagged iptables chains,
  - repair should only clean up AnonSurf-managed changes.
- Maintain backend fallback behavior:
  - firewall prefers nftables then iptables,
  - DNS backend detection supports systemd-resolved, resolvconf, or resolv.conf file mode.
- Tests should avoid host networking mutation; follow existing temp-file/unit style and use container smoke for real network/firewall behavior.
- Legacy tree under `legacy/` is reference only and not part of current install/build path.
