#!/usr/bin/env bash
# Publish site/ to the directory the launchd agent serves from, then restart it.
#
# The agent cannot read the repo directly: ~/Documents is TCC-protected on macOS
# and launchd-spawned processes are denied. Publishing a snapshot also keeps .git
# and work-in-progress files off the public site.
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
dest="${WIDJIRA_ROOT:-$HOME/Library/Application Support/widjira}"
label="com.isemi.widjira-site"
domain="gui/$(id -u)"

mkdir -p "$dest"
rsync -a --delete "$repo/site/" "$dest/site/"
install -m 755 "$repo/scripts/serve_site.py" "$dest/serve_site.py"

# Pages are read per request, so a restart only matters when serve_site.py changed.
if launchctl print "$domain/$label" >/dev/null 2>&1; then
  launchctl kickstart -k "$domain/$label"
else
  launchctl bootstrap "$domain" "$HOME/Library/LaunchAgents/$label.plist"
fi

echo "published $repo/site -> $dest/site"
