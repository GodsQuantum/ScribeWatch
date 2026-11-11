# ScribeWatch — Handoff canonique

Actualisé le **10/11/2026**. Branche : fix/bounded-local-clinical-llm-20261109. **Aucun déploiement sur les PC du cabinet dans cette reprise.**

## Objectif et architecture

Brouillon de **lettre de consultation dentaire en français**, fidèle à une transcription et destiné à une relecture médicale obligatoire. Séparer génération des faits et rendu PDF. Modèles locaux compatibles matériel varié ; secours CPU et contextes limités.

1. **Source entière en priorité** lorsque le tokenizer confirme entrée + réserve de génération dans le contexte réellement disponible.
2. Sinon **extraits par thème pris à plusieurs moments de la consultation** ; à défaut découpage chronologique avec raccord d'environ 850 caractères et garde contre les doublons.
3. Aucune invention de latéralité, acte réalisé, prescription, diagnostic, date, identifiant ou condition.
4. PDF déterministe indépendant de l'inférence : identité du dossier, deux colonnes, texte français, gras réel, tableau de suivi, pictogrammes vectoriels, avertissement brouillon.

## État code et tests réels sur CT700

- Backend Rust : src/llm.rs, changement de priorité des routes, recouvrement dans extraction segmentée ; **121/121 tests Rust PASS**, format PASS.
- Frontend : medical-report.ts, rendu papier sobre et gras, reconnaissance du Markdown structuré, assainissement des balises HTML de tableau.
- Tests Node rendus compatibles avec les binaires Linux/macOS/Windows sans TypeScript natif via dépendance de développement tsx et paths TypeScript relatifs.
- **80/80 tests frontend PASS**, svelte-check 0 erreur/0 avertissement, build Vite PASS, PDF fictif A4 effectivement généré et inspecté visuellement.
- Les tests API, Windows distant, données de consultation, GPU 4 Go et revue clinique humaine ne sont pas certifiés.

## Benchmarks sur la même transcription et référence privée

| Variante Qwen3.5 4B Q4_K_M sur CT700 | Temps total ou génération | Couverture lexicale indicative | Verdict |
| --- | ---: | ---: | --- |
| Baseline transcription entière | 150 s | 75,9 % | Non validé |
| Extraction EvidenceLedger chronologique + synthèse | env. 586 s | 83,9 % | Erreurs et omissions cliniques |
| Prompt français piloté, transcription entière | 174,75 s | 81,6 % | Erreurs cliniques persistantes |
| Index lexical de citations + transcription entière | 119,89 s | 72,4 % | Régression : rejet |
| 8 extractions thématiques de passages éloignés, assemblage sans second LLM | 334,77 s | 92,0 % (45/48) | Au moins 6 contradictions/ajouts non étayés : non validé |

**Important :** ce score est un simple écran lexical à 48 critères. Il ne mesure ni fidélité clinique ni similarité sémantique au document de référence. Rejeter une sortie avec faits erronés, inversions de dents, acte inventé, conclusion hâtive ou omission critique, indépendamment du score. Les noms et fichiers de patients, sources ASR, PDF de référence et rapports générés restent strictement dans le dossier privé Documents/ScribeWatch/Benchmark-prive/LLM-2026-11-09, hors Git et hors serveur externe.

## Reprise du 10/11 — limitation des inventions et des contradictions

- Audit FactEHR, VeriFact et recommandations NHS : le texte médical doit être comparé par **affirmations atomiques** aux preuves de la source, mais une citation exacte peut soutenir une conclusion erronée ; un même LLM utilisé comme juge n'est pas fiable pour certifier sa propre génération.
- Pilote privé de relecture locale sur **26 affirmations à risque** du précédent compte rendu thématique : Qwen 4B en a signalé **6**, mais a laissé passer au moins deux affirmations indûment certaines. Les vérifications déterministes ont porté les alertes combinées à **8 sur ces 26**, sans démontrer une précision clinique finale.
- Pilote privé « preuve d'abord » sur trois domaines : **128,33 s**, 23 éléments proposés, 21 citations verbatim rattachées à la transcription et 2 rejetées. Plusieurs des 21 éléments pourtant cités restaient factuellement problématiques (chronologie/conditions). **Procédé NON intégré** à la génération clinique.
- **Nouveau garde-fou natif** dans src/clinical_guard.rs, appliqué seulement au profil opt-in dental-consultation-complete-fr après production de Markdown : détection ciblée et mise en quarantaine d'affirmations non justifiées ou contradictoires (action programmée vs proposée, extraction conditionnelle vs obligatoire, questions de dépistage contredites ensuite, numérotation FDI/côté, nom de professionnel introuvable, procédure non confirmée comme réalisée, examen conditionnel, consultation future contredite par une action ultérieure). Les lignes problématiques sont remplacées par des mentions à valider ; **les lignes de tableau sont contrôlées aussi**. Source et original non modifiés. Aucun appel LLM supplémentaire ; fonctionnement CPU/OS indépendant.
- Régression privée sur la transcription inchangée : **4 passages à risque détectés** dans le compte rendu thématique, **2 latéralités discordantes** dans le rapport direct, **4 passages à risque** dans le pilote preuve d'abord. Ces nombres sont des alertes, non un taux de correction certifié.
- **139 tests Rust PASS**, **80 tests frontend PASS**, npm run check 0 erreur/avertissement et build frontend PASS ; pas de test E2E Windows ni de validation du compte rendu par le praticien. Un mot ambigu issu de l'ASR peut figurer dans une citation exacte et être néanmoins faux : seule une relecture de l'audio ou une seconde hypothèse ASR locale peut arbitrer.
- Confidentialité : tous les extraits, étiquettes de revue, sorties de modèle et rapports de comparaison restent uniquement dans Benchmark-prive ; le code Git ne comporte que des règles générales et des tests fictifs distincts. Logs : quantités, lignes et motifs statiques, jamais les citations réelles.

**Décision :** garder le garde-fou natif conservateur, sans en faire un label de qualité médicale. Rejeter le juge 4B et le pipeline citations comme certifications autonomes. Suite : identifier les passages ASR réellement ambigus avec l'audio source sur machine autorisée, puis validation humaine et tests E2E sur la machine Windows cible. Ne pas multiplier les essais multi-modèles avant une vérité terrain vérifiée.

## Reprise — longueur proportionnée et déploiement Windows (10/11/2026)

- **La référence originale de cette consultation fait 4 pages et environ 1 681 mots. Ce sont des mesures de ce seul cas, pas des bornes pour d'autres consultations.** La version à 8 pages était un assemblage de sorties thématiques répétitives ; elle ne doit pas être présentée comme sortie validée.
- Recherche consultée : étude d'évaluation ADS, npj Digital Medicine (2025), étude PDQI-9 (2025), AMA Journal of Ethics sur le « note bloat » (2025), NHS England ADS guidance mise à jour juillet 2026. Critères complémentaires : exactitude, exhaustivité, synthèse, concision, pertinence, structure, cohérence interne, utilité clinique, prudence. **Aucune longueur ou structure fixe** ; détails explicatifs réels préservés même sur consultations longues.
- Profil par défaut dental-consultation-complete-fr modifié : LONGUEUR ADAPTATIVE SANS QUOTA, une occurrence de chaque fait pertinent, rapprochement des passages distants, détails de raisonnement quand présents, pas de rubrique ajoutée sans sujet, pas de texte de remplissage. La même exigence est ajoutée aux profils anglais/chinois, aux consignes par modèle et au repli de transcription longue.
- Nouveaux prompts par langue de l'UI (FR/EN/zh-CN) : synchronisation uniquement des profils intégrés restés inchangés ; ne jamais écraser les prompts personnalisés ou l'association au fournisseur. Au démarrage et au changement de langue, synchronisation des profils intégrés.
- Sur CabinetDentaire2, accès SSH via Cloud9 → Résurrection fonctionnel ; Windows 11 Pro, GTX 1650 4 Go, 16 Go RAM. Version installée ScribeWatch 0.5.3 **toujours opérationnelle après rollback**, moteur ASR/LLM présents. Le backend et frontend candidats ont réussi le smoke test isolé ; remplacement in-place antérieur annulé suite à un contrôle d'installation. **Ne pas déclarer nouvelle version déployée** sans résultat /ready et test PDF réellement généré par l'app installée.
- Icône Applications installées : entrée Windows HKCU DisplayIcon corrigée en direct, et correction durable ajoutée au script NSIS pour futurs installs.
- Audio de comparaison présent uniquement sur CabinetDentaire2 dans dossier local privé ScribeWatch/private-benchmarks/input ; les transcriptions, PDF et logs médicaux doivent rester dans les espaces privés et ne jamais être inclus dans Git ni API publiques.
- Compte rendu en cours : une génération Qwen3.5 4B, **transcription entière**, prompt adaptatif, moteur Vulkan local, évaluation contre la référence **après** génération. Pas de certificat clinique avant relecture du praticien.
- Régression locale validée avant nouvelle génération : **146/146 tests Rust** ; derniers frontend **80/80** et Svelte check **0 erreur / 0 avertissement**. Recompiler et retester la livraison finale après le dernier amendement.

**Prochaine étape stricte :** inspecter le rapport adaptatif (fidélité et longueur uniquement comme conséquences des faits) et contrôler PDF généré avec le moteur de production ; si et seulement si convaincant, intégrer au bundle Windows, valider microphone → ASR → LLM → PDF sur CabinetDentaire2. Ne passer à CabinetDentaire1 qu'après validation du premier.

## CabinetDentaire2 — déploiement pilote réel du 10/11 (WireGuard Pelleport)

- Accès direct SSH Cloud9 vers cabinetdentaire2 et cabinetdentaire1 par interface WireGuard Pelleport (Cloud9 VPN 192.168.27.67) confirmé le 10/11 ; route directe /32 vers .74 et .10. Ancien ProxyCommand SSH via Résurrection toujours présent dans la config de Cloud9 : forcer ProxyCommand=none,ProxyJump=none pour les contrôles directs tant que WireGuard est actif. Tunnel wg-quick démarré ponctuellement ; ne pas prétendre autostart activé.
- Candidat ScribeWatch 0.5.3 + correctifs : backend Windows GNU cross-build SHA256 4141d4c24fc0ab058f9f3e6e12fe913f87d7c47f274757f51c0779c3cfa39037, frontend produit avec tests et Svelte check PASS ; smoke Windows isolé HTTP 200, locale FR→EN→FR PASS, aucune donnée clinique exposée. L'ancienne erreur du script de mise à niveau venait du launcher GUI sans stdout / $LASTEXITCODE, pas du backend.
- **Nouvelle version effectivement installée** sur CabinetDentaire2, /ready OK, launcher en session interactive, 28 tâches historiques préservées, rollback des fichiers/config disponible dans le dossier privé qa-cab2-20261110/rollback. Icône Windows corrigée via DisplayIcon et dans NSIS.
- Le profil dentaire FR a été associé à l'unique fournisseur LLM local existant Qwen3.5 4B Q4_K_M (sinon l'UI n'aurait aucun LLM à appeler). Audio de référence de 61 min 41 s, source privée sur le poste ; **job réel lancé par ScribeWatch sur Parakeet Ultra**, puis génération locale Qwen, sans cloud, sans copie publique. Ne considérer la QA médicale comme validée que lorsque le rapport final aura été confronté par le praticien à la référence.
- Les essais hors production avec long transcript complet, prompts successifs : ancien pilote 1 363 mots, couverture indicatrice 81,6 % ; premier adaptatif 770 mots, 64,4 % ; adaptatif compact 1 228 mots, 78,2 %. **Aucun de ces scores lexicaux ne prouve une fidélité médicale**, ils démontrent au contraire qu'il faut préserver toutes les explications utiles sans quotas de longueur. Le prompt livré est adaptatif, sans nombre cible de pages, de mots ou de sections.
- **NE PAS basculer CabinetDentaire1** avant un PDF sans labels de benchmark, réellement exporté par le code ScribeWatch depuis le job de CabinetDentaire2, puis relecture utilisateur/clinicien et validation des faits significatifs. Le prochain modèle Qwen3.8 4B Distil reste explicitement différé jusqu'à validation de Qwen3.5 4B.

## QA clinique réelle et correctifs de sûreté — 10 novembre 2026

- Deux jobs réels Windows sur le même enregistrement local de 61 min 41 s : Parakeet Ultra a reproduit presque à l'identique la transcription déjà archivée. Aucune correction prédictive LLM libre retenue.
- Premier job avec ancien llama.cpp orphelin à 8 192 tokens, non suivi par le nouveau backend : fallback thématique, 4 483 mots, 43 titres, 77 % de couverture lexicale indicative. Non acceptable.
- Ancien moteur arrêté après contrôle de 0 job actif et vérification de son emplacement. Redémarrage par API native embedded/mode avec Origin local valide ; moteur désormais supervisé par ScribeWatch, contexte réel 24 576 tokens.
- Second job réel avec 24 576 tokens : 1 390 mots, 13 titres, 72,4 % de couverture lexicale indicative, sans erreur technique. Présentation beaucoup plus concise, mais 12 éléments lexicaux pertinents absents et inversion critique entre état du nerf/pulpe et dévitalisation. Rejet comme compte rendu médical certifiable. Aucun PDF patient présenté comme validé.
- Garde-fou natif étendu : détecte les formulations associant dévitalisation certaine à absence explicite d'atteinte pulpaire et la durée de brossage attribuée par secteur sans confirmation source. Mise en quarantaine, pas de correction médicale automatique. 148 tests Rust PASS, dont 20 tests du garde-fou ; trois alertes spécifiques sur la partie structurée du document Windows.
- Backend de sécurité recompilé Windows SHA256 f7ce791b8f7e31b2511519d879b71893935a8cfb5582cf17a5afe94f9daf7e40 et installé sur CabinetDentaire2 après sauvegarde et arrêt/reprise contrôlés du modèle via API native. Vérification déploiement PASS : 30 jobs historiques conservés, GUI interactive, /ready et llama.cpp contexte 24 576 ; rollback conservé dans qa-cab2-20261110/rollback-guard.
- Prochaine étape : améliorer l'exhaustivité et la fidélité des recommandations, conditions et actes sans inversion ni omissions importantes avant présentation du PDF de production. Si le 4B local n'atteint pas le niveau nécessaire, signaler sa limite. Cab1 et Qwen3.8 toujours différés jusqu'à validation de Cab2.

## Qwen3.8 Distill — 10 novembre 2026

- Exécution demandée : empero-ai/Qwen3.8-4B-Distill-GGUF:Q4_K_M (modèle communautaire Qwen3.5-4B distillé, GGUF Apache-2.0), téléchargé via POST /models de llama.cpp natif sur CabinetDentaire2. La version Qwen3.5-4B antérieure reste installée pour rollback.
- Test synthétique Windows : activation chat_template_kwargs.enable_thinking=false ; réponse française avec condition pulpe conservable vs infection préservée, finish_reason=stop, aucun raisonnement parasite.
- Correctif backend ScribeWatch : traitement déterministe et sans bloc de raisonnement aussi pour Qwen3.8 ; consignes modèle génériques pour respecter faits distants, conditions, raisonnement et longueur adaptative, sans insérer la référence patient dans les prompts.
- Build Windows SHA256 2916b0104f16931bd8cd66d9c865ad903ae200927c289379d0f544383f1267fc installé sur CabinetDentaire2 après sauvegarde ; santé, contexte 24 576, 30 anciens jobs préservés. Profil dentaire lié au modèle Qwen3.8 (Qwen3.5 rollback disponible).
- Job réel terminé sur audio d'origine du cabinet (Parakeet Ultra puis Qwen3.8), identifiant privé b018c09f-b450-4a62-a0fc-b1be20fb03d1 ; PDF natif livré pour relecture, mais résultat non certifié. **Ne pas certifier qualité clinique sans inspecter résultat réel**, contradictions et données omises. PDF doit être exporté par frontend/src/lib/medical-report.ts en production (sans mentions de test) et présenté au demandeur même si relecture encore nécessaire.
- Git et handoff : conserver un seul canonique, pas de données patient, de transcription, ni de référence PDF dans Git.

## Déploiement UI CAB2 — profils, Unicode, GGUF et dossiers — 10 novembre 2026

- Demande pendant le pilote Qwen3.8 : accents brisés dans les profils, doublons cliniques, dossier GGUF inaccessible, libellé de découverte de modèles ambigu, Lemonade proposé en double, Parakeet Ultra non prioritaire, profils manquants dans Enregistreur direct, « Server » trompeur, destination Markdown non sélectionnable avant l'export. Aucune transcription ASR relancée.
- Backend : 5 profils intégrés canoniques (dental-consultation-complete-fr, medical-consultation, meeting-minutes, marketing-brainstorm, general-structured-note), sans supprimer les entrées historiques ni personnalisées. Réparation UTF-8 strictement des profils intégrés corrompus, préservation du LLM associé ; autobinding seulement quand un unique LLM est actif.
- Cinq noms en français « Consultation dentaire », « Consultation médicale », « Réunion et entretien », « Brainstorming », « Note structurée ». Prompts synchronisés à la langue de l'UI ; PowerShell 5.1 Invoke-RestMethod interprétait à tort en ANSI le JSON UTF-8, produisant un faux rejet de déploiement. Le premier rollback a restauré l'app. Deuxième déploiement réussi avec décodage UTF-8 explicite sur les octets JSON.
- Frontend : cinq profils dans l'enregistreur direct et Quick Transcribe, modèles sans fournisseur signalés. Parakeet Ultra prioritaire si fournisseur activé. Destination Markdown sélectionnable via le navigateur avant enregistrement, sans sauvegarder de données patient dans les préférences. Destination clarifiée entre dossier géré par ScribeWatch et dossier sélectionné dans le navigateur sur le même PC.
- Moteur : llama.cpp natif conservé, option Atomic Chat par API compatible lorsqu'il est installé (aucune installation forcée), suppression des offres Lemonade de l'interface. Bouton « Lister les modèles disponibles » avec aide sur l'endpoint, vérifié pour six modèles. API native avec contrôles origine locale de poste ouvre Explorer sur le véritable répertoire modèles GGUF embedded/llm-models (test d'ouverture réussi). Actualiser relit /models?reload=1 ; appel API natif réel sur CAB2 vérifié avec 6 modèles, sans redémarrage ni timer. Dernier complément déployé et testé le 10/11.
- Déploiement pré-rafraîchissement : backend Windows sha256 c0d837cbe2feab59718f28bed26f28f3386332fa39c94f7441414d6e129a5101, /ready, tray visible, cinq profils Unicode sains liés à Qwen3.8, 31 jobs préservés, zéro job actif, rollback au chemin qa-cab2-20261110/rollback-ux-verified-20261110. CabinetDentaire1 inchangé.
- Vérifications : 149 tests Rust et 80 frontend avant le rescan GGUF ; après ce changement, 149 Rust et 81 frontend réussis, 0 erreur/warning TypeScript. Le brouillon Qwen3.8 de 789 mots et 60,9 % de couverture indicative demeure non certifié, avec un PDF de référence privée à relire.
- Livraison finale : backend Windows SHA256 08e60b40d603dbb0f073f4fdb03caa63d2d17df3917ae0be683f4e847106263d + frontend mis à jour. CAB2 : /ready PASS, GUI Windows visible, 5 profils Unicode sains, 6 modèles GGUF rescannés par API, Qwen3.8 actif, 31 tâches préservées, 0 tâche active. Nettoyage effectué : 6 dossiers QA jetables et 10 fichiers de transfert/staging supprimés, 6 dossiers rollback conservés. CAB1 inchangé. Prochaine étape = qualité du rapport depuis la transcription Parakeet déjà conservée ; aucune retranscription de l'audio.

## Décisions et suite

- Les recherches sur le « lost in the middle » et l'évaluation clinique en 2025–2026 confirment que la capacité du contexte seule ne garantit pas la couverture. Un découpage temporel indépendant peut perdre des relations entre faits éloignés ; rechercher d'abord des extraits thématiques avec citations à la source.
- Le pilote thématique améliore fortement la couverture **lexicale** mais produit encore des erreurs cliniques importantes : **ne pas intégrer en production**. Audit de six anomalies conservé dans le dossier privé ; utiliser ce témoin négatif pour l'évaluation qualitative.
- Prochaine optimisation : vérification de chaque allégation clinique par citation source, refus des contradictions et hallucinations ; refaire une seule mesure end-to-end avant de considérer l'intégration du mode thématique.
- Ensuite comparer les autres modèles retenus avec un protocole commun et des réglages familiaux, puis vérifier microphone → ASR → modèle → PDF sur les deux PC du cabinet. Aucun rapport n'est utilisable cliniquement avant validation du praticien.

Fichier de statut unique : ce document. Le pointeur racine HANDOFF_CURRENT.md demeure inchangé ; les traces médicales et scripts de benchmarking restent privés.

## CAB2 — contrôle clinique et comparaison Atomic (11/11/2026)
- CAB2 vérifié : ScribeWatch opérationnel avec cinq profils et 31 jobs. Modèle dentaire Qwen3.5-4B Q4_K_M. Binaire Windows déployé : sha256 2be54d2b1aa2e9ef6cf0b48c046635b4dc21dcec9f47565fe6a50c62c25e8b37.
- Document patient : suppression du texte d'audit, références de documents, identifiants techniques ; contrôle factuel en interne. Les assertions non supportées sont filtrées, non certifiées.
- Atomic Chat 2.3.5 Windows et backends CUDA PrismML/TurboQuant sont installés séparément dans le répertoire QA de ScribeWatch. TurboQuant KV turbo3 ; les poids ternaires exigent PrismML. Production ScribeWatch inchangée.
- Benchmark : transcription Parakeet Ultra originale réutilisée, référence PDF jamais fournie au générateur. Matrice et PDFs privés dans /srv/storage/cloud/arezki/Documents/ScribeWatch/Benchmark-prive/LLM-2026-11-09/CT700/Cab2-Model-Comparison-20261110/. Le score de couverture est lexical et non une validation clinique.
- Meilleur rappel du panel actuel Qwen3.5 4B, 81.6% lexical, mais des affirmations inventées ont été détectées. Bonsai 9B 67.8% en direct ou 66.7% via Atomic (497s), et variante Bonsai 27B PTQ1_0 charge mais reste lente. Qwen3.5 9B Atomic+TurboQuant a dépassé 1100s sans livrer, donc rejeté sous cette config.
- Qwen3.6 14B A3B MoE test court sous TurboQuant : 0.68 tok/s, langue française inadéquate ; ne pas tenter la génération longue. Les essais suivants passent un contrôle court vitesse/mémoire avant rédaction complète.
- Stockage CAB2 : suppression ciblée des archives déjà installées, nettoyage des fichiers de staging hors ScribeWatch, et tâche hebdomadaire de purge uniquement des fichiers partiels et logs QA périmés. Aucun modèle actif, résultat clinique ou backup supprimé.
- Windows 11 25H2 est en attente de redémarrage Windows Update ; ne pas redémarrer pendant les tests. CAB1 non modifié. Qualification clinique toujours non atteinte.

## Ajustement PDF et audit compact — 11 novembre 2026
- Rendu PDF ScribeWatch : sections et sous-sections hiérarchisées, titres conservés avec au moins deux lignes suivantes ; rendu A4 inspecté et plus aucun titre orphelin. Les dix PDF privés déjà disponibles ont été régénérés.
- Frontend seul mis à jour sur CAB2 : HTTP 200, /ready PASS, 58 assets statiques ; rollback-layout-20261111 conservé, backend et profil dentaire Qwen3.5-4B inchangés.
- Atomic Chat TurboQuant Qwen3.6-14B-A3B Q4_K_M : 0,68 token/s en micro-essai, qualité linguistique insuffisante ; essai de rapport long écarté. Ministral 3 3B Q6_K : 20,26 tok/s sur prompt synthétique court, lancement d'un vrai test long ; cette vitesse courte n'est pas la vitesse du compte rendu réel.
- Inventaire Windows ScribeWatch hebdomadaire : 17 poids recensés, aucune suppression automatique de modèles ; purge sécurisée des fragments et logs inutiles seulement. Candidats supplémentaires téléchargés avec SHA-256 vérifié : Ministral 3 3B Q6_K et Gemma 4 E4B IQ3.
- Dossier de livraison privé : scripts de matrice, PDFs de production par modèle, index hors PDF et création ZIP confidentiel. Scores de couverture lexical uniquement, praticien reste validateur des faits et de la syntaxe.

## Résultats finaux CAB2, comparaison et ménage — 11 novembre 2026

- Correctifs locaux : contrôles des faits séparés des PDF patients, meilleure pagination A4. Tests vérifiés 152/152 Rust, 82/82 frontend, TypeScript 0 erreur.
- CAB2 prêt au contrôle : API ready, 5 profils, 31 jobs préservés, aucun job clinique actif, profil et fournisseur dentaire Qwen3.5-4B Q4_K_M. Backend SHA256 2be54d2b1aa2e9ef6cf0b48c046635b4dc21dcec9f47565fe6a50c62c25e8b37 et frontend SHA256 1ed7cbf14d16c6d6f93d4f801dca8376da2745242b9535ee14641e09343a817c.
- Essais Atomic isolés, aucune session résiduelle. Qwen3.5-4B sous TurboQuant avec tout le prompt : 64 tokens en 238,38 s, soit 0,27 token/s, moins bon que le moteur natif. Ministral 3B et Qwen3.5 9B n'ont pas fourni de note complète dans un délai raisonnable ; les MoE et Bonsai 27B ont été trop lents.
- Nettoyage contrôlé après les essais : 5 GGUF expérimentaux rejetés, chacun vérifié par SHA256, puis supprimés. Moteur redondant PrismML supprimé, hardlink MoE supplémentaire supprimé mais son original Hugging Face reste. 20,82 Gio libérés : C libre de 15,42 à 36,24 Gio. 13 poids répertoriés. Nettoyage temporaire hebdomadaire Windows validé, ne touche pas aux fichiers médicaux ni aux GGUF.
- Livraison privée : stockage Cloud9 Documents/ScribeWatch/Benchmark-prive/LLM-2026-11-09/CT700/Cab2-Model-Comparison-20261110/evaluation-bundle-private.zip. Archive vérifiée avec INDEX.html, 14 PDFs produits par le code ScribeWatch, original de référence, 48 critères x 14 variantes en HTML et CSV, métriques par domaine et alertes. Il s'agit de 14 variantes de sortie, et non 14 familles distinctes.
- Evaluation NON certifiée médicalement. Couverture textuelle 81,6 % pour le Qwen3.5 4B de référence interne, mais certaines sorties comportent des inventions ; ne pas présenter ce chiffre comme une fidélité médicale. Seule la relecture par le praticien pourra valider le contenu et la syntaxe. CabinetDentaire1 non touché.
- Les marqueurs Windows Update indiquent un redémarrage en attente ; aucun reboot déclenché. Garder les scripts de score et données cliniques hors Git, dans le stockage privé.

## Correctif FDI Unicode et comparatif recalculé — 11/11/2026

- Le rejeu des 14 brouillons dans le garde-fou Rust exact de production a découvert un panic réel avec des notations telles que « dent n°46 » (découpage UTF-8 en plein caractère °).
- Parser FDI corrigé : positionnement garanti sur les frontières de caractères UTF-8, gestion de dent 46, dent n°46, dent n° 46 et dent numéro 46. Test ciblé couvrant latéralités correctes/incorrectes et chaînes Unicode non conformes. Validation : 153/153 tests Rust, 82/82 frontend, TypeScript 0 erreur.
- Nouveau backend Windows CAB2 effectivement déployé, vérifié et redémarré : SHA256 EB8D5E78CDDA4A5DC502DE3D7D808B096A7044D1B37E96B30696ABCB35E412AB. 5 profils et 31 jobs conservés, ancienne version dans rollback-utf8-guard-20261111. UI SHA256 toujours 1ED7CBF14D16C6D6F93D4F801DCA8376DA2745242B9535EE14641E09343A817C.
- Les 14 sorties du corpus ont été soumises au vrai filtre clinique natif avant création des PDF de comparaison. Entre 0 et 4 lignes à risque ont été écartées par sortie. Les métriques sont désormais calculées sur le texte effectivement visible dans chaque PDF, tout en gardant le rappel brut avant filtre et le compte des lignes écartées.
- Classement lexical après filtre : Qwen3.5 4B pilote contexte intégral 80,5 %, Qwen3.5 4B version historique affinée 78,2 %, essais Qwen3.5 4B CAB2 77,0 %. Il ne s'agit toujours PAS de scores de qualité clinique certifiés.
- Livrable privé mis à jour au même emplacement Cloud9 : evaluation-bundle-private.zip, 14 PDF ScribeWatch filtrés et PDF original, INDEX.html, tableau CSV, grille 48 critères par modèle, avant/après filtre et nombre de pages. Vérifications ZIP PASS.
- Dépôt distant origin GodsQuantum/ScribeWatch de visibilité PUBLIC. Branche de travail et handoff restant locaux ; ne PAS pousser l'historique contenant des handoffs techniques sans audit de confidentialité, ni publier des données de consultation. CAB1 non déployé, relecture médicale à faire.

## Livraison finale de la pagination adaptative — 11/11/2026

- Le frontend CAB2 a ete remplace par le build verifie SHA256 65DA2F1A2173B9000CEEDBA148AA88D662C33EE83C5E5FCB16D3930D9C3EE63E (archive), index de production SHA256 3C70791D95DF688C6850D29F50BCDACFEA1FF8967F116AA7CB20C9FE2C2F64FB. Retour API HTTP 200, 58 assets, 5 profils, 31 jobs conserves, backend Windows EB8D5E78CDDA4A5DC502DE3D7D808B096A7044D1B37E96B30696ABCB35E412AB inchange. Rollback frontend : qa-cab2-20261110/rollback-adaptive-ui-20261111.
- Code PDF local : filtres patient pour rubriques de verification interne, auto-commentaires de modele et mentions de validation editoriale, sans supprimer les notes de suivi clinique. Pagination A4 adaptative si la page finale ne contient que peu de lignes, avec espacement discretement resserre ; aucune limite universelle de pages.
- Le lot prive de 14 PDF a ete regenere depuis la transcription Parakeet conservee et les 14 sorties precedentes : filtre Rust natif, filtre de contenu patient, rendu PDF identique a ScribeWatch. Controle visuel page finale reussi. La reference praticien est preservee intacte dans le ZIP.
- Matrice privee regeneree depuis le texte effectivement exporte. Score lexical indicatif apres garde-fou clinique : Qwen3.5-4B pilote long 80,5 %, ancienne revision affine 78,2 %, Qwen3.5-4B CAB2 77,0 %. Les trois variantes tiennent desormais sur 3 pages pour le contenu retenu, sans imposer 3 pages a d'autres consultations.
- Archive privee finale unique dans CT700/Cab2-Model-Comparison-20261110/evaluation-bundle-private.zip, integrity ZIP PASS, 15 PDF incluant l'original, 48 criteres et 14 colonnes de variantes, duree, poids relatif, omissions et lignes exclues. Ne pas publier sur GitHub, le depot origin est public.
- Les donnees cliniques ne sont pas des faits certifies par ces controles automatiques ; review medicale et validation explicite du praticien restent necessaires. CAB1 non modifie, ne pas le deployer avant cette validation.
