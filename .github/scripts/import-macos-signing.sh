#!/usr/bin/env bash
set +x
set -euo pipefail
umask 077

for name in MACOS_SIGNING_IDENTITY MACOS_CERTIFICATE_BASE64 MACOS_CERTIFICATE_PASSWORD \
  MACOS_NOTARY_API_KEY_BASE64 MACOS_NOTARY_KEY_ID MACOS_NOTARY_ISSUER_ID; do
  if [[ -z "${!name:-}" ]]; then
    printf 'Required signing configuration is missing: %s\n' "$name" >&2
    exit 1
  fi
done
if [[ "$MACOS_SIGNING_IDENTITY" != "Developer ID Application: "* || "$MACOS_SIGNING_IDENTITY" == *$'\n'* ]]; then
  printf '%s\n' 'MACOS_SIGNING_IDENTITY must name a Developer ID Application certificate.' >&2
  exit 1
fi

keychain="$RUNNER_TEMP/gupi-signing.keychain-db"
certificate="$RUNNER_TEMP/gupi-certificate.p12"
api_key="$RUNNER_TEMP/gupi-notary-key.p8"
keychain_password="$(openssl rand -base64 32)"
printf '%s' "$MACOS_CERTIFICATE_BASE64" | base64 --decode -o "$certificate"
printf '%s' "$MACOS_NOTARY_API_KEY_BASE64" | base64 --decode -o "$api_key"
chmod 600 "$certificate" "$api_key"
security create-keychain -p "$keychain_password" "$keychain"
security set-keychain-settings -lut 21600 "$keychain"
security unlock-keychain -p "$keychain_password" "$keychain"
security import "$certificate" -k "$keychain" -P "$MACOS_CERTIFICATE_PASSWORD" -T /usr/bin/codesign
security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k "$keychain_password" "$keychain"

# Store the team API key in this job's temporary keychain, validating credentials.
xcrun notarytool store-credentials gupi-release \
  --key "$api_key" --key-id "$MACOS_NOTARY_KEY_ID" --issuer "$MACOS_NOTARY_ISSUER_ID" \
  --keychain "$keychain"
rm -f "$certificate" "$api_key"
{
  printf 'GUPI_MACOS_KEYCHAIN=%s\n' "$keychain"
  printf '%s\n' 'GUPI_MACOS_NOTARY_PROFILE=gupi-release'
  printf 'GUPI_MACOS_SIGNING_IDENTITY=%s\n' "$MACOS_SIGNING_IDENTITY"
} >> "$GITHUB_ENV"
