#!/usr/bin/env bash
# Load the repository .env into the current shell.
# Usage from another script: source scripts/load-env.sh
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
env_file="$root/.env"

if [[ -f "$env_file" ]]; then
  set -a
  # shellcheck disable=SC1090
  source "$env_file"
  set +a
fi
