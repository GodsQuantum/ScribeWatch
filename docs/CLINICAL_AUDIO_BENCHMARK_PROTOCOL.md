# Protocole de certification privée — transcription et compte rendu clinique ScribeWatch

## Périmètre
Tester une **unique application ScribeWatch Windows autonome** sur le matériel réel de CabinetDentaire2, puis seulement après validation sur CabinetDentaire1. Vérifier le trajet microphone/audio -> ASR (continu ou streaming) -> structuration LLM locale -> rendu PDF unique patient/praticien. Ne jamais déclarer un modèle ou une installation « production ready » à partir d'un smoke test seul.

## Confidentialité — règles impératives
- L'enregistrement de test, les transcriptions, contenus médicaux, prompts enrichis de données, captures, PDFs de sortie et noms de patients sont **privés**. Ils ne doivent apparaître ni dans le dépôt Git, ni dans les issues/PR, logs CI, artefacts de release, traces publiques, benchmarks publics, recherches web, ni dans une API cloud.
- **Aucun nom, date, fait clinique ni extrait de l'audio réel** dans ce document, les fixtures, les commits ou les résultats publiés. N'utiliser que des fixtures synthétiques distinctes pour les tests versionnés.
- Traiter l'audio par des moteurs **locaux** sur postes autorisés. Les éventuels transferts se font par SSH. Aucune transmission à des services ASR/LLM tiers sans consentement distinct.
- Garder le chemin réel du fichier et les productions sensibles dans un inventaire opérationnel non suivi par Git et à accès restreint. Empêcher l'indexation des contenus patient par le dépôt. Supprimer seulement les temporaires prouvés jetables, jamais les données sources ou les bases métier.
- Les comptes rendus sont des **brouillons** à valider par le praticien ; ne pas présenter un texte généré comme preuve médico-légale de l'échange.

## Séquence et portes de validation
1. **Inventaire** : calculer SHA-256, durée, codec, fréquence, canaux, taille et état de confidentialité. Enregistrer l'environnement matériel, versions des moteurs/quantifications et conditions. Ne jamais modifier l'original.
2. **Référence de première qualité** : créer une transcription intégrale privée par le meilleur pipeline local disponible, éventuellement avec seconde hypothèse ASR indépendante pour relever les désaccords. Vérifier par écoute humaine les passages discordants avant de la qualifier de vérité terrain. Séparer explicitement la « baseline automatique » de toute référence humaine certifiée.
3. **Compte rendu premium de référence** : produire d'après la transcription revue un document unique professionnel patient+praticien, puis PDF A4. Ne retenir que les faits explicitement entendus ; distinguer patient / praticien, hypothèses / observations, proposition / décision et consentement réellement exprimé. Aucun diagnostic, rendez-vous, prescription, risque ou promesse ajoutés. S'inspirer du document de mise en page fourni précédemment **uniquement pour la forme**, jamais pour ses données ni son contenu.
4. **Benchmark ASR** sur CabinetDentaire2 : mêmes octets ou même audio PCM normalisé pour chaque candidat, mêmes réglages, cache/cold start explicités, durées exactes. Évaluer au minimum Parakeet Ultra, Qwen3-ASR 1.7B et Nemotron 3.5 0.6B quand disponibles et compatibles. Comparer latence à première transcription, débit de traitement, taux d'erreurs sur passages validés (WER/CER), terminologie dentaire, nombres/dents, négations, diarisation et horodatages disponibles. **Streaming** : mesurer temps premier texte, latence de finalisation, temps **PDF final**, ressources CPU/GPU et débit ; ne pas supposer qu'un affichage précoce améliore le résultat final.
5. **Sélection ASR** : prioriser exactitude clinique et absence d'omissions sur les négations/chiffres ; départager par temps bout en bout et robustesse aux faibles VRAM (y compris fallback CPU), sans gonfler artificiellement les scores.
6. **Benchmark LLM** : utiliser **exactement la même transcription retenue** pour tous les candidats. Comparer Qwen3.5-4B Q4_K_M (premier choix sur 4 Go), une alternative locale compatible et les prompts/templates. Mode non-thinking quand approprié ; borner la génération, les délais, le contexte et la température via **configuration native ScribeWatch**. Comparer aux faits vérifiés, lisibilité, langue, attribution de locuteurs, refus/non-consentement, PDF A4 et temps total. Ne pas noter une forme agréable comme fidèle si elle invente un fait.
7. **E2E et régression** : première installation Windows propre, double-clic sans console ni dépendance séparée, acquisition microphone/streaming, reprise après relancement, rapport PDF lisible, ressources, absence d'accès cloud non demandé, fonctionnement hors-ligne après acquisition de modèles, isolation des dossiers et autorisations. Contrôler comportement après reboot et installateur portable sans corrections manuelles. Puis seulement, déployer sur CabinetDentaire1.
8. **Handoff canonique** : après chaque essai, actualiser `handoff/HANDOFF_CURRENT.md` avec étape, verdict, métriques anonymes, environnement, choix, risque, raison du rejet, prochaine action, version/source de code. Les traces complètes restent hors Git. Ne pas créer de second handoff concurrent.

## Résultats — format privé (sans contenu patient)
Tableau : candidat, backend/version, modèle/quantification, streaming oui/non, durée audio, temps ASR à froid/à chaud, latence premier texte, temps LLM, temps PDF final, VRAM/RAM max, score d'erreur **uniquement si transcription revue**, défauts critiques, statut PASS/FAIL/BLOCKED.

**Ne pas publier les données brutes ni l'échantillon médical en exemple produit.**

## Optimisation après baseline — chaque modèle doit avoir sa meilleure configuration

- Baseline: même transcription ASR intégrale et Q4_K_M, génération achevée normalement et temps d'inférence mesuré, sans résumé final supprimant des faits.
- Accélération et contexte natifs: compter les tokens de transcription, prompt et réserve de sortie; adapter CUDA/Vulkan/CPU, KV q4_0 ou q8_0, contexte à la VRAM réelle, sans supposer que la présence d'une DLL prouve l'accélération.
- Prompts par modèle: sécurité générique commune, puis consignes différentes pour Qwen, Phi et Gemma en fonction des erreurs réellement observées. Ne jamais coder en dur un fait de la consultation ni fournir la référence médicale à un générateur ou correcteur.
- Comparaison sur des documents intégralement terminés: même source, temps cumulé de toutes les passes, PDF A4 de format identique. Un texte tronqué ou des sections fusionnées destructivement ne sont pas des candidats finis.
- Vérification bidirectionnelle: décomposer le document corrigé en faits atomiques pour vérifier omissions, contradictions et inversions ; chercher ensuite les ajouts propres au candidat dans la transcription pour distinguer ajouts justifiés et hallucinations. Priorité aux dents, chiffres, négations, actes/propositions, risques, médicaments et suivi.
- Second passage possible: relire la transcription originale avec le brouillon sans accéder au document de référence. Mesurer son coût et vérifier qu'il améliore réellement la fidélité. Une ressemblance lexicale élevée ne certifie pas la validité clinique.
- Confidentialité: seuls les prompts génériques et le protocole sont versionnables dans Git. Les identifiants, transcriptions, grilles médicales et sorties restent dans Benchmark-prive, jamais dans des artefacts publics.

## Métriques de fidélité clinique — évaluation multicouche (2026)

Pour chaque transcription source et compte rendu corrigé par le praticien, enregistrer deux axes distincts :

1. **Rappel des faits de référence** : décomposer le compte rendu corrigé en assertions atomiques (numéro de dent, constat, conseil, risque, option conditionnelle, décision, acte effectué, négation, chiffre). Vérifier leur présence dans la transcription source avant de sanctionner le LLM : un fait absent du texte ASR doit être traité comme limitation ASR, pas automatiquement comme omission du LLM. Mesurer la couverture par priorité clinique et signaler omissions et contradictions.
2. **Précision des affirmations du candidat** : décomposer le rapport généré en assertions atomiques, puis confronter celles qui ne sont pas dans la référence au texte ASR complet. Les ajouts peuvent être justifiés par la transcription même s'ils ne figurent pas dans la référence. Distinguer ajout pertinent, ajout non étayé, erreur chiffrée, attribution erronée, négation inversée et soins proposés transformés en soins effectués.
3. **Évaluation finale** : échelle de gravité (critique, majeur, mineur), fidélité clinique, lisibilité française, complétude PDF, temps **cumulé** de toutes les passes et ressources. Le score lexical automatisé n'est qu'un contrôle d'alerte ; un second évaluateur LLM ne remplace pas la relecture médicale du praticien.

Bibliographie méthodologique (aucune donnée patient) :
- Latif et al., *Evaluating Robustness of LLM-Based Ambient Scribes for SOAP Note Generation*, CHIL/PMLR 2026, https://proceedings.mlr.press/v333/latif26a.html — alignement des faits à la référence puis vérification des ajouts contre la transcription.
- Dahlberg et al., *Measuring the quality of AI-generated clinical notes*, Artificial Intelligence in Medicine 2026, DOI 10.1016/j.artmed.2026.103421 — limites des métriques ROUGE/BLEU, évaluation sémantique et humaine combinées.
- Munnangi et al., *FactEHR*, MLHC/PMLR 2025, https://proceedings.mlr.press/v298/munnangi25a.html — décomposition atomique et preuve d'implication des faits cliniques.

## Streaming et locuteurs : contraintes Parakeet Ultra (vérifiées 2026-11-09)

Selon la documentation actuelle de transcribe.cpp, Parakeet Ultra est un modèle **offline** avec segmentation interne VAD et horodatage ; le streaming à cache est prévu pour certains autres modèles, pas pour Ultra. La famille Parakeet ne fournit pas de diarisation dans l'API transcribe.cpp. Ne pas prétendre que les mots apparaissant en direct dans ScribeWatch proviennent d'un véritable moteur Parakeet Ultra streaming, ni attribuer les propos au patient ou au praticien sans preuve.

Mesurer la latence jusqu'au **PDF final** avant de remplacer le meilleur modèle offline par un modèle streaming. Les indications de vitesse « premier texte » ne prouvent pas un gain de qualité ni un gain de temps final. Référence technique à revalider lors des mises à jour : https://github.com/handy-computer/transcribe.cpp/blob/main/docs/models/parakeet.md


## Profil natif de consultation dentaire complète

Le profil intégré dental-consultation-complete-fr est une option de rédaction approfondie, distincte du profil historique. Aucun fait patient, exemple clinique réel ou référence de benchmark n'est intégré à son code.

- Sur le moteur llama.cpp local, pour une consultation longue et un contexte d'au moins 32 768 tokens, ScribeWatch extrait les faits en segments chronologiques. En cas de troncature, il scinde et reprend uniquement le passage concerné. La rédaction finale relit la transcription intégrale et l'inventaire des faits, sans accès au PDF clinique de référence.
- Sur des machines plus contraintes, il utilise une rédaction directe lorsque le texte tient dans le contexte, sinon des sections sans résumé destructeur. Il évite d'extraire préalablement des faits s'il sait que la synthèse finale dépassera le contexte.
- Le moteur embarqué rejette les sorties tronquées ou dont la terminaison normale n'est pas confirmée. Les fournisseurs externes conservent leur comportement OpenAI-compatible.
- La quantification du cache KV, les accélérations et le contexte sont sélectionnés via des capacités locales avec des réglages manuels encadrés. Chaque OS et GPU doit néanmoins passer un test d'accélération réel avant publication.
- Ce mode peut être sensiblement plus lent : mesurer le temps bout en bout ASR, extraction, synthèse et PDF sur la machine cible. La vérification clinique par le praticien reste indispensable.
