<p align="center">
  <img src="docs/logo.svg" width="128" alt="Logo ScribeWatch">
</p>

<h1 align="center">ScribeWatch</h1>

<p align="center">
  <strong>La voix entre. Le Markdown sort. Automatiquement.</strong><br>
  Transcription auto-hébergée de notes vocales pour Obsidian, dossiers Markdown et workflows audio automatisés.
</p>

<p align="center">
  <a href="https://github.com/GodsQuantum/ScribeWatch/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/GodsQuantum/ScribeWatch/actions/workflows/ci.yml/badge.svg"></a>
  <a href="LICENSE"><img alt="Licence MIT" src="https://img.shields.io/badge/license-MIT-3dd7cf"></a>
  <img alt="Rust" src="https://img.shields.io/badge/backend-Rust-ef7d57">
  <img alt="SvelteKit" src="https://img.shields.io/badge/UI-SvelteKit-ff3e00">
  <img alt="Docker" src="https://img.shields.io/badge/self--hosted-Docker-2496ed">
  <img alt="Compatible Obsidian" src="https://img.shields.io/badge/Obsidian-friendly-7c3aed">
</p>

<p align="center">🇬🇧 <a href="README.md">English README</a></p>

---

ScribeWatch transforme vos enregistrements en **notes Markdown propres et titrées** avec votre propre moteur speech-to-text compatible OpenAI. Déposez une note vocale dans **Quick Transcribe**, ou pointez un **Workflow** vers un dossier et laissez ScribeWatch traiter automatiquement les nouveaux audios.

Le cas d’usage naturel est **notes vocales → Obsidian**, mais rien n’est verrouillé à Obsidian : n’importe quel dossier Markdown fonctionne.

<p align="center">
  <img src="docs/screenshots/home-quick-transcribe.png" width="100%" alt="Tableau de bord ScribeWatch et Quick Transcribe">
</p>

## ✨ Pourquoi ScribeWatch ?

- **Les notes vocales deviennent de vrais fichiers utiles** — du `.md` lisible plutôt qu’une pile d’enregistrements oubliés.
- **Quick Transcribe** — glissez un fichier audio et récupérez immédiatement du Markdown.
- **Watch folders** — automatisez les nouveaux enregistrements d’un serveur, NAS ou dossier synchronisé.
- **Prêt pour Obsidian** — noms propres, titre H1, propriétés YAML et tags.
- **Votre moteur de transcription** — endpoint STT compatible OpenAI, notamment Speaches/Whisper.
- **Aucun LLM obligatoire** — titres et formatage sont déterministes ; la transcription reste la source de vérité.
- **Publication sûre** — le Markdown est publié avant l’archivage de l’audio, sans écraser une note existante.
- **Petite stack auto-hébergée** — backend Rust/Axum, UI SvelteKit, SQLite, un conteneur.

## 🎙️ Deux façons de l’utiliser

| | **Quick Transcribe** | **Watch folders** |
|---|---|---|
| Idéal pour | Une note vocale maintenant | Une capture récurrente / automatique |
| Entrée | Upload navigateur ou fichier serveur | Dossier serveur/NAS surveillé |
| Sortie | Dossier serveur ou votre ordinateur | Dossier Markdown |
| Audio original | Jamais déplacé | Archivé après réussite du Markdown |
| Exemple | « Je viens d’enregistrer une idée » | Téléphone/synchro → Obsidian automatiquement |

<p align="center">
  <img src="docs/screenshots/quick-transcribe.png" width="49%" alt="Quick Transcribe dans ScribeWatch">
  <img src="docs/screenshots/workflow-folders.png" width="49%" alt="Dossiers d’un workflow ScribeWatch">
</p>

## 🚀 Installation rapide

Prérequis : Docker Engine + Compose et un endpoint de transcription compatible OpenAI.

```bash
git clone https://github.com/GodsQuantum/ScribeWatch.git
cd ScribeWatch
cp .env.example .env
cp compose.example.yaml compose.yaml
mkdir -p config data watch notes archive

docker compose up --build -d
```

Ouvrez **http://127.0.0.1:3000**, puis :

1. Ajoutez votre moteur de transcription dans **Providers**.
2. Déposez un enregistrement dans **Quick Transcribe** — ou créez un Workflow.
3. Pour automatiser : choisissez **Watch folder → Markdown folder → Audio archive**.
4. Pointez le dossier Markdown vers votre vault Obsidian, dossier synchronisé, NAS ou autre destination Markdown.

> Par défaut ScribeWatch reste lié à la machine locale. Pour un accès LAN/VPN, modifiez `SCRIBEWATCH_BIND_HOST` et utilisez votre politique habituelle de firewall/reverse proxy authentifié.

## 🧠 Pensé pour Obsidian — sans dépendre d’Obsidian

Une transcription peut devenir :

```markdown
---
title: "Penser à réserver le train demain"
created: 2026-09-16T08:42:00Z
tags:
  - voice-note
  - transcription
  - scribewatch
source: "Enregistrement 42.m4a"
language: "fr"
---

# Penser à réserver le train demain

Penser à réserver le train demain...
```

ScribeWatch dérive le titre de la transcription, conserve des noms Unicode lisibles, déduplique les tags et transforme les collisions en `Titre (2).md`, `Titre (3).md`, etc.

<p align="center">
  <img src="docs/screenshots/mobile-quick-transcribe.png" width="390" alt="Quick Transcribe ScribeWatch sur mobile">
</p>

## 🔄 Fonctionnement d’un Workflow

```text
Dictaphone / synchro / NAS
            │
            ▼
       Watch folder
            │
       transcription
            │
            ▼
      Markdown folder ─────► Obsidian / notes / base de connaissances
            │
    publication réussie
            │
            ▼
       Audio archive
```

L’audio n’est **jamais archivé avant la publication réussie du Markdown**. Les événements filesystem natifs sont complétés par une réconciliation périodique, utile notamment avec certains dossiers réseau.

## 🔒 Sécurité par conception

- les clés API restent côté serveur et ne sont jamais renvoyées au navigateur ;
- l’exploration serveur est limitée aux racines explicitement autorisées et bloque les sorties par symlink ;
- les uploads navigateur sont bornés pendant le streaming puis nettoyés ;
- Quick Transcribe ne déplace ni ne supprime jamais la source originale ;
- une note Markdown existante n’est jamais écrasée silencieusement ;
- après redémarrage, les travaux interrompus sont enregistrés explicitement ;
- le conteneur d’exemple tourne non-root, avec rootfs read-only, capabilities supprimées et `no-new-privileges`.

Voir [`SECURITY.md`](.github/SECURITY.md).

## ⚙️ Configuration

| Variable | Défaut | Rôle |
|---|---|---|
| `SCRIBEWATCH_HOST` | `0.0.0.0` | Adresse HTTP interne au conteneur |
| `SCRIBEWATCH_PORT` | `3000` | Port HTTP |
| `SCRIBEWATCH_CONFIG_DIR` | `/config` | Répertoire SQLite/config |
| `SCRIBEWATCH_DATA_DIR` | `/data` | Staging Quick Transcribe |
| `SCRIBEWATCH_ALLOWED_ROOTS` | requis | Racines serveur accessibles, séparées par `:` |
| `SCRIBEWATCH_SCAN_SECONDS` | `5` | Fréquence de réconciliation des Watch folders |
| `SCRIBEWATCH_FILE_STABILITY_MS` | `2000` | Fenêtre de stabilité avant ingestion |
| `SCRIBEWATCH_MAX_TRANSCRIPTION_JOBS` | `2` | Transcriptions simultanées |
| `SCRIBEWATCH_MAX_UPLOAD_BYTES` | `2147483648` | Taille maximale d’un upload streamé |
| `SCRIBEWATCH_QUICK_RESULT_RETENTION_HOURS` | `24` | Rétention des résultats client |

Formats audio candidats : `mp3`, `wav`, `m4a`, `flac`, `ogg`, `opus`, `aac`, `wma`, `aiff`, `aif`, `caf`, `webm`.

## 🧩 Dossiers de l’ordinateur client

Un navigateur ne peut pas surveiller en permanence n’importe quel dossier d’un autre ordinateur. ScribeWatch garde cette frontière explicite :

- l’audio local entre par glisser-déposer ou sélecteur de fichier ;
- le Markdown terminé peut être écrit directement dans un dossier local lorsque le navigateur supporte File System Access en contexte sécurisé ;
- sinon le `.md` est téléchargé normalement.

Aucun handle de dossier local n’est envoyé ni stocké par le serveur ScribeWatch.

## 🛠️ Stack & développement

**Backend :** Rust 1.98 · Axum · Tokio · SQLite · notify
**Frontend :** Svelte 5 · SvelteKit 2 · TypeScript

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets
cd frontend && npm ci && npm test && npm run check && npm run build
```

Test end-to-end déterministe :

```bash
bash scripts/e2e-v02.sh
```

## 📡 API principale

- `POST /api/v1/quick/upload` — upload navigateur streamé
- `POST /api/v1/quick/server` — fichier audio serveur autorisé
- `GET /api/v1/jobs/{id}/markdown` — Markdown produit pour le client
- `GET|POST /api/v1/workflows` — automatisation Watch folder
- `GET|POST /api/v1/providers` — moteurs de transcription compatibles OpenAI
- `GET /api/v1/jobs`, `GET /api/v1/events` — historique et état live
- `GET /api/v1/health`, `GET /api/v1/ready` — santé/readiness

## 🤝 Contribuer

Issues et pull requests sont bienvenues. Voir [`CONTRIBUTING.md`](.github/CONTRIBUTING.md) et [`CODE_OF_CONDUCT.md`](.github/CODE_OF_CONDUCT.md).

Si ScribeWatch trouve sa place dans votre stack, une ⭐ aide d’autres utilisateurs de notes vocales et d’Obsidian à le découvrir.

## 📄 Licence

[MIT](LICENSE)
