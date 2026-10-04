#!/usr/bin/env bash
# Confirm Cargo.toml, the internal crate dependencies, and pyproject.toml
# all use one version. When RELEASE_TAG or RELEASE_INPUT is set, it must match.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

crate_version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -1)"
py_version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' crates/satva-python/pyproject.toml)"

if [[ -z "$crate_version" || -z "$py_version" ]]; then
  echo "Could not read the package version." >&2
  exit 1
fi

if [[ "$crate_version" != "$py_version" ]]; then
  echo "Cargo.toml is $crate_version but crates/satva-python/pyproject.toml is $py_version." >&2
  exit 1
fi

while IFS= read -r dep_version; do
  if [[ "$dep_version" != "$crate_version" ]]; then
    echo "A workspace dependency is $dep_version, package version is $crate_version." >&2
    echo "Update every satva-* version in Cargo.toml before releasing." >&2
    exit 1
  fi
done < <(sed -n 's/^satva-[a-z0-9-]* = { version = "\([^"]*\)".*/\1/p' Cargo.toml)

if [[ -n "${RELEASE_TAG:-}" ]]; then
  tag="${RELEASE_TAG#v}"
  if [[ "$tag" != "$crate_version" ]]; then
    echo "Tag $RELEASE_TAG does not match package version $crate_version." >&2
    exit 1
  fi
fi

if [[ -n "${RELEASE_INPUT:-}" && "$RELEASE_INPUT" != "$crate_version" ]]; then
  echo "Entered version $RELEASE_INPUT does not match package version $crate_version." >&2
  exit 1
fi

echo "$crate_version"
