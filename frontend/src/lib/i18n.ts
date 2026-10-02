import { writable, get } from 'svelte/store';

export type Locale = 'en' | 'fr' | 'zh-CN';

const dictionaries: Record<Exclude<Locale, 'en'>, Record<string, string>> = {
  fr: {
    'Home':'Accueil','Quick':'Rapide','Live':'Direct','Workflows':'Automatisations','AI Profiles':'Profils IA','Jobs':'Tâches',
    'audio → knowledge':'audio → connaissances','Transcript is canonical.':'La transcription est canonique.',
    'AI structure is optional and never replaces source transcription.':"La structure IA est facultative et ne remplace jamais la transcription source.",
    'Language':'Langue','Interface language':'Langue de l’interface',
    'Cannot reach ScribeWatch.':'ScribeWatch est inaccessible.','Retry':'Réessayer','Loading ScribeWatch…':'Chargement de ScribeWatch…',
    'SELF-HOSTED AUDIO NOTES':'NOTES AUDIO AUTO-HÉBERGÉES',
    'Turn voice notes into Markdown before you forget them.':'Transformez vos notes vocales en documents avant de les oublier.',
    'ScribeWatch listens to your folders — or one file right now — and turns audio into titled, Obsidian-ready notes using your own transcription provider.':
      'ScribeWatch surveille vos dossiers — ou traite un fichier immédiatement — et transforme l’audio en notes titrées prêtes pour Obsidian avec votre propre moteur de transcription.',
    'Quick Transcribe':'Transcription rapide','Set up a watch folder':'Configurer un dossier surveillé',
    'No LLM required':'Aucun LLM requis','OpenAI-compatible':'Compatible OpenAI','Original audio stays safe':'Audio original préservé',
    'WATCHING':'SURVEILLANCE','NOW':'EN COURS','NOTES':'NOTES','ATTENTION':'ATTENTION','active workflows':'automatisations actives','processing':'en traitement','completed':'terminées','failed':'échouées',
    'What’s happening':'Activité','Live transcription activity':'Activité de transcription','All jobs →':'Toutes les tâches →',
    'Quiet right now.':'Aucune activité.','Drop a voice note into Quick Transcribe or an enabled watch folder.':'Ajoutez une note audio dans Transcription rapide ou dans un dossier surveillé actif.',
    'Recent notes':'Notes récentes','Latest Markdown published':'Derniers documents publiés','Your first finished note will appear here.':'Votre première note terminée apparaîtra ici.',
    'ONE FILE · RIGHT NOW':'UN FICHIER · MAINTENANT','Turn one recording into a clean Markdown note without creating a workflow.':'Transformez un enregistrement en note propre sans créer d’automatisation.',
    'Add a transcription provider first.':'Ajoutez d’abord un fournisseur de transcription.','Choose your audio':'Choisir l’audio',
    'Upload from this computer or use a file already mounted on the server.':'Importez depuis cet ordinateur ou utilisez un fichier déjà monté sur le serveur.',
    'This computer':'Cet ordinateur','Server':'Serveur','Drop an audio note here':'Déposez une note audio ici','or click to choose a file from this computer':'ou cliquez pour choisir un fichier sur cet ordinateur',
    'Server audio file':'Fichier audio serveur','Choose an allowed server file':'Choisir un fichier serveur autorisé','Browse':'Parcourir',
    'Only explicitly mounted and allowed server roots are visible.':'Seules les racines serveur explicitement montées et autorisées sont visibles.',
    'Transcription':'Transcription','Pick an exact provider + model chain. Failed routes automatically fall through.':'Choisissez une chaîne précise fournisseur + modèle. En cas d’échec, la route suivante est essayée.',
    'AI structure':'Structure IA','optional':'facultatif','None — transcript only':'Aucune — transcription uniquement','Adds a structured view and always preserves the full transcript.':'Ajoute une vue structurée tout en conservant toujours la transcription complète.',
    'Advanced':'Avancé','YAML / Obsidian properties':'Propriétés YAML / Obsidian','Readable deterministic paragraphs':'Paragraphes déterministes lisibles',
    'Uses Unicode sentence boundaries and length rules only. No LLM rewrites the transcript.':'Utilise uniquement les limites de phrases Unicode et des règles de longueur. Aucun LLM ne réécrit la transcription.',
    'Save the note':'Enregistrer la note','Publish on the server or bring the Markdown back to this computer.':'Publiez sur le serveur ou récupérez le document sur cet ordinateur.',
    'Server folder':'Dossier serveur','Markdown destination':'Destination du document','Choose a notes folder':'Choisir un dossier de notes',
    'Save directly into a local folder':'Enregistrer directement dans un dossier local','Download the .md file':'Télécharger le fichier .md',
    'Transcribe to Markdown →':'Transcrire →','Uploading…':'Envoi…','Result':'Résultat','Your note appears here.':'Votre note apparaît ici.',
    'The title comes from the first useful words of the transcription — ready for Obsidian, any Markdown vault, or plain files.':'Le titre vient des premiers mots utiles de la transcription — prêt pour Obsidian, tout coffre Markdown ou des fichiers classiques.',
    'Transcription did not finish':'La transcription n’a pas abouti','Markdown ready':'Document prêt','Save Markdown…':'Enregistrer le Markdown…','Download Markdown':'Télécharger le Markdown',
    'Transcribe another':'Transcrire un autre fichier','Listening…':'Transcription…','Structuring with AI…':'Structuration par IA…','Writing Markdown…':'Écriture du document…','Queued…':'En attente…',
    'Choose server audio':'Choisir un audio serveur','Choose Markdown folder':'Choisir le dossier de notes',
    'BROWSER MICROPHONE':'MICROPHONE DU NAVIGATEUR','Live Recorder':'Enregistreur en direct',
    'Record from a browser microphone, stop when you are done, then run the exact same resilient transcription and optional AI-structure pipeline as a watched file.':
      'Enregistrez depuis le microphone du navigateur puis utilisez la même chaîne robuste de transcription et de structuration IA facultative qu’un fichier surveillé.',
    'Microphone':'Microphone','Ready to record':'Prêt à enregistrer','Microphone unavailable':'Microphone indisponible','Requesting microphone…':'Autorisation du microphone…','Uploading recording…':'Envoi de l’enregistrement…','Recording — press again to stop':'Enregistrement — appuyez à nouveau pour arrêter',
    'The recording is uploaded only after Stop, then the temporary audio is deleted after processing.':'L’enregistrement n’est envoyé qu’après Arrêter, puis l’audio temporaire est supprimé après traitement.',
    'Your audio stays in this browser until you stop.':'Votre audio reste dans ce navigateur jusqu’à l’arrêt.',
    'Request microphone again':'Redemander l’accès au microphone','Enable microphone':'Activer le microphone','Refresh microphones':'Actualiser les microphones',
    'Microphone access requires HTTPS (or localhost). Open ScribeWatch through its HTTPS address.':'L’accès au microphone nécessite HTTPS (ou localhost). Ouvrez ScribeWatch via son adresse HTTPS.',
    'Microphone enumeration is unavailable on this browser origin. Use HTTPS or local HTTP mode at http://127.0.0.1:3052.':'L’énumération des microphones est indisponible sur cette origine. Utilisez HTTPS ou le mode HTTP local sur http://127.0.0.1:3052.',
    'Microphone capture is unavailable on this browser origin. Use HTTPS or local HTTP mode at http://127.0.0.1:3052.':'La capture du microphone est indisponible sur cette origine. Utilisez HTTPS ou le mode HTTP local sur http://127.0.0.1:3052.',
    'Microphone permission is blocked. Allow it in the browser site permissions, then retry.':'L’autorisation du microphone est bloquée. Autorisez-la dans les permissions du site puis réessayez.',
    'No audio input device is currently visible to the browser.':'Aucun périphérique d’entrée audio n’est actuellement visible par le navigateur.',
    'Choose a transcription provider and model first.':'Choisissez d’abord un fournisseur et un modèle de transcription.',
    'Choose where to save the recording result first.':'Choisissez d’abord où enregistrer le résultat.',
    'Permission':'Autorisation','Secure context':'Contexte sécurisé','Devices':'Périphériques','granted':'accordée','prompt':'à demander','denied':'refusée','unknown':'inconnue','yes':'oui','no':'non',
    'None — deterministic transcript only':'Aucune — transcription déterministe uniquement','Destination':'Destination','YAML properties':'Propriétés YAML',
    'Recording result':'Résultat de l’enregistrement','Press the record button.':'Appuyez sur le bouton d’enregistrement.','Processing did not finish':'Le traitement n’a pas abouti','Note ready':'Note prête',
    'Export result':'Exporter le résultat','Download / save as':'Télécharger / enregistrer en',
    'HISTORY':'HISTORIQUE','Every automatic and one-off transcription keeps a durable lifecycle, including retries, fallback attempts and restart recovery.':'Chaque transcription automatique ou ponctuelle conserve un historique durable, avec réessais, routes de secours et récupération après redémarrage.',
    'All':'Toutes','Current':'En cours','Errors':'Erreurs','Done':'Terminées','No jobs in this view.':'Aucune tâche dans cette vue.','Source':'Source','Archive':'Archive','Processing error':'Erreur de traitement','AI STRUCTURED':'STRUCTURÉ PAR IA',
    'Transcript published without AI structure.':'Transcription publiée sans structure IA.','Transcription attempts':'Tentatives de transcription','Cancel':'Annuler','Remove record':'Supprimer l’historique',
    'OPTIONAL AI POST-PROCESSING':'POST-TRAITEMENT IA FACULTATIF','AI structure profiles':'Profils de structure IA',
    'Keep transcription deterministic, then optionally create a second structured view with an OpenAI-compatible LLM. The canonical transcript is always preserved.':'Conservez une transcription déterministe puis créez éventuellement une seconde vue structurée avec un LLM compatible OpenAI. La transcription canonique est toujours préservée.',
    'TRANSCRIPT FIRST':'TRANSCRIPTION D’ABORD','LLM providers':'Fournisseurs LLM','Structure profiles':'Profils de structure','Save provider':'Enregistrer le fournisseur','Discover models':'Découvrir les modèles','Delete':'Supprimer','Name':'Nom','Default model':'Modèle par défaut','API key':'Clé API','Provider enabled':'Fournisseur actif','Model':'Modèle','Structure prompt':'Prompt de structure','Save profile':'Enregistrer le profil','Custom profile':'Profil personnalisé',
    'Transcription providers':'Fournisseurs de transcription','New provider':'Nouveau fournisseur','Edit provider':'Modifier le fournisseur','Secret stored':'Secret enregistré','Transcription URL':'URL de transcription',
    'AUTOMATION':'AUTOMATISATION','Add workflow':'Ajouter une automatisation','New workflow':'Nouvelle automatisation','Edit workflow':'Modifier l’automatisation','Watch folder':'Dossier surveillé','Markdown folder':'Dossier de notes','Audio archive':'Archive audio','Scan now':'Analyser maintenant','Save workflow':'Enregistrer l’automatisation',
    'Choose folder':'Choisir un dossier','Filter folders':'Filtrer les dossiers','Filter audio files':'Filtrer les fichiers audio','Refresh':'Actualiser','Loading…':'Chargement…','Nothing matching here.':'Aucun résultat ici.','Back':'Retour','Open':'Ouvrir','Choose':'Choisir','Only configured allowed roots are visible.':'Seules les racines autorisées configurées sont visibles.','Choose this folder':'Choisir ce dossier',
    'Transcription chain':'Chaîne de transcription','Choose the exact provider + model order. If one route fails, ScribeWatch tries the next.':'Choisissez l’ordre exact fournisseur + modèle. Si une route échoue, ScribeWatch essaie la suivante.','Primary':'Principal','Provider':'Fournisseur','Choose provider':'Choisir un fournisseur','Choose model':'Choisir un modèle','Fallback after':'Bascule après','Never':'Jamais','Custom':'Personnalisé','Add fallback':'Ajouter une route de secours'
  },
  'zh-CN': {
    'Home':'首页','Quick':'快速','Live':'实时','Workflows':'工作流','AI Profiles':'AI 配置','Jobs':'任务',
    'audio → knowledge':'音频 → 知识','Transcript is canonical.':'转写文本为权威原文。','AI structure is optional and never replaces source transcription.':'AI 结构化是可选的，绝不替代原始转写。',
    'Language':'语言','Interface language':'界面语言','Cannot reach ScribeWatch.':'无法连接 ScribeWatch。','Retry':'重试','Loading ScribeWatch…':'正在加载 ScribeWatch…',
    'SELF-HOSTED AUDIO NOTES':'自托管音频笔记','Turn voice notes into Markdown before you forget them.':'把语音笔记及时变成文档。',
    'ScribeWatch listens to your folders — or one file right now — and turns audio into titled, Obsidian-ready notes using your own transcription provider.':'ScribeWatch 可监视文件夹或立即处理单个文件，并使用您自己的转写服务把音频转换为可直接用于 Obsidian 的带标题笔记。',
    'Quick Transcribe':'快速转写','Set up a watch folder':'设置监视文件夹','No LLM required':'无需 LLM','OpenAI-compatible':'兼容 OpenAI','Original audio stays safe':'保留原始音频',
    'WATCHING':'监视中','NOW':'当前','NOTES':'笔记','ATTENTION':'需关注','active workflows':'个活动工作流','processing':'处理中','completed':'已完成','failed':'失败',
    'What’s happening':'当前活动','Live transcription activity':'实时转写活动','All jobs →':'全部任务 →','Quiet right now.':'当前无活动。','Drop a voice note into Quick Transcribe or an enabled watch folder.':'将语音笔记放入快速转写或已启用的监视文件夹。','Recent notes':'最近笔记','Latest Markdown published':'最近发布的文档','Your first finished note will appear here.':'第一条完成的笔记会显示在这里。',
    'ONE FILE · RIGHT NOW':'单个文件 · 立即处理','Turn one recording into a clean Markdown note without creating a workflow.':'无需创建工作流，即可将一段录音变成整洁笔记。','Add a transcription provider first.':'请先添加转写服务。',
    'Choose your audio':'选择音频','Upload from this computer or use a file already mounted on the server.':'从本机上传，或使用服务器已挂载的文件。','This computer':'此电脑','Server':'服务器','Drop an audio note here':'将音频笔记拖到这里','or click to choose a file from this computer':'或点击从本机选择文件',
    'Server audio file':'服务器音频文件','Choose an allowed server file':'选择允许的服务器文件','Browse':'浏览','Only explicitly mounted and allowed server roots are visible.':'只显示明确挂载并允许的服务器根目录。',
    'Transcription':'转写','Pick an exact provider + model chain. Failed routes automatically fall through.':'选择准确的服务商 + 模型链；失败时自动尝试下一条线路。','AI structure':'AI 结构化','optional':'可选','None — transcript only':'无 — 仅转写','Adds a structured view and always preserves the full transcript.':'添加结构化视图，同时始终保留完整转写。',
    'Advanced':'高级','YAML / Obsidian properties':'YAML / Obsidian 属性','Readable deterministic paragraphs':'可读的确定性分段','Uses Unicode sentence boundaries and length rules only. No LLM rewrites the transcript.':'仅使用 Unicode 句子边界和长度规则；LLM 不会改写转写内容。',
    'Save the note':'保存笔记','Publish on the server or bring the Markdown back to this computer.':'发布到服务器，或将文档保存到本机。','Server folder':'服务器文件夹','Markdown destination':'文档目标位置','Choose a notes folder':'选择笔记文件夹',
    'Save directly into a local folder':'直接保存到本地文件夹','Download the .md file':'下载 .md 文件','Transcribe to Markdown →':'开始转写 →','Uploading…':'正在上传…','Result':'结果','Your note appears here.':'您的笔记会显示在这里。','The title comes from the first useful words of the transcription — ready for Obsidian, any Markdown vault, or plain files.':'标题来自转写中最先出现的有效内容，可用于 Obsidian、Markdown 库或普通文件。',
    'Transcription did not finish':'转写未完成','Markdown ready':'文档已就绪','Save Markdown…':'保存 Markdown…','Download Markdown':'下载 Markdown','Transcribe another':'转写另一个文件','Listening…':'正在转写…','Structuring with AI…':'AI 正在结构化…','Writing Markdown…':'正在写入文档…','Queued…':'已排队…','Choose server audio':'选择服务器音频','Choose Markdown folder':'选择笔记文件夹',
    'BROWSER MICROPHONE':'浏览器麦克风','Live Recorder':'实时录音','Record from a browser microphone, stop when you are done, then run the exact same resilient transcription and optional AI-structure pipeline as a watched file.':'使用浏览器麦克风录音；停止后，将通过与监视文件相同的可靠转写和可选 AI 结构化流程处理。',
    'Microphone':'麦克风','Ready to record':'可以录音','Microphone unavailable':'麦克风不可用','Requesting microphone…':'正在请求麦克风权限…','Uploading recording…':'正在上传录音…','Recording — press again to stop':'正在录音 — 再按一次停止','The recording is uploaded only after Stop, then the temporary audio is deleted after processing.':'只有停止后才上传录音，处理完成后会删除临时音频。','Your audio stays in this browser until you stop.':'停止前音频只保留在此浏览器中。',
    'Request microphone again':'再次请求麦克风权限','Enable microphone':'启用麦克风','Refresh microphones':'刷新麦克风','Microphone access requires HTTPS (or localhost). Open ScribeWatch through its HTTPS address.':'麦克风访问需要 HTTPS（或 localhost）。请通过 ScribeWatch 的 HTTPS 地址打开。','Microphone enumeration is unavailable on this browser origin. Use HTTPS or local HTTP mode at http://127.0.0.1:3052.':'当前浏览器来源无法枚举麦克风。请使用 HTTPS，或通过 http://127.0.0.1:3052 使用本地 HTTP 模式。','Microphone capture is unavailable on this browser origin. Use HTTPS or local HTTP mode at http://127.0.0.1:3052.':'当前浏览器来源无法捕获麦克风。请使用 HTTPS，或通过 http://127.0.0.1:3052 使用本地 HTTP 模式。','Microphone permission is blocked. Allow it in the browser site permissions, then retry.':'麦克风权限已被阻止。请在浏览器网站权限中允许后重试。','No audio input device is currently visible to the browser.':'浏览器当前看不到任何音频输入设备。','Choose a transcription provider and model first.':'请先选择转写服务商和模型。','Choose where to save the recording result first.':'请先选择录音结果的保存位置。','Permission':'权限','Secure context':'安全上下文','Devices':'设备','granted':'已允许','prompt':'待询问','denied':'已拒绝','unknown':'未知','yes':'是','no':'否',
    'None — deterministic transcript only':'无 — 仅确定性转写','Destination':'目标位置','YAML properties':'YAML 属性','Recording result':'录音结果','Press the record button.':'按下录音按钮。','Processing did not finish':'处理未完成','Note ready':'笔记已就绪','Export result':'导出结果','Download / save as':'下载 / 另存为',
    'HISTORY':'历史','Every automatic and one-off transcription keeps a durable lifecycle, including retries, fallback attempts and restart recovery.':'每次自动或单次转写都会保留完整生命周期，包括重试、备用线路和重启恢复。','All':'全部','Current':'当前','Errors':'错误','Done':'完成','No jobs in this view.':'此视图中没有任务。','Source':'来源','Archive':'归档','Processing error':'处理错误','AI STRUCTURED':'AI 已结构化','Transcript published without AI structure.':'转写已发布，但未完成 AI 结构化。','Transcription attempts':'转写尝试','Cancel':'取消','Remove record':'删除记录',
    'OPTIONAL AI POST-PROCESSING':'可选 AI 后处理','AI structure profiles':'AI 结构化配置','Keep transcription deterministic, then optionally create a second structured view with an OpenAI-compatible LLM. The canonical transcript is always preserved.':'保持确定性转写，并可选择使用兼容 OpenAI 的 LLM 生成第二个结构化视图；始终保留权威转写。','TRANSCRIPT FIRST':'转写优先','LLM providers':'LLM 服务商','Structure profiles':'结构配置','Save provider':'保存服务商','Discover models':'发现模型','Delete':'删除','Name':'名称','Default model':'默认模型','API key':'API 密钥','Provider enabled':'启用服务商','Model':'模型','Structure prompt':'结构提示词','Save profile':'保存配置','Custom profile':'自定义配置',
    'Transcription providers':'转写服务商','New provider':'新建服务商','Edit provider':'编辑服务商','Secret stored':'密钥已保存','Transcription URL':'转写 URL',
    'AUTOMATION':'自动化','Add workflow':'添加工作流','New workflow':'新建工作流','Edit workflow':'编辑工作流','Watch folder':'监视文件夹','Markdown folder':'笔记文件夹','Audio archive':'音频归档','Scan now':'立即扫描','Save workflow':'保存工作流',
    'Choose folder':'选择文件夹','Filter folders':'筛选文件夹','Filter audio files':'筛选音频文件','Refresh':'刷新','Loading…':'加载中…','Nothing matching here.':'这里没有匹配项。','Back':'返回','Open':'打开','Choose':'选择','Only configured allowed roots are visible.':'只显示已配置允许的根目录。','Choose this folder':'选择此文件夹',
    'Transcription chain':'转写链','Choose the exact provider + model order. If one route fails, ScribeWatch tries the next.':'选择准确的服务商 + 模型顺序；如果一条线路失败，ScribeWatch 会尝试下一条。','Primary':'主线路','Provider':'服务商','Choose provider':'选择服务商','Choose model':'选择模型','Fallback after':'多久后切换','Never':'永不','Custom':'自定义','Add fallback':'添加备用线路'
  }
};


const supplemental: Record<Exclude<Locale, 'en'>, Record<string, string>> = {
  fr: {
    '← Back':'← Retour','Add':'Ajouter','Optional Obsidian tags. Comma or Enter adds a tag.':'Tags Obsidian facultatifs. Virgule ou Entrée ajoute un tag.',
    'Model discovery unavailable.':'Découverte des modèles indisponible.','+ Add fallback':'+ Ajouter une route de secours',
    'Any OpenAI-compatible audio transcription endpoint can be local or remote. ScribeWatch stores secrets server-side.':'Tout endpoint de transcription audio compatible OpenAI peut être local ou distant. ScribeWatch conserve les secrets côté serveur.',
    '+ Add provider':'+ Ajouter un fournisseur','No providers configured.':'Aucun fournisseur configuré.',
    'Use the complete OpenAI-compatible transcription endpoint. Nothing is hardcoded to a specific service.':'Utilisez l’endpoint complet de transcription compatible OpenAI. Aucun service n’est codé en dur.',
    'The stored value is never returned to this browser.':'La valeur enregistrée n’est jamais renvoyée à ce navigateur.',
    'Model discovery timeout (seconds)':'Délai de découverte des modèles (secondes)',
    'This only bounds model discovery. Transcription has no provider-wide processing timeout; optional fallback timeouts are configured per route.':'Cela limite uniquement la découverte des modèles. La transcription n’a pas de délai global par fournisseur ; les délais de secours facultatifs se règlent par route.',
    'Models reported by provider':'Modèles annoncés par le fournisseur',
    'Watch audio continuously, publish the note where you keep knowledge, then archive the source only after publication succeeds.':'Surveillez l’audio en continu, publiez la note dans votre espace de connaissances, puis archivez la source uniquement après publication réussie.',
    '+ Add workflow':'+ Ajouter une automatisation','Create a transcription provider before enabling a workflow.':'Créez un fournisseur de transcription avant d’activer une automatisation.','No workflows yet.':'Aucune automatisation pour le moment.',
    'Empty or “auto” lets the provider detect it.':'Vide ou « auto » laisse le fournisseur détecter la langue.',
    'Runs after transcription. Failure never blocks the canonical transcript or source archiving.':'S’exécute après la transcription. Un échec ne bloque jamais la transcription canonique ni l’archivage de la source.',
    'Subfolders are watched recursively. Any stable file containing an FFmpeg-decodable audio stream is detected by content, regardless of extension, then normalized before STT. The Audio archive tree is always excluded.':'Les sous-dossiers sont surveillés récursivement. Tout fichier stable contenant un flux audio décodable par FFmpeg est détecté par son contenu, quelle que soit son extension, puis normalisé avant le STT. Toute l’arborescence de l’archive audio est toujours exclue.',
    'Point this at an Obsidian vault folder, or leave empty to publish beside each source audio, including inside subfolders.':'Choisissez un dossier de coffre Obsidian, ou laissez vide pour publier à côté de chaque audio source, y compris dans les sous-dossiers.',
    'The source moves here only after the Markdown note is safely published. This folder may live inside the Watch folder; its entire tree is excluded from watching.':'La source n’est déplacée ici qu’après publication sûre de la note. Ce dossier peut se trouver dans le dossier surveillé ; toute son arborescence est exclue de la surveillance.',
    'Obsidian / Markdown':'Obsidian / Markdown','Every note gets a transcript-derived title. YAML properties stay optional.':'Chaque note reçoit un titre dérivé de la transcription. Les propriétés YAML restent facultatives.',
    'Add YAML properties':'Ajouter les propriétés YAML','Groups the transcript with Unicode sentence boundaries and length rules; it never rewrites the words.':'Regroupe la transcription selon les limites de phrases Unicode et des règles de longueur ; les mots ne sont jamais réécrits.',
    'Watch this folder continuously':'Surveiller ce dossier en continu',
    'Audio → canonical transcript → optional LLM structure → Markdown with both views.':'Audio → transcription canonique → structure LLM facultative → document contenant les deux vues.',
    'If the LLM fails, ScribeWatch still publishes the transcript.':'Si le LLM échoue, ScribeWatch publie quand même la transcription.',
    '+ Provider':'+ Fournisseur','No LLM provider configured.':'Aucun fournisseur LLM configuré.','Chat completions URL':'URL Chat Completions','Timeout (seconds)':'Délai (secondes)',
    '+ Custom profile':'+ Profil personnalisé','Built-in template':'Modèle intégré','Editable for your deployment, but protected from deletion.':'Modifiable pour votre déploiement, mais protégé contre la suppression.',
    'Description':'Description','LLM provider':'Fournisseur LLM','Not connected':'Non connecté',
    'ScribeWatch adds its own immutable anti-hallucination / prompt-injection system instruction before this profile.':'ScribeWatch ajoute avant ce profil sa propre instruction système immuable contre les hallucinations et l’injection de prompt.',
    'The profile creates an additional structured view. The complete transcript remains in the same Markdown file.':'Le profil crée une vue structurée supplémentaire. La transcription complète reste dans le même document.',
    'Browser security requires the final local save to remain an explicit action.':'La sécurité du navigateur impose que l’enregistrement local final reste une action explicite.',
    'Stop once. ScribeWatch uploads the completed browser recording, normalizes it with FFmpeg, transcribes it, and optionally structures it.':'À l’arrêt, ScribeWatch envoie l’enregistrement terminé, le normalise avec FFmpeg, le transcrit puis le structure éventuellement.',
    'Promise':'État','bytes':'octets','Markdown':'Markdown',
    'General medicine consultation draft':'Compte rendu de consultation de médecine générale','Dental consultation draft':'Compte rendu de consultation dentaire',
    'Medical consultation draft':'Compte rendu de consultation médicale','General structured note':'Note structurée générale','Meeting minutes':'Compte rendu de réunion','Interview / research':'Entretien / recherche','Marketing brainstorm':'Brainstorming marketing',
    '✦ Quick Transcribe':'✦ Transcription rapide','✓ No LLM required':'✓ Aucun LLM requis','✓ OpenAI-compatible':'✓ Compatible OpenAI','✓ Original audio stays safe':'✓ Audio original préservé',
    'Local microphone mode…':'Mode microphone local…','Install local microphone mode':'Installer le mode microphone local','Close':'Fermer','Download installer':'Télécharger l’installateur',
    'NAS hostname / IPv4':'Nom d’hôte NAS / IPv4','ScribeWatch HTTP port':'Port HTTP ScribeWatch','Local port':'Port local','Client operating system':'Système du poste client',
    'This does not install another ScribeWatch. It creates a localhost-only proxy on this computer to the existing ScribeWatch HTTP service on your NAS, then you use':'Cela n’installe pas une autre instance de ScribeWatch. Un proxy uniquement localhost est créé sur cet ordinateur vers le service HTTP ScribeWatch déjà présent sur le NAS ; utilisez ensuite',
    'You opened ScribeWatch through HTTPS. Enter the NAS local hostname or IPv4 address and its plain HTTP ScribeWatch port below; do not enter the public HTTPS reverse-proxy address.':'Vous avez ouvert ScribeWatch via HTTPS. Indiquez ci-dessous le nom local ou l’IPv4 du NAS et le port HTTP direct de ScribeWatch ; n’utilisez pas l’adresse HTTPS publique du reverse proxy.',
    'Run the downloaded .cmd once; Windows will request administrator approval.':'Exécutez une fois le fichier .cmd téléchargé ; Windows demandera une autorisation administrateur.',
    'Run once in Terminal with':'Exécutez une fois dans Terminal avec','It installs a per-user LaunchAgent.':'Un LaunchAgent utilisateur sera installé.','Run once with':'Exécutez une fois avec','It installs a per-user systemd socket.':'Un socket systemd utilisateur sera installé.',
    'The generated installer is specific to the NAS address above, binds only to 127.0.0.1, persists across logins, opens the local URL after installation, and supports':'L’installateur généré est spécifique à l’adresse NAS ci-dessus, écoute uniquement sur 127.0.0.1, persiste entre les connexions, ouvre l’URL locale après installation et accepte',
    'Local microphone installer downloaded. Run it once on this computer.':'Installateur du mode microphone local téléchargé. Exécutez-le une fois sur cet ordinateur.'
  },
  'zh-CN': {
    '← Back':'← 返回','Add':'添加','Optional Obsidian tags. Comma or Enter adds a tag.':'可选 Obsidian 标签。输入逗号或按 Enter 添加标签。',
    'Model discovery unavailable.':'无法发现模型。','+ Add fallback':'+ 添加备用线路',
    'Any OpenAI-compatible audio transcription endpoint can be local or remote. ScribeWatch stores secrets server-side.':'任何兼容 OpenAI 的音频转写端点都可以是本地或远程的。ScribeWatch 将密钥保存在服务器端。',
    '+ Add provider':'+ 添加服务商','No providers configured.':'尚未配置服务商。',
    'Use the complete OpenAI-compatible transcription endpoint. Nothing is hardcoded to a specific service.':'请使用完整的 OpenAI 兼容转写端点；ScribeWatch 不绑定任何特定服务。',
    'The stored value is never returned to this browser.':'保存的密钥不会返回给此浏览器。',
    'Model discovery timeout (seconds)':'模型发现超时（秒）',
    'This only bounds model discovery. Transcription has no provider-wide processing timeout; optional fallback timeouts are configured per route.':'此设置仅限制模型发现。转写没有服务商级处理超时；可选备用超时按线路配置。',
    'Models reported by provider':'服务商返回的模型',
    'Watch audio continuously, publish the note where you keep knowledge, then archive the source only after publication succeeds.':'持续监视音频，将笔记发布到知识库，并仅在发布成功后归档源文件。',
    '+ Add workflow':'+ 添加工作流','Create a transcription provider before enabling a workflow.':'启用工作流前请先创建转写服务商。','No workflows yet.':'尚无工作流。',
    'Empty or “auto” lets the provider detect it.':'留空或使用“auto”让服务商自动检测语言。',
    'Runs after transcription. Failure never blocks the canonical transcript or source archiving.':'在转写后运行。失败不会阻止权威转写或源文件归档。',
    'Subfolders are watched recursively. Any stable file containing an FFmpeg-decodable audio stream is detected by content, regardless of extension, then normalized before STT. The Audio archive tree is always excluded.':'子文件夹会被递归监视。任何包含 FFmpeg 可解码音频流的稳定文件都会按内容识别，不受扩展名影响，并在 STT 前标准化。音频归档目录树始终被排除。',
    'Point this at an Obsidian vault folder, or leave empty to publish beside each source audio, including inside subfolders.':'选择 Obsidian 库文件夹，或留空以发布到每个源音频旁边，包括子文件夹中的音频。',
    'The source moves here only after the Markdown note is safely published. This folder may live inside the Watch folder; its entire tree is excluded from watching.':'只有 Markdown 笔记安全发布后，源文件才会移动到这里。此文件夹可以位于监视文件夹内部，其整个目录树都会被排除在监视之外。',
    'Obsidian / Markdown':'Obsidian / Markdown','Every note gets a transcript-derived title. YAML properties stay optional.':'每条笔记都会获得由转写生成的标题；YAML 属性仍为可选。',
    'Add YAML properties':'添加 YAML 属性','Groups the transcript with Unicode sentence boundaries and length rules; it never rewrites the words.':'按 Unicode 句子边界和长度规则对转写分组；绝不改写原词。',
    'Watch this folder continuously':'持续监视此文件夹',
    'Audio → canonical transcript → optional LLM structure → Markdown with both views.':'音频 → 权威转写 → 可选 LLM 结构化 → 同时保留两种视图的文档。',
    'If the LLM fails, ScribeWatch still publishes the transcript.':'即使 LLM 失败，ScribeWatch 仍会发布转写。',
    '+ Provider':'+ 服务商','No LLM provider configured.':'尚未配置 LLM 服务商。','Chat completions URL':'Chat Completions URL','Timeout (seconds)':'超时（秒）',
    '+ Custom profile':'+ 自定义配置','Built-in template':'内置模板','Editable for your deployment, but protected from deletion.':'可针对部署进行编辑，但受保护不能删除。',
    'Description':'说明','LLM provider':'LLM 服务商','Not connected':'未连接',
    'ScribeWatch adds its own immutable anti-hallucination / prompt-injection system instruction before this profile.':'ScribeWatch 会在此配置前加入不可修改的防幻觉与防提示注入系统指令。',
    'The profile creates an additional structured view. The complete transcript remains in the same Markdown file.':'该配置会生成额外的结构化视图；完整转写仍保留在同一文档中。',
    'Browser security requires the final local save to remain an explicit action.':'浏览器安全机制要求最终本地保存必须由用户明确触发。',
    'Stop once. ScribeWatch uploads the completed browser recording, normalizes it with FFmpeg, transcribes it, and optionally structures it.':'停止后，ScribeWatch 会上传完整录音，使用 FFmpeg 标准化、转写，并可选择结构化。',
    'Promise':'状态','bytes':'字节','Markdown':'Markdown',
    'General medicine consultation draft':'全科医学就诊记录草稿','Dental consultation draft':'牙科就诊记录草稿',
    'Medical consultation draft':'医疗就诊记录草稿','General structured note':'通用结构化笔记','Meeting minutes':'会议纪要','Interview / research':'访谈 / 研究','Marketing brainstorm':'营销头脑风暴',
    '✦ Quick Transcribe':'✦ 快速转写','✓ No LLM required':'✓ 无需 LLM','✓ OpenAI-compatible':'✓ 兼容 OpenAI','✓ Original audio stays safe':'✓ 保留原始音频'
  }
};

function detectedLocale(): Locale {
  if (typeof window === 'undefined') return 'en';
  const stored = window.localStorage.getItem('scribewatch:locale');
  if (stored === 'fr' || stored === 'zh-CN' || stored === 'en') return stored;
  const language = (navigator.language || '').toLowerCase();
  if (language.startsWith('fr')) return 'fr';
  if (language.startsWith('zh')) return 'zh-CN';
  return 'en';
}

export const locale = writable<Locale>('en');

export function initLocale() {
  const value = detectedLocale();
  setLocale(value);
  return value;
}

export function setLocale(value: Locale) {
  locale.set(value);
  if (typeof document !== 'undefined') document.documentElement.lang = value;
  if (typeof localStorage !== 'undefined') localStorage.setItem('scribewatch:locale', value);
}

export function t(value: Locale, source: string) {
  if (value === 'en') return source;
  return supplemental[value][source] ?? dictionaries[value][source] ?? source;
}

function translatedText(source: string, value: Locale) {
  const match = source.match(/^(\s*)([\s\S]*?)(\s*)$/);
  if (!match) return t(value, source);
  return match[1] + t(value, match[2]) + match[3];
}

export function localize(node: HTMLElement) {
  const textSources = new WeakMap<Text, string>();
  const attrSources = new WeakMap<Element, Map<string, string>>();
  const attrs = ['placeholder','title','aria-label'];
  let current = get(locale);

  const translateTextNode = (textNode: Text) => {
    let source = textSources.get(textNode);
    if (source === undefined) {
      source = textNode.data;
      textSources.set(textNode, source);
    } else {
      const expected = translatedText(source, current);
      if (textNode.data !== expected && textNode.data !== source) {
        source = textNode.data;
        textSources.set(textNode, source);
      }
    }
    const next = translatedText(source, current);
    if (textNode.data !== next) textNode.data = next;
  };

  const translateElement = (element: Element) => {
    let sources = attrSources.get(element);
    if (!sources) {
      sources = new Map();
      attrSources.set(element, sources);
    }
    for (const attr of attrs) {
      const raw = element.getAttribute(attr);
      if (raw === null) continue;
      let source = sources.get(attr);
      if (source === undefined) {
        source = raw;
        sources.set(attr, source);
      } else {
        const expected = t(current, source);
        if (raw !== expected && raw !== source) {
          source = raw;
          sources.set(attr, source);
        }
      }
      const next = t(current, source);
      if (raw !== next) element.setAttribute(attr, next);
    }
  };

  const walk = (root: Node) => {
    if (root.nodeType === Node.TEXT_NODE) translateTextNode(root as Text);
    if (root.nodeType === Node.ELEMENT_NODE) translateElement(root as Element);
    for (const child of Array.from(root.childNodes)) walk(child);
  };

  const observer = new MutationObserver((mutations) => {
    for (const mutation of mutations) {
      if (mutation.type === 'characterData') walk(mutation.target);
      else if (mutation.type === 'attributes') walk(mutation.target);
      else for (const added of Array.from(mutation.addedNodes)) walk(added);
    }
  });
  observer.observe(node, {subtree:true,childList:true,characterData:true,attributes:true,attributeFilter:attrs});
  const unsubscribe = locale.subscribe((value) => { current = value; walk(node); });
  walk(node);
  return { destroy(){ unsubscribe(); observer.disconnect(); } };
}
