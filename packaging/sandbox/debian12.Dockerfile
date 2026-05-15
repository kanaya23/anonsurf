FROM debian:12

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    ca-certificates \
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
    tor \
  && rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    sh -s -- -y --profile minimal --component rustfmt,clippy

ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /workspace/anonsurf
COPY . .

CMD ["bash", "packaging/sandbox/smoke.sh"]
