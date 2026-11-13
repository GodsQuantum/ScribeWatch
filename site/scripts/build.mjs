import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {ASSETS} from '../src/worker.mjs';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const pub=path.join(root,'public');
const app=path.resolve(root,'..');
const sourceVersion=JSON.parse(fs.readFileSync(path.join(app,'frontend/package.json'),'utf8')).version;
const locales={
  en:{
    title:'ScribeWatch — Voice recordings to useful documents',
    description:'Record, transcribe and structure audio. Private, local-first notes, automatic transcription and PDFs. Open-source desktop installers for Windows, macOS and Linux.',
    nav:['How it works','Screenshots','Download'], badge:'OPEN SOURCE · LOCAL FIRST',
    heading:'Your recordings. Useful notes. On your terms.',
    intro:'Record a consultation, meeting or idea. ScribeWatch keeps the original transcript and can turn it into a structured document, Markdown note or PDF. No cloud account required.',
    demo:'See the interface', system:'Choose your platform', screenCaption:`Actual ScribeWatch v${sourceVersion} interface — demonstration data only`,
    how:'From sound to document', howLead:'One coherent workspace for recording, transcription and review.',
    features:[
      ['01 / CAPTURE','Record or import','Use REC with browser microphone crash recovery, upload an existing file or monitor a folder.'],
      ['02 / TRANSCRIBE','Keep the original','Preserve the canonical transcript. Use a local speech model or your own compatible endpoint.'],
      ['03 / STRUCTURE','Make it readable','Generate structured notes and export as Markdown, TXT, HTML, DOCX, ODT or PDF.']
    ],
    gallery:'The actual interface',galleryLead:'Screens captured from the current source version, without private or clinical data.',
    shots:['Record directly in REC','Documents and notes are organised in one place','Manage engines, models and profiles in Settings'],
    trust:[
      ['Your audio stays under your control','Local processing on the full Windows standalone package after first-time model downloads.'],
      ['Original transcript preserved','AI structuring never replaces the source transcript.'],
      ['Open-source and self-hostable','MIT license, Docker option and versioned GitHub releases.']
    ],
    download:'Choose your ScribeWatch',downloadLead:'Platform-specific official builds from the latest stable GitHub release.',
    cards:[
      ['Windows','Full standalone · recommended','Includes local STT and LLM engines. On first launch it downloads Parakeet Ultra and Qwen3.5-4B weights, then prepares five report profiles.','Recommended','Full installer','Lightweight installer'],
      ['macOS','Apple Silicon or Intel','Lightweight desktop build. Connect your own STT and optional LLM engine; the full Windows native AI bundle is not yet included.','Two architectures','Apple Silicon','Intel'],
      ['Linux','x86-64 or ARM64','AppImage desktop build. Use a compatible transcription service or local engine configured separately.','AppImages','x86-64','ARM64']
    ],
    warning:'Important:',warningBody:'The full one-click local AI setup currently targets Windows x64 only. Initial model downloads require internet; other builds require provider setup. Generated clinical reports are drafts for practitioner review. Public binaries may trigger unsigned/notarization warnings.',
    github:'GitHub repository',releases:'All releases',author:'Created by Arezki Chougar · MIT licensed',screen:'Product interface'
  },
  fr:{
    title:'ScribeWatch — De la voix au document',
    description:'Enregistrez, transcrivez et structurez vos audios. Notes privées, comptes rendus et PDF en local. Application open source pour Windows, macOS et Linux.',
    nav:['Fonctionnement','Captures','Téléchargements'],badge:'OPEN SOURCE · PRIORITÉ AU LOCAL',
    heading:'De la voix au document, sans détour.',
    intro:'Enregistrez une consultation, une réunion ou une idée. ScribeWatch conserve la transcription originale et peut créer un document structuré, une note Markdown ou un PDF. Sans compte cloud.',
    demo:'Voir l’interface',system:'Choisir ma plateforme',screenCaption:`Interface réelle de ScribeWatch v${sourceVersion} — données de démonstration uniquement`,
    how:'Du son au document',howLead:'Un même espace pour enregistrer, transcrire et relire.',
    features:[
      ['01 / CAPTURE','Enregistrer ou importer','Enregistrez dans REC avec récupération après interruption, importez un fichier ou surveillez un dossier.'],
      ['02 / TRANSCRIPTION','Préserver l’original','Conservez la transcription canonique. Utilisez un modèle local ou votre propre endpoint compatible.'],
      ['03 / STRUCTURATION','Obtenir un vrai document','Structurez les notes et exportez en Markdown, TXT, HTML, DOCX, ODT ou PDF.']
    ],
    gallery:'L’interface, telle qu’elle est',galleryLead:'Captures de la version actuelle du code, sans aucune donnée privée ni clinique.',
    shots:['Enregistrement direct dans REC','Documents et notes dans le même espace','Moteurs, modèles et profils dans Paramètres'],
    trust:[
      ['Vos audios restent sous votre contrôle','Sur Windows autonome complet, traitement local après téléchargement initial des modèles.'],
      ['Transcription source préservée','La structuration IA ne remplace jamais la transcription originale.'],
      ['Open source et auto-hébergeable','Licence MIT, option Docker et releases versionnées sur GitHub.']
    ],
    download:'Choisissez votre ScribeWatch',downloadLead:'Installateurs officiels propres à chaque système depuis la dernière release GitHub stable.',
    cards:[
      ['Windows','Autonome complet · recommandé','Intègre les moteurs STT et LLM locaux. Au premier lancement : téléchargement de Parakeet Ultra et Qwen3.5-4B, puis configuration de cinq profils.','Recommandé','Installation complète','Installation légère'],
      ['macOS','Apple Silicon ou Intel','Application légère. Connectez votre moteur de transcription et éventuellement un LLM : le pack IA autonome Windows n’est pas encore inclus.','Deux architectures','Apple Silicon','Intel'],
      ['Linux','x86-64 ou ARM64','AppImage. Utilisez un fournisseur STT compatible ou un moteur local configuré séparément.','AppImages','x86-64','ARM64']
    ],
    warning:'À savoir :',warningBody:'L’installation IA locale entièrement automatique concerne actuellement Windows x64. Les modèles nécessitent une connexion initiale ; les autres éditions demandent une configuration des moteurs. Les documents médicaux générés restent des brouillons à relire par le praticien. Les installateurs ne sont pas tous signés/notarisés.',
    github:'Dépôt GitHub',releases:'Toutes les versions',author:'Créé par Arezki Chougar · Licence MIT',screen:'Interface de l’application'
  },
  'zh-CN':{
    title:'ScribeWatch — 将语音转为实用文档',
    description:'录音、转写、整理文档。隐私优先，本地转录与 PDF 报告。适用于 Windows、macOS 和 Linux 的开源应用。',
    nav:['工作流程','界面截图','下载'],badge:'开源 · 本地优先',
    heading:'从录音到文档，一步完成。',
    intro:'录制会诊、会议或灵感。ScribeWatch 保留原始转录，并可生成结构化文档、Markdown 笔记或 PDF，无需云端账户。',
    demo:'查看界面',system:'选择平台',screenCaption:`ScribeWatch v${sourceVersion} 真实界面，仅使用演示数据`,
    how:'从声音到文档',howLead:'在同一个工作空间中完成录音、转录与检查。',
    features:[
      ['01 / 采集','录音或导入','在录音界面使用浏览器麦克风，支持中断恢复；也可上传音频或监控文件夹。'],
      ['02 / 转录','保留原文','保留完整原始转录。可以使用本地语音模型或自有兼容接口。'],
      ['03 / 整理','导出文档','生成结构化笔记，支持 Markdown、TXT、HTML、DOCX、ODT 和 PDF。']
    ],
    gallery:'真实应用界面',galleryLead:'来自最新正式版本，截图不包含私人或医疗资料。',
    shots:['在录音界面直接录制','集中管理文档和笔记','在设置中管理引擎、模型与模板'],
    trust:[
      ['音频由您掌控','完整 Windows 独立版在首次下载模型后可本地处理音频。'],
      ['原始转录始终保留','AI 整理不会覆盖原始转录文本。'],
      ['开源并支持自托管','MIT 许可证、Docker 方案、GitHub 正式版本。']
    ],
    download:'选择 ScribeWatch 版本',downloadLead:'从 GitHub 最新稳定版本下载适合您系统的安装程序。',
    cards:[
      ['Windows','完整独立版 · 推荐','包含本地 STT 与 LLM 引擎。首次启动自动下载 Parakeet Ultra 和 Qwen3.5-4B 模型并准备五个报告模板。','推荐','完整安装包','轻量安装包'],
      ['macOS','Apple Silicon 或 Intel','轻量桌面版。需要自行配置转录服务和可选 LLM；目前不包含 Windows 完整本地 AI 套件。','两种架构','Apple Silicon','Intel'],
      ['Linux','x86-64 或 ARM64','AppImage 桌面版。需另外配置兼容转录服务或本地引擎。','AppImage','x86-64','ARM64']
    ],
    warning:'使用前须知：',warningBody:'自动完成本地 AI 配置目前仅支持 Windows x64。首次下载模型需要联网；其他系统需手动配置模型服务。医疗报告仅是需专业人员审核的草稿。未签名或未公证的安装包可能显示安全提示。',
    github:'GitHub 仓库',releases:'全部版本',author:'Arezki Chougar 开发 · MIT 许可证',screen:'应用界面'
  }
};
function esc(value) {return String(value).replace(/[&<>"']/g,(c)=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));}
function render(lang, t) {
  const url='https://scribewatch.arezkichougar.com/'+lang+'/';
  const link=(to,label)=>'<a href="'+to+'">'+esc(label)+'</a>';
  const shots=['record','documents','settings'];
  const targets=[['windows-full','windows-light'],['mac-arm64','mac-amd64'],['linux-amd64','linux-arm64']];
  const localeExtra = {
    en: {mac:'Local AI standalone edition — first launch downloads models',linux:'Local AI AppImage and installable Debian package — first launch downloads models',warning:'Bundled AI editions automatically download local Parakeet Ultra and Qwen3.5-4B models on first launch when available. Keep an internet connection for initial downloads. Clinical documents require practitioner review. Unnotarized or unsigned binaries may trigger security warnings.',labels:{'mac-arm64-full':'Apple Silicon · full AI','mac-amd64-full':'Intel · full AI','linux-amd64-full':'x86-64 · full AI AppImage','linux-arm64-full':'ARM64 · full AI AppImage','linux-amd64-deb':'Ubuntu / Debian x86-64 · installable','linux-arm64-deb':'Ubuntu / Debian ARM64 · installable'}},
    fr: {mac:'Version autonome avec IA locale — téléchargement des modèles au premier lancement',linux:'AppImage avec IA locale et paquet Debian installable — téléchargement des modèles au premier lancement',warning:'Les éditions complètes avec IA locale téléchargent Parakeet Ultra et Qwen3.5-4B au premier lancement. Une connexion Internet est nécessaire initialement. Les documents cliniques exigent une relecture professionnelle. Les binaires non signés ou non notarisés peuvent déclencher un avertissement de sécurité.',labels:{'mac-arm64-full':'Apple Silicon · IA complète','mac-amd64-full':'Intel · IA complète','linux-amd64-full':'x86-64 · AppImage IA complète','linux-arm64-full':'ARM64 · AppImage IA complète','linux-amd64-deb':'Ubuntu / Debian x86-64 · à installer','linux-arm64-deb':'Ubuntu / Debian ARM64 · à installer'}},
    'zh-CN': {mac:'完整本地 AI 版，首次启动自动下载模型',linux:'完整 AI AppImage 和可安装 Debian 软件包，首次启动自动下载模型',warning:'完整本地 AI 版本首次启动时下载 Parakeet Ultra 和 Qwen3.5-4B 模型。首次下载需要联网。医疗文档需要专业人员审核。未签名或未经 Apple 公证的程序可能显示安全警告。',labels:{'mac-arm64-full':'Apple Silicon · 完整 AI','mac-amd64-full':'Intel · 完整 AI','linux-amd64-full':'x86-64 · 完整 AI AppImage','linux-arm64-full':'ARM64 · 完整 AI AppImage','linux-amd64-deb':'Ubuntu / Debian x86-64 · 可安装','linux-arm64-deb':'Ubuntu / Debian ARM64 · 可安装'}}
  }[lang];
  const additionalTargets=[[],['mac-arm64-full','mac-amd64-full'],['linux-amd64-full','linux-arm64-full','linux-amd64-deb','linux-arm64-deb']];
  const lines=[
  '<!doctype html><html lang="'+lang+'"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">',
  '<title>'+esc(t.title)+'</title><meta name="description" content="'+esc(t.description)+'">',
  '<meta name="theme-color" content="#0a1110"><meta name="color-scheme" content="dark"><meta name="robots" content="index,follow">',
  '<link rel="canonical" href="'+url+'"><link rel="icon" type="image/svg+xml" href="/assets/logo.svg">',
  '<link rel="alternate" hreflang="en" href="https://scribewatch.arezkichougar.com/en/">',
  '<link rel="alternate" hreflang="fr" href="https://scribewatch.arezkichougar.com/fr/">',
  '<link rel="alternate" hreflang="zh-CN" href="https://scribewatch.arezkichougar.com/zh-CN/">',
  '<link rel="alternate" hreflang="x-default" href="https://scribewatch.arezkichougar.com/en/">',
  '<meta property="og:type" content="website"><meta property="og:site_name" content="ScribeWatch">',
  '<meta property="og:title" content="'+esc(t.title)+'"><meta property="og:description" content="'+esc(t.description)+'">',
  '<meta property="og:url" content="'+url+'"><meta property="og:image" content="https://scribewatch.arezkichougar.com/assets/screens/'+lang+'-home.png">',
  '<meta name="twitter:card" content="summary_large_image"><link rel="stylesheet" href="/assets/style.css"><script defer src="/assets/client.js"></script></head><body>',
  '<header class="header"><div class="wrap head"><a class="brand" href="/'+lang+'/"><img src="/assets/logo.svg" alt="" width="36" height="36">ScribeWatch</a>',
  '<nav class="nav" aria-label="Primary">'+link('#features',t.nav[0])+link('#screens',t.nav[1])+link('#downloads',t.nav[2])+'</nav>',
  '<div class="lang" aria-label="Language">'+['en','fr','zh-CN'].map((l)=>'<a '+(lang===l?'aria-current="page" ':'')+'hreflang="'+l+'" lang="'+l+'" href="/'+l+'/">'+({'en':'EN','fr':'FR','zh-CN':'中文'}[l])+'</a>').join('')+'</div></div></header>',
  '<main><section class="hero"><div class="wrap hero-grid"><div><p class="kicker">'+esc(t.badge)+' · <span id="release-tag">v0.6.2</span></p>',
  '<h1>'+esc(t.heading)+'</h1><p class="intro">'+esc(t.intro)+'</p><div class="actions"><a class="button primary" id="smart-download" href="#downloads">'+esc(t.system)+'</a><a class="button secondary" href="#screens">'+esc(t.demo)+'</a></div><p class="platform-note" id="platform-note"></p></div>',
  '<div><div class="preview"><div class="chrome"><i></i><i></i><i></i><small>ScribeWatch / Home</small></div><img src="/assets/screens/'+lang+'-home.png" alt="'+esc(t.screen)+'" width="1440" height="900"></div><p class="preview-caption">'+esc(t.screenCaption)+'</p></div></div></section>',
  '<section class="section" id="features"><div class="wrap"><div class="section-head"><h2>'+esc(t.how)+'</h2><p class="lead">'+esc(t.howLead)+'</p></div><div class="features">',
  ...t.features.map(([step,h,p])=>'<article class="feature"><div class="feature-index">'+esc(step)+'</div><h3>'+esc(h)+'</h3><p>'+esc(p)+'</p></article>'),
  '</div></div></section>',
  '<section class="section" id="screens"><div class="wrap"><div class="section-head"><h2>'+esc(t.gallery)+'</h2><p class="lead">'+esc(t.galleryLead)+'</p></div><div class="shots">',
  ...shots.map((shot,i)=>'<figure class="shot"><img loading="lazy" src="/assets/screens/'+lang+'-'+shot+'.png" width="1440" height="900" alt="'+esc(t.shots[i])+'"><figcaption>'+esc(t.shots[i])+'</figcaption></figure>'),
  '</div></div></section>',
  '<aside class="trust"><div class="wrap trust-inner">',
  ...t.trust.map(([h,p])=>'<div><strong>'+esc(h)+'</strong><span>'+esc(p)+'</span></div>'),
  '</div></aside>',
  '<section class="section" id="downloads"><div class="wrap"><div class="section-head"><h2>'+esc(t.download)+'</h2><p class="lead">'+esc(t.downloadLead)+'</p></div><div class="downloads">',
  ...t.cards.map((card,i)=>'<article class="download-card '+(i===0?'emphasis':'')+'" data-platform="'+['windows','macos','linux'][i]+'" data-full-copy="'+esc(i===1?localeExtra.mac:i===2?localeExtra.linux:'')+'"><span class="tag">'+esc(card[3])+'</span><div><h3>'+esc(card[0])+'</h3><p>'+esc(card[1])+'</p></div><p data-platform-description>'+esc(card[2])+'</p>'+
     targets[i].map((kind,j)=>'<div class="asset"><a data-download="'+kind+'" href="https://github.com/GodsQuantum/ScribeWatch/releases/latest">'+esc(card[j+4])+'</a><span data-size="'+kind+'"></span></div>').join('')+
     additionalTargets[i].map(kind=>'<div class="asset" hidden><a data-download="'+kind+'" href="https://github.com/GodsQuantum/ScribeWatch/releases/latest">'+esc(localeExtra.labels[kind])+'</a><span data-size="'+kind+'"></span></div>').join('')+'</article>'),
  '</div><p class="caution" data-full-warning="'+esc(localeExtra.warning)+'"><strong>'+esc(t.warning)+'</strong> <span data-warning-body>'+esc(t.warningBody)+'</span></p></div></section></main>',
  '<footer class="footer"><div class="wrap footer-inner"><span>'+esc(t.author)+'</span><div class="footer-links">',
  link('https://github.com/GodsQuantum/ScribeWatch',t.github)+link('https://github.com/GodsQuantum/ScribeWatch/releases/latest',t.releases),
  '</div></div></footer></body></html>'
  ];
  return lines.join('\n');
}
fs.mkdirSync(pub,{recursive:true});
fs.mkdirSync(path.join(pub,'assets','screens'),{recursive:true});
fs.copyFileSync(path.join(app,'docs','logo.svg'),path.join(pub,'assets','logo.svg'));
fs.copyFileSync(path.join(root,'src','style.css'),path.join(pub,'assets','style.css'));
fs.copyFileSync(path.join(root,'src','client.js'),path.join(pub,'assets','client.js'));
fs.writeFileSync(path.join(pub,'assets','release-map.json'),JSON.stringify(Object.fromEntries(Object.entries(ASSETS).map(([k,rx])=>[k,rx.source]))));
for(const [lang,t] of Object.entries(locales)){
  const dir=path.join(pub,lang);
  fs.mkdirSync(dir,{recursive:true});
  fs.writeFileSync(path.join(dir,'index.html'),render(lang,t));
  for(const screen of ['home','record','documents','settings']){
    const src=path.join(app,'docs','screenshots',lang,screen+'.png');
    const bytes=fs.readFileSync(src);
    if(bytes.subarray(1,4).toString()!=='PNG' || bytes.byteLength<10000)throw new Error('Missing or invalid live capture: '+src);
    fs.writeFileSync(path.join(pub,'assets','screens',lang+'-'+screen+'.png'),bytes);
  }
}
fs.writeFileSync(path.join(pub,'index.html'),"<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"robots\" content=\"index,follow\"><title>ScribeWatch</title><script>(function(){const locale=(navigator.language||\"\").toLowerCase();location.replace(locale.startsWith(\"fr\")?\"/fr/\":locale.startsWith(\"zh\")?\"/zh-CN/\":\"/en/\");})()</script></head><body><noscript><p><a href=\"/en/\">English</a> · <a href=\"/fr/\">Français</a> · <a href=\"/zh-CN/\">中文</a></p></noscript></body></html>");
fs.writeFileSync(path.join(pub,'robots.txt'),'User-agent: *\nAllow: /\nSitemap: https://scribewatch.arezkichougar.com/sitemap.xml\n');
fs.writeFileSync(path.join(pub,'sitemap.xml'),'<?xml version="1.0" encoding="UTF-8"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">'+Object.keys(locales).map((l)=>'<url><loc>https://scribewatch.arezkichougar.com/'+l+'/</loc></url>').join('')+'</urlset>');
console.log('BUILT',Object.keys(locales).join(','),'real screenshot assets:',Object.keys(locales).length*4);
