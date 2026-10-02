#!/usr/bin/env bash
set -euo pipefail
PORT="${SCRIBEWATCH_LOCAL_PORT:-3052}"
SOCKET="$HOME/.config/systemd/user/scribewatch-local.socket"
SERVICE="$HOME/.config/systemd/user/scribewatch-local.service"
DESKTOP="$HOME/.local/share/applications/scribewatch-local.desktop"

if [[ "${1:-}" == "--uninstall" ]]; then
  systemctl --user disable --now scribewatch-local.socket scribewatch-local.service 2>/dev/null || true
  rm -f "$SOCKET" "$SERVICE" "$DESKTOP"
  systemctl --user daemon-reload
  echo "ScribeWatch local microphone mode removed."
  exit 0
fi

TARGET="${1:-${SCRIBEWATCH_TARGET:-}}"
if [[ -z "$TARGET" ]]; then
  echo "Usage: $0 <NAS-host-or-IP:ScribeWatch-HTTP-port>" >&2
  echo "Example: $0 192.168.1.217:3052" >&2
  exit 2
fi
PROXY="$(command -v systemd-socket-proxyd || true)"
[[ -n "$PROXY" ]] || { echo "systemd-socket-proxyd not found" >&2; exit 1; }
mkdir -p "$HOME/.config/systemd/user" "$HOME/.local/share/applications"
cat >"$SOCKET" <<EOF
[Unit]
Description=ScribeWatch local HTTP loopback socket
[Socket]
ListenStream=127.0.0.1:$PORT
NoDelay=true
[Install]
WantedBy=sockets.target
EOF
cat >"$SERVICE" <<EOF
[Unit]
Description=ScribeWatch local loopback proxy
After=network-online.target
[Service]
ExecStart=$PROXY $TARGET
NoNewPrivileges=yes
PrivateTmp=yes
ProtectSystem=strict
ProtectHome=yes
EOF
cat >"$DESKTOP" <<EOF
[Desktop Entry]
Type=Application
Name=ScribeWatch Local Microphone
Comment=Trusted localhost HTTP mode for browser microphone capture
Exec=xdg-open http://127.0.0.1:$PORT
Icon=audio-input-microphone
Terminal=false
Categories=AudioVideo;Utility;
EOF
systemctl --user daemon-reload
systemctl --user disable --now scribewatch-local.socket scribewatch-local.service 2>/dev/null || true
systemctl --user enable --now scribewatch-local.socket
echo "ScribeWatch local mode: http://127.0.0.1:$PORT -> $TARGET"
command -v xdg-open >/dev/null && xdg-open "http://127.0.0.1:$PORT" >/dev/null 2>&1 || true
