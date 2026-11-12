#!/bin/sh
# Native deb installs the qualified AppDir; no daemon or postinstall hook.
set -eu
[ "$#" -eq 4 ] || { echo "Usage: build-deb.sh APPDIR OUT_DIR VERSION amd64|arm64" >&2; exit 64; }
appdir=$1
outdir=$2
version=$3
arch=$4
case "$arch" in amd64|arm64) ;; *) echo "Unsupported Debian architecture" >&2; exit 64;; esac
case "$version" in *[!0-9.]*|'') echo "Invalid version" >&2; exit 64;; esac
for needed in \
  "$appdir/AppRun" \
  "$appdir/usr/bin/scribewatch" \
  "$appdir/usr/share/scribewatch/engines/BUNDLED-RUNTIMES-SHA256.json" \
  "$appdir/usr/share/scribewatch/engines/speaches/Cloud9-Speaches" \
  "$appdir/usr/share/scribewatch/engines/llm/llama-server" \
  "$appdir/usr/share/applications/io.github.GodsQuantum.ScribeWatch.desktop" \
  "$appdir/usr/share/icons/hicolor/scalable/apps/scribewatch.svg"
do
  [ -f "$needed" ] || { echo "Missing native package member: $needed" >&2; exit 66; }
done
command -v dpkg-deb >/dev/null || { echo "dpkg-deb unavailable" >&2; exit 69; }
mkdir -p -- "$outdir"
parent=$(mktemp -d /tmp/scribewatch-deb-XXXXXXXX)
trap 'rm -rf -- "$parent"' EXIT HUP INT TERM
debroot="$parent/package"
mkdir -p -- "$debroot/DEBIAN" "$debroot/opt/scribewatch" \
  "$debroot/usr/bin" "$debroot/usr/share/applications" \
  "$debroot/usr/share/icons/hicolor/scalable/apps"
cp -al -- "$appdir/." "$debroot/opt/scribewatch/" 2>/dev/null ||
  cp -a -- "$appdir/." "$debroot/opt/scribewatch/"
cat > "$debroot/usr/bin/scribewatch" <<'EOF'
#!/bin/sh
APPDIR=/opt/scribewatch
export APPDIR
exec "$APPDIR/AppRun" "$@"
EOF
chmod 755 "$debroot/usr/bin/scribewatch"
cp -- "$appdir/usr/share/applications/io.github.GodsQuantum.ScribeWatch.desktop" \
  "$debroot/usr/share/applications/"
cp -- "$appdir/usr/share/icons/hicolor/scalable/apps/scribewatch.svg" \
  "$debroot/usr/share/icons/hicolor/scalable/apps/"
installed_kib=$(du -sk "$debroot/opt/scribewatch" | cut -f1)
cat > "$debroot/DEBIAN/control" <<EOF
Package: scribewatch
Version: $version
Section: sound
Priority: optional
Architecture: $arch
Maintainer: ScribeWatch contributors
Installed-Size: $installed_kib
Depends: libc6, ca-certificates
Homepage: https://github.com/GodsQuantum/ScribeWatch
Description: Local-first audio transcription and structured reports
 ScribeWatch includes Speaches Reloaded and llama.cpp on the local host.
 Parakeet Ultra and Qwen3.5-4B model weights download on first use.
EOF
output="$outdir/scribewatch_${version}_${arch}.deb"
[ ! -e "$output" ] || { echo "Refusing to overwrite $output" >&2; exit 73; }
dpkg-deb --build --root-owner-group -Zzstd "$debroot" "$output"
(cd "$outdir" && sha256sum "$(basename "$output")" > "$(basename "$output").sha256")
printf 'DEB_PACKAGE=%s\n' "$output"
