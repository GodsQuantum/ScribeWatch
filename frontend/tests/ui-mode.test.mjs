import test from 'node:test';
import assert from 'node:assert/strict';
import { navigationItems, navigationAllows, normalizeMode, normalizeView } from '../src/lib/ui-mode.ts';

test('new installs expose only the four essential destinations', () => {
  assert.equal(normalizeMode(null), 'simple');
  assert.deepEqual(navigationItems('simple').map(item => item.id), ['home', 'live', 'jobs', 'settings']);
  assert.equal(navigationAllows('simple', 'providers'), false);
  assert.equal(navigationAllows('simple', 'settings'), true);
});

test('advanced mode preserves existing workflows without showing two configuration sections in the rail', () => {
  assert.deepEqual(navigationItems('advanced').map(item => item.id),
    ['home', 'quick', 'live', 'workflows', 'jobs', 'settings']);
  assert.equal(navigationAllows('advanced', 'workflows'), true);
  assert.equal(navigationAllows('advanced', 'ai'), false);
  assert.equal(normalizeMode('advanced'), 'advanced');
  assert.equal(normalizeMode('anything-else'), 'simple');
});

test('legacy AI and STT tabs open inside settings rather than disappearing', () => {
  assert.equal(normalizeView('ai', 'advanced'), 'settings');
  assert.equal(normalizeView('providers', 'advanced'), 'settings');
  assert.equal(normalizeView('dashboard', 'simple'), 'home');
  assert.equal(normalizeView('workflows', 'simple'), 'live');
  assert.equal(normalizeView('jobs', 'simple'), 'jobs');
  assert.equal(normalizeView('live', 'advanced'), 'live');
});


test('simple clinical folder messages follow FR EN ZH', async () => {
  const {t}=await import('../src/lib/i18n.ts');
  assert.equal(t('en','Choose destination folder'),'Choose destination folder');
  assert.equal(t('fr','Choose destination folder'),'Choisir le dossier de sortie');
  assert.equal(t('zh-CN','Choose destination folder'),'选择保存文件夹');
  for(const locale of ['fr','zh-CN'])assert.notEqual(t(locale,'Automatic save was incomplete. Use the export buttons to save the remaining files.'),'Automatic save was incomplete. Use the export buttons to save the remaining files.');
});
