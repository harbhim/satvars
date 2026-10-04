#!/usr/bin/env bash
# Publish the current version to crates.io and PyPI from this machine.
# Reads CARGO_REGISTRY_TOKEN and MATURIN_PYPI_TOKEN from .env.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

# shellcheck disable=SC1091
source "$root/scripts/load-env.sh"

if [[ -z "${CARGO_REGISTRY_TOKEN:-}" ]]; then
  echo "Set CARGO_REGISTRY_TOKEN in .env. Copy .env.example to .env and paste the crates.io token there." >&2
  exit 1
fi

if [[ -z "${MATURIN_PYPI_TOKEN:-}" ]]; then
  echo "Set MATURIN_PYPI_TOKEN in .env. Copy .env.example to .env and paste the PyPI token there." >&2
  exit 1
fi

version="$(bash "$root/scripts/release-version.sh")"
bash "$root/scripts/publish-crates.sh"

if [[ ! -x "$root/.venv/bin/maturin" ]]; then
  python3 -m venv "$root/.venv"
  "$root/.venv/bin/pip" install 'maturin>=1.8,<2'
fi

echo "Publishing Python package satva $version"
"$root/.venv/bin/maturin" publish --skip-existing -m crates/satva-python/Cargo.toml
echo "Published satva $version"
