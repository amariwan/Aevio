#!/usr/bin/env bash
# Source installation of only the `aevio` CLI, for platforms without a release
# download. No Tauri/Swift build, GUI bundle, launch agent, or shell config
# changes. aevio finds the installed app when it needs it.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
exec cargo install --locked --path apps/cli "$@"
