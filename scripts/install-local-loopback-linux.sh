#!/usr/bin/env bash
set -euo pipefail
TARGET="${SCRIBEWATCH_TARGET:-192.168.1.217:3052}"
PORT="${SCRIBEWATCH_LOCAL_PORT:-3052}"
PROXY=/usr/lib/systemd/systemd-socket-proxyd
[[ -x "$PROXY" ]] || { echo "systemd-socket-proxyd not found" >&2; exit 1; }
mkdir -p "$HOME/.config/systemd/user" "$HOME/.local/share/applications"
cat >"$HOME/.config/systemd/user/scribewatch-local.socket" <<EOF
[Unit]
Description=ScribeWatch local HTTP loopback socket
[Socket]
ListenStream=127.0.0.1:$PORT
NoDelay=true
[Install]
WantedBy=sockets.target
EOF
cat >"$HOME/.config/systemd/user/scribewatch-local.service" <<EOF
[Unit]
Description=ScribeWatch loopback proxy to Cloud9
After=network-online.target
[Service]
ExecStart=$PROXY $TARGET
NoNewPrivileges=yes
PrivateTmp=yes
ProtectSystem=strict
ProtectHome=yes
EOF

cat >"$HOME/.local/share/applications/scribewatch-local.desktop" <<EOF
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
systemctl --user enable --now scribewatch-local.socket
echo "ScribeWatch local mode: http://127.0.0.1:$PORT -> $TARGET"