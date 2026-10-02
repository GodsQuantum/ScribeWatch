export type LocalProxyPlatform = 'linux' | 'windows' | 'macos';

export interface LocalProxyTarget {
  host: string;
  port: number;
}

export interface LocationLike {
  protocol: string;
  hostname: string;
  port: string;
}

function loopbackHost(host: string): boolean {
  return ['localhost', '127.0.0.1', '::1', '[::1]'].includes(host.trim().toLowerCase());
}

export function inferLocalProxyTarget(location: LocationLike): LocalProxyTarget {
  const host = location.hostname.trim();
  if (location.protocol === 'http:' && host && !loopbackHost(host)) {
    return { host, port: Number(location.port || '80') };
  }
  return { host: '', port: 3052 };
}

export function detectLocalProxyPlatform(nav: {
  userAgent?: string;
  platform?: string;
  userAgentData?: { platform?: string };
}): LocalProxyPlatform {
  const value = `${nav.userAgentData?.platform ?? ''} ${nav.platform ?? ''} ${nav.userAgent ?? ''}`.toLowerCase();
  if (value.includes('win')) return 'windows';
  if (value.includes('mac')) return 'macos';
  return 'linux';
}

function validPort(value: number): boolean {
  return Number.isInteger(value) && value >= 1 && value <= 65535;
}

function normalizedHost(value: string): string {
  const host = value.trim();
  if (!host || host.length > 253 || !/^[A-Za-z0-9.-]+$/.test(host) || host.includes('..')) {
    throw new Error('Enter a valid NAS hostname or IPv4 address.');
  }
  return host;
}

export function localProxyFilename(platform: LocalProxyPlatform): string {
  if (platform === 'windows') return 'ScribeWatch-Local-Proxy-Windows.cmd';
  if (platform === 'macos') return 'ScribeWatch-Local-Proxy-macOS.command';
  return 'ScribeWatch-Local-Proxy-Linux.sh';
}

function linuxInstaller(host: string, targetPort: number, localPort: number): string {
  return `#!/usr/bin/env bash
set -euo pipefail
TARGET_HOST='${host}'
TARGET_PORT='${targetPort}'
LOCAL_PORT='${localPort}'
SOCKET="$HOME/.config/systemd/user/scribewatch-local.socket"
SERVICE="$HOME/.config/systemd/user/scribewatch-local.service"
DESKTOP="$HOME/.local/share/applications/scribewatch-local.desktop"

if [[ "\${1:-}" == "--uninstall" ]]; then
  systemctl --user disable --now scribewatch-local.socket scribewatch-local.service 2>/dev/null || true
  rm -f "$SOCKET" "$SERVICE" "$DESKTOP"
  systemctl --user daemon-reload
  echo "ScribeWatch local microphone mode removed."
  exit 0
fi

PROXY="$(command -v systemd-socket-proxyd || true)"
[[ -n "$PROXY" ]] || { echo "systemd-socket-proxyd is required on this Linux workstation." >&2; exit 1; }
mkdir -p "$HOME/.config/systemd/user" "$HOME/.local/share/applications"
cat >"$SOCKET" <<EOF
[Unit]
Description=ScribeWatch local HTTP loopback socket
[Socket]
ListenStream=127.0.0.1:$LOCAL_PORT
NoDelay=true
[Install]
WantedBy=sockets.target
EOF
cat >"$SERVICE" <<EOF
[Unit]
Description=ScribeWatch local loopback proxy
After=network-online.target
[Service]
ExecStart=$PROXY $TARGET_HOST:$TARGET_PORT
NoNewPrivileges=yes
PrivateTmp=yes
ProtectSystem=strict
ProtectHome=yes
EOF
cat >"$DESKTOP" <<EOF
[Desktop Entry]
Type=Application
Name=ScribeWatch Local Microphone
Comment=Open the NAS ScribeWatch instance through trusted localhost HTTP
Exec=xdg-open http://127.0.0.1:$LOCAL_PORT
Icon=audio-input-microphone
Terminal=false
Categories=AudioVideo;Utility;
EOF
systemctl --user daemon-reload
systemctl --user disable --now scribewatch-local.socket scribewatch-local.service 2>/dev/null || true
systemctl --user enable --now scribewatch-local.socket
echo "ScribeWatch local mode: http://127.0.0.1:$LOCAL_PORT -> $TARGET_HOST:$TARGET_PORT"
command -v xdg-open >/dev/null && xdg-open "http://127.0.0.1:$LOCAL_PORT" >/dev/null 2>&1 || true
`;
}

function windowsInstaller(host: string, targetPort: number, localPort: number): string {
  return `@echo off
setlocal EnableExtensions
set "TARGET_HOST=${host}"
set "TARGET_PORT=${targetPort}"
set "LOCAL_PORT=${localPort}"

net session >nul 2>&1
if errorlevel 1 (
  powershell -NoProfile -ExecutionPolicy Bypass -Command "Start-Process -FilePath '%~f0' -ArgumentList '%*' -Verb RunAs"
  exit /b
)

if /I "%~1"=="--uninstall" (
  netsh interface portproxy delete v4tov4 listenaddress=127.0.0.1 listenport=%LOCAL_PORT% >nul 2>&1
  echo ScribeWatch local microphone mode removed.
  pause
  exit /b 0
)

for /f "usebackq delims=" %%I in (\`powershell -NoProfile -Command "$a=[System.Net.Dns]::GetHostAddresses('%TARGET_HOST%') ^| Where-Object {$_.AddressFamily -eq 'InterNetwork'} ^| Select-Object -First 1; if($a){$a.IPAddressToString}"\`) do set "TARGET_IP=%%I"
if not defined TARGET_IP (
  echo Could not resolve %TARGET_HOST% to an IPv4 address.
  pause
  exit /b 1
)

sc start iphlpsvc >nul 2>&1
netsh interface portproxy delete v4tov4 listenaddress=127.0.0.1 listenport=%LOCAL_PORT% >nul 2>&1
netsh interface portproxy add v4tov4 listenaddress=127.0.0.1 listenport=%LOCAL_PORT% connectaddress=%TARGET_IP% connectport=%TARGET_PORT%
if errorlevel 1 (
  echo Failed to create the loopback proxy.
  pause
  exit /b 1
)
echo ScribeWatch local mode: http://127.0.0.1:%LOCAL_PORT% -^> %TARGET_HOST%:%TARGET_PORT%
start "" "http://127.0.0.1:%LOCAL_PORT%"
pause
`;
}

function macInstaller(host: string, targetPort: number, localPort: number): string {
  return `#!/bin/zsh
set -euo pipefail
TARGET_HOST='${host}'
TARGET_PORT='${targetPort}'
LOCAL_PORT='${localPort}'
BASE="$HOME/Library/Application Support/ScribeWatch"
PROXY="$BASE/local-proxy.pl"
PLIST="$HOME/Library/LaunchAgents/com.scribewatch.localproxy.plist"
LABEL="com.scribewatch.localproxy"

if [[ "\${1:-}" == "--uninstall" ]]; then
  launchctl bootout "gui/$UID" "$PLIST" >/dev/null 2>&1 || true
  rm -f "$PLIST" "$PROXY"
  rmdir "$BASE" >/dev/null 2>&1 || true
  echo "ScribeWatch local microphone mode removed."
  exit 0
fi

[[ -x /usr/bin/perl ]] || { echo "This installer requires the macOS system Perl runtime." >&2; exit 1; }
mkdir -p "$BASE" "$HOME/Library/LaunchAgents"
cat >"$PROXY" <<'PERL'
use strict;
use warnings;
use IO::Socket::INET;
use IO::Select;

my ($host, $port, $listen_port) = @ARGV;
my $server = IO::Socket::INET->new(
  LocalAddr => '127.0.0.1',
  LocalPort => $listen_port,
  Proto => 'tcp',
  Listen => 32,
  ReuseAddr => 1,
) or die "listen failed: $!\n";

$SIG{CHLD} = 'IGNORE';
while (my $client = $server->accept()) {
  my $pid = fork();
  if (!defined $pid) { close $client; next; }
  if ($pid == 0) {
    close $server;
    my $remote = IO::Socket::INET->new(
      PeerHost => $host,
      PeerPort => $port,
      Proto => 'tcp',
      Timeout => 10,
    ) or exit 1;
    my $selector = IO::Select->new($client, $remote);
    while (1) {
      for my $fh ($selector->can_read()) {
        my $buffer = '';
        my $read = sysread($fh, $buffer, 65536);
        exit 0 if !defined($read) || $read == 0;
        my $out = fileno($fh) == fileno($client) ? $remote : $client;
        my $offset = 0;
        while ($offset < $read) {
          my $written = syswrite($out, $buffer, $read - $offset, $offset);
          exit 0 unless $written;
          $offset += $written;
        }
      }
    }
  }
  close $client;
}
PERL
chmod 700 "$PROXY"
cat >"$PLIST" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>$LABEL</string>
  <key>ProgramArguments</key>
  <array>
    <string>/usr/bin/perl</string>
    <string>$PROXY</string>
    <string>$TARGET_HOST</string>
    <string>$TARGET_PORT</string>
    <string>$LOCAL_PORT</string>
  </array>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><true/>
</dict>
</plist>
EOF
launchctl bootout "gui/$UID" "$PLIST" >/dev/null 2>&1 || true
launchctl bootstrap "gui/$UID" "$PLIST"
echo "ScribeWatch local mode: http://127.0.0.1:$LOCAL_PORT -> $TARGET_HOST:$TARGET_PORT"
open "http://127.0.0.1:$LOCAL_PORT"
`;
}

export function buildLocalProxyInstaller(
  platform: LocalProxyPlatform,
  rawHost: string,
  targetPort: number,
  localPort = 3052
): string {
  const host = normalizedHost(rawHost);
  if (!validPort(targetPort) || !validPort(localPort)) throw new Error('Ports must be between 1 and 65535.');
  if (platform === 'windows') return windowsInstaller(host, targetPort, localPort).replace(/\n/g, '\r\n');
  if (platform === 'macos') return macInstaller(host, targetPort, localPort);
  return linuxInstaller(host, targetPort, localPort);
}
