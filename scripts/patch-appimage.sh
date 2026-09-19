#!/usr/bin/env bash

set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: $0 INPUT.AppImage OUTPUT.AppImage" >&2
  exit 2
fi

input=$1
output=$2

if [[ ! -f "$input" ]]; then
  echo "AppImage not found: $input" >&2
  exit 1
fi

if [[ $(uname -m) != x86_64 ]]; then
  echo "AppImage compatibility patching currently supports x86_64 only." >&2
  exit 1
fi

work_dir=$(mktemp -d)
trap 'rm -rf "$work_dir"' EXIT

source_appimage="$work_dir/source.AppImage"
cp "$input" "$source_appimage"
chmod +x "$source_appimage"

(
  cd "$work_dir"
  "$source_appimage" --appimage-extract >/dev/null
)
app_dir="$work_dir/squashfs-root"
lib_dir="$app_dir/usr/lib"

if [[ ! -d "$lib_dir" ]]; then
  echo "The AppImage does not contain the expected usr/lib directory." >&2
  exit 1
fi

# Tauri 2.11's Ubuntu-built AppImage bundles desktop infrastructure libraries
# that must remain compatible with the host Mesa, GVFS, and GLib stack. On
# newer distributions those copies cause GVFS symbol failures and make
# WebKitGTK abort while creating its EGL display. Keep application libraries
# bundled while resolving this tightly coupled infrastructure from the host.
# See https://github.com/tauri-apps/tauri/issues/15665.
shopt -s nullglob
host_library_patterns=(
  'libwayland-*.so*'
  'libglib-2.0.so*'
  'libgio-2.0.so*'
  'libgobject-2.0.so*'
  'libgmodule-2.0.so*'
  'libmount.so*'
  'libblkid.so*'
  'libselinux.so*'
  'libpcre2-8.so*'
  'libzstd.so*'
  'libelf.so*'
  'libffi.so*'
)

removed=0
for pattern in "${host_library_patterns[@]}"; do
  matches=("$lib_dir"/$pattern)
  for library in "${matches[@]}"; do
    rm -f -- "$library"
    ((removed += 1))
  done
done

if ((removed == 0)); then
  echo "No conflicting host-integration libraries were found." >&2
  exit 1
fi

for pattern in "${host_library_patterns[@]}"; do
  matches=("$lib_dir"/$pattern)
  if ((${#matches[@]} != 0)); then
    echo "Conflicting library remains after AppImage patching: ${matches[0]}" >&2
    exit 1
  fi
done

runtime_offset=$($source_appimage --appimage-offset)
if [[ ! $runtime_offset =~ ^[1-9][0-9]*$ ]]; then
  echo "Could not determine the AppImage runtime size." >&2
  exit 1
fi
head -c "$runtime_offset" "$source_appimage" >"$work_dir/runtime-x86_64"

cache_root=${XDG_CACHE_HOME:-"$HOME/.cache"}
plugin=$(find "$cache_root/tauri" -maxdepth 1 -type f -name 'linuxdeploy-plugin-appimage.AppImage' -print -quit)
if [[ -z $plugin ]]; then
  echo "Tauri's cached AppImage packaging plugin was not found." >&2
  exit 1
fi

tool_dir="$work_dir/appimagetool"
mkdir "$tool_dir"
(
  cd "$tool_dir"
  "$plugin" --appimage-extract >/dev/null
)
appimagetool="$tool_dir/squashfs-root/usr/bin/appimagetool"
mksquashfs_dir="$tool_dir/squashfs-root/appimagetool-prefix/usr/bin"
if [[ ! -x "$appimagetool" || ! -x "$mksquashfs_dir/mksquashfs" ]]; then
  echo "Tauri's AppImage packaging tools are incomplete." >&2
  exit 1
fi

patched_appimage="$work_dir/patched.AppImage"
PATH="$mksquashfs_dir:$PATH" \
  ARCH=x86_64 \
  LINUXDEPLOY_OUTPUT_VERSION=0.1.0 \
  "$appimagetool" \
  --runtime-file "$work_dir/runtime-x86_64" \
  "$app_dir" \
  "$patched_appimage"

install -m 0755 "$patched_appimage" "$output"
echo "Repacked AppImage after removing $removed conflicting host libraries."
