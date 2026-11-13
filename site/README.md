# ScribeWatch website

Source for https://scribewatch.arezkichougar.com — public GitHub Pages site with an optional Cloudflare Worker implementation.

## Maintenance

Run npm install once in this directory, then npm run check. The build generates English, French and Simplified Chinese SEO pages, a sitemap, and copies the real application screenshots into the ignored site/public directory. npm run deploy verifies and deploys via Wrangler only when a scoped Cloudflare token is available.

Wrangler declares a Custom Domain. On successful deployment Cloudflare configures its DNS record and TLS certificate. Do not create an unrelated placeholder DNS record. For GitHub Actions: add a CLOUDFLARE_API_TOKEN secret, a CLOUDFLARE_ACCOUNT_ID variable, and set SCRIBEWATCH_SITE_DEPLOY_READY=true once permissions are validated. Credentials are never committed.

The twelve supported download targets resolve exact binary assets from the latest stable GitHub release. In case of upstream failure the user is sent to the official Releases page rather than an invented link. Ambiguous macOS architectures require an explicit choice rather than a mistaken Apple Silicon/Intel download. There are no analytics, cookies, trackers, external fonts or private clinical data.

Screenshots are captured using ../scripts/capture-screenshots.cjs on a clean, isolated instance matching the current source version (currently v0.6.4) with Playwright. Original screenshots live only in docs/screenshots/{locale}; the site build copies them into its generated assets.

Design: existing audio-to-document icon, neutral paper/ink canvas, one restrained green-teal accent and a large clickable preview of the actual UI. The first screen explains the outcome and local-only processing, with hardware guidance and advanced downloads further down. The lightbox supports Escape and arrow keys. Reviewed against the Unslop skill by Damir Sokolovsky, CC BY 4.0: https://github.com/heydamir/unslop .

## Static fallback (Cloudflare DNS + GitHub Pages)

When no Workers Scripts deployment token is configured, the same generated `site/public/` directory can be published in a `gh-pages` deployment branch. The DNS-only Cloudflare CNAME points the dedicated subdomain to GitHub Pages while the original Worker source remains available for a future native deployment. The browser falls back to GitHub’s public stable-release API when the Worker’s `/api/release` endpoint does not exist. Asset patterns come from the canonical Worker source and only verified GitHub release URLs are accepted. All initial download links remain safely pointed to the official Releases page if JavaScript or the API is unavailable. Do not alter the apex website DNS record.

### Automated GitHub Pages publication

The first-party GitHub Actions workflow `.github/workflows/site-pages.yml`
rebuilds and tests the EN/FR/ZH-CN static website whenever `site/` or
`docs/screenshots/` changes on `main`, then updates the existing `gh-pages`
branch with a fast-forward commit (never a force-push). The DNS CNAME,
HTTPS certificate and page-source settings remain unchanged. The public
GitHub release API is used to discover the newest signed-off installer;
the site never exposes unreleased candidates. Cloudflare Workers remains
an optional native deployment path when scoped credentials are available.

## 2026-11-13 editorial and privacy design

The homepage addresses ordinary users before explaining AI technology. It covers
recording and transcription, drafting by document type, full-local privacy
(with an explicit exception for deliberately selected remote providers),
recommended RAM and CPU/GPU trade-offs, and offline operation after the
first model download. The full installers are the primary path; lightweight
builds for people with existing STT/LLM services are inside an advanced
expandable section. Every downloadable asset is discovered from the newest
published stable GitHub release and must match the allowlisted filename.

The page uses the **Unslop** editorial method (Damir Sokolovsky, CC BY 4.0);
its source is https://github.com/heydamir/unslop. Visual spot checks use real
Playwright at 375, 768 and 1440 CSS pixels, all three locales, including a
full-size modal, ESC dismissal and mobile overflow detection.
