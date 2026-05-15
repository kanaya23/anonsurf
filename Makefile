PREFIX ?= /usr
SYSCONFDIR ?= /etc
LIBEXECDIR ?= $(PREFIX)/libexec/anonsurf-rs
SYSTEMDUNITDIR ?= /lib/systemd/system
DBUSPOLICYDIR ?= $(PREFIX)/share/dbus-1/system.d
POLKITDIR ?= $(PREFIX)/share/polkit-1/actions
APPLICATIONDIR ?= $(PREFIX)/share/applications
ICONDIR ?= $(PREFIX)/share/icons/hicolor/256x256/apps

.PHONY: all build test install clean

all: build

build:
	cargo build --release --workspace

test:
	cargo test --workspace

install:
	@for bin in target/release/anonsurf target/release/anonsurf-gui target/release/anonsurfd; do \
		if [ ! -x $$bin ]; then \
			echo "missing $$bin; run 'make build' as your user, then rerun 'sudo make install'"; \
			exit 1; \
		fi; \
	done
	@if find Cargo.toml Cargo.lock crates -type f -newer target/release/anonsurf | grep -q .; then \
		echo "release binaries are outdated; run 'make build' as your user, then rerun 'sudo make install'"; \
		exit 1; \
	fi
	install -Dm755 target/release/anonsurf $(DESTDIR)$(PREFIX)/bin/anonsurf
	install -Dm755 target/release/anonsurf-gui $(DESTDIR)$(PREFIX)/bin/anonsurf-gui
	install -Dm755 target/release/anonsurfd $(DESTDIR)$(LIBEXECDIR)/anonsurfd
	install -Dm644 packaging/systemd/anonsurfd.service $(DESTDIR)$(SYSTEMDUNITDIR)/anonsurfd.service
	install -Dm644 packaging/dbus/org.anonsurf.rs1.conf $(DESTDIR)$(DBUSPOLICYDIR)/org.anonsurf.rs1.conf
	install -Dm644 packaging/polkit/org.anonsurf.rs1.policy $(DESTDIR)$(POLKITDIR)/org.anonsurf.rs1.policy
	install -Dm644 packaging/desktop/org.anonsurf.rs1.desktop $(DESTDIR)$(APPLICATIONDIR)/org.anonsurf.rs1.desktop
	install -Dm644 packaging/completions/anonsurf.bash $(DESTDIR)$(PREFIX)/share/bash-completion/completions/anonsurf
	install -Dm644 packaging/completions/_anonsurf $(DESTDIR)$(PREFIX)/share/zsh/vendor-completions/_anonsurf
	install -Dm644 packaging/completions/anonsurf.fish $(DESTDIR)$(PREFIX)/share/fish/vendor_completions.d/anonsurf.fish
	install -Dm644 packaging/config/config.toml $(DESTDIR)$(SYSCONFDIR)/anonsurf-rs/config.toml
	install -Dm644 configs/bridges.txt $(DESTDIR)$(SYSCONFDIR)/anonsurf-rs/bridges.txt
	install -Dm644 configs/onion.pac $(DESTDIR)$(SYSCONFDIR)/anonsurf-rs/onion.pac
	install -Dm644 icons/anonsurf.png $(DESTDIR)$(ICONDIR)/anonsurf.png

clean:
	cargo clean
