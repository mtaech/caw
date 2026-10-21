#!/bin/bash
# Development-only setup: install a hicolor icon and a .desktop entry so the
# GPUI window shows an icon in the dock / taskbar on Wayland (GNOME, KDE).
# It points at `cargo run`; for a real install, ship the release binary and
# update Exec in the desktop file.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

ICON_SVG_SRC="$ROOT/assets/icons/caw.svg"
ICON_SVG_DEST="$HOME/.local/share/icons/hicolor/scalable/apps/caw.svg"
ICON_PNG_SRC="$ROOT/assets/icons/caw.png"
ICON_PNG_DEST="$HOME/.local/share/icons/hicolor/256x256/apps/caw.png"
mkdir -p "$(dirname "$ICON_SVG_DEST")" "$(dirname "$ICON_PNG_DEST")"
cp -f "$ICON_SVG_SRC" "$ICON_SVG_DEST"
cp -f "$ICON_PNG_SRC" "$ICON_PNG_DEST"
echo "installed icons -> $ICON_SVG_DEST, $ICON_PNG_DEST"
gtk-update-icon-cache -f "$HOME/.local/share/icons/hicolor" 2>/dev/null || true

DESKTOP_DEST="$HOME/.local/share/applications/caw.desktop"
# Older dev setups used a reverse-DNS filename; the Wayland app_id is `caw`, so the
# desktop entry must match it for taskbar/dock icons to resolve.
rm -f "$HOME/.local/share/applications/cn.cyber-nest.caw.desktop"
mkdir -p "$(dirname "$DESKTOP_DEST")"
cat > "$DESKTOP_DEST" <<EOF
[Desktop Entry]
Type=Application
Name=Caw
Comment=Local music player (GPUI)
Exec=$ROOT/scripts/run-dev.sh
Icon=caw
Terminal=false
Categories=AudioVideo;Audio;Player;
StartupWMClass=caw
EOF
echo "installed desktop entry -> $DESKTOP_DEST"
# Refresh KDE's desktop-file and icon caches so the new entry/icon is picked up.
kbuildsycoca6 --noincremental 2>/dev/null || true
