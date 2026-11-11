//! Generic, source-indexed recall helpers.
//! No clinical benchmark terms, user identities or profile-specific facts.
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Passage {
    pub id: usize,
    pub text: String,
}

pub fn source_action_candidates(transcript: &str, max_results: usize) -> Vec<Passage> {
    const CUES: &[(&[&str], usize)] = &[
        (
            &[
                "je vais",
                "je fais",
                "je propose",
                "on va",
                "i will",
                "we will",
                "i propose",
                "we propose",
            ],
            5,
        ),
        (
            &[
                "rendez-vous",
                "créneau",
                "creneau",
                "ordonn",
                "prescri",
                "prise de",
                "analys",
                "planning",
                "rappel",
                "on vous appelle",
                "appointment",
                "slot",
                "call you",
                "prescription",
                "follow-up",
                "follow up",
                "schedule",
                "deliverable",
            ],
            7,
        ),
        (
            &[
                "si ", "en cas", "soit ", "priorit", "échec", "echec", "risque", "if ", "unless",
                "failure", "priority", "risk",
            ],
            2,
        ),
        (
            &[
                "janvier",
                "février",
                "fevrier",
                "mars",
                "avril",
                "mai",
                "juin",
                "juillet",
                "août",
                "aout",
                "septembre",
                "octobre",
                "novembre",
                "décembre",
                "decembre",
                "january",
                "february",
                "march",
                "april",
                "may",
                "june",
                "july",
                "august",
                "september",
                "october",
                "november",
                "december",
            ],
            2,
        ),
    ];
    let mut scored = transcript
        .split("\n\n")
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .enumerate()
        .filter(|(_, paragraph)| (25..=950).contains(&paragraph.chars().count()))
        .map(|(index, paragraph)| {
            let lower = paragraph.to_lowercase();
            let score = CUES
                .iter()
                .map(|(words, weight)| {
                    words.iter().filter(|cue| lower.contains(**cue)).count() * weight
                })
                .sum::<usize>();
            (
                score,
                Passage {
                    id: index + 1,
                    text: paragraph.to_owned(),
                },
            )
        })
        .filter(|(score, _)| *score >= 7)
        .collect::<Vec<_>>();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.id.cmp(&a.1.id)));
    scored
        .into_iter()
        .take(max_results.min(4))
        .map(|(_, p)| p)
        .collect()
}

/// Highest-salience source with a possible choice, condition, failure,
/// uncertainty or outcome. Operates identically across domains/languages,
/// never referring to a particular patient or reference document.
pub fn source_conditional_candidate(transcript: &str) -> Option<Passage> {
    const CUES: &[(&[&str], usize)] = &[
        (&["si ", "soit ", "if ", "unless ", "in case"], 2),
        (&["en cas", "échec", "echec", "failure", "failed"], 3),
        (
            &[
                "réussite",
                "reussite",
                "taux",
                "%",
                "success rate",
                "success",
            ],
            4,
        ),
        (
            &[
                "on essaie",
                "essayer",
                "tenter",
                "garder",
                "préserver",
                "preserver",
                "protéger",
                "proteger",
                "possibilité",
                "possibilite",
                "option",
                "condition",
                "attempt",
                "try to",
                "preserve",
            ],
            2,
        ),
        (&["risque", "risk"], 2),
    ];
    transcript
        .split("\n\n")
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .enumerate()
        .filter(|(_, p)| (25..=950).contains(&p.chars().count()))
        .map(|(index, p)| {
            let lower = p.to_lowercase();
            let score = CUES
                .iter()
                .map(|(tokens, weight)| {
                    tokens
                        .iter()
                        .filter(|token| lower.contains(**token))
                        .count()
                        * weight
                })
                .sum::<usize>();
            (
                score,
                Passage {
                    id: index + 1,
                    text: p.to_owned(),
                },
            )
        })
        .filter(|(score, _)| *score >= 7)
        .max_by(|a, b| a.0.cmp(&b.0).then(a.1.id.cmp(&b.1.id)))
        .map(|(_, p)| p)
}

/// Find an omitted contrast or negation without any disease/patient keys.
/// Include the adjacent source turn so that the model has enough context to
/// preserve the actual distinction; never add anything without an exact quote.
pub fn source_contrast_candidate(transcript: &str, existing_note: &str) -> Option<Passage> {
    let paragraphs = transcript
        .split("\n\n")
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>();
    let known = fingerprint(existing_note)
        .split_whitespace()
        .filter(|word| word.chars().count() >= 5)
        .map(str::to_owned)
        .collect::<std::collections::HashSet<_>>();
    let mut scored = Vec::new();
    for (index, pair) in paragraphs.windows(2).enumerate() {
        let text = format!("{}\n\n{}", pair[0], pair[1]);
        if !(70..=900).contains(&text.chars().count()) {
            continue;
        }
        let mut sum = 0usize;
        for paragraph in pair {
            let folded = fingerprint(paragraph);
            let bounded = format!(" {folded} ");
            let has = |word: &str| -> bool { bounded.contains(word) };
            let negations = 5 * usize::from(has(" jamais ") || has(" never "))
                + 2 * usize::from((has(" ne ") && has(" pas ")) || has(" not "))
                + 3 * usize::from(has(" mais ") || has(" but "))
                + 3 * usize::from(has(" en revanche ") || has(" however "));
            let novelty = folded
                .split_whitespace()
                .filter(|word| word.chars().count() >= 5 && !known.contains(*word))
                .collect::<std::collections::HashSet<_>>()
                .len()
                .min(6);
            sum += negations + novelty;
        }
        let has_contrast = {
            let folded = fingerprint(&text);
            [
                " jamais ",
                " never ",
                " ne ",
                " not ",
                " mais ",
                " however ",
                " but ",
            ]
            .iter()
            .any(|word| format!(" {folded} ").contains(word))
        };
        if has_contrast && sum >= 12 {
            scored.push((sum, index));
        }
    }
    let (_score, index) = scored
        .into_iter()
        .max_by(|left, right| left.0.cmp(&right.0).then(right.1.cmp(&left.1)))?;
    Some(Passage {
        id: index + 1,
        text: format!("{}\n\n{}", paragraphs[index], paragraphs[index + 1]),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceAction {
    pub source_id: usize,
    pub quote: String,
    pub sentence: String,
}
/// This gate does NOT prove semantic entailment. It can reject ungrounded
/// source IDs, missing verbatim evidence, invented numbers, false attribution
/// and overconfident completed actions. Clinical review is still required.
pub fn parse_cautious_source_action(
    response: &str,
    passage: &Passage,
    existing_note: &str,
) -> Option<SourceAction> {
    let value: Value = serde_json::from_str(response).ok()?;
    let quote = value.get("quote")?.as_str()?.trim();
    let sentence = value.get("sentence")?.as_str()?.trim();
    let status = value.get("status")?.as_str()?;
    if ![
        "already_done",
        "discussed_or_proposed",
        "conditional",
        "uncertain",
    ]
    .contains(&status)
    {
        return None;
    }
    if quote.chars().count() < 16 || quote.chars().count() > 320 || !passage.text.contains(quote) {
        return None;
    }
    if sentence.chars().count() < 24
        || sentence.chars().count() > 340
        || sentence.contains(['\n', '\r', '<', '>', '@'])
        || sentence.contains("http:")
        || sentence.contains("https:")
    {
        return None;
    }
    let origin_numbers = numerical_tokens(quote);
    if numerical_tokens(sentence)
        .iter()
        .any(|num| !origin_numbers.contains(num))
    {
        return None;
    }
    let normalized = fingerprint(sentence);
    let origin = fingerprint(quote);
    // Do not invent a role. Speaker diarization is not guaranteed by ASR.
    for title in [
        "le patient",
        "la patiente",
        "le praticien",
        "le medecin",
        "la medecin",
        "the patient",
        "the doctor",
        "the clinician",
    ] {
        if normalized.contains(title) && !origin.contains(title) {
            return None;
        }
    }
    let overconfident = [
        "rendez vous confirme",
        "a ete realise",
        "a ete prescrite",
        "a ete prescrit",
        "a ete programme",
        "a ete administre",
        "was completed",
        "has been completed",
    ]
    .iter()
    .any(|phrase| normalized.contains(phrase));
    if overconfident && status != "already_done" {
        return None;
    }
    // If the model admits source uncertainty, require phrasing that remains
    // explicitly tentative. It must never silently turn into a prescription.
    if status == "uncertain"
        && ![
            "evoqu",
            "envisag",
            "propos",
            "pourrait",
            "possible",
            "tentative",
            "discut",
            "may",
            "could",
            "consider",
        ]
        .iter()
        .any(|phrase| normalized.contains(phrase))
    {
        return None;
    }
    if fingerprint(existing_note).contains(&normalized) {
        return None;
    }
    // Require lexical novelty from the ORIGINAL excerpt; do not "recover"
    // an action that is already covered elsewhere in the initial report.
    let old = fingerprint(existing_note);
    let common = [
        "nous", "vous", "avec", "pour", "alors", "donc", "comme", "tout", "ceci", "cela", "dans",
        "cette", "enfin", "serait", "voire", "avait", "ainsi", "elle", "notre", "votre", "their",
        "this", "that", "from", "with", "would", "could", "have", "will", "then", "there",
        "should",
    ];
    let new_source_tokens = origin
        .split_whitespace()
        .filter(|word| word.len() >= 4 && !common.contains(word))
        .filter(|word| !old.split_whitespace().any(|token| token == *word))
        .count();
    if new_source_tokens < 2 {
        return None;
    }
    Some(SourceAction {
        source_id: passage.id,
        quote: quote.to_owned(),
        sentence: sentence.to_owned(),
    })
}

fn numerical_tokens(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    for c in s.chars() {
        if c.is_ascii_digit() {
            buf.push(c);
        } else if !buf.is_empty() {
            out.push(std::mem::take(&mut buf));
        }
    }
    if !buf.is_empty() {
        out.push(buf);
    }
    out
}
fn fingerprint(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_action_candidates_rank_decisions_and_scheduling() {
        let input = [
            "A generic background discussion has started.",
            "I propose a final review and we will call you if there is a slot.",
            "The weather was quite pleasant at the office today.",
            "We will send the document in November if approved.",
        ]
        .join("\n\n");
        let candidates = source_action_candidates(&input, 2);
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].id, 2);
        assert!(candidates.iter().any(|p| p.id == 4));
        assert!(candidates.iter().all(|p| input.contains(&p.text)));
    }

    #[test]
    fn contrast_candidate_preserves_original_negation_with_neighbor_context() {
        let source = [
            "A routine introduction without meaningful facts.",
            "An ordinary administrative question was asked.",
            "The plan may work in the first situation.",
            "However it never works for the second case, so a different plan may be needed.",
            "An unrelated goodbye was recorded.",
        ]
        .join("\n\n");
        let candidate = source_contrast_candidate(&source, "A routine introduction.").unwrap();
        assert!(candidate.text.contains("never works for the second case"));
        assert!(
            candidate.text.contains("The plan may work")
                || candidate.text.contains("unrelated goodbye")
        );
        assert!(source.contains(&candidate.text));
        assert!(source_contrast_candidate("Only a brief greeting.", "").is_none());
    }

    #[test]
    fn a_general_conditional_event_is_ranked_by_modality_not_domain() {
        let text = [
            "The team greeted the visitor and took some notes.",
            "If the experiment fails, attempt a second trial; success rate is around 60%.",
            "They discussed the weather and the journey home.",
        ]
        .join("\n\n");
        let candidate = source_conditional_candidate(&text).unwrap();
        assert_eq!(candidate.id, 2);
        assert!(candidate.text.contains("second trial"));
        assert!(source_conditional_candidate("Just a brief introduction.").is_none());
    }

    #[test]
    fn cautious_source_action_blocks_forged_quotes_and_attributions() {
        let passage = Passage {
            id: 9,
            text: "We will send two reports if another slot opens in April.".into(),
        };
        let ok = r#"{"status":"uncertain","quote":"We will send two reports if another slot opens in April.","sentence":"Two reports may be discussed if another slot opens in April."}"#;
        assert!(
            parse_cautious_source_action(ok, &passage, "Summary of unrelated meeting.").is_some()
        );
        let forged = r#"{"status":"uncertain","quote":"A prior signed contract was accepted.","sentence":"A new contract may be considered in April."}"#;
        assert!(parse_cautious_source_action(forged, &passage, "").is_none());
        let role = r#"{"status":"uncertain","quote":"We will send two reports if another slot opens in April.","sentence":"The doctor may send two reports if a slot opens in April."}"#;
        assert!(parse_cautious_source_action(role, &passage, "").is_none());
        let wrong_number = r#"{"status":"uncertain","quote":"We will send two reports if another slot opens in April.","sentence":"Three reports may be needed if a slot opens in April 2028."}"#;
        assert!(parse_cautious_source_action(wrong_number, &passage, "").is_none());
    }
}
