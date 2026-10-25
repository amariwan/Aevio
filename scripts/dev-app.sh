#!/usr/bin/env bash
# Start the full Aevio app: Tauri shell + embedded Next.js dev server.
# Uses an isolated dev identifier so it never collides with your real app data
# or with other concurrent dev instances (see AGENTS.md).
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
exec bun tauri dev --config '{"identifier":"com.offlinestudios.aevio.dev"}' "$@"
