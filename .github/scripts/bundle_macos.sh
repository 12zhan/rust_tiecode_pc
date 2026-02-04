#!/usr/bin/env bash
set -euo pipefail

APP_NAME="Tiecode"
BIN_NAME="tiecode"

VERSION="$(
  awk '
    $0 ~ /^\[package\]/ {in_pkg=1; next}
    $0 ~ /^\[/ && $0 !~ /^\[package\]/ {in_pkg=0}
    in_pkg {
      if ($0 ~ /^version[[:space:]]*=/) {
        line = $0
        sub(/^version[[:space:]]*=[[:space:]]*\"/, "", line)
        sub(/\".*$/, "", line)
        print line
        exit
      }
    }
  ' Cargo.toml
)"
if [[ -z "${VERSION}" ]]; then
  echo "Failed to detect version from Cargo.toml" >&2
  exit 1
fi

DIST_DIR="dist"
APP_DIR="${DIST_DIR}/${APP_NAME}.app"

rm -rf "${APP_DIR}"
mkdir -p "${APP_DIR}/Contents/MacOS" "${APP_DIR}/Contents/Resources"

if [[ ! -f "target/release/${BIN_NAME}" ]]; then
  echo "Missing binary: target/release/${BIN_NAME}" >&2
  exit 1
fi

cp "target/release/${BIN_NAME}" "${APP_DIR}/Contents/MacOS/${BIN_NAME}"
chmod +x "${APP_DIR}/Contents/MacOS/${BIN_NAME}"

if [[ -d assets ]]; then
  cp -R assets "${APP_DIR}/Contents/Resources/assets"
fi

cat > "${APP_DIR}/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>en</string>
  <key>CFBundleExecutable</key>
  <string>${BIN_NAME}</string>
  <key>CFBundleIdentifier</key>
  <string>com.tiecode.app</string>
  <key>CFBundleName</key>
  <string>${APP_NAME}</string>
  <key>CFBundleDisplayName</key>
  <string>${APP_NAME}</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>${VERSION}</string>
  <key>CFBundleVersion</key>
  <string>${VERSION}</string>
  <key>CFBundleIconFile</key>
  <string>icon.icns</string>
  <key>NSHighResolutionCapable</key>
  <true/>
</dict>
</plist>
EOF

ICON_SRC="assets/icon.ico"
ICON_DST="${APP_DIR}/Contents/Resources/icon.icns"
if [[ -f "${ICON_SRC}" ]]; then
  ICON_INPUT="${ICON_SRC}"
  ICON_TMP_DIR="$(mktemp -d)"
  if ! sips -g pixelWidth "${ICON_SRC}" >/dev/null 2>&1; then
    echo "sips cannot read ${ICON_SRC}, falling back to ImageMagick" >&2
    brew install imagemagick
    ICON_INPUT="${ICON_TMP_DIR}/icon.png"
    magick "${ICON_SRC}" -resize 1024x1024 "${ICON_INPUT}"
  fi

  ICONSET_DIR="$(mktemp -d)/icon.iconset"
  mkdir -p "${ICONSET_DIR}"

  sips -z 16 16 "${ICON_INPUT}" --out "${ICONSET_DIR}/icon_16x16.png" >/dev/null
  sips -z 32 32 "${ICON_INPUT}" --out "${ICONSET_DIR}/icon_16x16@2x.png" >/dev/null
  sips -z 32 32 "${ICON_INPUT}" --out "${ICONSET_DIR}/icon_32x32.png" >/dev/null
  sips -z 64 64 "${ICON_INPUT}" --out "${ICONSET_DIR}/icon_32x32@2x.png" >/dev/null
  sips -z 128 128 "${ICON_INPUT}" --out "${ICONSET_DIR}/icon_128x128.png" >/dev/null
  sips -z 256 256 "${ICON_INPUT}" --out "${ICONSET_DIR}/icon_128x128@2x.png" >/dev/null
  sips -z 256 256 "${ICON_INPUT}" --out "${ICONSET_DIR}/icon_256x256.png" >/dev/null
  sips -z 512 512 "${ICON_INPUT}" --out "${ICONSET_DIR}/icon_256x256@2x.png" >/dev/null
  sips -z 512 512 "${ICON_INPUT}" --out "${ICONSET_DIR}/icon_512x512.png" >/dev/null
  sips -z 1024 1024 "${ICON_INPUT}" --out "${ICONSET_DIR}/icon_512x512@2x.png" >/dev/null

  iconutil -c icns "${ICONSET_DIR}" -o "${ICON_DST}"
else
  echo "Warning: ${ICON_SRC} not found, bundle will have no app icon" >&2
fi

mkdir -p "${DIST_DIR}"
rm -f "${DIST_DIR}/tiecode-macos.zip"
ditto -c -k --sequesterRsrc --keepParent "${APP_DIR}" "${DIST_DIR}/tiecode-macos.zip"

echo "Created ${DIST_DIR}/tiecode-macos.zip"
