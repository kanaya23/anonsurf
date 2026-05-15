#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
engine="${ANONSURF_CONTAINER_ENGINE:-}"

if [[ -z "${engine}" ]]; then
  if command -v podman >/dev/null 2>&1; then
    engine=podman
  elif command -v docker >/dev/null 2>&1; then
    engine=docker
  else
    echo "Neither podman nor docker was found." >&2
    exit 1
  fi
fi

image="${ANONSURF_SANDBOX_IMAGE:-anonsurf-rs-debian12-smoke}"

engine_cmd=("${engine}")
if [[ "${engine}" == "podman" ]]; then
  if [[ -z "${XDG_RUNTIME_DIR:-}" || ! -w "${XDG_RUNTIME_DIR}" ]]; then
    export XDG_RUNTIME_DIR="/tmp/anonsurf-podman-run"
  fi
  if [[ ! -w "${HOME:-/}" ]]; then
    export HOME="/tmp/anonsurf-podman-home"
  fi
  export XDG_CACHE_HOME="${XDG_CACHE_HOME:-/tmp/anonsurf-podman-cache}"
  export TMPDIR="${TMPDIR:-/tmp}"
  mkdir -p "${XDG_RUNTIME_DIR}"
  mkdir -p "${HOME}" "${XDG_CACHE_HOME}"
  engine_cmd=(
    podman
    --root "${ANONSURF_PODMAN_ROOT:-/tmp/anonsurf-podman-root}"
    --runroot "${ANONSURF_PODMAN_RUNROOT:-/tmp/anonsurf-podman-runroot}"
  )
fi

cd "${repo_root}"
"${engine_cmd[@]}" build -f packaging/sandbox/debian12.Dockerfile -t "${image}" .
"${engine_cmd[@]}" run --rm \
  --cap-add=NET_ADMIN \
  --cap-add=NET_RAW \
  --tmpfs /run \
  --tmpfs /tmp \
  "${image}"
