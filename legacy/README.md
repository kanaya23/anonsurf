# Legacy implementation

This directory keeps the pre-Rust AnonSurf implementation for reference while
`anonsurf-rs` replaces the active build and packaging path.

The contents here are not installed by the new Makefile or Debian package:

- `src/`: old Nim/Vala CLI and GTK code.
- `scripts/`: old shell daemon helpers.
- `sys-units/`: old systemd unit.
- `launchers/`: old desktop launchers.
- `torrc.base`: old system Tor config template.
