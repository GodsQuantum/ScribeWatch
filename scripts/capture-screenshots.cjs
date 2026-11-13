/**
 * Reproducible, privacy-safe screenshots of the current tagged code.
 * Use only against a freshly initialised demo instance, NEVER production with patient data.
 * npm install --no-save playwright; npx playwright install chromium
 */
const { chromium } = require('playwright');
const path = require('node:path');
const fs = require('node:fs');
const base = process.env.SCRIBEWATCH_SCREENSHOT_URL || 'http://127.0.0.1:39200';
const out = path.resolve(__dirname, '../docs/screenshots');
const expectedVersion = process.env.SCRIBEWATCH_EXPECT_VERSION || require('../frontend/package.json').version;
(async () => {
  const browser = await chromium.launch({headless:true,args:['--no-sandbox'],...(process.env.SCRIBEWATCH_CHROMIUM_EXECUTABLE?{executablePath:process.env.SCRIBEWATCH_CHROMIUM_EXECUTABLE}:{})});
  try {
    const response = await fetch(new URL('/api/v1/ready', base));
    const status = await response.json();
    if (status.version !== expectedVersion || status.status !== 'ready') {
      throw new Error('Refusing screenshots from stale/unready app: ' + JSON.stringify(status));
    }
    for (const [locale, dir] of [['en','en'], ['fr','fr'], ['zh-CN','zh-CN']]) {
      fs.mkdirSync(path.join(out,dir), {recursive:true});
      const context = await browser.newContext({viewport:{width:1440,height:900}, deviceScaleFactor:1, reducedMotion:'reduce'});
      await context.addInitScript((lang)=>localStorage.setItem('scribewatch:locale', lang), locale);
      const page = await context.newPage();
      await page.goto(base,{waitUntil:'domcontentloaded'});
      await page.waitForFunction(() => document.querySelectorAll("nav button").length >= 4); await page.waitForTimeout(1600); await page.evaluate(()=>document.fonts.ready);
      const nav=page.locator('nav').first().getByRole('button');
      await nav.nth(0).click(); await page.waitForTimeout(450);
      await page.screenshot({path:path.join(out,dir,'home.png'),fullPage:false});
      if (await nav.count() < 4) throw new Error('Unexpected versioned navigation: '+await page.locator('nav').first().innerText());
      for (const [index,file] of [[1,'record.png'],[2,'documents.png'],[3,'settings.png']]) {
        await nav.nth(index).click();
        await page.waitForTimeout(220);
        await page.screenshot({path:path.join(out,dir,file),fullPage:false});
      }
      const mobile=await browser.newContext({viewport:{width:390,height:844},deviceScaleFactor:1,reducedMotion:'reduce'});
      await mobile.addInitScript((lang)=>localStorage.setItem('scribewatch:locale',lang),locale);
      const mpage=await mobile.newPage();
      await mpage.goto(base,{waitUntil:'domcontentloaded'}); await mpage.waitForTimeout(1700);
      await mpage.screenshot({path:path.join(out,dir,'mobile.png'),fullPage:false});
      await mobile.close();
      await context.close();
      console.log('CAPTURED',locale,['home.png','record.png','documents.png','settings.png','mobile.png'].join(','));
    }
  } finally { await browser.close(); }
})().catch(e=>{console.error(e);process.exitCode=1});
