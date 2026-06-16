#!/bin/bash
# Caw development setup: install icons and desktop file so the app
# shows an icon in the dock / taskbar under Wayland (GNOME, KDE).
# On Wayland, window icons are resolved via .desktop file matching,
# not via GTK's gtk_window_set_icon(). This script creates the
# required symlinks for `cargo tauri dev`.

set -euo pipefail

# Resolve the project root (parent of scripts/)
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# ── 1. Install icon into the system theme ──
ICON_SRC="$ROOT/src-tauri/icons/icon.png"
ICON_DEST="$HOME/.local/share/icons/hicolor/128x128/apps/caw.png"

mkdir -p "$(dirname "$ICON_DEST")"
if [ ! -f "$ICON_DEST" ]; then
  cp "$ICON_SRC" "$ICON_DEST"
  echo "installed icon -> $ICON_DEST"
else
  echo "icon already exists: $ICON_DEST"
fi

# Update icon cache (harmless if hicolor theme doesn't exist)
gtk-update-icon-cache -f ~/.local/share/icons/hicolor 2>/dev/null || true

# ── 2. Install .desktop file ──
DESKTOP_DEST="$HOME/.local/share/applications/cn.cyber-nest.caw.desktop"

cat > "$DESKTOP_DEST" << EOF
[Desktop Entry]
Type=Application
Name=Caw (dev)
Comment=Local music player (development)
Exec=$ROOT/scripts/run-dev.sh
Icon=$ROOT/src-tauri/icons/icon.png
Terminal=false
Categories=Audio;Music;Player;
StartupWMClass=Caw
EOF
echo "installed .desktop -> $DESKTOP_DEST"

echo ""
echo "Done. Restart GNOME Shell (Alt+F2, r) or relogin, then launch with:"
echo "  cd $ROOT && cargo tauri dev"
