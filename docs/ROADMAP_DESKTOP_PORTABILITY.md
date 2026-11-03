# ScribeWatch — feuille de route UI, espaces, moteurs et signature

État observé au 3 novembre 2026. Source de vérité : Applications/ScribeWatch ; checkpoint de session : handoff/HANDOFF_CURRENT.md. Ce document décrit le chemin jusqu'à certification complète, sans transformer des objectifs en résultats acquis. Les captures Askara servent uniquement de référence ergonomique : aucun nom ou donnée personnelle ne doit apparaître dans le code, les tests, les exemples ou les publications.

## Architecture conservée — code commun

Une seule interface Svelte 5 pour navigateur, Linux x64/ARM64, macOS Intel/ARM et Windows x64, livrée par le même backend Rust. Les routes API et la base SQLite sont communes. Le desktop est une enveloppe native, non une application distincte. Chaque installation offre : transcription rapide, enregistrement LIVE et récupération, workflows de dossiers, fournisseurs externes, structuration LLM facultative, historique, exports et configuration. La transcription originale demeure la source canonique.

Deux moteurs sont optionnels et indépendants : Cloud9-Speaches pour STT + alignement, llama.cpp pour LLM. Pour chacun : Interne (enfant lancé localement), Externe (endpoint existant sur cet hôte ou un autre), Désactivé. Les poids sont téléchargés sur action explicite. On ne tue jamais un processus tiers par simple conflit de port.

État acquis : Windows RC6 full standalone non signée, un installeur NSIS unique, Cloud9-Speaches et llama.cpp CPU/CUDA/Vulkan, réel test Windows installation/démarrage API des deux moteurs/arrêt/désinstallation PASS, main af8a6cb. macOS/Linux livrent encore les paquets légers, avec cette même UI mais sans tous les binaires internes. Systray natif ScribeWatch déjà présent dans packaging/desktop/launcher/src/desktop_tray.rs.

## Lot 1 — Thèmes et espaces génériques

[x] Conserver Midnight par défaut, proposer Arctic (bleu clair), Graphite, Ivory et Iris. Préférences de couleur locales à l'appareil ; jamais de contenu clinique dans localStorage.
[x] Sidebar de navigation par espaces, création et suppression, boîte des non-classés, recherche Ctrl/⌘+K, liste limitée à 30 entrées visibles et chargement progressif.
[x] Sauvegarder les espaces dans SQLite serveur et les affectations des jobs dans des objets distincts afin que les workers/retries n'effacent pas les regroupements. Supprimer un espace ne touche ni aux sources audio, ni aux résultats, ni à l'historique.
[x] Nouveaux Quick et LIVE affectés à l'espace sélectionné ; anciens jobs classables manuellement.
[x] Test HTTP/SQLite réel : validation, redémarrage, affectation et conservation des jobs après suppression d'un espace.
[ ] Vérification visuelle réelle sur écrans 320 à 4K, macOS, Linux, Windows ; clavier, lecteur d'écran, zoom 200 %, contraste mesuré ; corrections si nécessaire.
[ ] Puis seulement, fusion sous contrôle CI et publication d'une nouvelle release UX issue du main certifié.

Améliorations UX ultérieures : renommage, archivage et recherche par date/type, raccourcis Documents, création d'espace depuis le recorder, filtres par statut et traitement des très grands historiques (1k/10k). Ne pas déplacer les fichiers ni créer un second catalogue métier.

## Lot 2 — Audit robustesse fonctionnelle

- Relire toute la chaîne d'enregistrement : autorisation micro, changement de device/hotplug, start/stop, interruption/reprise OPFS, perte réseau, réupload, audio final, conservation de source.
- Tester les fournisseurs et fallback STT, la détection de modèles, les erreurs HTTP, l'alignement, les délais, le choix de modèle LLM, le flux de téléchargement et déchargement VRAM.
- Vérifier exports MD/TXT/HTML/DOCX/ODT/PDF, statut des jobs et reprises. Un échec de résumé IA ne doit jamais modifier ni empêcher la transcription originale.
- Garder logs bornés, connexions limitées à localhost pour les moteurs embarqués, aucun jeton ni transcription dans les journaux, aucun service de fond supplémentaire.
- E2E avec données intégralement synthétiques, sans vrai nom de personne, après changement d'une étape métier ; ne pas refaire les certifications déjà valides si le code et l'environnement sont inchangés.

## Lot 3 — Llama.cpp, Atomic TurboQuant et Cloud9-engine

- Point de départ Windows NVIDIA : llama.cpp upstream b12074 déjà empaqueté, routeur natif, maximum un modèle actif, marge GPU 512 Mio et sleep-idle 45 s. La marge est contrôlée lors du chargement, pas une surveillance préemptive de toute VRAM externe.
- Alternative Atomic TurboQuant : fork AtomicBot b10298-2.1.0, release du 27 octobre 2026, KV TurboQuant. Base upstream plus ancienne que b12074 et paramétrages possibles spécifiques à AMD. TurboQuant agit surtout sur le cache KV, ne garantit ni débit supérieur ni mémoire libérée plus vite.
- Cloud9-engine actuel : routeur/orchestrateur multi-backends et recherche de patches Radeon 780M. Ses benchmarks Atomic1.6/Atomic2.1/llama.cpp sont réels pour 780M avec un modèle et des flags particuliers ; ils ne prouvent pas un champion universel sur Windows NVIDIA.
- A/B non biaisé requis avant tout remplacement : exactement même modèle GGUF, prompt, contexte, K/V réellement appliqué, longueur de sortie, simultanéité, température, latence démarrage, tok/s, mémoire CPU/GPU, idle unload, rechargement et fidélité du compte rendu.
- Profils à tester indépendamment : Cabinet2 i5-10500/16 Go/GTX 1650 4 Go ; Cabinet1 32 Go/RTX 3070 8 Go ; Cloud9 780M. Prévoir CPU fallback. Évaluer liquid 1.7B, Qwen 3.5 4B/9B et autre modèle réellement disponible sans inventer de numéro de version.
- Sources upstream : https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md et https://github.com/AtomicBot-ai/atomic-llama-cpp-turboquant. Ne pas reconditionner un fork non vérifié par défaut.

## Lot 4 — Full standalone macOS/Linux/Windows

1. Généraliser packaging/desktop/standalone_bundle.py, aujourd'hui Windows-only, avec archives vérifiées SHA256 par plateforme : Windows x64 CPU/Vulkan/CUDA, macOS Intel/ARM Metal/CPU, Linux x64/ARM64 Vulkan/CPU. Un seul serveur Rust/UI ; exécutables natifs adaptés à chaque OS.
2. Embarquer licences et notices originales ; tenir un manifeste de fichiers et d'empreintes ; poids téléchargés après consentement seulement. Confirmer les bibliothèques dynamiques et la compatibilité libc, Metal, CUDA/Vulkan. Ne pas installer un daemon root permanent.
3. Préserver les installateurs légers en parallèle comme repli. Intégrer les binaires seulement dans les variantes standalone, choisir le GPU disponible et revenir CPU après échec.
4. Systray : une seule instance, ouvrir/masquer fenêtre, état enregistrement, pause watchers, accès paramètres, arrêter les moteurs internes, quitter proprement. Tester le même comportement et l'arrêt des enfants sur Windows/macOS/Linux.
5. Matrice CI par architecture : installer ou monter le package, démarrer la véritable API STT et LLM sans modèle lourd, cataloguer les modèles, stopper, désinstaller, vérifier absence de processus orphelin et empreinte de l'artefact. Sur Linux AppImage, tester aussi FUSE/portable runtime.
6. Tests utilisateurs sur les deux cabinets : qualité des micros, perte réseau, trafic concurrent Handy/AutoSubs et VRAM, stabilité de longues transcriptions, sécurité des dossiers. Validation humaine pour tout document clinique.

## Lot 5 — Signature, provenance et releases

Inventaire réel du 3 novembre : aucun secret de signature GitHub Actions pour ScribeWatch et aucun certificat p12/pfx dans les dossiers de travail concernés. La signature éditeur reconnue est donc BLOQUÉE : ne pas fabriquer une auto-signature et la présenter comme reconnue.

Windows : certificat Authenticode OV/EV ou Azure Trusted Signing, signtool avec timestamp RFC3161, vérification indépendante de Get-AuthenticodeSignature. macOS : Apple Developer ID, signature de tous les frameworks et exécutables, hardened runtime, notarisation puis stapling. Linux : fichier SHA256 + signature détachée GPG/minisign avec clé publique officielle et provenance GitHub Actions.

Conserver les identifiants uniquement dans les secrets CI autorisés, jamais dans Git ou logs. Publier une nouvelle release si la signature change le hash de l'ancien artefact ; conserver les RC précédentes avec statut non signé. Les workflows de PR ne doivent pas avoir accès aux certificats de signature.

## Livraison et retour arrière

Avant fusion : Rust + Clippy, frontend check/test/build, API SQLite E2E, compilation Windows/macOS/Linux, tray tests, manifest/checksums et sécurité des installateurs. Comparer l'arbre Git testé et le commit de main ; promouvoir seulement un artefact exact avec sa somme SHA. Pas de déploiement CT300/CT410 hors demande séparée. Le rollback du thème ne change aucun fichier utilisateur ; les nouveaux objets space/job_space dans SQLite sont ignorés par une ancienne version (pas de migration destructive). Backup SQLite cohérent avant première migration serveur de production.

Critère final : UI fonctionnellement identique sur chaque plateforme, moteurs intégrés facultatifs disponibles dans chaque variante standalone certifiée, signature reconnue ou absence annoncée, performance mesurée et données originales préservées.
