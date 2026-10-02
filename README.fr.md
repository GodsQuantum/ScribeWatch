<p align="center">
  <img src="docs/logo.svg" width="128" alt="Logo ScribeWatch">
</p>

<h1 align="center">ScribeWatch</h1>

<p align="center">
  <strong>La voix entre. Le Markdown sort. Automatiquement.</strong><br>
  Transcription auto-h��berg��e de notes vocales pour Obsidian, dossiers Markdown et workflows audio automatis��s.
</p>

<p align="center">
  <a href="https://github.com/GodsQuantum/ScribeWatch/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/GodsQuantum/ScribeWatch/actions/workflows/ci.yml/badge.svg"></a>
  <a href="LICENSE"><img alt="Licence MIT" src="https://img.shields.io/badge/license-MIT-3dd7cf"></a>
  <img alt="Rust" src="https://img.shields.io/badge/backend-Rust-ef7d57">
  <img alt="SvelteKit" src="https://img.shields.io/badge/UI-SvelteKit-ff3e00">
  <img alt="Docker" src="https://img.shields.io/badge/self--hosted-Docker-2496ed">
  <img alt="Compatible Obsidian" src="https://img.shields.io/badge/Obsidian-friendly-7c3aed">
</p>

<p align="center">�������� <a href="README.md">English README</a></p>

---

ScribeWatch transforme vos enregistrements en **notes Markdown propres et titr��es** avec votre propre moteur speech-to-text compatible OpenAI. D��posez une note vocale dans **Quick Transcribe**, enregistrez directement dans **Live Recorder**, ou pointez un **Workflow** vers un dossier et laissez ScribeWatch traiter automatiquement les nouveaux audios.

Le transcript reste canonique et d��terministe. Pour les usages qui demandent davantage de structure, un LLM compatible OpenAI peut ajouter apr��s transcription une vue structur��e r��utilisable ��� r��union, consultation, brainstorm ou profil personnalis�� ��� **sans jamais remplacer le transcript original**.

Le cas d���usage naturel est **notes vocales ��� Obsidian**, mais rien n���est verrouill�� �� Obsidian : n���importe quel dossier Markdown fonctionne.

<p align="center">
  <img src="docs/screenshots/home-quick-transcribe.png" width="100%" alt="Tableau de bord ScribeWatch et Quick Transcribe">
</p>

## ��� Pourquoi ScribeWatch ?

- **Les notes vocales deviennent de vrais fichiers utiles** ��� du `.md` lisible plut��t qu���une pile d���enregistrements oubli��s.
- **Quick Transcribe** ��� glissez un fichier audio et r��cup��rez imm��diatement du Markdown.
- **Live Recorder** ��� enregistrez un micro navigateur avec checkpoints OPFS progressifs, r��cup��ration apr��s crash/rechargement, **Continuer** apr��s interruption, ��coute avec r��glette de l���audio r��cup��r�� et conservation facultative du M4A final c��t�� serveur ou client.
- **Watch folders** ��� automatisez les nouveaux enregistrements d���un serveur, NAS ou dossier synchronis��.
- **Ingestion audio par contenu** ��� FFprobe d��tecte une piste audio d��codable au lieu de faire confiance �� l���extension ; FFmpeg normalise ensuite la source avant STT.
- **Profils de structure IA optionnels** ��� r��union, documentation de consultation m��dicale, brainstorm, interview et prompts personnalis��s peuvent ajouter une vue structur��e apr��s transcription.
- **Exports multi-formats** ��� le Markdown reste la source ; une note termin��e peut ��tre export��e en MD, TXT, HTML, DOCX, ODT ou PDF.
- **Pr��t pour Obsidian** ��� noms propres, titre H1, propri��t��s YAML et tags.
- **Votre moteur de transcription** ��� endpoint STT compatible OpenAI, notamment Speaches/Whisper.
- **Cha��nes de fallback r��silientes** ��� m��langez providers et mod��les dans l���ordre voulu. Si une route ��choue, ScribeWatch essaie automatiquement la suivante ��� y compris un autre mod��le du m��me provider.
- **Aucun LLM obligatoire** ��� titres et formatage sont d��terministes ; la transcription reste la source de v��rit��.
- **Paragraphes lisibles et d��terministes** ��� limites de phrases Unicode (UAX #29) et r��gles fixes de longueur rendent les longs transcripts lisibles sans paraphrase, r��sum�� ni titres invent��s.
- **Publication s��re** ��� le Markdown est publi�� avant l���archivage de l���audio, sans ��craser une note existante.
- **Petite stack auto-h��berg��e** ��� backend Rust/Axum, UI SvelteKit, SQLite, un conteneur.

## ������� Trois fa��ons de l���utiliser

| | **Quick Transcribe** | **Live Recorder** | **Watch folders** |
|---|---|---|---|
| Id��al pour | Un enregistrement existant | Enregistrer maintenant au micro | Une capture r��currente / automatique |
| Entr��e | Upload navigateur ou fichier serveur | Micro du navigateur | Dossier serveur/NAS surveill�� |
| Sortie | Dossier serveur ou votre ordinateur | Dossier serveur ou votre ordinateur | Dossier Markdown |
| Audio original | Jamais d��plac�� | Checkpoints navigateur r��cup��rables ; source serveur accept��e conserv��e 24 h pour Retry ; copie M4A permanente facultative | Archiv�� apr��s r��ussite du Markdown |
| Structure IA | Optionnelle | Optionnelle | Optionnelle |
| Exemple | �� Je viens d���enregistrer une id��e �� | R��union / consultation / brainstorm | T��l��phone/synchro ��� Obsidian automatiquement |

<p align="center">
  <img src="docs/screenshots/quick-transcribe.png" width="49%" alt="Quick Transcribe dans ScribeWatch">
  <img src="docs/screenshots/workflow-folders.png" width="49%" alt="Dossiers d���un workflow ScribeWatch">
</p>

## ���� Fallbacks provider + mod��le

Chaque route de transcription est un couple explicite **provider + mod��le**. ScribeWatch aspire la liste des mod��les expos��s par chaque provider configur�� : vous choisissez donc exactement la cha��ne voulue, y compris plusieurs mod��les d���un m��me provider.

```text
Speaches / whisper-large-v3
          ��� ��chec
Speaches / distil-whisper-large-v3
          ��� ��chec
OpenAI / gpt-4o-transcribe
          ���
Markdown
```

Chaque provider poss��de un timeout de requ��te r��ellement appliqu��, tandis que chaque route peut d��finir un d��lai de fallback ind��pendant et ��ventuellement plus court (`Jamais`, 10/30/60 minutes ou personnalis��). Une panne r��seau, timeout/erreur provider, rate limit ou r��ponse invalide peut passer �� la route suivante ; une annulation utilisateur stoppe toute la cha��ne. L���historique conserve chaque tentative et le provider/mod��le r��ellement utilis��.

## ���� Installation rapide

Pr��requis : Docker Engine + Compose et un endpoint de transcription compatible OpenAI.

```bash
git clone https://github.com/GodsQuantum/ScribeWatch.git
cd ScribeWatch
cp .env.example .env
cp compose.example.yaml compose.yaml
mkdir -p config data watch notes archive

docker compose pull
docker compose up -d
```

Ouvrez **http://127.0.0.1:3000**, puis :

> La capture micro du navigateur exige un contexte s��curis��. `localhost` / `127.0.0.1` convient en local ; pour les clients distants, utilisez normalement **HTTPS**. Si un poste acc��de au ScribeWatch du NAS en HTTP local, ouvrez **Live ��� Mode microphone local���** pour g��n��rer un installateur pr��configur�� **Linux, Windows ou macOS**. Il n���installe pas une seconde instance de ScribeWatch : seulement un proxy TCP limit�� �� `127.0.0.1` vers l���instance du NAS, puis ouvre `http://127.0.0.1:3052`. Linux utilise un socket systemd utilisateur, Windows `netsh interface portproxy`, et macOS un LaunchAgent utilisateur. Chaque installateur accepte aussi `--uninstall`. Le navigateur ne pouvant pas ex��cuter silencieusement un installateur syst��me, il faut lancer une fois le fichier t��l��charg��.

### Enregistrements LIVE longs et r��cup��ration apr��s crash

Pendant un LIVE, chaque tranche audio remise par MediaRecorder est ��crite progressivement dans l���**Origin Private File System (OPFS)** du navigateur au lieu de conserver toute la consultation en RAM JavaScript. ScribeWatch demande un stockage navigateur persistant lorsqu���il est disponible, force un checkpoint suppl��mentaire quand la page devient cach��e et utilise un Screen Wake Lock lorsque le navigateur le permet.

Apr��s un crash ou un rechargement, la session inachev��e r��appara��t. Vous pouvez **��couter et d��placer la r��glette** dans le dernier segment sauvegard��, lancer **��couter les 30 derni��res secondes**, puis **Continuer**. Continuer d��marre un nouveau segment MediaRecorder dans la m��me session logique ; le trou d���interruption estim�� est conserv�� comme m��tadonn��e et aucun faux silence n���est invent��.

Au Stop/finalisation, les segments termin��s sont envoy��s en streaming vers Cloud9 puis FFmpeg produit un seul M4A AAC mono canonique dans l���ordre du manifeste. L���audio final peut ��tre conserv�� en **aucune copie permanente**, dans un **dossier serveur choisi**, ou dans un **dossier choisi sur cet ordinateur**. La r��cup��ration locale n���est supprim��e qu���apr��s acceptation durable par le serveur et, si demand��, r��ussite de la copie locale.

Les sources Quick/LIVE accept��es restent disponibles **24 heures par d��faut**, y compris pour les jobs Error/Interrupted/Cancelled, afin qu���un Retry apr��s ��chec STT ou red��marrage de ScribeWatch dispose toujours du vrai fichier source. Cette fonction am��liore fortement la r��silience aux pertes d���enregistrement ; elle ne constitue **pas** �� elle seule une certification de dossier m��dical ni une conformit�� r��glementaire sant��.

1. Ajoutez votre moteur de transcription dans **STT**.
2. Utilisez **Quick** pour un fichier existant, **Live** pour un micro navigateur, ou cr��ez un **Workflow**.
3. Optionnel : ajoutez un LLM compatible OpenAI dans **AI Profiles** puis reliez un ou plusieurs profils de structure.
4. Pour automatiser : choisissez **Watch folder ��� Markdown folder ��� Audio archive**. Le dossier surveill�� est parcouru r��cursivement.
5. Laissez **Markdown folder** vide pour publier chaque note �� c��t�� de son audio source, y compris dans les sous-dossiers. **Audio archive** peut ��tre plac�� dans le Watch folder : toute son arborescence est alors toujours exclue de la surveillance.

> Par d��faut ScribeWatch reste li�� �� la machine locale. Pour un acc��s LAN/VPN, modifiez `SCRIBEWATCH_BIND_HOST` et utilisez votre politique habituelle de firewall/reverse proxy authentifi��.

## ���� Pens�� pour Obsidian ��� sans d��pendre d���Obsidian

Une transcription peut devenir :

```markdown
---
title: "Penser �� r��server le train demain"
created: 2026-09-16T08:42:00Z
tags:
  - voice-note
  - transcription
  - scribewatch
source: "Enregistrement 42.m4a"
provider: "Speaches local"
model: "whisper-large-v3"
language: "fr"
---

# Penser �� r��server le train demain

Penser �� r��server le train demain...
```

ScribeWatch d��rive le titre de la transcription, conserve des noms Unicode lisibles, d��duplique les tags et transforme les collisions en `Titre (2).md`, `Titre (3).md`, etc.

### Formatage d��terministe du transcript

Par d��faut ScribeWatch d��coupe le corps du transcript en paragraphes lisibles **sans LLM**. Les limites de phrases Unicode suivent **Unicode Standard Annex #29** via `unicode-segmentation` en Rust ; les phrases sont regroup��es avec des limites fixes de phrases/mots/caract��res ; un STT sans ponctuation est d��coup�� par blocs fixes de mots ; et les paragraphes d��j�� pr��sents sont conserv��s. **Aucun mot n'est r����crit, r��sum�� ou reclass�� s��mantiquement.**

D��cochez **Readable deterministic paragraphs** dans Quick Transcribe ou un Workflow pour conserver exactement le corps brut renvoy�� par le provider.

### Profils de structure IA optionnels

Le LLM est une **deuxi��me couche facultative**. ScribeWatch termine toujours le STT en premier et conserve le transcript intact. Lorsqu���un profil de structure est choisi, la note contient les deux vues :

```text
Audio ��� normalisation FFmpeg ��� STT ��� transcript canonique
                                         ������ Markdown d��terministe
                                         ������ profil IA optionnel ��� Notes structur��es
```

Les profils int��gr��s couvrent la note structur��e g��n��rale, les r��unions/appels, les brouillons de documentation de consultation m��dicale, les brainstorms marketing et les interviews/recherches. Ils peuvent ��tre reli��s �� n���importe quel endpoint chat-completions compatible OpenAI et modifi��s pour votre d��ploiement ; des profils enti��rement personnalis��s peuvent aussi ��tre cr����s.

Les longs transcripts sont trait��s en blocs d�����l��ments factuels born��s avant la synth��se finale. Le prompt syst��me traite le transcript comme une donn��e non fiable, interdit d���inventer des faits et impose de conserver explicitement les incertitudes. Si le LLM ��choue, **le job r��ussit quand m��me avec le transcript canonique** et l���erreur de structuration est enregistr��e s��par��ment.

<p align="center">
  <img src="docs/screenshots/mobile-quick-transcribe.png" width="390" alt="Quick Transcribe ScribeWatch sur mobile">
</p>

## ���� Fonctionnement d���un Workflow

```text
Dictaphone / synchro / NAS
            ���
            ���
       Watch folder
            ���
       transcription
            ���
            ���
      Markdown folder ������������������ Obsidian / notes / base de connaissances
            ���
    publication r��ussie
            ���
            ���
       Audio archive
```

L���audio n���est **jamais archiv�� avant la publication r��ussie du Markdown**. Les Watch folders sont r��cursifs : les ��v��nements filesystem natifs et la r��conciliation p��riodique couvrent aussi les sous-dossiers. Quand **Markdown folder** est vide, la note est publi��e dans le dossier exact contenant l���audio d��tect��. Toute l���arborescence configur��e comme **Audio archive** est ignor��e avant m��me le sondage audio ; une archive telle que `Watch/Vocaux` peut donc vivre dans l���arborescence surveill��e sans ��tre r��ing��r��e.

## ���� S��curit�� par conception

- les cl��s API restent c��t�� serveur et ne sont jamais renvoy��es au navigateur ;
- l���exploration serveur est limit��e aux racines explicitement autoris��es et bloque les sorties par symlink ;
- les uploads navigateur sont born��s pendant le streaming ; les sources Quick/LIVE r��cup��rables sont conserv��es pendant la fen��tre de r��tention configur��e au lieu d�����tre supprim��es apr��s un ��chec de transcription ;
- Quick Transcribe ne d��place ni ne supprime jamais la source originale ;
- une note Markdown existante n���est jamais ��cras��e silencieusement ;
- apr��s red��marrage, les travaux interrompus sont enregistr��s explicitement ;
- la structure LLM est best-effort : elle ne peut ni remplacer ni invalider un transcript canonique r��ussi ;
- l���export de documents neutralise les images Markdown et d��sactive le HTML brut avant conversion Pandoc ;
- le conteneur d���exemple tourne non-root, avec rootfs read-only, capabilities supprim��es et `no-new-privileges`.

Voir [`SECURITY.md`](.github/SECURITY.md).

## ������ Configuration

| Variable | D��faut | R��le |
|---|---|---|
| `SCRIBEWATCH_HOST` | `0.0.0.0` | Adresse HTTP interne au conteneur |
| `SCRIBEWATCH_PORT` | `3000` | Port HTTP |
| `SCRIBEWATCH_CONFIG_DIR` | `/config` | R��pertoire SQLite/config |
| `SCRIBEWATCH_DATA_DIR` | `/data` | Staging Quick Transcribe |
| `SCRIBEWATCH_ALLOWED_ROOTS` | requis | Racines serveur accessibles, s��par��es par `:` |
| `SCRIBEWATCH_SCAN_SECONDS` | `5` | Fr��quence de r��conciliation des Watch folders |
| `SCRIBEWATCH_FILE_STABILITY_MS` | `2000` | Fen��tre de stabilit�� avant ingestion |
| `SCRIBEWATCH_MAX_TRANSCRIPTION_JOBS` | `2` | Transcriptions simultan��es |
| `SCRIBEWATCH_MAX_UPLOAD_BYTES` | `2147483648` | Taille maximale d���un upload stream�� |
| `SCRIBEWATCH_QUICK_RESULT_RETENTION_HOURS` | `24` | R��tention des r��sultats client |
| `SCRIBEWATCH_QUICK_SOURCE_RETENTION_HOURS` | `24` | R��tention des sources Quick/LIVE r��cup��rables et du staging LIVE orphelin |

L���ingestion audio est **bas��e sur le contenu, pas sur l���extension**. Les fichiers stables d���un Watch folder et les uploads Quick sont sond��s avec FFprobe ; tout fichier contenant une piste audio d��codable par le FFmpeg embarqu�� est accept��, puis normalis�� en WAV PCM mono 16 kHz avant STT.

## ���� Dossiers de l���ordinateur client

Un navigateur ne peut pas surveiller en permanence n���importe quel dossier d���un autre ordinateur. ScribeWatch garde cette fronti��re explicite :

- l���audio local entre par glisser-d��poser, s��lecteur de fichier ou capture micro dans Live Recorder ;
- le Markdown termin�� peut ��tre ��crit directement dans un dossier local lorsque le navigateur supporte File System Access en contexte s��curis�� ;
- le M4A final d���un LIVE peut aussi ��tre enregistr�� dans un dossier client choisi apr��s finalisation canonique sur Cloud9 ; le handle du dossier reste c��t�� navigateur ;
- sinon le fichier est t��l��charg�� normalement.

Aucun handle de dossier local n���est envoy�� ni stock�� par le serveur ScribeWatch.

## ������� Stack & d��veloppement

**Backend :** Rust 1.98 �� Axum �� Tokio �� SQLite �� notify
**Frontend :** Svelte 5 �� SvelteKit 2 �� TypeScript
**M��dia / exports :** FFmpeg + FFprobe �� Pandoc �� WeasyPrint

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets
cd frontend && npm ci && npm test && npm run check && npm run build
```

Test end-to-end d��terministe :

```bash
bash scripts/e2e-v02.sh
```

## ���� API principale

- `POST /api/v1/quick/upload` ��� upload navigateur stream��
- `POST /api/v1/quick/server` ��� fichier audio serveur autoris��
- `POST /api/v1/live/upload` ��� finalisation LIVE multi-segments stream��e avec acceptation idempotente par session
- `GET /api/v1/jobs/{id}/audio` ��� M4A canonique conserv�� pour les jobs LIVE accept��s
- `GET /api/v1/jobs/{id}/markdown` ��� Markdown produit pour le client
- `GET /api/v1/jobs/{id}/export/{format}` ��� export MD/TXT/HTML/DOCX/ODT/PDF
- `GET|POST /api/v1/workflows` ��� automatisation Watch folder
- `GET|POST /api/v1/providers` ��� moteurs de transcription compatibles OpenAI
- `GET|POST /api/v1/llm-providers` ��� providers LLM compatibles OpenAI optionnels
- `GET|POST /api/v1/structure-profiles` ��� profils de structure LLM r��utilisables
- `GET /api/v1/jobs`, `GET /api/v1/events` ��� historique et ��tat live
- `GET /api/v1/health`, `GET /api/v1/ready` ��� sant��/readiness

## ���� Contribuer

Issues et pull requests sont bienvenues. Voir [`CONTRIBUTING.md`](.github/CONTRIBUTING.md) et [`CODE_OF_CONDUCT.md`](.github/CODE_OF_CONDUCT.md).

Si ScribeWatch trouve sa place dans votre stack, une ��� aide d���autres utilisateurs de notes vocales et d���Obsidian �� le d��couvrir.

## ���� Licence

[MIT](LICENSE)
