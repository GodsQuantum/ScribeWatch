const { chromium } = require('playwright-core');
const fs = require('fs');
const path = require('path');

const base = process.env.SCRIBEWATCH_SCREENSHOT_URL || 'http://127.0.0.1:39200';
const out = path.resolve(__dirname, '../docs/screenshots');
const executablePath = process.env.SCRIBEWATCH_BROWSER || '/opt/brave-bin/brave';
fs.mkdirSync(out, { recursive: true });

(async () => {
  const browser = await chromium.launch({ executablePath, headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 980 }, deviceScaleFactor: 1 });
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(base, { waitUntil: 'networkidle' });
  await page.screenshot({ path: path.join(out, 'home-quick-transcribe.png'), fullPage: true });

  await page.getByRole('button', { name: 'Workflows' }).click();
  await page.getByRole('button', { name: /Voice notes/ }).click();
  await page.screenshot({ path: path.join(out, 'workflow-folders.png'), fullPage: true });

  const mobile = await browser.newPage({ viewport: { width: 390, height: 844 }, deviceScaleFactor: 1 });
  await mobile.emulateMedia({ reducedMotion: 'reduce' });
  await mobile.goto(base, { waitUntil: 'networkidle' });
  await mobile.getByRole('navigation', { name: 'Mobile navigation' }).getByRole('button', { name: /Quick Transcribe/ }).click();
  await mobile.screenshot({ path: path.join(out, 'mobile-quick-transcribe.png'), fullPage: false });

  await page.getByRole('navigation', { name: 'Main navigation' }).getByRole('button', { name: /Quick Transcribe/ }).click();
  await page.screenshot({ path: path.join(out, 'quick-transcribe.png'), fullPage: true });
  await browser.close();
  console.log('SCRIBEWATCH_SCREENSHOTS=PASS');
})().catch((error) => {
  console.error(error);
  process.exit(1);
});