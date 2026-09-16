<p align="center">
  <img src="docs/logo.svg" width="128" alt="Logo ScribeWatch">
</p>
<h1 align="center">ScribeWatch</h1>
<p align="center"><strong>Transformez vos notes vocales en Markdown avant de les oublier.</strong></p>
<p align="center"><a href="README.md">English README</a></p>

ScribeWatch est une application auto-hébergée qui transforme des enregistrements en notes Markdown propres et titrées avec votre propre moteur de transcription compatible OpenAI. Le cas d’usage central est simple : **note vocale → coffre Obsidian**, sans imposer Obsidian ni aucun LLM.

## Deux façons de l’utiliser

**Quick Transcribe** traite un fichier immédiatement : audio depuis l’ordinateur ou fichier déjà présent sur le serveur, puis sortie `.md` vers un dossier serveur ou retour vers l’ordinateur.

**Watch folders** automatise le flux : ScribeWatch attend qu’un audio soit stable, le transcrit, publie le Markdown dans le dossier choisi, puis archive l’audio source uniquement après publication réussie.

![Accueil ScribeWatch](docs/screenshots/home-quick-transcribe.png)<details>
<summary>Voir Quick Transcribe, l’éditeur de workflows et le mobile</summary>

![Quick Transcribe](docs/screenshots/quick-transcribe.png)

![Dossiers Watch, Markdown et Archive](docs/screenshots/workflow-folders.png)

<img src="docs/screenshots/mobile-quick-transcribe.png" width="390" alt="Quick Transcribe sur mobile">
</details>

## Pensé pour Obsidian

Le titre et le nom du fichier Markdown sont dérivés des premiers mots utiles de la transcription. Les propriétés YAML peuvent inclure titre, date, tags, source audio, workflow, provider, modèle et langue. Les tags par défaut sont `voice-note`, `transcription` et `scribewatch`, auxquels s’ajoutent vos tags de workflow.

Aucun LLM n’est nécessaire pour le titre ou le formatage.
## Démarrage rapide avec Docker Compose

```bash
cp .env.example .env
mkdir -p config data watch notes archive
cp compose.example.yaml compose.yaml
docker compose up --build -d
```

Ouvrez `http://127.0.0.1:3000`. Ajoutez ensuite un endpoint de transcription compatible OpenAI dans **Providers**, puis utilisez **Quick Transcribe** ou créez un workflow.

Un workflow peut choisir indépendamment :

1. **Watch folder** — arrivée des audios ;
2. **Markdown folder** — destination des notes, par exemple un dossier de vault Obsidian ;
3. **Audio archive** — destination de l’audio original après publication.

Laisser Markdown folder vide conserve le comportement historique : la note est écrite à côté de l’audio source.
## Garanties importantes

- l’audio d’un workflow n’est archivé qu’après publication durable du Markdown ;
- une note existante n’est jamais écrasée : les collisions deviennent `Titre (2).md`, `Titre (3).md`, etc. ;
- Quick Transcribe ne déplace ni ne supprime jamais la source originale ;
- les uploads navigateur sont bornés pendant le streaming puis nettoyés ;
- les clés API restent côté serveur ;
- le navigateur serveur est limité aux racines explicitement autorisées et bloque les sorties par symlink ;
- après redémarrage, un job actif devient explicitement `interrupted` plutôt que d’être déclaré terminé à tort.

## Dossiers de l’ordinateur client

Un serveur web distant ne peut pas surveiller librement les dossiers d’un ordinateur client. ScribeWatch sépare donc clairement les deux mondes : l’audio local entre par glisser-déposer ou sélecteur de fichier ; le Markdown peut être écrit directement dans un dossier local lorsque le navigateur autorise File System Access en contexte sécurisé, sinon le `.md` est téléchargé normalement.

Aucun handle de dossier local n’est transmis ni conservé par le serveur.

Pour les variables d’environnement, l’API et les commandes de développement complètes, voir [README.md](README.md). Licence MIT.