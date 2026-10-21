#!/usr/bin/env bash
# Build Linux packages for Caw: .deb, .rpm, .AppImage and a portable tarball.
#
# Usage:
#   ./scripts/package.sh [deb|rpm|appimage|tarball|all]   (default: all)
#
# Tools:
#   cargo          always required
#   dpkg-deb       for .deb   (Debian/Ubuntu: dpkg-dev)
#   rpmbuild       for .rpm   (Fedora: rpm-build, Debian/Ubuntu: rpm)
#   appimagetool   for .AppImage
#                  https://github.com/AppImage/appimagetool/releases
#                  (set APPIMAGETOOL=/path/to/appimagetool if it is not on PATH)
#
# Missing tools are reported and that format is skipped; the script exits
# non-zero only when nothing requested could be built.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

TARGET="${1:-all}"
VERSION="$(awk -F'"' '/^version = /{print $2; exit}' Cargo.toml)"
ARCH_RAW="$(uname -m)"
case "$ARCH_RAW" in
  x86_64|amd64)  DEB_ARCH=amd64;  RPM_ARCH=x86_64;  APPIMAGE_ARCH=x86_64 ;;
  aarch64|arm64) DEB_ARCH=arm64;  RPM_ARCH=aarch64; APPIMAGE_ARCH=aarch64 ;;
  *)             DEB_ARCH="$ARCH_RAW"; RPM_ARCH="$ARCH_RAW"; APPIMAGE_ARCH="$ARCH_RAW" ;;
esac

DIST="$ROOT/dist"
PKGROOT="$DIST/pkgroot"
MAINTAINER="Caw contributors"
SUMMARY="Local music player built with GPUI"
DESCRIPTION="Caw is a local music player with a GPUI (gpui-kit) interface, Symphonia decoding, cpal output, SQLite playlists and MPRIS/tray integration on Linux."

say() { printf "==> %s\n" "$*"; }
skip() { printf "!!! %s\n" "$*" >&2; }
need() { command -v "$1" >/dev/null 2>&1; }

# SKIP_BUILD=1 reuses an existing target/release/caw (used by CI/containers).
if [ "${SKIP_BUILD:-0}" = "1" ]; then
  say "Reusing existing release binary (SKIP_BUILD=1)"
else
  say "Building release binary (cargo build --release --locked)"
  cargo build --release --locked
fi
BIN="$ROOT/target/release/caw"
if [ ! -x "$BIN" ]; then
  echo "release binary not found: $BIN (run without SKIP_BUILD)" >&2
  exit 1
fi

say "Staging package tree"
# Keep previously built artifacts (dist/*.deb etc.); only reset staging dirs.
rm -rf "$PKGROOT" "$DIST/deb" "$DIST/rpmbuild" "$DIST/AppDir"
mkdir -p "$DIST"
mkdir -p "$PKGROOT/usr/bin" \
         "$PKGROOT/usr/share/applications" \
         "$PKGROOT/usr/share/icons/hicolor/scalable/apps" \
         "$PKGROOT/usr/share/icons/hicolor/256x256/apps" \
         "$PKGROOT/usr/share/doc/caw"
# Install with explicit modes: the repo checkout may be 0600/0775 (umask, editor),
# and a non world-readable caw.desktop makes the app menu entry unusable.
install -m 0755 "$BIN" "$PKGROOT/usr/bin/caw"
strip "$PKGROOT/usr/bin/caw" 2>/dev/null || true
install -m 0644 "$ROOT/packaging/caw.desktop" "$PKGROOT/usr/share/applications/caw.desktop"
install -m 0644 "$ROOT/assets/icons/caw.svg" "$PKGROOT/usr/share/icons/hicolor/scalable/apps/caw.svg"
install -m 0644 "$ROOT/assets/icons/caw.png" "$PKGROOT/usr/share/icons/hicolor/256x256/apps/caw.png"
install -m 0644 "$ROOT/README.md" "$ROOT/PLAN.md" "$PKGROOT/usr/share/doc/caw/"
INSTALLED_KB="$(du -sk "$PKGROOT/usr" | cut -f1)"

build_tarball() {
  say "Building portable tarball"
  local name="caw-${VERSION}-linux-${RPM_ARCH}"
  local stage="$DIST/$name"
  mkdir -p "$stage"
  install -m 0755 "$PKGROOT/usr/bin/caw" "$stage/caw"
  install -m 0644 "$ROOT/README.md" "$ROOT/PLAN.md" "$stage/"
  install -m 0644 "$ROOT/assets/icons/caw.svg" "$stage/caw.svg"
  install -m 0644 "$ROOT/assets/icons/caw.png" "$stage/caw.png"
  tar -C "$DIST" -czf "$DIST/$name.tar.gz" "$name"
  rm -rf "$stage"
  (cd "$DIST" && sha256sum "$name.tar.gz" > "$name.tar.gz.sha256")
}

build_deb() {
  if ! need dpkg-deb; then
    skip "dpkg-deb not found; skipping .deb (install dpkg-dev / dpkg)"
    return 1
  fi
  say "Building .deb"
  local root="$DIST/deb"
  rm -rf "$root"
  mkdir -p "$root/DEBIAN"
  cp -a "$PKGROOT/usr" "$root/"
  cat > "$root/DEBIAN/control" <<EOF
Package: caw
Version: $VERSION
Architecture: $DEB_ARCH
Maintainer: $MAINTAINER
Section: sound
Priority: optional
Installed-Size: $INSTALLED_KB
Depends: libasound2, libxkbcommon0, libwayland-client0, libx11-6, libxcb1, libfontconfig1, libfreetype6, libssl3, libvulkan1
Homepage: https://github.com/mtaech/caw
Description: $SUMMARY
 $DESCRIPTION
EOF
  dpkg-deb --root-owner-group -Zxz --build "$root" \
    "$DIST/caw_${VERSION}_${DEB_ARCH}.deb" >/dev/null
  rm -rf "$root"
}

build_rpm() {
  if ! need rpmbuild; then
    skip "rpmbuild not found; skipping .rpm (Fedora: rpm-build, Debian/Ubuntu: rpm)"
    return 1
  fi
  say "Building .rpm"
  local top="$DIST/rpmbuild"
  rm -rf "$top"
  mkdir -p "$top/BUILD" "$top/RPMS" "$top/SOURCES" "$top/SPECS" "$top/SRPMS"
  local build_date
  build_date="$(date +"%a %b %d %Y")"
  cat > "$top/SPECS/caw.spec" <<EOF
Name: caw
Version: $VERSION
Release: 1
Summary: $SUMMARY
License: MIT
URL: https://github.com/mtaech/caw
BuildArch: $RPM_ARCH
AutoReqProv: no
Requires: alsa-lib, xkbcommon, wayland, libX11, libxcb, fontconfig, freetype, openssl-libs, vulkan-loader

%description
$DESCRIPTION

%prep
%build
%install
mkdir -p %{buildroot}/usr/bin \
         %{buildroot}/usr/share/applications \
         %{buildroot}/usr/share/icons/hicolor/scalable/apps \
         %{buildroot}/usr/share/icons/hicolor/256x256/apps
install -m 0755 "$PKGROOT/usr/bin/caw" %{buildroot}/usr/bin/caw
install -m 0644 "$PKGROOT/usr/share/applications/caw.desktop" \
  %{buildroot}/usr/share/applications/caw.desktop
install -m 0644 "$PKGROOT/usr/share/icons/hicolor/scalable/apps/caw.svg" \
  %{buildroot}/usr/share/icons/hicolor/scalable/apps/caw.svg
install -m 0644 "$PKGROOT/usr/share/icons/hicolor/256x256/apps/caw.png" \
  %{buildroot}/usr/share/icons/hicolor/256x256/apps/caw.png

%files
/usr/bin/caw
/usr/share/applications/caw.desktop
/usr/share/icons/hicolor/scalable/apps/caw.svg
/usr/share/icons/hicolor/256x256/apps/caw.png

%changelog
* $build_date $MAINTAINER - $VERSION-1
- Release $VERSION
EOF
  rpmbuild -bb --define "_topdir $top" "$top/SPECS/caw.spec" >/dev/null
  find "$top/RPMS" -name "*.rpm" -exec cp {} "$DIST/" \;
  rm -rf "$top"
}

build_appimage() {
  local tool="${APPIMAGETOOL:-appimagetool}"
  if [ ! -x "$tool" ] && ! command -v "$tool" >/dev/null 2>&1; then
    skip "appimagetool not found; skipping .AppImage"
    skip "  download: https://github.com/AppImage/appimagetool/releases"
    skip "  or set APPIMAGETOOL=/path/to/appimagetool"
    return 1
  fi
  say "Building .AppImage"
  local appdir="$DIST/AppDir"
  rm -rf "$appdir"
  mkdir -p "$appdir/usr/bin" \
           "$appdir/usr/share/applications" \
           "$appdir/usr/share/icons/hicolor/scalable/apps" \
           "$appdir/usr/share/icons/hicolor/256x256/apps"
  install -m 0755 "$PKGROOT/usr/bin/caw" "$appdir/usr/bin/caw"
  install -m 0644 "$ROOT/packaging/caw.desktop" "$appdir/caw.desktop"
  install -m 0644 "$ROOT/assets/icons/caw.svg" "$appdir/caw.svg"
  install -m 0644 "$ROOT/assets/icons/caw.png" "$appdir/caw.png"
  install -m 0755 "$ROOT/packaging/AppRun" "$appdir/AppRun"
  install -m 0644 "$ROOT/assets/icons/caw.svg" \
     "$appdir/usr/share/icons/hicolor/scalable/apps/caw.svg"
  install -m 0644 "$ROOT/assets/icons/caw.png" \
     "$appdir/usr/share/icons/hicolor/256x256/apps/caw.png"
  install -m 0644 "$ROOT/packaging/caw.desktop" \
     "$appdir/usr/share/applications/caw.desktop"

  local out="$DIST/Caw-${VERSION}-${APPIMAGE_ARCH}.AppImage"
  if ARCH="$APPIMAGE_ARCH" "$tool" --appimage-extract-and-run --no-appstream \
       "$appdir" "$out" >/dev/null 2>&1; then
    :
  else
    ARCH="$APPIMAGE_ARCH" "$tool" "$appdir" "$out"
  fi
  chmod +x "$out"
  rm -rf "$appdir"
}

want() { case ",$TARGET," in *",$1,"*) return 0 ;; esac; [ "$TARGET" = all ]; }
built=0
failed=""

if want tarball; then build_tarball; built=$((built + 1)); fi
if want deb; then if build_deb; then built=$((built + 1)); else failed="$failed deb"; fi; fi
if want rpm; then if build_rpm; then built=$((built + 1)); else failed="$failed rpm"; fi; fi
if want appimage; then if build_appimage; then built=$((built + 1)); else failed="$failed appimage"; fi; fi

echo
say "Artifacts in $DIST:"
ls -1 "$DIST" | grep -E "\.(deb|rpm|AppImage|tar\.gz|sha256)$" || true

if [ "$built" -eq 0 ]; then
  echo "No package could be built for target: $TARGET" >&2
  exit 1
fi
if [ -n "$failed" ]; then
  echo "Skipped/failed formats:$failed" >&2
fi
