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

## Décisions et suite

- Les recherches sur le « lost in the middle » et l'évaluation clinique en 2025–2026 confirment que la capacité du contexte seule ne garantit pas la couverture. Un découpage temporel indépendant peut perdre des relations entre faits éloignés ; rechercher d'abord des extraits thématiques avec citations à la source.
- Le pilote thématique améliore fortement la couverture **lexicale** mais produit encore des erreurs cliniques importantes : **ne pas intégrer en production**. Audit de six anomalies conservé dans le dossier privé ; utiliser ce témoin négatif pour l'évaluation qualitative.
- Prochaine optimisation : vérification de chaque allégation clinique par citation source, refus des contradictions et hallucinations ; refaire une seule mesure end-to-end avant de considérer l'intégration du mode thématique.
- Ensuite comparer les autres modèles retenus avec un protocole commun et des réglages familiaux, puis vérifier microphone → ASR → modèle → PDF sur les deux PC du cabinet. Aucun rapport n'est utilisable cliniquement avant validation du praticien.

Fichier de statut unique : ce document. Le pointeur racine HANDOFF_CURRENT.md demeure inchangé ; les traces médicales et scripts de benchmarking restent privés.
