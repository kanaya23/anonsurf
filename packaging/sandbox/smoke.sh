#!/usr/bin/env bash
set -euo pipefail

if [[ "${EUID}" -ne 0 ]]; then
  echo "sandbox smoke must run as root inside the container" >&2
  exit 1
fi

cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
make install DESTDIR=/

if [[ ! -S /run/dbus/system_bus_socket ]]; then
  mkdir -p /run/dbus
  dbus-daemon --system --fork --nopidfile
fi

/usr/libexec/anonsurf-rs/anonsurfd &
daemon_pid=$!

cleanup() {
  set +e
  anonsurf --json repair >/tmp/anonsurf-repair.json 2>/tmp/anonsurf-repair.err
  kill "${daemon_pid}" 2>/dev/null
}
trap cleanup EXIT

sleep 2

anonsurf doctor
anonsurf --json status
anonsurf --json start
sleep 4
anonsurf --json status
anonsurf --json tor-check || true
anonsurf --json repair
anonsurf --json status
