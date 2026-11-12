const RELEASE_API = 'https://api.github.com/repos/GodsQuantum/ScribeWatch/releases/latest';
export const ASSETS = Object.freeze({
  'windows-full': /-windows-amd64-standalone-setup\.exe$/i,
  'windows-light': /-windows-amd64-setup\.exe$/i,
  'mac-arm64': /-darwin-arm64\.dmg$/i,
  'mac-amd64': /-darwin-amd64\.dmg$/i,
  'linux-amd64': /-x86_64\.AppImage$/i,
  'linux-arm64': /-aarch64\.AppImage$/i,
  'mac-arm64-full': /-darwin-arm64-standalone\.dmg$/i,
  'mac-amd64-full': /-darwin-amd64-standalone\.dmg$/i,
  'linux-amd64-full': /-x86_64-standalone\.AppImage$/i,
  'linux-arm64-full': /-aarch64-standalone\.AppImage$/i,
  'linux-amd64-deb': /^scribewatch_[0-9]+\.[0-9]+\.[0-9]+_amd64\.deb$/i,
  'linux-arm64-deb': /^scribewatch_[0-9]+\.[0-9]+\.[0-9]+_arm64\.deb$/i
});
export function selectAsset(release, target) {
  if (!Object.hasOwn(ASSETS, target) || !Array.isArray(release?.assets)) return null;
  return release.assets.find((asset) => ASSETS[target].test(asset.name) &&
    /^https:\/\/github\.com\/GodsQuantum\/ScribeWatch\/releases\/download\//.test(asset.browser_download_url || '')) || null;
}
async function releaseInfo() {
  const response = await fetch(RELEASE_API, {
    headers: { Accept: 'application/vnd.github+json', 'User-Agent': 'ScribeWatch-Website' },
    cf: { cacheEverything: true, cacheTtl: 600 }
  });
  if (!response.ok) throw new Error('GitHub release unavailable: ' + response.status);
  const data = await response.json();
  if (data.draft || data.prerelease || !/^v\d+\.\d+\.\d+$/.test(data.tag_name)) throw new Error('Invalid latest release');
  return data;
}
function language(header) {
  const v = (header || '').toLowerCase();
  if (v.startsWith('fr') || /,\s*fr/.test(v)) return 'fr';
  if (v.startsWith('zh') || /,\s*zh/.test(v)) return 'zh-CN';
  return 'en';
}
export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    if (!['GET', 'HEAD'].includes(request.method)) return new Response('Method not allowed', {status:405,headers:{Allow:'GET, HEAD'}});
    if (url.pathname === '/') return Response.redirect(new URL('/' + language(request.headers.get('accept-language')) + '/',url),302);
    if (url.pathname === '/api/release') {
      try {
        const release = await releaseInfo();
        const assets = {};
        for (const target of Object.keys(ASSETS)) {
          const asset = selectAsset(release, target);
          assets[target] = asset ? { name: asset.name, size: asset.size, url: '/download/' + target } : null;
        }
        return Response.json({ version: release.tag_name, assets }, {headers:{'Cache-Control':'public, max-age=300','X-Content-Type-Options':'nosniff'}});
      } catch {
        return Response.json({error:'release_unavailable'},{status:503,headers:{'Cache-Control':'no-store'}});
      }
    }
    if (url.pathname.startsWith('/download/')) {
      const target = url.pathname.slice('/download/'.length);
      if (!Object.hasOwn(ASSETS,target)) return new Response('Not found',{status:404});
      try {
        const asset = selectAsset(await releaseInfo(),target);
        if (asset) return Response.redirect(asset.browser_download_url,302);
      } catch {}
      return Response.redirect('https://github.com/GodsQuantum/ScribeWatch/releases/latest',302);
    }
    return env.ASSETS.fetch(request);
  }
};
