# ScribeWatch website

Source for https://scribewatch.arezkichougar.com — a separate Cloudflare Worker using Workers Static Assets.

## Maintenance

Run npm install once in this directory, then npm run check. The build generates English, French and Simplified Chinese SEO pages, a sitemap, and copies the real application screenshots into the ignored site/public directory. npm run deploy verifies and deploys via Wrangler only when a scoped Cloudflare token is available.

Wrangler declares a Custom Domain. On successful deployment Cloudflare configures its DNS record and TLS certificate. Do not create an unrelated placeholder DNS record. For GitHub Actions: add a CLOUDFLARE_API_TOKEN secret, a CLOUDFLARE_ACCOUNT_ID variable, and set SCRIBEWATCH_SITE_DEPLOY_READY=true once permissions are validated. Credentials are never committed.

The six download routes resolve exact binary assets from the latest stable GitHub release. In case of upstream failure the user is sent to the official Releases page rather than an invented link. Ambiguous macOS architectures require an explicit choice rather than a mistaken Apple Silicon/Intel download. There are no analytics, cookies, trackers, external fonts or private clinical data.

Screenshots are captured using ../scripts/capture-screenshots.cjs on a clean, isolated v0.6.2 demo instance with Playwright. Original screenshots live only in docs/screenshots/{locale}; the site build copies them into its generated assets.

Brand: existing audio-to-document icon, mint accent, graphite background and actual ScribeWatch UI. Reviewed against the Unslop skill by Damir Sokolovsky, CC BY 4.0: https://github.com/heydamir/unslop .
