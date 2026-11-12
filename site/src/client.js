(() => {
  'use strict';
  const lang = document.documentElement.lang || 'en';
  const words = {
    en: {windows:'Download for Windows', mac:'Download for macOS', linux:'Download for Linux', chooseMac:'Choose your Mac version', chooseLinux:'Choose your Linux version', other:'Choose your platform', ready:'Recommended for your system', select:'Select a platform below'},
    fr: {windows:'Télécharger pour Windows', mac:'Télécharger pour macOS', linux:'Télécharger pour Linux', chooseMac:'Choisir la version macOS', chooseLinux:'Choisir la version Linux', other:'Choisir ma plateforme', ready:'Recommandé pour votre système', select:'Sélectionnez votre plateforme ci-dessous'},
    'zh-CN': {windows:'下载 Windows 版', mac:'下载 macOS 版', linux:'下载 Linux 版', chooseMac:'选择 macOS 版本', chooseLinux:'选择 Linux 版本', other:'选择平台', ready:'适用于当前系统', select:'请在下方选择平台'}
  }[lang] || {};
  const button = document.getElementById('smart-download');
  const note = document.getElementById('platform-note');
  const ua = navigator.userAgent || '';
  let detectedTarget = null;
  async function choose() {
    let target = null;
    let label = words.other;
    let architecture = '';
    try {
      const values = navigator.userAgentData?.getHighEntropyValues ? await navigator.userAgentData.getHighEntropyValues(['architecture']) : null;
      architecture = (values?.architecture || '').toLowerCase();
    } catch {}
    if (/android|iphone|ipad|ipod/i.test(ua) || (/macintosh/i.test(ua) && navigator.maxTouchPoints > 1)) {
      label = words.other; // Desktop AppImages and installers are not mobile packages.
    } else if (/windows/i.test(ua)) {
      label = words.windows;
      if (!/arm/i.test(architecture) && !/arm/i.test(ua)) target = 'windows-full';
    } else if (/macintosh|mac os/i.test(ua)) {
      label = words.chooseMac;
      // Safari can report Intel on Apple Silicon: only select an architecture if verified.
      if (/arm/i.test(architecture)) { target = 'mac-arm64'; label = words.mac; }
      else if (/x86|x64|amd64/i.test(architecture)) { target = 'mac-amd64'; label = words.mac; }
    } else if (/linux/i.test(ua)) {
      label = words.chooseLinux;
      if (/aarch64|arm64/i.test(architecture + ' ' + ua)) { target = 'linux-arm64'; label = words.linux; }
      else if (/x86_64|amd64|x64/i.test(architecture + ' ' + ua)) { target = 'linux-amd64'; label = words.linux; }
    }
    detectedTarget = target;
    button.textContent = label;
    button.href = target ? '/download/' + target : '#downloads';
    note.textContent = target ? words.ready : words.select;
  }
  const choiceReady = choose();
  fetch('/api/release', { headers: {Accept:'application/json'} })
    .then((r) => r.ok ? r.json() : null)
    .then(async (data) => {
      await choiceReady;
      if (!data?.version) return;
      const tag = document.getElementById('release-tag');
      if (tag) tag.textContent = data.version;
      // Only promote a full local-AI download if GitHub already published it.
      // The currently official lightweight DMG/AppImage remains the fallback.
      if (detectedTarget) {
        const next = ['mac-arm64','mac-amd64','linux-amd64','linux-arm64'].includes(detectedTarget)
          && data.assets?.[detectedTarget + '-full']
          ? detectedTarget + '-full'
          : detectedTarget;
        if (data.assets?.[next]) button.href = '/download/' + next;
      }
      const hasMacFull = Boolean(data.assets?.['mac-arm64-full'] || data.assets?.['mac-amd64-full']);
      const hasLinuxFull = Boolean(data.assets?.['linux-amd64-full'] || data.assets?.['linux-arm64-full']);
      for (const [platform, enabled] of [['macos', hasMacFull], ['linux', hasLinuxFull]]) {
        if (!enabled) continue;
        const card = document.querySelector('[data-platform="' + platform + '"]');
        const description = card?.querySelector('[data-platform-description]');
        if (description && card?.dataset.fullCopy) description.textContent = card.dataset.fullCopy;
      }
      if (hasMacFull || hasLinuxFull) {
        const caution = document.querySelector('[data-full-warning]');
        const warningText = caution?.querySelector('[data-warning-body]');
        if (warningText && caution?.dataset.fullWarning) warningText.textContent = caution.dataset.fullWarning;
      }
      for (const [key, asset] of Object.entries(data.assets || {})) {
        const size = document.querySelector('[data-size="' + key + '"]');
        if (size && asset?.size) size.textContent = (asset.size / 1024 / 1024).toFixed(0) + ' MB';
        const link = document.querySelector('[data-download="' + key + '"]');
        if (link) {
          const row = link.closest('.asset');
          if (row) row.hidden = !asset;
        }
      }
    }).catch(() => {});
})();
