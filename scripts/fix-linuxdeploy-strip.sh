#!/bin/bash
#
# fix-linuxdeploy-strip.sh
#
# Fix the linuxdeploy AppImage bundled with Tauri so it uses the system strip
# instead of its own ancient strip (2.35) which can't handle Fedora 44+'s
# .relr.dyn ELF sections.
#
# Usage: ./scripts/fix-linuxdeploy-strip.sh
#
# After running this, `npx tauri build --bundles appimage` should succeed.

set -euo pipefail

# ── colours ──────────────────────────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[0;33m'; CYAN='\033[0;36m'
BOLD='\033[1m'; NC='\033[0m'

info()  { echo -e "${GREEN}${BOLD}::${NC} $*"; }
warn()  { echo -e "${YELLOW}${BOLD}!!${NC} $*"; }
err()   { echo -e "${RED}${BOLD}!!${NC} $*"; }
header(){ echo; echo -e "${CYAN}══════════════════════════════════════════════════${NC}"; echo -e "${CYAN}  $*${NC}"; echo -e "${CYAN}══════════════════════════════════════════════════${NC}"; }

# ── paths ────────────────────────────────────────────────────────────────────
CACHE_DIR="${HOME}/.cache/tauri"
ORIGINAL="${CACHE_DIR}/linuxdeploy-x86_64.AppImage"
BACKUP="${CACHE_DIR}/linuxdeploy-x86_64.AppImage.orig"

WORK_DIR=$(mktemp -d /tmp/ld-fix-XXXXXX)
EXTRACT_DIR="${WORK_DIR}/squashfs-root"
FIXED_IMAGE="${WORK_DIR}/linuxdeploy-x86_64.AppImage"

cleanup() { rm -rf "${WORK_DIR}"; }
trap cleanup EXIT

# ── checks ───────────────────────────────────────────────────────────────────
header "Pre-flight checks"

if [ ! -f "${ORIGINAL}" ]; then
    err "linuxdeploy AppImage not found at ${ORIGINAL}"
    err "Run 'npx tauri build --bundles appimage' once to download it first."
    exit 1
fi

SYSTEM_STRIP=$(command -v strip 2>/dev/null || echo "")
if [ -z "${SYSTEM_STRIP}" ]; then
    err "System strip not found.  Install binutils."
    exit 1
fi

SYSTEM_STRIP_VER=$("${SYSTEM_STRIP}" --version 2>&1 | head -1)
info "System strip: ${SYSTEM_STRIP_VER}"

# Check that system strip can handle .relr.dyn (binutils >= 2.40)
STRIP_MAJOR=$("${SYSTEM_STRIP}" --version 2>&1 | sed -nE 's/.* ([0-9]+)\.[0-9]+.*/\1/p')
STRIP_MINOR=$("${SYSTEM_STRIP}" --version 2>&1 | sed -nE 's/.* [0-9]+\.([0-9]+).*/\1/p')
if [ -n "${STRIP_MAJOR}" ] && [ -n "${STRIP_MINOR}" ]; then
    if [ "${STRIP_MAJOR}" -lt 2 ] || { [ "${STRIP_MAJOR}" -eq 2 ] && [ "${STRIP_MINOR}" -lt 40 ]; }; then
        warn "System strip may be too old for .relr.dyn (need >= 2.40, have ${STRIP_MAJOR}.${STRIP_MINOR})"
        warn "Proceeding anyway — if the build fails, update binutils."
    else
        info "System strip >= 2.40 ✓  (supports .relr.dyn)"
    fi
fi

# Check that appimagetool (from existing plugin, if any) is usable
APPIMAGETOOL="${CACHE_DIR}/.appimagetool"
if [ ! -x "${APPIMAGETOOL}" ]; then
    info "appimagetool not cached. Will extract from linuxdeploy-plugin-appimage if available."
    # Try to find it in extracted linuxdeploy
    PLUGIN_IMAGE="${CACHE_DIR}/linuxdeploy-plugin-appimage.AppImage"
    if [ -f "${PLUGIN_IMAGE}" ]; then
        info "Found linuxdeploy-plugin-appimage.AppImage, will extract appimagetool from it."
    else
        warn "linuxdeploy-plugin-appimage.AppImage not found either."
        warn "Will try using the built-in appimagetool from the linuxdeploy plugin (if available)."
    fi
fi

info "All checks passed."

# ── step 1: extract the original linuxdeploy AppImage ────────────────────────
header "Step 1/4 — Extracting original linuxdeploy AppImage"

info "Extracting to ${EXTRACT_DIR} …"
cd "${WORK_DIR}"
"${ORIGINAL}" --appimage-extract > /dev/null 2>&1

if [ ! -d "${EXTRACT_DIR}" ]; then
    err "Extraction failed — squashfs-root not found."
    exit 1
fi
info "Extracted ✓"

# ── step 2: replace strip ────────────────────────────────────────────────────
header "Step 2/4 — Replacing bundled strip with system strip"

BUNDLED_STRIP="${EXTRACT_DIR}/usr/bin/strip"
if [ ! -f "${BUNDLED_STRIP}" ]; then
    err "No bundled strip found at ${BUNDLED_STRIP} (unexpected AppImage structure)."
    exit 1
fi

BUNDLED_VER=$("${BUNDLED_STRIP}" --version 2>&1 | head -1)
info "Bundled strip: ${BUNDLED_VER}"

cp "${SYSTEM_STRIP}" "${BUNDLED_STRIP}"
info "Replaced ✓  (now: $("${BUNDLED_STRIP}" --version 2>&1 | head -1))"

# ── step 3: embed gtk/gstreamer plugins ─────────────────────────────────────
header "Step 3/4 — Embedding gtk & gstreamer plugins"

for PLUGIN in gtk gstreamer; do
    SRC="${CACHE_DIR}/linuxdeploy-plugin-${PLUGIN}.sh"
    DST="${EXTRACT_DIR}/usr/bin/linuxdeploy-plugin-${PLUGIN}"
    if [ -f "${SRC}" ]; then
        cp "${SRC}" "${DST}"
        chmod +x "${DST}"
        info "  ${PLUGIN} ✓"
    else
        warn "  ${PLUGIN} plugin script not found at ${SRC} — skipping"
    fi
done

# ── step 4: repack as AppImage ───────────────────────────────────────────────
header "Step 4/4 — Repacking fixed AppImage"

# Find appimagetool — try cached, then plugin, then embedded one
APPIMAGETOOL=""
MKSQUASHFS=""

# Check if we have appimagetool in the extracted plugin
EMBEDDED_APPIMAGETOOL="${EXTRACT_DIR}/plugins/linuxdeploy-plugin-appimage/appimagetool-prefix/usr/bin/appimagetool"
EMBEDDED_LIBS="${EXTRACT_DIR}/plugins/linuxdeploy-plugin-appimage/appimagetool-prefix/usr/lib"
EMBEDDED_MKSQUASHFS="${EXTRACT_DIR}/plugins/linuxdeploy-plugin-appimage/appimagetool-prefix/usr/lib/appimagekit/mksquashfs"

if [ -x "${EMBEDDED_APPIMAGETOOL}" ]; then
    APPIMAGETOOL="${EMBEDDED_APPIMAGETOOL}"
    MKSQUASHFS="${EMBEDDED_MKSQUASHFS}"
    info "Using embedded appimagetool from linuxdeploy plugin."
fi

if [ -z "${APPIMAGETOOL}" ] || [ ! -x "${APPIMAGETOOL}" ]; then
    err "Cannot find appimagetool anywhere."
    err "Expected at: ${EMBEDDED_APPIMAGETOOL}"
    exit 1
fi

# appimagetool needs LD_LIBRARY_PATH for its bundled libgpgme etc.
LD_LIBRARY_PATH="${EMBEDDED_LIBS}:${LD_LIBRARY_PATH:-}" \
    "${APPIMAGETOOL}" "${EXTRACT_DIR}" "${FIXED_IMAGE}" > /dev/null 2>&1

if [ ! -f "${FIXED_IMAGE}" ]; then
    err "Repacking failed — output not created."
    exit 1
fi

info "Repacked ✓  ($(du -h "${FIXED_IMAGE}" | cut -f1))"

# ── install ──────────────────────────────────────────────────────────────────
header "Installing"

# Back up original if not already backed up
if [ ! -f "${BACKUP}" ]; then
    cp "${ORIGINAL}" "${BACKUP}"
    info "Backed up original → ${BACKUP}"
fi

cp "${FIXED_IMAGE}" "${ORIGINAL}"
chmod +x "${ORIGINAL}"

info "Installed fixed linuxdeploy → ${ORIGINAL}"

# Verify
echo
info "Verifying …"
FILE_INFO=$(file "${ORIGINAL}")
if echo "${FILE_INFO}" | grep -q "ELF"; then
    VER_STR=$("${ORIGINAL}" --version 2>&1 | head -1)
    STRIP_STR=$(mktemp -d /tmp/ld-check-XXXXXX)
    cd "${STRIP_STR}"
    "${ORIGINAL}" --appimage-extract > /dev/null 2>&1
    NEW_STRIP_VER=$(squashfs-root/usr/bin/strip --version 2>&1 | head -1)
    rm -rf "${STRIP_STR}"
    echo
    info "✔  ${BOLD}linuxdeploy:${NC}  ${VER_STR}"
    info "✔  ${BOLD}strip:${NC}        ${NEW_STRIP_VER}"
else
    warn "File type doesn't look like an AppImage: ${FILE_INFO}"
fi

# ── done ─────────────────────────────────────────────────────────────────────
header "Done"
echo
echo -e "  Run ${GREEN}${BOLD}npx tauri build --bundles appimage${NC} to build the AppImage."
echo
echo -e "  ${YELLOW}If it still fails with the gtk plugin, run:${NC}"
echo -e "  ${YELLOW}    rm -rf src-tauri/target/release/bundle${NC}"
echo
