#!/usr/bin/env bash
set -euo pipefail

args=(bundle --target "$GUPI_TARGET")
if [[ "$GUPI_TARGET" == *-apple-darwin && "$GUPI_MACOS_SIGNING" == developer-id ]]; then
  args+=(
    --macos-signing developer-id
    --signing-identity "$GUPI_MACOS_SIGNING_IDENTITY"
    --notary-profile "$GUPI_MACOS_NOTARY_PROFILE"
    --keychain "$GUPI_MACOS_KEYCHAIN"
  )
fi
cargo run -p xtask --locked -- "${args[@]}"
