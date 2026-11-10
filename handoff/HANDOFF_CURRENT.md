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

## Décisions et suite

- Les recherches sur le « lost in the middle » et l'évaluation clinique en 2025–2026 confirment que la capacité du contexte seule ne garantit pas la couverture. Un découpage temporel indépendant peut perdre des relations entre faits éloignés ; rechercher d'abord des extraits thématiques avec citations à la source.
- Le pilote thématique améliore fortement la couverture **lexicale** mais produit encore des erreurs cliniques importantes : **ne pas intégrer en production**. Audit de six anomalies conservé dans le dossier privé ; utiliser ce témoin négatif pour l'évaluation qualitative.
- Prochaine optimisation : vérification de chaque allégation clinique par citation source, refus des contradictions et hallucinations ; refaire une seule mesure end-to-end avant de considérer l'intégration du mode thématique.
- Ensuite comparer les autres modèles retenus avec un protocole commun et des réglages familiaux, puis vérifier microphone → ASR → modèle → PDF sur les deux PC du cabinet. Aucun rapport n'est utilisable cliniquement avant validation du praticien.

Fichier de statut unique : ce document. Le pointeur racine HANDOFF_CURRENT.md demeure inchangé ; les traces médicales et scripts de benchmarking restent privés.
