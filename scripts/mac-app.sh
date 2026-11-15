#!/usr/bin/env bash
# Builds a debug .app bundle and opens it so macOS grants permissions to the tracker itself.
set -euo pipefail
cd "$(dirname "$0")/.."

[ -f .env.local ] && set -a && . ./.env.local && set +a
export APPLE_SIGNING_IDENTITY="${APPLE_SIGNING_IDENTITY:--}"

APP="target/debug/bundle/macos/Classic EXP Tracker.app"
pkill -f "$APP/Contents/MacOS/" 2>/dev/null || true
pnpm tauri build --debug --bundles app
codesign -dv "$APP" 2>&1 | grep -E "Authority|Signature" || true
open "$APP"
