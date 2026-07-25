#!/bin/bash
# Build Mac App Store + upload len App Store Connect (TestFlight).
#
# Pipeline: tauri build (khong private API, sandbox-ready) -> nhung provisioning
# profile -> re-sign voi entitlements sandbox -> productbuild .pkg (cert
# Installer) -> altool upload bang API key.
#
# Yeu cau tren may:
#   - Keychain co "Apple Distribution: ISEMI COMPANY LIMITED (MVZQJ4M3LF)"
#     va "3rd Party Mac Developer Installer: ..." (da Always Allow cho codesign/
#     productbuild — lan dau se hien hop thoai).
#   - ~/.appstoreconnect/private_keys/AuthKey_883W4Z3Z99.p8 + CLI `asc` co
#     profile de lay issuer id.
#   - src-tauri/Jiraw_MAS.provisionprofile (profile MAC_APP_STORE, tao qua API).
#
# Luu y: MACOSX_DEPLOYMENT_TARGET=12.0 la BAT BUOC — build arm64-only bi Apple
# tu choi (90869) neu target < 12.0. Man hinh phai thuc khi ky (dark wake lam
# Security tu choi ky: CSSMERR_CSP_IN_DARK_WAKE) — vi the co caffeinate.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
export PATH="$HOME/.cargo/bin:$PATH"
export MACOSX_DEPLOYMENT_TARGET=12.0
DIST='Apple Distribution: ISEMI COMPANY LIMITED (MVZQJ4M3LF)'
INST='3rd Party Mac Developer Installer: ISEMI COMPANY LIMITED (MVZQJ4M3LF)'
APP="src-tauri/target/release/bundle/macos/Widget for Jira.app"
PKG="src-tauri/target/release/bundle/JiraW-mas.pkg"

echo "=== [1/5] tauri build (MAS config, khong private API)"
[ -d node_modules ] || yarn install --silent
./node_modules/.bin/tauri build --config src-tauri/tauri.mas.conf.json -- --no-default-features

echo "=== [2/5] nhung profile + re-sign entitlements sandbox"
cp src-tauri/Jiraw_MAS.provisionprofile "$APP/Contents/embedded.provisionprofile"
caffeinate -dims codesign --force --timestamp=none \
  --entitlements src-tauri/entitlements.mas.plist --sign "$DIST" "$APP"
codesign --verify --deep --strict "$APP"

echo "=== [3/5] soi binary sach private API"
if strings "$APP/Contents/MacOS/jira-widget" | grep -qE "drawsTransparentBackground|_setDrawsBackground"; then
  echo "LOI: con chuoi private API trong binary"; exit 1
fi

echo "=== [4/5] productbuild .pkg"
rm -f "$PKG"
caffeinate -dims productbuild --component "$APP" /Applications --sign "$INST" "$PKG"

echo "=== [5/5] upload App Store Connect"
ISS=$(asc auth issuer-id | tr -d '[:space:]')
caffeinate -dims xcrun altool --upload-app -f "$PKG" -t macos \
  --apiKey 883W4Z3Z99 --apiIssuer "$ISS"
echo "=== XONG — cho ASC processing roi gan build vao version."
