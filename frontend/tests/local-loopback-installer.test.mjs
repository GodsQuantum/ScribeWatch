import test from 'node:test';
import assert from 'node:assert/strict';
import {
  buildLocalProxyInstaller,
  detectLocalProxyPlatform,
  inferLocalProxyTarget,
  localProxyFilename
} from '../src/lib/local-loopback-installer.ts';

test('local target is inferred only from plain remote HTTP', () => {
  assert.deepEqual(inferLocalProxyTarget({protocol:'http:', hostname:'nas.local', port:'3052'}), {host:'nas.local', port:3052});
  assert.deepEqual(inferLocalProxyTarget({protocol:'https:', hostname:'scribe.example.com', port:''}), {host:'', port:3052});
  assert.deepEqual(inferLocalProxyTarget({protocol:'http:', hostname:'127.0.0.1', port:'3052'}), {host:'', port:3052});
});

test('desktop platform detection covers Windows, macOS and Linux', () => {
  assert.equal(detectLocalProxyPlatform({platform:'Win32'}), 'windows');
  assert.equal(detectLocalProxyPlatform({platform:'MacIntel'}), 'macos');
  assert.equal(detectLocalProxyPlatform({platform:'Linux x86_64'}), 'linux');
});

test('installers bind only localhost and stay reversible', () => {
  const linux=buildLocalProxyInstaller('linux','nas.local',3052);
  const windows=buildLocalProxyInstaller('windows','192.168.1.217',3052);
  const mac=buildLocalProxyInstaller('macos','nas.local',3052);
  assert.match(linux,/systemd-socket-proxyd/);
  assert.match(linux,/127\.0\.0\.1:\$LOCAL_PORT/);
  assert.match(linux,/--uninstall/);
  assert.match(windows,/netsh interface portproxy add v4tov4/);
  assert.match(windows,/listenaddress=127\.0\.0\.1/);
  assert.match(windows,/--uninstall/);
  assert.match(mac,/Library\/LaunchAgents/);
  assert.match(mac,/IO::Socket::INET/);
  assert.match(mac,/127\.0\.0\.1/);
  assert.match(mac,/--uninstall/);
});

test('installer names are OS-specific and unsafe host input is rejected', () => {
  assert.equal(localProxyFilename('linux'),'ScribeWatch-Local-Proxy-Linux.sh');
  assert.equal(localProxyFilename('windows'),'ScribeWatch-Local-Proxy-Windows.cmd');
  assert.equal(localProxyFilename('macos'),'ScribeWatch-Local-Proxy-macOS.command');
  assert.throws(()=>buildLocalProxyInstaller('linux','nas.local;rm -rf /',3052),/valid NAS hostname/);
});
