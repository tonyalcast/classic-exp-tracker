#!/usr/bin/env bash
# One-time fix: lets codesign use the signing key without a keychain password prompt on every build.
set -euo pipefail
cd "$(dirname "$0")/.."

[ -f .env.local ] && set -a && . ./.env.local && set +a
IDENTITY="${APPLE_SIGNING_IDENTITY:-}"
if [ -z "$IDENTITY" ] || [ "$IDENTITY" = "-" ]; then
  echo "APPLE_SIGNING_IDENTITY is not set to a certificate in .env.local; nothing to fix." >&2
  exit 1
fi

echo "Allowing codesign to use the private key \"$IDENTITY\" without prompting."
echo "macOS will ask for your login keychain password once."
security set-key-partition-list -S apple-tool:,apple:,codesign: -s -l "$IDENTITY" "$HOME/Library/Keychains/login.keychain-db" >/dev/null
echo "Done. Rebuilds signed with \"$IDENTITY\" no longer ask for the keychain password."
