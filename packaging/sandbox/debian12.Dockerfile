FROM debian:12

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    ca-certificates \
    cargo \
    curl \
    dbus \
    iproute2 \
    iptables \
    libadwaita-1-dev \
    libgtk-4-dev \
    nftables \
    obfs4proxy \
    pkg-config \
    policykit-1 \
    rustc \
    tor \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace/anonsurf
COPY . .

CMD ["bash", "packaging/sandbox/smoke.sh"]
