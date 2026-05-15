# Test Strategy

The Rust test suite is split into two safety levels.

## Safe Local Tests

These never mutate host networking:

```sh
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --release --workspace
make install DESTDIR=/tmp/anonsurf-install-check
```

DNS integration tests redirect resolver writes to temporary files. Firewall tests
exercise generated rules, not the host firewall.

## Container Smoke

For real Tor/firewall/DNS behavior, run the container harness:

```sh
tools/sandbox-smoke.sh
```

The harness builds a Debian 12 image, installs AnonSurf inside the container,
starts a container-local system D-Bus, starts `anonsurfd`, runs start/status/
tor-check/repair through the CLI, and mutates only the container network
namespace. It requires Docker or Podman and `NET_ADMIN` inside the container.
