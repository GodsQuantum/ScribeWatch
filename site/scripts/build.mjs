import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {ASSETS} from '../src/worker.mjs';

const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const app=path.resolve(root,'..');
const pub=path.join(root,'public');
const sourceVersion=JSON.parse(fs.readFileSync(path.join(app,'frontend/package.json'),'utf8')).version;
const liveRelease='https://github.com/GodsQuantum/ScribeWatch/releases/latest';

const locales={
  fr:{
    title:'ScribeWatch — Vos notes, sans cloud',
    description:'Enregistrez une consultation, une note vocale ou une réunion. Obtenez une transcription et un compte rendu sur votre ordinateur, sans envoyer vos fichiers au cloud.',
    nav:['Pourquoi','Comment ça marche','Captures','Installer'],
    kicker:'Le carnet de notes qui écoute',
    hero:'Parlez. Gardez vos idées. Pas vos fichiers dans un cloud.',
    sub:'Enregistrez une consultation, une réunion ou une pensée à la volée. Retrouvez vos paroles à l’écrit et transformez-les en documents clairs, directement sur votre ordinateur.',
    button:'Installer ScribeWatch',explore:'Découvrir l’application',
    secure:'Sans compte, sans abonnement, sans envoi de vos enregistrements avec les modèles locaux',
    preview:'Voir l’interface ScribeWatch en grand',
    screenshot:'Capture réelle de l’application — données de démonstration, jamais de patients',
    privacy:'Votre voix n’appartient qu’à vous.',
    privacyLead:'Avec la version complète et ses modèles installés sur votre ordinateur, les enregistrements et les textes sont traités sur place. Ils ne sont pas envoyés à un service distant pour être transcrits ou rédigés.',
    privacyNote:'Une connexion est nécessaire au premier lancement pour télécharger les modèles. Ensuite, la transcription locale fonctionne hors ligne. Si vous choisissez volontairement un service externe dans les réglages avancés, ses règles de confidentialité s’appliquent.',
    how:'De la parole à la page.',
    howLead:'Trois gestes suffisent pour reprendre le fil d’une conversation.',
    steps:[
      ['Enregistrez','Lancez REC, parlez, puis arrêtez. Vous pouvez aussi importer un fichier audio déjà enregistré.'],
      ['Retrouvez chaque mot','ScribeWatch transcrit l’audio en texte et conserve la transcription originale.'],
      ['Repartez avec un document','Choisissez un type de note : compte rendu de consultation, réunion, idées ou simple transcription. Exportez en PDF, Word ou texte.']
    ],
    useCases:'Fait pour vos vraies journées.',
    uses:[
      ['Consultations','Gardez les éléments abordés pendant un échange et préparez un compte rendu structuré. Une relecture par le professionnel reste indispensable.'],
      ['Notes vocales','Dites ce que vous avez en tête. Récupérez un texte que vous pouvez classer ou modifier.'],
      ['Réunions et entretiens','Conservez l’essentiel d’une discussion, les décisions et les choses à faire.']
    ],
    screens:'L’application, en vrai.',
    screensLead:'Aucune maquette : cliquez sur une image pour explorer l’interface en taille réelle.',
    shots:['Enregistrer ou importer un audio','Retrouver et modifier vos documents','Configurer vos préférences'],
    zoom:'Agrandir la capture',close:'Fermer la capture',previous:'Capture précédente',next:'Capture suivante',
    install:'Prêt à essayer ?',
    installLead:'Choisissez votre ordinateur. Pour la plupart des utilisateurs, prenez la version complète : elle inclut ce qu’il faut pour transcrire et créer des documents.',
    full:'Version complète',fullText:'Pour enregistrer, transcrire et créer des documents en local. Les modèles nécessaires sont téléchargés au premier lancement.',
    available:'Téléchargements officiels de la dernière version stable',
    downloads:{windows:'Windows',macos:'macOS',linux:'Linux'},
    fullTargets:{'windows-full':'Installer la version complète','mac-arm64-full':'Mac Apple Silicon — complète','mac-amd64-full':'Mac Intel — complète','linux-amd64-full':'Linux x86-64 — complète','linux-arm64-full':'Linux ARM64 — complète','linux-amd64-deb':'Debian / Ubuntu — paquet .deb','linux-arm64-deb':'Debian / Ubuntu ARM64 — paquet .deb'},
    fallback:'Si la version complète de votre système n’est pas encore proposée dans la dernière release, vous pouvez consulter les éditions avancées ci-dessous.',
    advanced:'Déjà équipé d’un moteur de transcription ? Versions légères pour utilisateurs avancés',
    advancedLead:'Ces éditions n’installent pas de moteur de transcription ni de rédaction. Elles demandent un service compatible que vous avez déjà configuré. Elles ne sont pas le meilleur choix pour commencer.',
    lightTargets:{'windows-light':'Windows — légère','mac-arm64':'macOS Apple Silicon — légère','mac-amd64':'macOS Intel — légère','linux-amd64':'Linux x86-64 — légère','linux-arm64':'Linux ARM64 — légère'},
    source:'Voir toutes les versions sur GitHub',
    requirements:'De quoi a besoin votre ordinateur ?',
    requirementsLead:'Aucun abonnement ni serveur à installer pour l’édition complète. Voici des repères pratiques, pas une garantie de performance sur tous les appareils.',
    need:[
      ['Pour transcrire','Un ordinateur 64 bits récent, un microphone si vous enregistrez en direct et de l’espace libre pour télécharger le moteur vocal.'],
      ['Pour créer des comptes rendus','16 Go de mémoire conseillés pour travailler confortablement avec le modèle local de rédaction. Sur une machine moins puissante, les traitements peuvent être plus lents.'],
      ['Pour aller plus vite','Une carte graphique compatible peut accélérer le traitement. Sans carte graphique dédiée, l’application peut utiliser le processeur, avec des délais plus longs.']
    ],
    requirementsNote:'Prévoyez plusieurs gigaoctets libres pour les moteurs et leurs fichiers. La quantité exacte dépend de votre système et des modèles choisis. Les fichiers PDF, Word et texte sont exportés sans installer Pandoc ou LibreOffice dans les nouvelles éditions complètes.',
    questions:'Les réponses, simplement.',
    faq:[
      ['Est-ce que mes audios partent sur Internet ?','Non, lorsque vous utilisez les moteurs locaux de la version complète : l’audio et le texte sont traités sur votre appareil. Le téléchargement initial des modèles nécessite Internet ; les services externes restent un choix avancé.'],
      ['Faut-il connaître l’intelligence artificielle ?','Non. Installez la version complète, laissez l’application préparer les modèles au premier lancement, puis enregistrez ou importez votre audio.'],
      ['Et si mon ordinateur n’a pas de grosse carte graphique ?','Le traitement peut fonctionner sur le processeur, mais il sera plus lent. Pour la rédaction de comptes rendus, 16 Go de RAM sont conseillés.'],
      ['Puis-je l’utiliser pour une consultation médicale ?','Oui, il existe des profils pour les consultations dentaires et médicales. Le texte produit est un brouillon à vérifier et corriger par le praticien avant utilisation clinique.'],
      ['Ai-je besoin de rester connecté ?','Une connexion est nécessaire la première fois pour récupérer les modèles. Avec les modèles locaux installés, les enregistrements et les comptes rendus peuvent ensuite être traités hors ligne.']
    ],
    caution:'Documents médicaux : une relecture professionnelle reste indispensable. La qualité dépend de l’enregistrement, de la langue et du matériel.',
    github:'Code source',releases:'Toutes les versions',attribution:'Projet indépendant · Libre et open source',copyright:'© ScribeWatch'
  },
  en:{
    title:'ScribeWatch — Your notes, without the cloud',
    description:'Record an appointment, voice note or meeting. Transcribe and create structured documents right on your computer, without uploading your recordings to a cloud.',
    nav:['Why it matters','How it works','Screenshots','Download'],
    kicker:'A notebook that listens',
    hero:'Say it out loud. Keep the notes, not a cloud copy.',
    sub:'Record an appointment, meeting or passing thought. Turn your words into searchable text and well-organized documents, right on your own computer.',
    button:'Get ScribeWatch',explore:'Explore the app',
    secure:'No account, no subscription, no recording uploads with local models',
    preview:'Open ScribeWatch screenshot at full size',
    screenshot:'Real app screenshot, demo content only — never patient data',
    privacy:'Your voice stays yours.',
    privacyLead:'With the complete edition and its locally installed models, your recordings and text are processed on your device. They are not sent to a remote service for transcription or writing.',
    privacyNote:'You need Internet once to download the models. After that, local processing works offline. If you intentionally configure an external provider in advanced settings, that provider’s privacy terms apply.',
    how:'From speaking to keeping.',
    howLead:'Three simple steps to turn a conversation into something you can use.',
    steps:[
      ['Record','Press REC, speak, and stop. Or import an audio file you already have.'],
      ['Get the words','ScribeWatch transcribes the audio into text and preserves the original transcript.'],
      ['Take away a document','Pick a note type: consultation, meeting, ideas or plain transcript. Export as PDF, Word or text.']
    ],
    useCases:'For the things you actually do.',
    uses:[
      ['Appointments','Keep the points covered in a conversation and draft a structured report. A professional must review medical reports.'],
      ['Voice memos','Speak your thoughts. Get editable text you can find and organize later.'],
      ['Meetings and interviews','Keep a readable record of discussions, decisions and follow-up actions.']
    ],
    screens:'A look inside.',
    screensLead:'Not mockups. Click any screenshot to inspect the real interface at full size.',
    shots:['Record or import an audio file','Review and edit your documents','Make it yours in Settings'],
    zoom:'Enlarge screenshot',close:'Close screenshot',previous:'Previous screenshot',next:'Next screenshot',
    install:'Ready to use it?',
    installLead:'Pick your computer. Most people should choose the complete edition: it includes everything needed to transcribe and write locally.',
    full:'Complete edition',fullText:'Record, transcribe and create documents locally. The required models download on first launch.',
    available:'Official downloads from the latest stable release',
    downloads:{windows:'Windows',macos:'macOS',linux:'Linux'},
    fullTargets:{'windows-full':'Download complete edition','mac-arm64-full':'Apple Silicon — complete','mac-amd64-full':'Intel Mac — complete','linux-amd64-full':'Linux x86-64 — complete','linux-arm64-full':'Linux ARM64 — complete','linux-amd64-deb':'Debian / Ubuntu — .deb package','linux-arm64-deb':'Debian / Ubuntu ARM64 — .deb package'},
    fallback:'If the complete build for your system is not yet listed in the latest release, see the advanced editions below.',
    advanced:'Already running your own speech engine? Lightweight builds for advanced users',
    advancedLead:'These editions do not install a transcription or writing engine. They require an existing compatible service you have already configured and are not recommended for first-time users.',
    lightTargets:{'windows-light':'Windows — lightweight','mac-arm64':'Apple Silicon — lightweight','mac-amd64':'Intel Mac — lightweight','linux-amd64':'Linux x86-64 — lightweight','linux-arm64':'Linux ARM64 — lightweight'},
    source:'Browse all GitHub releases',
    requirements:'What does your computer need?',
    requirementsLead:'No subscription or separate server for the complete edition. These are practical guidelines, not guaranteed minimums for every system.',
    need:[
      ['To transcribe','A recent 64-bit computer, microphone for live recording and available disk space for the speech model.'],
      ['To write structured reports','16 GB RAM is recommended for comfortable use of the local writing model. Lower-spec computers may be slower.'],
      ['To make it faster','A compatible graphics card can speed things up. Without a dedicated GPU, the app can fall back to the processor, which takes longer.']
    ],
    requirementsNote:'Keep several gigabytes of storage free for the models and their files. Exact requirements depend on your system and selected models. New complete editions include document exporters without installing Pandoc or LibreOffice separately.',
    questions:'A few simple answers.',
    faq:[
      ['Does my audio go to the Internet?','Not when you use the local models in the complete edition. Audio and text are processed on your device. The first model download needs Internet; external providers remain an optional advanced choice.'],
      ['Do I need to understand AI?','No. Install the complete edition, let it set up the local models on first launch, then record or import audio.'],
      ['What if I do not have a powerful graphics card?','The processor can do the work, only more slowly. For structured reports, 16 GB RAM is recommended.'],
      ['Can I use it for medical consultations?','Yes. There are dental and medical templates. Every generated report is a draft requiring professional review and correction before clinical use.'],
      ['Does it work without Wi-Fi?','The first run needs Internet to download the models. Once those are installed, local recordings and reports can be processed offline.']
    ],
    caution:'Medical records require professional review. Output quality depends on recording quality, language and hardware.',
    github:'Source code',releases:'All releases',attribution:'Independent project · Free and open source',copyright:'© ScribeWatch'
  },
  'zh-CN':{
    title:'ScribeWatch — 笔记留在电脑里',
    description:'在本机录音、转写和整理就诊、语音笔记或会议内容。使用本地模型时，不需将录音上传云端。',
    nav:['为什么选它','使用方法','界面预览','下载安装'],
    kicker:'会听写的笔记本',
    hero:'说出想法。留下文字，不留下云端副本。',
    sub:'录制问诊、会议和生活中的灵感。在自己的电脑上把录音转成文字，整理成清晰的文档。',
    button:'下载 ScribeWatch',explore:'了解应用',
    secure:'本地模式无需账号、无需订阅、无需上传录音',
    preview:'放大查看 ScribeWatch 界面',screenshot:'真实程序截图，仅使用演示数据，不包含患者资料',
    privacy:'你的声音，留在你的设备上。',
    privacyLead:'完整版本在你的电脑上运行录音转写和文档整理模型，处理内容不必发送到远程服务。',
    privacyNote:'首次启动需要联网下载模型，完成后可离线处理。如果你在高级设置中选择外部服务，则适用该服务的隐私政策。',
    how:'从录音到文档，只需三步。',howLead:'不用反复听录音，也能找回一段对话的重要内容。',
    steps:[['开始录音','点击 REC 开始和结束录音，也可以导入已有音频。'],['查看文字','应用转写录音，并保留完整原始转录。'],['导出文档','选择问诊、会议、头脑风暴或原文模式，再导出 PDF、Word 或文本。']],
    useCases:'适合日常工作与生活。',
    uses:[['就诊记录','整理咨询中的要点。医疗内容在使用前必须由专业人员审核。'],['语音备忘录','随时说出想法，得到可以编辑和归档的文字。'],['会议与访谈','整理讨论、决策和后续事项。']],
    screens:'实际应用界面。',screensLead:'不是设计稿。点击任意截图即可放大查看。',shots:['录制或导入音频','查看和编辑文档','调整个人设置'],zoom:'放大截图',close:'关闭截图',previous:'上一张截图',next:'下一张截图',
    install:'准备开始？',installLead:'选择你的电脑。大多数用户适合完整版本：无需自己配置转写和文档整理服务。',
    full:'完整版本',fullText:'在本机录音、转写和整理文档。第一次启动时自动下载所需模型。',
    available:'最新稳定版的官方安装包',downloads:{windows:'Windows',macos:'macOS',linux:'Linux'},
    fullTargets:{'windows-full':'下载安装完整版本','mac-arm64-full':'Apple Silicon — 完整版','mac-amd64-full':'Intel Mac — 完整版','linux-amd64-full':'Linux x86-64 — 完整版','linux-arm64-full':'Linux ARM64 — 完整版','linux-amd64-deb':'Debian / Ubuntu — .deb 安装包','linux-arm64-deb':'Debian / Ubuntu ARM64 — .deb 安装包'},
    fallback:'如果最新版本暂未提供适合你的完整安装包，可以查看下方高级版本。',
    advanced:'已经配置转写服务？高级用户可选轻量版本',
    advancedLead:'轻量版不自带录音转写或文档整理引擎，需要自行配置兼容服务，不建议普通用户首次使用。',
    lightTargets:{'windows-light':'Windows — 轻量版','mac-arm64':'Apple Silicon — 轻量版','mac-amd64':'Intel Mac — 轻量版','linux-amd64':'Linux x86-64 — 轻量版','linux-arm64':'Linux ARM64 — 轻量版'},
    source:'查看 GitHub 上的所有版本',
    requirements:'需要怎样的电脑？',requirementsLead:'完整版本无需订阅或单独配置服务器。以下为一般建议，并非所有设备的保证最低配置。',
    need:[['转录音频','较新的 64 位电脑、录音用麦克风以及下载语音模型的磁盘空间。'],['生成结构化报告','建议使用 16 GB 内存。配置较低的设备可能运行更慢。'],['提高速度','兼容的显卡可以加速，没有独立显卡时也可以使用处理器，但耗时更长。']],
    requirementsNote:'请为模型预留数 GB 磁盘空间，实际大小取决于系统和所选模型。新版完整安装包内置文档导出工具，无需自行安装 Pandoc 或 LibreOffice。',
    questions:'常见问题。',
    faq:[['音频会上传云端吗？','使用完整版的本地模型时不会。首次下载模型需要联网，外部服务仅作为高级可选配置。'],['需要了解人工智能吗？','不需要。安装完整版，首次启动等待模型下载完成后即可录音或导入音频。'],['没有强大的显卡怎么办？','可以使用处理器，但速度较慢。整理结构化报告建议 16 GB 内存。'],['可以用于医疗咨询吗？','有牙科和医疗报告模板，但生成内容必须由专业人员审核修改后才能用于临床。'],['离线可以使用吗？','首次需要联网下载模型，之后即可在本机离线处理录音和文档。']],
    caution:'医疗报告必须由专业人员审核。结果质量受录音清晰度、语言和设备性能影响。',
    github:'源代码',releases:'所有版本',attribution:'独立项目 · 免费开源',copyright:'© ScribeWatch'
  }
};

const esc=(s)=>String(s).replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
const link=(href,label,attrs='')=>'<a href="'+href+'" '+attrs+'>'+esc(label)+'</a>';
function render(lang,t){
  const url='https://scribewatch.arezkichougar.com/'+lang+'/';
  const screen=(name,alt,priority=false)=>'<img '+(priority?'fetchpriority="high"':'loading="lazy"')+' src="/assets/screens/'+lang+'-'+name+'.png" alt="'+esc(alt)+'" width="1440" height="900">';
  const zoomButton=(name,label,extra='')=>'<button type="button" class="shot-zoom '+extra+'" data-zoom="'+name+'" aria-label="'+esc(t.zoom)+': '+esc(label)+'">'+screen(name,label)+'<span class="zoom-label">'+esc(t.zoom)+' ↗</span></button>';
  const primary=['windows-full','mac-arm64-full','mac-amd64-full','linux-amd64-full','linux-arm64-full','linux-amd64-deb','linux-arm64-deb'];
  const lite=['windows-light','mac-arm64','mac-amd64','linux-amd64','linux-arm64'];
  const full=(key)=>'<a class="asset asset-primary" data-download="'+key+'" hidden href="'+liveRelease+'">'+esc(t.fullTargets[key])+' <span data-size="'+key+'"></span><span aria-hidden="true">↗</span></a>';
  const light=(key)=>'<a class="asset asset-light" data-download="'+key+'" hidden href="'+liveRelease+'">'+esc(t.lightTargets[key])+' <span data-size="'+key+'"></span></a>';
  return [
'<!doctype html><html lang="'+lang+'"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">',
'<title>'+esc(t.title)+'</title><meta name="description" content="'+esc(t.description)+'">',
'<meta name="theme-color" content="#f8f9f9"><meta name="color-scheme" content="light"><meta name="robots" content="index,follow">',
'<link rel="canonical" href="'+url+'"><link rel="icon" href="/assets/logo.svg" type="image/svg+xml">',
...['en','fr','zh-CN'].map(l=>'<link rel="alternate" hreflang="'+l+'" href="https://scribewatch.arezkichougar.com/'+l+'/">'),
'<link rel="alternate" hreflang="x-default" href="https://scribewatch.arezkichougar.com/en/">',
'<meta property="og:type" content="website"><meta property="og:site_name" content="ScribeWatch">',
'<meta property="og:title" content="'+esc(t.title)+'"><meta property="og:description" content="'+esc(t.description)+'"><meta property="og:url" content="'+url+'">',
'<meta property="og:image" content="https://scribewatch.arezkichougar.com/assets/screens/'+lang+'-home.png"><meta name="twitter:card" content="summary_large_image">',
'<link rel="stylesheet" href="/assets/style.css"><script defer src="/assets/client.js"></script></head><body>',
'<a class="skip" href="#main">Skip to content</a><header class="site-header"><div class="wrap top">',
'<a class="brand" href="/'+lang+'/"><img src="/assets/logo.svg" width="38" height="38" alt="">ScribeWatch</a>',
'<nav class="nav" aria-label="Main menu">'+t.nav.map((label,i)=>link(['#privacy','#how','#screens','#downloads'][i],label)).join('')+'</nav>',
'<div class="languages" aria-label="Languages">'+['en','fr','zh-CN'].map(l=>link('/'+l+'/',{en:'EN',fr:'FR','zh-CN':'中文'}[l],'lang="'+l+'" hreflang="'+l+'" '+(l===lang?'aria-current="page"':'') )).join('')+'</div>',
'</div></header><main id="main">',
'<section class="hero"><div class="wrap hero-layout"><div class="hero-copy">',
'<p class="eyebrow">'+esc(t.kicker)+'</p><h1>'+esc(t.hero)+'</h1><p class="hero-summary">'+esc(t.sub)+'</p>',
'<div class="hero-actions"><a id="smart-download" class="button button-primary" href="#downloads">'+esc(t.button)+'</a><a class="button button-text" href="#screens">'+esc(t.explore)+' <span aria-hidden="true">↗</span></a></div>',
'<p class="hero-footnote">'+esc(t.secure)+'</p><span id="platform-note" class="sr-only"></span>',
'</div><div class="hero-media"><div class="preview"><div class="preview-bar"><span class="window-dots" aria-hidden="true"><b></b><b></b><b></b></span><small>ScribeWatch</small><span class="record-indicator" aria-hidden="true"></span></div>',
zoomButton('home',t.preview,'preview-shot')+'</div><p class="preview-caption">'+esc(t.screenshot)+'</p></div></div></section>',
'<section class="privacy" id="privacy"><div class="wrap privacy-grid"><div><span class="section-number">01 / '+esc(t.nav[0])+'</span><h2>'+esc(t.privacy)+'</h2></div><div><p class="privacy-lead">'+esc(t.privacyLead)+'</p><p class="privacy-note">'+esc(t.privacyNote)+'</p></div></div></section>',
'<section class="section" id="how"><div class="wrap"><div class="section-intro"><span class="section-number">02 / '+esc(t.nav[1])+'</span><h2>'+esc(t.how)+'</h2><p>'+esc(t.howLead)+'</p></div>',
'<ol class="steps">'+t.steps.map(([heading,body],i)=>'<li class="step"><span class="step-number">'+String(i+1).padStart(2,'0')+'</span><h3>'+esc(heading)+'</h3><p>'+esc(body)+'</p></li>').join('')+'</ol>',
'<div class="usecases"><h3>'+esc(t.useCases)+'</h3><div class="uses">'+t.uses.map(([heading,body])=>'<article><h4>'+esc(heading)+'</h4><p>'+esc(body)+'</p></article>').join('')+'</div></div></div></section>',
'<section class="section section-gallery" id="screens"><div class="wrap"><div class="section-intro"><span class="section-number">03 / '+esc(t.nav[2])+'</span><h2>'+esc(t.screens)+'</h2><p>'+esc(t.screensLead)+'</p></div><div class="gallery">',
...['record','documents','settings'].map((name,i)=>'<figure class="gallery-item '+(i===0?'gallery-main':'')+'">'+zoomButton(name,t.shots[i])+'<figcaption>'+esc(t.shots[i])+'</figcaption></figure>'),
'</div></div></section>',
'<section class="section section-install" id="downloads"><div class="wrap"><div class="section-intro"><span class="section-number">04 / '+esc(t.nav[3])+'</span><h2>'+esc(t.install)+'</h2><p>'+esc(t.installLead)+'</p></div>',
'<div class="install-panel"><div class="install-panel-header"><div><span class="recommendation">'+esc(t.full)+'</span><h3>'+esc(t.fullText)+'</h3></div><p><span id="release-tag">v0.6.2</span><br>'+esc(t.available)+'</p></div>',
'<div class="platform-grid">',
'<article data-platform="windows"><h4>'+esc(t.downloads.windows)+'</h4>'+full('windows-full')+'</article>',
'<article data-platform="macos"><h4>'+esc(t.downloads.macos)+'</h4>'+full('mac-arm64-full')+full('mac-amd64-full')+'</article>',
'<article data-platform="linux"><h4>'+esc(t.downloads.linux)+'</h4>'+full('linux-amd64-full')+full('linux-arm64-full')+full('linux-amd64-deb')+full('linux-arm64-deb')+'</article>',
'</div><p class="fallback">'+esc(t.fallback)+'</p></div>',
'<details class="advanced"><summary>'+esc(t.advanced)+'<span aria-hidden="true">+</span></summary><div class="advanced-inner"><p>'+esc(t.advancedLead)+'</p><div class="advanced-assets">'+lite.map(light).join('')+'</div></div></details>',
'<p class="all-releases">'+link(liveRelease,t.source)+'</p></div></section>',
'<section class="section requirements" id="requirements"><div class="wrap"><div class="section-intro"><span class="section-number">05 / '+esc(t.requirements)+'</span><h2>'+esc(t.requirements)+'</h2><p>'+esc(t.requirementsLead)+'</p></div><div class="requirements-grid">'+t.need.map(([name,body])=>'<article><h3>'+esc(name)+'</h3><p>'+esc(body)+'</p></article>').join('')+'</div><p class="requirements-note">'+esc(t.requirementsNote)+'</p></div></section>',
'<section class="section section-faq"><div class="wrap faq-layout"><div><span class="section-number">06 / FAQ</span><h2>'+esc(t.questions)+'</h2></div><div class="faqs">'+t.faq.map(([q,a])=>'<details><summary>'+esc(q)+'<span aria-hidden="true">+</span></summary><p>'+esc(a)+'</p></details>').join('')+'</div></div></section>',
'<div class="clinical-note"><div class="wrap">'+esc(t.caution)+'</div></div>',
'</main><dialog id="image-dialog" aria-label="'+esc(t.zoom)+'"><div class="dialog-controls"><span>ScribeWatch</span><button type="button" data-close aria-label="'+esc(t.close)+'">✕</button></div><img id="dialog-image" alt=""><div class="dialog-bottom"><button type="button" data-prev>'+esc(t.previous)+'</button><span id="dialog-caption"></span><button type="button" data-next>'+esc(t.next)+'</button></div></dialog>',
'<footer class="site-footer"><div class="wrap footer-inner"><div><strong>ScribeWatch</strong><p>'+esc(t.attribution)+'</p></div><div>'+link('https://github.com/GodsQuantum/ScribeWatch',t.github)+link(liveRelease,t.releases)+'</div></div><div class="wrap footer-credit"><small>Design review guided by <a href="https://github.com/heydamir/unslop" rel="noreferrer">Unslop</a>, CC BY 4.0 · Damir Sokolovsky</small></div></footer>',
'</body></html>'
].join('\n');
}
fs.mkdirSync(pub,{recursive:true});
fs.mkdirSync(path.join(pub,'assets','screens'),{recursive:true});
fs.copyFileSync(path.join(app,'docs/logo.svg'),path.join(pub,'assets/logo.svg'));
fs.copyFileSync(path.join(root,'src/style.css'),path.join(pub,'assets/style.css'));
fs.copyFileSync(path.join(root,'src/client.js'),path.join(pub,'assets/client.js'));
fs.writeFileSync(path.join(pub,'assets/release-map.json'),JSON.stringify(Object.fromEntries(Object.entries(ASSETS).map(([k,rx])=>[k,rx.source]))));
for(const [lang,t] of Object.entries(locales)){
  const dir=path.join(pub,lang);
  fs.mkdirSync(dir,{recursive:true});
  fs.writeFileSync(path.join(dir,'index.html'),render(lang,t));
  for(const screen of ['home','record','documents','settings']){
    const src=path.join(app,'docs/screenshots',lang,screen+'.png');
    const bytes=fs.readFileSync(src);
    if(bytes.subarray(1,4).toString()!=='PNG'||bytes.byteLength<10000)throw new Error('Missing real screenshot: '+src);
    fs.writeFileSync(path.join(pub,'assets','screens',lang+'-'+screen+'.png'),bytes);
  }
}
fs.writeFileSync(path.join(pub,'index.html'),'<html lang="en"><head><meta charset="utf-8"><meta name="robots" content="index,follow"><title>ScribeWatch</title><script>(function(){const l=(navigator.language||"").toLowerCase();location.replace(l.startsWith("fr")?"/fr/":l.startsWith("zh")?"/zh-CN/":"/en/");})()</script></head><body><noscript><a href="/en/">English</a> · <a href="/fr/">Français</a> · <a href="/zh-CN/">中文</a></noscript></body></html>');
fs.writeFileSync(path.join(pub,'robots.txt'),'User-agent: *\nAllow: /\nSitemap: https://scribewatch.arezkichougar.com/sitemap.xml\n');
fs.writeFileSync(path.join(pub,'sitemap.xml'),'<?xml version="1.0" encoding="UTF-8"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">'+Object.keys(locales).map(l=>'<url><loc>https://scribewatch.arezkichougar.com/'+l+'/</loc></url>').join('')+'</urlset>');
console.log('BUILT',Object.keys(locales).join(','),'real screenshots',Object.keys(locales).length*4,'source version',sourceVersion);
