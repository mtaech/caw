#!/bin/bash
# Development-only setup: install a hicolor icon and a .desktop entry so the
# GPUI window shows an icon in the dock / taskbar on Wayland (GNOME, KDE).
# It points at `cargo run`; for a real install, ship the release binary and
# update Exec in the desktop file.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

ICON_SRC="$ROOT/assets/icons/music.svg"
ICON_DEST="$HOME/.local/share/icons/hicolor/scalable/apps/caw.svg"
mkdir -p "$(dirname "$ICON_DEST")"
cp -f "$ICON_SRC" "$ICON_DEST"
echo "installed icon -> $ICON_DEST"
gtk-update-icon-cache -f "$HOME/.local/share/icons/hicolor" 2>/dev/null || true

DESKTOP_DEST="$HOME/.local/share/applications/cn.cyber-nest.caw.desktop"
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
