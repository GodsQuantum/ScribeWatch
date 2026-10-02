import { writable, get } from 'svelte/store';

export type Locale = 'en' | 'fr' | 'zh-CN';

const dictionaries: Record<Exclude<Locale, 'en'>, Record<string, string>> = {
  fr: {
    'Home':'Accueil','Quick':'Rapide','Live':'Direct','Workflows':'Automatisations','AI Profiles':'Profils IA','Jobs':'T��ches',
    'audio ��� knowledge':'audio ��� connaissances','Transcript is canonical.':'La transcription est canonique.',
    'AI structure is optional and never replaces source transcription.':"La structure IA est facultative et ne remplace jamais la transcription source.",
    'Language':'Langue','Interface language':'Langue de l���interface',
    'Cannot reach ScribeWatch.':'ScribeWatch est inaccessible.','Retry':'R��essayer','Loading ScribeWatch���':'Chargement de ScribeWatch���',
    'SELF-HOSTED AUDIO NOTES':'NOTES AUDIO AUTO-H��BERG��ES',
    'Turn voice notes into Markdown before you forget them.':'Transformez vos notes vocales en documents avant de les oublier.',
    'ScribeWatch listens to your folders ��� or one file right now ��� and turns audio into titled, Obsidian-ready notes using your own transcription provider.':
      'ScribeWatch surveille vos dossiers ��� ou traite un fichier imm��diatement ��� et transforme l���audio en notes titr��es pr��tes pour Obsidian avec votre propre moteur de transcription.',
    'Quick Transcribe':'Transcription rapide','Set up a watch folder':'Configurer un dossier surveill��',
    'No LLM required':'Aucun LLM requis','OpenAI-compatible':'Compatible OpenAI','Original audio stays safe':'Audio original pr��serv��',
    'WATCHING':'SURVEILLANCE','NOW':'EN COURS','NOTES':'NOTES','ATTENTION':'ATTENTION','active workflows':'automatisations actives','processing':'en traitement','completed':'termin��es','failed':'��chou��es',
    'What���s happening':'Activit��','Live transcription activity':'Activit�� de transcription','All jobs ���':'Toutes les t��ches ���',
    'Quiet right now.':'Aucune activit��.','Drop a voice note into Quick Transcribe or an enabled watch folder.':'Ajoutez une note audio dans Transcription rapide ou dans un dossier surveill�� actif.',
    'Recent notes':'Notes r��centes','Latest Markdown published':'Derniers documents publi��s','Your first finished note will appear here.':'Votre premi��re note termin��e appara��tra ici.',
    'ONE FILE �� RIGHT NOW':'UN FICHIER �� MAINTENANT','Turn one recording into a clean Markdown note without creating a workflow.':'Transformez un enregistrement en note propre sans cr��er d���automatisation.',
    'Add a transcription provider first.':'Ajoutez d���abord un fournisseur de transcription.','Choose your audio':'Choisir l���audio',
    'Upload from this computer or use a file already mounted on the server.':'Importez depuis cet ordinateur ou utilisez un fichier d��j�� mont�� sur le serveur.',
    'This computer':'Cet ordinateur','Server':'Serveur','Drop an audio note here':'D��posez une note audio ici','or click to choose a file from this computer':'ou cliquez pour choisir un fichier sur cet ordinateur',
    'Server audio file':'Fichier audio serveur','Choose an allowed server file':'Choisir un fichier serveur autoris��','Browse':'Parcourir',
    'Only explicitly mounted and allowed server roots are visible.':'Seules les racines serveur explicitement mont��es et autoris��es sont visibles.',
    'Transcription':'Transcription','Pick an exact provider + model chain. Failed routes automatically fall through.':'Choisissez une cha��ne pr��cise fournisseur + mod��le. En cas d�����chec, la route suivante est essay��e.',
    'AI structure':'Structure IA','optional':'facultatif','None ��� transcript only':'Aucune ��� transcription uniquement','Adds a structured view and always preserves the full transcript.':'Ajoute une vue structur��e tout en conservant toujours la transcription compl��te.',
    'Advanced':'Avanc��','YAML / Obsidian properties':'Propri��t��s YAML / Obsidian','Readable deterministic paragraphs':'Paragraphes d��terministes lisibles',
    'Uses Unicode sentence boundaries and length rules only. No LLM rewrites the transcript.':'Utilise uniquement les limites de phrases Unicode et des r��gles de longueur. Aucun LLM ne r����crit la transcription.',
    'Save the note':'Enregistrer la note','Publish on the server or bring the Markdown back to this computer.':'Publiez sur le serveur ou r��cup��rez le document sur cet ordinateur.',
    'Server folder':'Dossier serveur','Markdown destination':'Destination du document','Choose a notes folder':'Choisir un dossier de notes',
    'Save directly into a local folder':'Enregistrer directement dans un dossier local','Download the .md file':'T��l��charger le fichier .md',
    'Transcribe to Markdown ���':'Transcrire ���','Uploading���':'Envoi���','Result':'R��sultat','Your note appears here.':'Votre note appara��t ici.',
    'The title comes from the first useful words of the transcription ��� ready for Obsidian, any Markdown vault, or plain files.':'Le titre vient des premiers mots utiles de la transcription ��� pr��t pour Obsidian, tout coffre Markdown ou des fichiers classiques.',
    'Transcription did not finish':'La transcription n���a pas abouti','Markdown ready':'Document pr��t','Save Markdown���':'Enregistrer le Markdown���','Download Markdown':'T��l��charger le Markdown',
    'Transcribe another':'Transcrire un autre fichier','Listening���':'Transcription���','Structuring with AI���':'Structuration par IA���','Writing Markdown���':'��criture du document���','Queued���':'En attente���',
    'Choose server audio':'Choisir un audio serveur','Choose Markdown folder':'Choisir le dossier de notes',
    'BROWSER MICROPHONE':'MICROPHONE DU NAVIGATEUR','Live Recorder':'Enregistreur en direct',
    'Record from a browser microphone, stop when you are done, then run the exact same resilient transcription and optional AI-structure pipeline as a watched file.':
      'Enregistrez depuis le microphone du navigateur puis utilisez la m��me cha��ne robuste de transcription et de structuration IA facultative qu���un fichier surveill��.',
    'Microphone':'Microphone','Ready to record':'Pr��t �� enregistrer','Microphone unavailable':'Microphone indisponible','Requesting microphone���':'Autorisation du microphone���','Uploading recording���':'Envoi de l���enregistrement���','Recording ��� press again to stop':'Enregistrement ��� appuyez �� nouveau pour arr��ter',
    'The recording is uploaded only after Stop, then the temporary audio is deleted after processing.':'L���enregistrement n���est envoy�� qu���apr��s Arr��ter, puis l���audio temporaire est supprim�� apr��s traitement.',
    'Your audio stays in this browser until you stop.':'Votre audio reste dans ce navigateur jusqu����� l���arr��t.',
    'Request microphone again':'Redemander l���acc��s au microphone','Enable microphone':'Activer le microphone','Refresh microphones':'Actualiser les microphones',
    'Microphone access requires HTTPS (or localhost). Open ScribeWatch through its HTTPS address.':'L���acc��s au microphone n��cessite HTTPS (ou localhost). Ouvrez ScribeWatch via son adresse HTTPS.',
    'Microphone enumeration is unavailable on this browser origin. Use HTTPS or local HTTP mode at http://127.0.0.1:3052.':'L�����num��ration des microphones est indisponible sur cette origine. Utilisez HTTPS ou le mode HTTP local sur http://127.0.0.1:3052.',
    'Microphone capture is unavailable on this browser origin. Use HTTPS or local HTTP mode at http://127.0.0.1:3052.':'La capture du microphone est indisponible sur cette origine. Utilisez HTTPS ou le mode HTTP local sur http://127.0.0.1:3052.',
    'Microphone permission is blocked. Allow it in the browser site permissions, then retry.':'L���autorisation du microphone est bloqu��e. Autorisez-la dans les permissions du site puis r��essayez.',
    'No audio input device is currently visible to the browser.':'Aucun p��riph��rique d���entr��e audio n���est actuellement visible par le navigateur.',
    'Choose a transcription provider and model first.':'Choisissez d���abord un fournisseur et un mod��le de transcription.',
    'Choose where to save the recording result first.':'Choisissez d���abord o�� enregistrer le r��sultat.',
    'Permission':'Autorisation','Secure context':'Contexte s��curis��','Devices':'P��riph��riques','granted':'accord��e','prompt':'�� demander','denied':'refus��e','unknown':'inconnue','yes':'oui','no':'non',
    'None ��� deterministic transcript only':'Aucune ��� transcription d��terministe uniquement','Destination':'Destination','YAML properties':'Propri��t��s YAML',
    'Recording result':'R��sultat de l���enregistrement','Press the record button.':'Appuyez sur le bouton d���enregistrement.','Processing did not finish':'Le traitement n���a pas abouti','Note ready':'Note pr��te',
    'Export result':'Exporter le r��sultat','Download / save as':'T��l��charger / enregistrer en',
    'HISTORY':'HISTORIQUE','Every automatic and one-off transcription keeps a durable lifecycle, including retries, fallback attempts and restart recovery.':'Chaque transcription automatique ou ponctuelle conserve un historique durable, avec r��essais, routes de secours et r��cup��ration apr��s red��marrage.',
    'All':'Toutes','Current':'En cours','Errors':'Erreurs','Done':'Termin��es','No jobs in this view.':'Aucune t��che dans cette vue.','Source':'Source','Archive':'Archive','Processing error':'Erreur de traitement','AI STRUCTURED':'STRUCTUR�� PAR IA',
    'Transcript published without AI structure.':'Transcription publi��e sans structure IA.','Transcription attempts':'Tentatives de transcription','Cancel':'Annuler','Remove record':'Supprimer l���historique',
    'OPTIONAL AI POST-PROCESSING':'POST-TRAITEMENT IA FACULTATIF','AI structure profiles':'Profils de structure IA',
    'Keep transcription deterministic, then optionally create a second structured view with an OpenAI-compatible LLM. The canonical transcript is always preserved.':'Conservez une transcription d��terministe puis cr��ez ��ventuellement une seconde vue structur��e avec un LLM compatible OpenAI. La transcription canonique est toujours pr��serv��e.',
    'TRANSCRIPT FIRST':'TRANSCRIPTION D���ABORD','LLM providers':'Fournisseurs LLM','Structure profiles':'Profils de structure','Save provider':'Enregistrer le fournisseur','Discover models':'D��couvrir les mod��les','Delete':'Supprimer','Name':'Nom','Default model':'Mod��le par d��faut','API key':'Cl�� API','Provider enabled':'Fournisseur actif','Model':'Mod��le','Structure prompt':'Prompt de structure','Save profile':'Enregistrer le profil','Custom profile':'Profil personnalis��',
    'Transcription providers':'Fournisseurs de transcription','New provider':'Nouveau fournisseur','Edit provider':'Modifier le fournisseur','Secret stored':'Secret enregistr��','Transcription URL':'URL de transcription',
    'AUTOMATION':'AUTOMATISATION','Add workflow':'Ajouter une automatisation','New workflow':'Nouvelle automatisation','Edit workflow':'Modifier l���automatisation','Watch folder':'Dossier surveill��','Markdown folder':'Dossier de notes','Audio archive':'Archive audio','Scan now':'Analyser maintenant','Save workflow':'Enregistrer l���automatisation',
    'Choose folder':'Choisir un dossier','Filter folders':'Filtrer les dossiers','Filter audio files':'Filtrer les fichiers audio','Refresh':'Actualiser','Loading���':'Chargement���','Nothing matching here.':'Aucun r��sultat ici.','Back':'Retour','Open':'Ouvrir','Choose':'Choisir','Only configured allowed roots are visible.':'Seules les racines autoris��es configur��es sont visibles.','Choose this folder':'Choisir ce dossier',
    'Transcription chain':'Cha��ne de transcription','Choose the exact provider + model order. If one route fails, ScribeWatch tries the next.':'Choisissez l���ordre exact fournisseur + mod��le. Si une route ��choue, ScribeWatch essaie la suivante.','Primary':'Principal','Provider':'Fournisseur','Choose provider':'Choisir un fournisseur','Choose model':'Choisir un mod��le','Fallback after':'Bascule apr��s','Never':'Jamais','Custom':'Personnalis��','Add fallback':'Ajouter une route de secours'
  },
  'zh-CN': {
    'Home':'������','Quick':'������','Live':'������','Workflows':'���������','AI Profiles':'AI ������','Jobs':'������',
    'audio ��� knowledge':'������ ��� ������','Transcript is canonical.':'������������������������������','AI structure is optional and never replaces source transcription.':'AI ���������������������������������������������������',
    'Language':'������','Interface language':'������������','Cannot reach ScribeWatch.':'������������ ScribeWatch���','Retry':'������','Loading ScribeWatch���':'������������ ScribeWatch���',
    'SELF-HOSTED AUDIO NOTES':'���������������������','Turn voice notes into Markdown before you forget them.':'������������������������������������',
    'ScribeWatch listens to your folders ��� or one file right now ��� and turns audio into titled, Obsidian-ready notes using your own transcription provider.':'ScribeWatch ������������������������������������������������������������������������������������������������������������������ Obsidian ���������������������',
    'Quick Transcribe':'������������','Set up a watch folder':'���������������������','No LLM required':'������ LLM','OpenAI-compatible':'������ OpenAI','Original audio stays safe':'������������������',
    'WATCHING':'���������','NOW':'������','NOTES':'������','ATTENTION':'���������','active workflows':'������������������','processing':'���������','completed':'���������','failed':'������',
    'What���s happening':'������������','Live transcription activity':'������������������','All jobs ���':'������������ ���','Quiet right now.':'������������������','Drop a voice note into Quick Transcribe or an enabled watch folder.':'������������������������������������������������������������������','Recent notes':'������������','Latest Markdown published':'���������������������','Your first finished note will appear here.':'���������������������������������������������',
    'ONE FILE �� RIGHT NOW':'������������ �� ������������','Turn one recording into a clean Markdown note without creating a workflow.':'������������������������������������������������������������������','Add a transcription provider first.':'���������������������������',
    'Choose your audio':'������������','Upload from this computer or use a file already mounted on the server.':'���������������������������������������������������������','This computer':'���������','Server':'���������','Drop an audio note here':'���������������������������','or click to choose a file from this computer':'������������������������������',
    'Server audio file':'���������������������','Choose an allowed server file':'������������������������������','Browse':'������','Only explicitly mounted and allowed server roots are visible.':'������������������������������������������������������',
    'Transcription':'������','Pick an exact provider + model chain. Failed routes automatically fall through.':'������������������������ + ���������������������������������������������������','AI structure':'AI ���������','optional':'������','None ��� transcript only':'��� ��� ���������','Adds a structured view and always preserves the full transcript.':'���������������������������������������������������������',
    'Advanced':'������','YAML / Obsidian properties':'YAML / Obsidian ������','Readable deterministic paragraphs':'������������������������','Uses Unicode sentence boundaries and length rules only. No LLM rewrites the transcript.':'��������� Unicode ������������������������������LLM ���������������������������',
    'Save the note':'������������','Publish on the server or bring the Markdown back to this computer.':'���������������������������������������������������','Server folder':'������������������','Markdown destination':'������������������','Choose a notes folder':'���������������������',
    'Save directly into a local folder':'������������������������������','Download the .md file':'������ .md ������','Transcribe to Markdown ���':'������������ ���','Uploading���':'���������������','Result':'������','Your note appears here.':'���������������������������������','The title comes from the first useful words of the transcription ��� ready for Obsidian, any Markdown vault, or plain files.':'������������������������������������������������������������ Obsidian���Markdown ���������������������',
    'Transcription did not finish':'���������������','Markdown ready':'���������������','Save Markdown���':'������ Markdown���','Download Markdown':'������ Markdown','Transcribe another':'���������������������','Listening���':'���������������','Structuring with AI���':'AI ������������������','Writing Markdown���':'���������������������','Queued���':'������������','Choose server audio':'���������������������','Choose Markdown folder':'���������������������',
    'BROWSER MICROPHONE':'������������������','Live Recorder':'������������','Record from a browser microphone, stop when you are done, then run the exact same resilient transcription and optional AI-structure pipeline as a watched file.':'��������������������������������������������������������������������������������������������������� AI ������������������������',
    'Microphone':'���������','Ready to record':'������������','Microphone unavailable':'������������������','Requesting microphone���':'������������������������������','Uploading recording���':'���������������������','Recording ��� press again to stop':'������������ ��� ������������������','The recording is uploaded only after Stop, then the temporary audio is deleted after processing.':'������������������������������������������������������������������������','Your audio stays in this browser until you stop.':'���������������������������������������������',
    'Request microphone again':'���������������������������','Enable microphone':'���������������','Refresh microphones':'���������������','Microphone access requires HTTPS (or localhost). Open ScribeWatch through its HTTPS address.':'��������������������� HTTPS������ localhost��������������� ScribeWatch ��� HTTPS ���������������','Microphone enumeration is unavailable on this browser origin. Use HTTPS or local HTTP mode at http://127.0.0.1:3052.':'������������������������������������������������������ HTTPS������������ http://127.0.0.1:3052 ������������ HTTP ���������','Microphone capture is unavailable on this browser origin. Use HTTPS or local HTTP mode at http://127.0.0.1:3052.':'������������������������������������������������������ HTTPS������������ http://127.0.0.1:3052 ������������ HTTP ���������','Microphone permission is blocked. Allow it in the browser site permissions, then retry.':'������������������������������������������������������������������������������','No audio input device is currently visible to the browser.':'���������������������������������������������������','Choose a transcription provider and model first.':'���������������������������������������','Choose where to save the recording result first.':'������������������������������������������','Permission':'������','Secure context':'���������������','Devices':'������','granted':'���������','prompt':'���������','denied':'���������','unknown':'������','yes':'���','no':'���',
    'None ��� deterministic transcript only':'��� ��� ������������������','Destination':'������������','YAML properties':'YAML ������','Recording result':'������������','Press the record button.':'���������������������','Processing did not finish':'���������������','Note ready':'���������������','Export result':'������������','Download / save as':'������ / ���������',
    'HISTORY':'������','Every automatic and one-off transcription keeps a durable lifecycle, including retries, fallback attempts and restart recovery.':'���������������������������������������������������������������������������������������������������������','All':'������','Current':'������','Errors':'������','Done':'������','No jobs in this view.':'���������������������������','Source':'������','Archive':'������','Processing error':'������������','AI STRUCTURED':'AI ������������','Transcript published without AI structure.':'������������������������������ AI ������������','Transcription attempts':'������������','Cancel':'������','Remove record':'������������',
    'OPTIONAL AI POST-PROCESSING':'������ AI ���������','AI structure profiles':'AI ���������������','Keep transcription deterministic, then optionally create a second structured view with an OpenAI-compatible LLM. The canonical transcript is always preserved.':'������������������������������������������������ OpenAI ��� LLM ������������������������������������������������������������','TRANSCRIPT FIRST':'������������','LLM providers':'LLM ���������','Structure profiles':'������������','Save provider':'���������������','Discover models':'������������','Delete':'������','Name':'������','Default model':'������������','API key':'API ������','Provider enabled':'���������������','Model':'������','Structure prompt':'���������������','Save profile':'������������','Custom profile':'���������������',
    'Transcription providers':'���������������','New provider':'���������������','Edit provider':'���������������','Secret stored':'���������������','Transcription URL':'������ URL',
    'AUTOMATION':'���������','Add workflow':'���������������','New workflow':'���������������','Edit workflow':'���������������','Watch folder':'���������������','Markdown folder':'���������������','Audio archive':'������������','Scan now':'������������','Save workflow':'���������������',
    'Choose folder':'���������������','Filter folders':'���������������','Filter audio files':'������������������','Refresh':'������','Loading���':'������������','Nothing matching here.':'������������������������','Back':'������','Open':'������','Choose':'������','Only configured allowed roots are visible.':'���������������������������������������','Choose this folder':'������������������',
    'Transcription chain':'���������','Choose the exact provider + model order. If one route fails, ScribeWatch tries the next.':'������������������������ + ������������������������������������������ScribeWatch ���������������������','Primary':'���������','Provider':'���������','Choose provider':'���������������','Choose model':'������������','Fallback after':'���������������','Never':'������','Custom':'���������','Add fallback':'������������������'
  }
};


const supplemental: Record<Exclude<Locale, 'en'>, Record<string, string>> = {
  fr: {
    '��� Back':'��� Retour','Add':'Ajouter','Optional Obsidian tags. Comma or Enter adds a tag.':'Tags Obsidian facultatifs. Virgule ou Entr��e ajoute un tag.',
    'Model discovery unavailable.':'D��couverte des mod��les indisponible.','+ Add fallback':'+ Ajouter une route de secours',
    'Any OpenAI-compatible audio transcription endpoint can be local or remote. ScribeWatch stores secrets server-side.':'Tout endpoint de transcription audio compatible OpenAI peut ��tre local ou distant. ScribeWatch conserve les secrets c��t�� serveur.',
    '+ Add provider':'+ Ajouter un fournisseur','No providers configured.':'Aucun fournisseur configur��.',
    'Use the complete OpenAI-compatible transcription endpoint. Nothing is hardcoded to a specific service.':'Utilisez l���endpoint complet de transcription compatible OpenAI. Aucun service n���est cod�� en dur.',
    'The stored value is never returned to this browser.':'La valeur enregistr��e n���est jamais renvoy��e �� ce navigateur.',
    'Model discovery timeout (seconds)':'D��lai de d��couverte des mod��les (secondes)',
    'This only bounds model discovery. Transcription has no provider-wide processing timeout; optional fallback timeouts are configured per route.':'Cela limite uniquement la d��couverte des mod��les. La transcription n���a pas de d��lai global par fournisseur ; les d��lais de secours facultatifs se r��glent par route.',
    'Models reported by provider':'Mod��les annonc��s par le fournisseur',
    'Watch audio continuously, publish the note where you keep knowledge, then archive the source only after publication succeeds.':'Surveillez l���audio en continu, publiez la note dans votre espace de connaissances, puis archivez la source uniquement apr��s publication r��ussie.',
    '+ Add workflow':'+ Ajouter une automatisation','Create a transcription provider before enabling a workflow.':'Cr��ez un fournisseur de transcription avant d���activer une automatisation.','No workflows yet.':'Aucune automatisation pour le moment.',
    'Empty or ���auto��� lets the provider detect it.':'Vide ou �� auto �� laisse le fournisseur d��tecter la langue.',
    'Runs after transcription. Failure never blocks the canonical transcript or source archiving.':'S���ex��cute apr��s la transcription. Un ��chec ne bloque jamais la transcription canonique ni l���archivage de la source.',
    'Subfolders are watched recursively. Any stable file containing an FFmpeg-decodable audio stream is detected by content, regardless of extension, then normalized before STT. The Audio archive tree is always excluded.':'Les sous-dossiers sont surveill��s r��cursivement. Tout fichier stable contenant un flux audio d��codable par FFmpeg est d��tect�� par son contenu, quelle que soit son extension, puis normalis�� avant le STT. Toute l���arborescence de l���archive audio est toujours exclue.',
    'Point this at an Obsidian vault folder, or leave empty to publish beside each source audio, including inside subfolders.':'Choisissez un dossier de coffre Obsidian, ou laissez vide pour publier �� c��t�� de chaque audio source, y compris dans les sous-dossiers.',
    'The source moves here only after the Markdown note is safely published. This folder may live inside the Watch folder; its entire tree is excluded from watching.':'La source n���est d��plac��e ici qu���apr��s publication s��re de la note. Ce dossier peut se trouver dans le dossier surveill�� ; toute son arborescence est exclue de la surveillance.',
    'Obsidian / Markdown':'Obsidian / Markdown','Every note gets a transcript-derived title. YAML properties stay optional.':'Chaque note re��oit un titre d��riv�� de la transcription. Les propri��t��s YAML restent facultatives.',
    'Add YAML properties':'Ajouter les propri��t��s YAML','Groups the transcript with Unicode sentence boundaries and length rules; it never rewrites the words.':'Regroupe la transcription selon les limites de phrases Unicode et des r��gles de longueur ; les mots ne sont jamais r����crits.',
    'Watch this folder continuously':'Surveiller ce dossier en continu',
    'Audio ��� canonical transcript ��� optional LLM structure ��� Markdown with both views.':'Audio ��� transcription canonique ��� structure LLM facultative ��� document contenant les deux vues.',
    'If the LLM fails, ScribeWatch still publishes the transcript.':'Si le LLM ��choue, ScribeWatch publie quand m��me la transcription.',
    '+ Provider':'+ Fournisseur','No LLM provider configured.':'Aucun fournisseur LLM configur��.','Chat completions URL':'URL Chat Completions','Timeout (seconds)':'D��lai (secondes)',
    '+ Custom profile':'+ Profil personnalis��','Built-in template':'Mod��le int��gr��','Editable for your deployment, but protected from deletion.':'Modifiable pour votre d��ploiement, mais prot��g�� contre la suppression.',
    'Description':'Description','LLM provider':'Fournisseur LLM','Not connected':'Non connect��',
    'ScribeWatch adds its own immutable anti-hallucination / prompt-injection system instruction before this profile.':'ScribeWatch ajoute avant ce profil sa propre instruction syst��me immuable contre les hallucinations et l���injection de prompt.',
    'The profile creates an additional structured view. The complete transcript remains in the same Markdown file.':'Le profil cr��e une vue structur��e suppl��mentaire. La transcription compl��te reste dans le m��me document.',
    'Browser security requires the final local save to remain an explicit action.':'La s��curit�� du navigateur impose que l���enregistrement local final reste une action explicite.',
    'Stop once. ScribeWatch uploads the completed browser recording, normalizes it with FFmpeg, transcribes it, and optionally structures it.':'�� l���arr��t, ScribeWatch envoie l���enregistrement termin��, le normalise avec FFmpeg, le transcrit puis le structure ��ventuellement.',
    'Promise':'��tat','bytes':'octets','Markdown':'Markdown',
    'General medicine consultation draft':'Compte rendu de consultation de m��decine g��n��rale','Dental consultation draft':'Compte rendu de consultation dentaire',
    'Medical consultation draft':'Compte rendu de consultation m��dicale','General structured note':'Note structur��e g��n��rale','Meeting minutes':'Compte rendu de r��union','Interview / research':'Entretien / recherche','Marketing brainstorm':'Brainstorming marketing',
    '��� Quick Transcribe':'��� Transcription rapide','��� No LLM required':'��� Aucun LLM requis','��� OpenAI-compatible':'��� Compatible OpenAI','��� Original audio stays safe':'��� Audio original pr��serv��',
    'Local microphone mode���':'Mode microphone local���','Install local microphone mode':'Installer le mode microphone local','Close':'Fermer','Download installer':'T��l��charger l���installateur',
    'NAS hostname / IPv4':'Nom d���h��te NAS / IPv4','ScribeWatch HTTP port':'Port HTTP ScribeWatch','Local port':'Port local','Client operating system':'Syst��me du poste client',
    'This does not install another ScribeWatch. It creates a localhost-only proxy on this computer to the existing ScribeWatch HTTP service on your NAS, then you use':'Cela n���installe pas une autre instance de ScribeWatch. Un proxy uniquement localhost est cr���� sur cet ordinateur vers le service HTTP ScribeWatch d��j�� pr��sent sur le NAS ; utilisez ensuite',
    'You opened ScribeWatch through HTTPS. Enter the NAS local hostname or IPv4 address and its plain HTTP ScribeWatch port below; do not enter the public HTTPS reverse-proxy address.':'Vous avez ouvert ScribeWatch via HTTPS. Indiquez ci-dessous le nom local ou l���IPv4 du NAS et le port HTTP direct de ScribeWatch ; n���utilisez pas l���adresse HTTPS publique du reverse proxy.',
    'Run the downloaded .cmd once; Windows will request administrator approval.':'Ex��cutez une fois le fichier .cmd t��l��charg�� ; Windows demandera une autorisation administrateur.',
    'Run once in Terminal with':'Ex��cutez une fois dans Terminal avec','It installs a per-user LaunchAgent.':'Un LaunchAgent utilisateur sera install��.','Run once with':'Ex��cutez une fois avec','It installs a per-user systemd socket.':'Un socket systemd utilisateur sera install��.',
    'The generated installer is specific to the NAS address above, binds only to 127.0.0.1, persists across logins, opens the local URL after installation, and supports':'L���installateur g��n��r�� est sp��cifique �� l���adresse NAS ci-dessus, ��coute uniquement sur 127.0.0.1, persiste entre les connexions, ouvre l���URL locale apr��s installation et accepte',
    'Local microphone installer downloaded. Run it once on this computer.':'Installateur du mode microphone local t��l��charg��. Ex��cutez-le une fois sur cet ordinateur.'
  },
  'zh-CN': {
    '��� Back':'��� ������','Add':'������','Optional Obsidian tags. Comma or Enter adds a tag.':'������ Obsidian ��������������������������� Enter ���������������',
    'Model discovery unavailable.':'���������������������','+ Add fallback':'+ ������������������',
    'Any OpenAI-compatible audio transcription endpoint can be local or remote. ScribeWatch stores secrets server-side.':'������������ OpenAI ������������������������������������������������������ScribeWatch ���������������������������������',
    '+ Add provider':'+ ���������������','No providers configured.':'������������������������',
    'Use the complete OpenAI-compatible transcription endpoint. Nothing is hardcoded to a specific service.':'������������������ OpenAI ���������������������ScribeWatch ������������������������������',
    'The stored value is never returned to this browser.':'���������������������������������������������',
    'Model discovery timeout (seconds)':'���������������������������',
    'This only bounds model discovery. Transcription has no provider-wide processing timeout; optional fallback timeouts are configured per route.':'������������������������������������������������������������������������������������������������������������',
    'Models reported by provider':'������������������������',
    'Watch audio continuously, publish the note where you keep knowledge, then archive the source only after publication succeeds.':'���������������������������������������������������������������������������������������������',
    '+ Add workflow':'+ ���������������','Create a transcription provider before enabling a workflow.':'������������������������������������������������','No workflows yet.':'������������������',
    'Empty or ���auto��� lets the provider detect it.':'������������������auto������������������������������������',
    'Runs after transcription. Failure never blocks the canonical transcript or source archiving.':'������������������������������������������������������������������������',
    'Subfolders are watched recursively. Any stable file containing an FFmpeg-decodable audio stream is detected by content, regardless of extension, then normalized before STT. The Audio archive tree is always excluded.':'��������������������������������������������� FFmpeg ��������������������������������������������������������������������������������������� STT ������������������������������������������������������',
    'Point this at an Obsidian vault folder, or leave empty to publish beside each source audio, including inside subfolders.':'������ Obsidian ���������������������������������������������������������������������������������������������',
    'The source moves here only after the Markdown note is safely published. This folder may live inside the Watch folder; its entire tree is excluded from watching.':'������ Markdown ������������������������������������������������������������������������������������������������������������������������������������������������������������',
    'Obsidian / Markdown':'Obsidian / Markdown','Every note gets a transcript-derived title. YAML properties stay optional.':'���������������������������������������������������YAML ���������������������',
    'Add YAML properties':'������ YAML ������','Groups the transcript with Unicode sentence boundaries and length rules; it never rewrites the words.':'��� Unicode ������������������������������������������������������������������',
    'Watch this folder continuously':'������������������������',
    'Audio ��� canonical transcript ��� optional LLM structure ��� Markdown with both views.':'������ ��� ������������ ��� ������ LLM ��������� ��� ������������������������������������',
    'If the LLM fails, ScribeWatch still publishes the transcript.':'������ LLM ���������ScribeWatch ���������������������',
    '+ Provider':'+ ���������','No LLM provider configured.':'������������ LLM ������������','Chat completions URL':'Chat Completions URL','Timeout (seconds)':'���������������',
    '+ Custom profile':'+ ���������������','Built-in template':'������������','Editable for your deployment, but protected from deletion.':'���������������������������������������������������������',
    'Description':'������','LLM provider':'LLM ���������','Not connected':'���������',
    'ScribeWatch adds its own immutable anti-hallucination / prompt-injection system instruction before this profile.':'ScribeWatch ���������������������������������������������������������������������������������',
    'The profile creates an additional structured view. The complete transcript remains in the same Markdown file.':'���������������������������������������������������������������������������������������',
    'Browser security requires the final local save to remain an explicit action.':'���������������������������������������������������������������������������',
    'Stop once. ScribeWatch uploads the completed browser recording, normalizes it with FFmpeg, transcribes it, and optionally structures it.':'������������ScribeWatch ������������������������������ FFmpeg ���������������������������������������������',
    'Promise':'������','bytes':'������','Markdown':'Markdown',
    'General medicine consultation draft':'������������������������������','Dental consultation draft':'������������������������',
    'Medical consultation draft':'������������������������','General structured note':'���������������������','Meeting minutes':'������������','Interview / research':'������ / ������','Marketing brainstorm':'������������������',
    '��� Quick Transcribe':'��� ������������','��� No LLM required':'��� ������ LLM','��� OpenAI-compatible':'��� ������ OpenAI','��� Original audio stays safe':'��� ������������������'
  }
};


Object.assign(supplemental.fr, {
  'Long recordings are checkpointed locally while you speak. A reload or browser crash can recover the audio already delivered by the browser, then Continue in a new segment.':'Les enregistrements longs sont sauvegard��s localement par ��tapes pendant que vous parlez. Apr��s un rechargement ou un crash du navigateur, l���audio d��j�� re��u peut ��tre r��cup��r�� puis poursuivi dans un nouveau segment.',
  'Recovery':'R��cup��ration','Protected':'Prot��g��','Browser storage':'Stockage navigateur','Persistent':'Persistant','Best effort':'Non garanti','Unavailable':'Indisponible','Checking���':'V��rification���',
  'Recovered LIVE recording':'Enregistrement LIVE r��cup��r��','available':'disponible','Started':'D��but','Saved duration':'Dur��e sauvegard��e','Protected audio':'Audio prot��g��','Segments':'Segments','Interruption gap':'Interruption estim��e',
  'Listen to last 30 seconds':'��couter les 30 derni��res secondes','Continue':'Continuer','Finish and transcribe':'Terminer et transcrire','Save audio':'Enregistrer l���audio','Delete recovery':'Supprimer la r��cup��ration',
  'The session exists, but no browser-delivered audio checkpoint was completed before the interruption.':'La session existe, mais aucun bloc audio fourni par le navigateur n���a ��t�� sauvegard�� avant l���interruption.',
  'Finalizing recording���':'Finalisation de l���enregistrement���','Audio is being checkpointed locally.':'L���audio est sauvegard�� progressivement en local.','is already protected.':'est d��j�� prot��g��.',
  'Browser-delivered audio is stored progressively for crash/reload recovery before the final server upload.':'L���audio fourni par le navigateur est sauvegard�� progressivement afin de permettre une r��cup��ration apr��s crash ou rechargement avant l���envoi final au serveur.',
  'Keep final audio':'Conserver l���audio final','No copy':'Aucune copie','Audio destination':'Destination audio','Choose final audio folder':'Choisir le dossier audio final',
  'Choose a local audio folder':'Choisir un dossier audio local','Final M4A will download after server finalization':'Le M4A final sera t��l��charg�� apr��s finalisation sur le serveur','Choose folder':'Choisir le dossier',
  'The browser recovery copy is kept until the finalized M4A is successfully saved.':'La copie de r��cup��ration du navigateur est conserv��e jusqu����� l���enregistrement r��ussi du M4A final.',
  'No permanent audio copy. The server still keeps the accepted LIVE source for the 24-hour retry window.':'Aucune copie audio permanente. Le serveur conserve tout de m��me la source LIVE accept��e pendant 24 heures pour permettre un nouvel essai.',
  'Save final audio���':'Enregistrer l���audio final���','Final audio saved.':'Audio final enregistr��.','Recovered recording continued in a new segment.':'Enregistrement r��cup��r�� poursuivi dans un nouveau segment.',
  'Recovered recording queued for transcription.':'Enregistrement r��cup��r�� mis en file pour transcription.','Recovered recording finalized and saved.':'Enregistrement r��cup��r�� finalis�� et enregistr��.','Recovered recording deleted.':'Enregistrement r��cup��r�� supprim��.',
  'Recording started with durable local recovery.':'Enregistrement d��marr�� avec r��cup��ration locale durable.','Choose a server folder for the final audio or another retention mode.':'Choisissez un dossier serveur pour l���audio final ou un autre mode de conservation.'
});
Object.assign(supplemental['zh-CN'], {
  'Long recordings are checkpointed locally while you speak. A reload or browser crash can recover the audio already delivered by the browser, then Continue in a new segment.':'������������������������������������������������������������������������������������������������������������������������������������������������������������������',
  'Recovery':'������','Protected':'���������','Browser storage':'���������������','Persistent':'������','Best effort':'������������','Unavailable':'���������','Checking���':'������������',
  'Recovered LIVE recording':'������������������������','available':'������','Started':'������������','Saved duration':'���������������','Protected audio':'���������������','Segments':'������','Interruption gap':'������������',
  'Listen to last 30 seconds':'������������ 30 ���','Continue':'������','Finish and transcribe':'���������������','Save audio':'������������','Delete recovery':'������������������',
  'Keep final audio':'������������������','No copy':'���������������','Audio destination':'������������������','Choose final audio folder':'���������������������������',
  'Choose a local audio folder':'���������������������������','Final M4A will download after server finalization':'������������������������������������ M4A','Choose folder':'���������������',
  'The browser recovery copy is kept until the finalized M4A is successfully saved.':'��������������������������������������� M4A ���������������������',
  'No permanent audio copy. The server still keeps the accepted LIVE source for the 24-hour retry window.':'������������������������������������������������������������������������������ 24 ������������������������',
  'Save final audio���':'���������������������','Final audio saved.':'������������������������','Recovered recording continued in a new segment.':'������������������������������������������',
  'Recording started with durable local recovery.':'������������������������������������������������'
});

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
