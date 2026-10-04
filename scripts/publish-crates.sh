#!/usr/bin/env bash
# Publish the Rust crates to crates.io in dependency order.
# Skips a crate when that exact version is already on crates.io.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

# shellcheck disable=SC1091
source "$root/scripts/load-env.sh"

version="$(bash "$root/scripts/release-version.sh")"
echo "Publishing Rust crates $version"

crates=(
  satva-types
  satva-expr
  satva-parser
  satva-arrow
  satva-core
  satva-io
  satva-execution
  satva-runner
  satva-cli
)

for crate in "${crates[@]}"; do
  code="$(curl -s -o /dev/null -w "%{http_code}" -A "satva-release (github.com/harbhim/satvars)" \
    "https://crates.io/api/v1/crates/${crate}/${version}")"
  if [[ "$code" == "200" ]]; then
    echo "$crate $version is already on crates.io"
    continue
  fi
  echo "Publishing $crate $version"
  cargo publish -p "$crate"
done
