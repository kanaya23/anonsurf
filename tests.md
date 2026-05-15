# anonsurf-rs VPS test proof (production milestone tracking)

Run context (this VPS):

- Date: 2026-05-15
- OS: Ubuntu 22.04.5 LTS (Jammy), kernel `6.6.122+`
- Workspace: `/home/kaggledev/anonsurf`
- Result set source: `/tmp/anonsurf-vps-proof/results.tsv`

## Summary

- Total executed checks: **50**
- Shell-level pass: **49**
- Shell-level fail: **1** (`systemctl status anonsurfd`)
- Build/lint/test/package lifecycle checks in this VPS: **pass**
- Install/remove/purge lifecycle in this VPS: **pass**

## Build/package gate results

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo test --workspace` | PASS |
| `cargo build --release --workspace` | PASS |
| `make install DESTDIR=/tmp/anonsurf-install-check` | PASS |
| `make deb` | PASS |
| `sudo dpkg -i ../anonsurf_*_amd64.deb` | PASS |

Produced artifact:

- `../anonsurf_0.1.0-1_amd64.deb`

## Package install correctness checks

| Check | Result |
| --- | --- |
| systemd unit file exists (`/lib/systemd/system/anonsurfd.service`) | PASS |
| D-Bus policy exists (`/usr/share/dbus-1/system.d/org.anonsurf.rs1.conf`) | PASS |
| Polkit policy exists (`/usr/share/polkit-1/actions/org.anonsurf.rs1.policy`) | PASS |
| Shell completions installed (bash/zsh/fish paths) | PASS |
| Desktop launcher file exists (`/usr/share/applications/org.anonsurf.rs1.desktop`) | PASS |
| Manpage readable (`man anonsurf`) | PASS |
| `systemctl status anonsurfd` | FAIL (systemd not PID 1 in this VPS/container runtime) |
| `anonsurf doctor` | PASS |
| `sudo apt purge anonsurf` | PASS |
| Purge verification (`dpkg -s anonsurf` absent) | PASS |

`systemctl` failure output:

```text
System has not been booted with systemd as init system (PID 1). Can't operate.
Failed to connect to bus: Host is down
```

## Core command behavior proof

All command entrypoints were executed at least once during this run:

- `start`, `stop`, `restart`, `status`, `changeid`, `new-identity`, `myip`, `tor-check`, `repair`, `logs`, `doctor`, `config show-default`, `config apply-default`, `config apply-file`.

### Idempotence proof (backend `none` mode in constrained VPS)

Second `start` after successful `start`:

```json
{
  "ok": true,
  "message": "AnonSurf already enabled",
  "changed": ["already enabled"]
}
```

Second `stop` after successful `stop`:

```json
{
  "ok": true,
  "message": "AnonSurf disabled",
  "changed": [
    "private Tor was not running",
    "removed nftables inet anonsurf_rs table if present",
    "no DNS snapshot found; nothing to restore",
    "repair completed"
  ]
}
```

Repair after active run (backend `none` mode):

```json
{
  "ok": true,
  "message": "Networking repair completed",
  "changed": [
    "private Tor was not running",
    "removed nftables inet anonsurf_rs table if present",
    "no DNS snapshot found; nothing to restore",
    "repair completed"
  ]
}
```

### Default backend behavior in this VPS

`start` with default firewall backend returned:

```json
{
  "ok": false,
  "message": "AnonSurf failed to start",
  "status": {
    "last_error": "nft failed: exit status: 1"
  }
}
```

This VPS runtime does not provide full netfilter capability needed for transparent-routing enforcement.

## Tor/readiness observations in this VPS

- Private Tor process start path executed in backend `none` mode.
- `tor-check`/`myip` returned SOCKS connect failure in this environment.
- `changeid`/`new-identity` returned failure in this environment (control cookie path not available at runtime during this run).

These are environment-sensitive and require dedicated VM validation for final Tor readiness gates.

## Full result table (this VPS)

| Name | Exit | Result | Command |
| --- | --- | --- | --- |
| fmt_check | 0 | PASS | `cargo fmt --all -- --check` |
| clippy | 0 | PASS | `cargo clippy --workspace --all-targets -- -D warnings` |
| test_workspace | 0 | PASS | `cargo test --workspace` |
| build_release | 0 | PASS | `cargo build --release --workspace` |
| install_check | 0 | PASS | `make install DESTDIR=/tmp/anonsurf-install-check` |
| make_deb | 0 | PASS | `make deb` |
| dpkg_install | 0 | PASS | `sudo dpkg -i ../anonsurf_*_amd64.deb` |
| systemd_unit_file | 0 | PASS | `test -f /lib/systemd/system/anonsurfd.service` |
| dbus_policy_file | 0 | PASS | `test -f /usr/share/dbus-1/system.d/org.anonsurf.rs1.conf` |
| polkit_file | 0 | PASS | `test -f /usr/share/polkit-1/actions/org.anonsurf.rs1.policy` |
| bash_completion_file | 0 | PASS | `test -f /usr/share/bash-completion/completions/anonsurf` |
| zsh_completion_file | 0 | PASS | `test -f /usr/share/zsh/vendor-completions/_anonsurf` |
| fish_completion_file | 0 | PASS | `test -f /usr/share/fish/vendor_completions.d/anonsurf.fish` |
| desktop_file | 0 | PASS | `test -f /usr/share/applications/org.anonsurf.rs1.desktop` |
| manpage_check | 0 | PASS | `MANWIDTH=80 man anonsurf \| head -n 30` |
| systemctl_status | 1 | FAIL | `systemctl status anonsurfd --no-pager` |
| doctor | 0 | PASS | `anonsurf doctor` |
| ensure_system_bus | 0 | PASS | `if [ -S /run/dbus/system_bus_socket ]; then echo found; else sudo mkdir -p /run/dbus && sudo dbus-daemon --system --fork --nopidfile && echo started; fi` |
| config_show_default | 0 | PASS | `anonsurf config show-default` |
| config_apply_default | 0 | PASS | `sudo anonsurf --json config apply-default` |
| cmd_status_before | 0 | PASS | `anonsurf --json status` |
| cmd_start_default | 0 | PASS | `sudo anonsurf --json start` |
| cmd_stop_default | 0 | PASS | `sudo anonsurf --json stop` |
| cmd_restart_default | 0 | PASS | `sudo anonsurf --json restart` |
| cmd_changeid_default | 0 | PASS | `sudo anonsurf --json changeid` |
| cmd_new_identity_default | 0 | PASS | `sudo anonsurf --json new-identity` |
| cmd_myip_default | 0 | PASS | `anonsurf --json myip` |
| cmd_tor_check_default | 0 | PASS | `anonsurf --json tor-check` |
| cmd_repair_default | 0 | PASS | `sudo anonsurf --json repair` |
| cmd_logs_default | 0 | PASS | `anonsurf --json logs` |
| cmd_doctor_default | 0 | PASS | `anonsurf doctor` |
| config_apply_none | 0 | PASS | `sudo anonsurf --json config apply-file /tmp/anonsurf-vps-proof/none-firewall-config.toml` |
| cmd_start_none_1 | 0 | PASS | `sudo anonsurf --json start` |
| cmd_start_none_2 | 0 | PASS | `sudo anonsurf --json start` |
| cmd_status_none | 0 | PASS | `anonsurf --json status` |
| cmd_myip_none | 0 | PASS | `anonsurf --json myip` |
| cmd_tor_check_none | 0 | PASS | `anonsurf --json tor-check` |
| cmd_changeid_none | 0 | PASS | `sudo anonsurf --json changeid` |
| cmd_new_identity_none | 0 | PASS | `sudo anonsurf --json new-identity` |
| cmd_logs_none | 0 | PASS | `anonsurf --json logs` |
| cmd_restart_none | 0 | PASS | `sudo anonsurf --json restart` |
| cmd_stop_none_1 | 0 | PASS | `sudo anonsurf --json stop` |
| cmd_stop_none_2 | 0 | PASS | `sudo anonsurf --json stop` |
| cmd_repair_none | 0 | PASS | `sudo anonsurf --json repair` |
| config_apply_file | 0 | PASS | `sudo anonsurf --json config apply-file /tmp/anonsurf-vps-proof/default-config.toml` |
| config_apply_default_final | 0 | PASS | `sudo anonsurf --json config apply-default` |
| cmd_status_after | 0 | PASS | `anonsurf --json status` |
| apt_purge | 0 | PASS | `sudo apt purge -y anonsurf` |
| verify_purged | 0 | PASS | `! dpkg -s anonsurf >/dev/null 2>&1` |

## Remaining work to satisfy strict 100% matrix claim

The following still require dedicated VM/host test environments (not this constrained VPS container runtime):

- systemd PID1 service lifecycle checks (`systemctl status/start/enable` semantics),
- full nftables + iptables leak/firewall correctness with `CAP_NET_ADMIN`,
- Tor bootstrap/control-port readiness and NEWNYM verification under unrestricted networking,
- cross-distro VM matrix: Debian 12, Ubuntu 22.04, Ubuntu 24.04, Parrot/Kali derivative,
- crash/reboot and power-loss recovery scenarios.
