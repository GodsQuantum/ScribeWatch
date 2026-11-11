//! Conservative, offline, source-aware clinical draft screen.
//! This is NOT an entailment engine: only high-risk inconsistencies are
//! quarantined; a claim not flagged here is never declared clinically verified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewReason {
    UnconfirmedAppointment,
    ProcedureContradictsNegativeIndication,
    ConflictingHistory,
    InconsistentDentalLaterality,
    UnverifiedClinicianName,
    UnconfirmedPerformedCare,
    UnverifiedFutureConsultation,
    ConditionalExamPresentedAsMandatory,
    UnverifiedDentalCausality,
    UnverifiedProcedureDuration,
    PersonalContactInBody,
    UnverifiedAnesthesiaType,
    UnverifiedAnatomicalClaim,
    ContradictedWisdomImpaction,
    UnsupportedFluorideOnlyCariesCare,
    ConditionalPulpCareMadeMandatory,
}

impl ReviewReason {
    pub(crate) fn description(self) -> &'static str {
        match self {
            Self::UnconfirmedAppointment => "rendez-vous annoncé comme confirmé sans preuve claire",
            Self::ProcedureContradictsNegativeIndication => {
                "acte annoncé comme nécessaire malgré une indication négative ou conditionnelle"
            }
            Self::ConflictingHistory => {
                "absence de symptôme contredite ou nuancée par un autre passage"
            }
            Self::InconsistentDentalLaterality => {
                "discordance possible entre numéro dentaire FDI et côté déclaré"
            }
            Self::UnverifiedClinicianName => "nom de professionnel cité mais absent du verbatim",
            Self::UnconfirmedPerformedCare => {
                "soin ou examen présenté comme réalisé sans confirmation de l'acte"
            }
            Self::UnverifiedFutureConsultation => {
                "acte attribué à une consultation future non mentionnée dans la source"
            }
            Self::ConditionalExamPresentedAsMandatory => {
                "examen conditionnel transformé en obligation non confirmée"
            }
            Self::UnverifiedDentalCausality => {
                "condition d'un acte dentaire potentiellement inversée"
            }
            Self::UnverifiedProcedureDuration => {
                "durée attribuée à un secteur sans confirmation dans la source"
            }
            Self::PersonalContactInBody => {
                "donnée d'identité à laisser exclusivement dans l'en-tête"
            }
            Self::UnverifiedAnesthesiaType => "type d'anesthésie absent de la transcription",
            Self::UnverifiedAnatomicalClaim => "affirmation anatomique absente de la transcription",
            Self::ContradictedWisdomImpaction => {
                "enclavement explicite des dents de sagesse nié dans la lettre"
            }
            Self::UnsupportedFluorideOnlyCariesCare => {
                "fluor présenté comme seul soin d'une carie de sillon non reminéralisable"
            }
            Self::ConditionalPulpCareMadeMandatory => {
                "possibilité de préserver la vitalité pulpaire remplacée par une dévitalisation systématique"
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReviewFinding {
    pub line: usize,
    pub reason: ReviewReason,
}

#[derive(Debug, Clone)]
pub struct ScreenResult {
    pub markdown: String,
    pub findings: Vec<ReviewFinding>,
}

// Accent folding is deliberately limited to normalizing common French
// characters; no locale, network, tokenizer or external service required.
fn fold(s: &str) -> String {
    s.to_lowercase()
        .replace(['é', 'è', 'ê', 'ë'], "e")
        .replace(['à', 'â', 'ä'], "a")
        .replace(['î', 'ï'], "i")
        .replace(['ô', 'ö'], "o")
        .replace(['ù', 'û', 'ü'], "u")
        .replace('ç', "c")
        .replace('œ', "oe")
        .replace(['’', '‘'], "'")
}

fn mentions_booked_action(line: &str) -> bool {
    [
        "a programme un rendez-vous",
        "rendez-vous confirme",
        "rendez-vous fixe",
        "rendez vous confirme",
        "rendez vous fixe",
        "a fixe un rendez-vous",
        "rendez-vous a ete programme",
        "rendez-vous est programme",
    ]
    .iter()
    .any(|needle| line.contains(needle))
}

fn transcript_confirms_appointment(source: &str) -> bool {
    // Dates/months, waiting lists and callbacks do NOT confirm booking.
    [
        "rendez-vous est confirme",
        "rendez-vous est fixe",
        "rendez-vous a ete fixe",
        "rendez-vous a ete programme",
        "on a fixe le rendez-vous",
        "j'ai fixe le rendez-vous",
        "rendez-vous est pris",
        "rendez-vous a ete pris",
    ]
    .iter()
    .any(|needle| source.contains(needle))
}

fn mandatory_extraction(line: &str) -> bool {
    let is_extraction = ["extraire", "extrait", "extraction", "enlever"]
        .iter()
        .any(|word| line.contains(word));
    let mandatory = [
        "il faudra",
        "faudra les",
        "doivent etre",
        "devront etre",
        "sera extrait",
    ]
    .iter()
    .any(|word| line.contains(word));
    is_extraction && mandatory
}

fn negative_extraction_indication(source: &str, line: &str) -> bool {
    // Avoid treating an unrelated tooth extraction as contradicted by a
    // statement about wisdom teeth; the qualifier must match.
    let wisdom = line.contains("sagesse") && source.contains("sagesse");
    if !wisdom {
        return false;
    }
    let negative_phrases = [
        "pas d'indication a les extraire",
        "pas d'indication a extraire",
        "pas besoin de les extraire",
        "ne faut pas les extraire",
        "pas necessaire de les extraire",
    ];
    negative_phrases.iter().any(|s| source.contains(s))
}

fn conflicting_symptom_status(source: &str, line: &str) -> bool {
    // Strict, domain-independent principle: a screening denial cannot erase
    // a later explicit problem. This initial implementation checks sleep,
    // a common consultation topic, without interpreting a medical diagnosis.
    let denial = line.contains("aucun probleme")
        || line.contains("pas de probleme")
        || line.contains("aucun trouble")
        || line.contains("pas de trouble");
    if !denial || !line.contains("sommeil") {
        return false;
    }
    if [
        "probleme de sommeil depuis",
        "problemes de sommeil depuis",
        "trouble du sommeil depuis",
        "troubles du sommeil depuis",
        "probleme de sommeil il y a",
        "sommeil depuis",
    ]
    .iter()
    .any(|fragment| source.contains(fragment))
    {
        return true;
    }
    // In noisy ASR a question, "Oui, ça fait longtemps ?" and
    // A short duration answer may be split into separate utterances.
    // Preserve this ambiguity for the practitioner rather than
    // asserting that the symptom is absent.
    let paragraphs: Vec<&str> = source.split("\n\n").collect();
    for (idx, p) in paragraphs.iter().enumerate() {
        if !p.contains("sommeil") || !p.contains("pas de probleme") {
            continue;
        }
        let followup = paragraphs
            .iter()
            .skip(idx + 1)
            .take(4)
            .copied()
            .collect::<Vec<_>>()
            .join(" ");
        if (followup.contains("ca fait longtemps")
            || followup.contains("depuis")
            || followup.contains("il y a"))
            && ["mois", "semain", "depuis", "annee", "quelques jours"]
                .iter()
                .any(|v| followup.contains(v))
        {
            return true;
        }
    }
    false
}

fn unverified_clinician_name(source: &str, original_line: &str) -> bool {
    // Check titles only, not every capitalized word (which could be a
    // sentence start, medical abbreviation, or product brand).
    let fields: Vec<&str> = original_line.split_whitespace().collect();
    for (index, field) in fields.iter().enumerate() {
        let title = fold(field.trim_matches(|c: char| !c.is_alphabetic()));
        if !["dr", "dre", "docteur", "docteure"].contains(&title.as_str()) {
            continue;
        }
        if let Some(next) = fields.get(index + 1) {
            let surname = next.trim_matches(|c: char| !c.is_alphabetic() && c != '-');
            if surname.chars().count() >= 4 && !source.contains(&fold(surname)) {
                return true;
            }
        }
    }
    false
}

fn unconfirmed_performed_care(source: &str, line: &str) -> bool {
    // Past-tense claims of actual intervention are much riskier than summaries
    // of treatment options. A proposed, future or conditional intervention
    // must never become a procedure already performed.
    let performed = [
        "a ete realise",
        "a ete effectue",
        "a ete pratique",
        "a ete prescri",
        "ont ete realise",
        "a subi",
        "a recu",
        "avons realise",
        "avons effectue",
        "a ete traite",
    ]
    .iter()
    .any(|form| line.contains(form));
    if !performed {
        return false;
    }
    const PROCEDURES: &[(&str, &[&str])] = &[
        ("devital", &["devital"]),
        ("extraction", &["extraction", "extraite", "extrait"]),
        ("panoramique", &["panoramique"]),
        ("radiograph", &["radiograph", "radio"]),
        ("anesthes", &["anesthes"]),
        ("prise de sang", &["prise de sang", "bilan sanguin"]),
        ("fluor", &["fluor"]),
        ("traitement", &["traitement", "soin"]),
    ];
    let target = PROCEDURES
        .iter()
        .find(|(key, _)| line.contains(key))
        .map(|(_, terms)| *terms);
    let Some(target) = target else {
        return false;
    };
    // Require a completed-action assertion tied to the same clinical
    // procedure in the *same clause*, not merely in the same long ASR
    // paragraph ("radio faite, mais devitalisation envisagee").
    let completed_actions = [
        "a ete realise",
        "a ete effectue",
        "a ete pratique",
        "a ete prescrit",
        "a ete traite",
        "nous avons fait",
        "on a fait",
        "j'ai fait",
        "on vient de",
        "j'ai realise",
        "je viens de",
    ];
    !source
        .split(|c: char| ['.', '!', '?', ';', ',', '\n'].contains(&c))
        .flat_map(|clause| clause.split(" mais "))
        .any(|clause| {
            target.iter().any(|word| clause.contains(word))
                && completed_actions
                    .iter()
                    .any(|action| clause.contains(action))
        })
}

fn unverified_future_consultation(source: &str, line: &str) -> bool {
    let future = [
        "prochaine consultation",
        "prochain rendez-vous",
        "prochaine seance",
        "consultation suivante",
        "au prochain rendez-vous",
        "la prochaine fois",
    ];
    if !future.iter().any(|p| line.contains(p)) {
        return false;
    }
    // A next visit for procedure A is not evidence that procedure B
    // was deferred. Conversely, a later on-the-spot demonstration
    // may supersede an earlier "we'll do that next time".
    let ignored = [
        "pour",
        "les",
        "des",
        "avec",
        "sera",
        "prevu",
        "prevue",
        "rendez",
        "vous",
        "consultation",
        "prochaine",
        "prochain",
        "seance",
        "suivante",
        "lors",
        "patient",
        "praticien",
        "faire",
        "cette",
        "pendant",
        "explication",
    ];
    let significant: Vec<String> = line
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 5 && !ignored.contains(w))
        .map(|w| w.chars().take(5).collect::<String>())
        .collect();
    let source_paragraphs: Vec<&str> = source.split("\n\n").collect();
    let supporting_future: Vec<usize> = source_paragraphs
        .iter()
        .enumerate()
        .filter(|(_, paragraph)| {
            future.iter().any(|p| paragraph.contains(p))
                && (significant.is_empty()
                    || significant
                        .iter()
                        .filter(|word| paragraph.contains(word.as_str()))
                        .count()
                        >= 1)
        })
        .map(|(i, _)| i)
        .collect();
    if supporting_future.is_empty() {
        return true;
    }
    let present_actions = [
        "je vais vous montrer",
        "je vous montre",
        "on vous montre",
        "je fais ce geste",
        "nous venons de",
        "on vient de",
        "je viens de vous montrer",
    ];
    supporting_future.iter().any(|&earlier| {
        source_paragraphs.iter().skip(earlier + 1).any(|paragraph| {
            present_actions.iter().any(|word| paragraph.contains(word))
                && !future.iter().any(|word| paragraph.contains(word))
                && significant
                    .iter()
                    .any(|word| paragraph.contains(word.as_str()))
        })
    })
}

fn conditional_exam_presented_as_mandatory(source: &str, line: &str) -> bool {
    // This deliberately produces a review flag, not a medical conclusion:
    // the same radiographic examination may be discussed at several moments.
    let mandatory = [
        "est necessaire",
        "sera necessaire",
        "sera realise",
        "devra etre realise",
        "doit etre realise",
    ]
    .iter()
    .any(|p| line.contains(p));
    if !mandatory || !line.contains("panoramiq") {
        return false;
    }
    source.split("\n\n").any(|p| {
        p.contains("panoramiq")
            && (p.contains("si ") || p.contains("extraction") || p.contains("extraire"))
    })
}

fn unsupported_anesthesia_type(source: &str, line: &str) -> bool {
    line.contains("anesthesie generale") && !source.contains("anesthesie generale")
}

fn unsupported_gingival_anatomy(source: &str, line: &str) -> bool {
    (line.contains("gencive") || line.contains("gingiv"))
        && (line.contains("non vascularis") || line.contains("non-vascularis"))
        && !(source.contains("non vascularis") || source.contains("non-vascularis"))
}

fn personal_contact_in_body(original_line: &str) -> bool {
    // A separate, controlled form supplies the recipient's identity.
    // Prevent accidental leakage of addresses, phone numbers or email
    // handles from an ASR hallucination into the clinical letter's body.
    let tokens = original_line.split_whitespace().collect::<Vec<_>>();
    let has_email = tokens.iter().any(|word| {
        let stripped = word.trim_matches(|c: char| c == ',' || c == ';' || c == '(' || c == ')');
        stripped.contains('@') && stripped.contains('.')
    });
    let has_street_address = tokens.windows(2).any(|pair| {
        let number = pair[0].trim_matches(|c: char| !c.is_ascii_digit());
        (1..=4).contains(&number.len())
            && number.chars().all(|c| c.is_ascii_digit())
            && [
                "rue",
                "avenue",
                "boulevard",
                "impasse",
                "allée",
                "allee",
                "route",
                "chemin",
            ]
            .contains(&fold(pair[1]).trim_matches(|c: char| !c.is_alphabetic()))
    });
    // Generic French ten-digit phone; ignore tooth codes and dosages.
    let digit_run = original_line.chars().filter(|c| c.is_ascii_digit()).count();
    let phone = digit_run >= 10
        && tokens.windows(5).any(|group| {
            let text = group.join("");
            let digits = text
                .chars()
                .filter(|c| c.is_ascii_digit())
                .collect::<String>();
            digits.len() == 10 && digits.starts_with('0')
        });
    has_email || has_street_address || phone
}

fn suspicious_pulp_condition(line: &str) -> bool {
    // Pure high-risk review trigger, NOT a clinical rule or correction:
    // do not infer a root-canal procedure from a sentence that explicitly
    // states the pulp/nerve is not affected. The clinician must verify.
    let claim_is_procedure = line.contains("devitalis")
        || line.contains("pulpectomi")
        || line.contains("traitement endodontique");
    let claim_negates_pulp = [
        "nerf non atteint",
        "pulpe non atteinte",
        "ne touche pas le nerf",
        "n'atteint pas la pulpe",
        "sans atteinte de la pulpe",
    ]
    .iter()
    .any(|pattern| line.contains(pattern));
    claim_is_procedure && claim_negates_pulp
}

fn unsupported_sector_duration(source: &str, line: &str) -> bool {
    // A total brushing duration must not be multiplied by the number of
    // quadrants/sectors. Flag if the source doesn't assert that mapping.
    let unit = [
        "minute par secteur",
        "minutes par secteur",
        "minute par quadrant",
        "minutes par quadrant",
    ];
    let has_sector = unit.iter().any(|x| line.contains(x));
    has_sector && !unit.iter().any(|x| source.contains(x))
}

fn fdi_laterality_conflict(line: &str) -> bool {
    // Recognize unambiguous FDI numbering adjacent to its own description.
    // In French source notes, the notation may be "dent 46", "dent n°46",
    // "dent n° 46" or "dent numéro 46". NEVER slice arbitrary UTF-8 bytes
    // into a &str: the degree sign in n° is a multibyte character.
    let mut position = 0;
    while let Some(offset) = line[position..].find("dent ") {
        let after_dent = position + offset + "dent ".len();
        let suffix = &line[after_dent..];
        let suffix = suffix
            .strip_prefix("n°")
            .or_else(|| suffix.strip_prefix("nº"))
            .or_else(|| suffix.strip_prefix("numero "))
            .unwrap_or(suffix)
            .trim_start();
        let at = line.len() - suffix.len();
        // A failed match is not a reason to skip another tooth later
        // in the sentence. Both at and position remain UTF-8 boundaries.
        position = at;
        if let Some(bytes) = line.as_bytes().get(at..at.saturating_add(2))
            && bytes.len() == 2
            && bytes.iter().all(u8::is_ascii_digit)
        {
            let quadrant = bytes[0] - b'0';
            let tooth = bytes[1] - b'0';
            if (1..=8).contains(&tooth) && (1..=8).contains(&quadrant) {
                // Safe UTF-8 boundary: exactly two ASCII digits were consumed.
                let tail = line[at + 2..].trim_start();
                if tail.starts_with('(')
                    && let Some(end) = tail.find(')')
                    && end <= 75
                {
                    let descriptor = &tail[..end];
                    let left = descriptor.contains("gauche");
                    let right = descriptor.contains("droite");
                    if left ^ right {
                        let expected_right = matches!(quadrant, 1 | 4 | 5 | 8);
                        if (expected_right && left) || (!expected_right && right) {
                            return true;
                        }
                    }
                }
            }
        }
        if position >= line.len() {
            break;
        }
    }
    false
}

// Source-to-draft contradiction checks, not medical inference. Trigger only
// on unequivocal source statements, leaving ambiguous ASR to the clinician.
fn contradicted_wisdom_impaction(source: &str, line: &str) -> bool {
    // A negative source assertion ("not impacted") is NOT positive evidence.
    // Require a positively asserted source phrase before challenging the note.
    [
        "elles sont enclave",
        "sagesse sont enclave",
        "dents de sagesse enclave",
        "sagesse enclave",
    ]
    .iter()
    .any(|phrase| source.contains(phrase))
        && line.contains("sagesse")
        && [
            "ne sont pas enclave",
            "ne sont pas encore enclave",
            "sont non enclave",
            "elles ne sont pas enclave",
            "elles ne seraient pas enclave",
        ]
        .iter()
        .any(|phrase| line.contains(phrase))
}

fn unsupported_fluoride_only_caries_care(source: &str, line: &str) -> bool {
    let source_rejects = [
        "sillon ne se remineralise",
        "sillons ne se remineralisent",
        "carie de sillon ne se remineralise",
    ]
    .iter()
    .any(|phrase| source.contains(phrase));
    let caries_treatment = line.contains("traitement de la carie")
        || line.contains("soins de la carie")
        || line.contains("traiter la carie")
        || line.contains("arret du processus carieux")
        || line.contains("stopper la carie")
        || line.contains("guerir la carie");
    let fluoride_as_care = line.contains("application de fluor")
        || line.contains("fluorure pour remineralis")
        || line.contains("fluor pour remineralis");
    let accurately_qualified = [
        "ne suffit pas",
        "ne se remineralise",
        "ne permet pas de traiter",
        "ne remplace pas",
        "soin restaurateur",
        "restauration en resine",
    ]
    .iter()
    .any(|phrase| line.contains(phrase));
    source_rejects && caries_treatment && fluoride_as_care && !accurately_qualified
}

fn conditional_pulp_care_made_mandatory(source: &str, line: &str) -> bool {
    // Only quarantine an unconditional procedure when the ORIGINAL source
    // clearly discusses a possible vitality-preserving attempt. The source
    // can be noisy ASR: the prefix "vi" accommodates both "vivante" and
    // a common transcription typo without inventing any clinical facts.
    let source_allows_preservation = [
        "garder la dent vi",
        "conserver la vitalite",
        "preserver la vitalite",
        "preserver la dent vivante",
        "essayer de sauver la dent",
    ]
    .iter()
    .any(|phrase| source.contains(phrase));
    let report_makes_mandatory = [
        "devra etre devitalise",
        "doit etre devitalise",
        "sera obligatoirement devitalise",
        "il faudra devitaliser",
        "devitalisation obligatoire",
    ]
    .iter()
    .any(|phrase| line.contains(phrase));
    let conditions_mentioned =
        line.contains("si ") && (line.contains("pulpe") || line.contains("nerf"));
    let line_preserves_alternative = [
        "en cas d'echec",
        "apres echec",
        "on essaie de garder",
        "tenter de garder",
        "essayer de conserver",
        "possibilite de conserver",
        "si la conservation echoue",
    ]
    .iter()
    .any(|phrase| line.contains(phrase));
    source_allows_preservation
        && report_makes_mandatory
        && conditions_mentioned
        && !line_preserves_alternative
}

pub fn screen_clinical_report(source: &str, markdown: &str) -> ScreenResult {
    let source = fold(source);
    let mut kept = Vec::new();
    let mut findings = Vec::new();
    for (idx, line) in markdown.lines().enumerate() {
        let plain = fold(&line.replace("**", "").replace("__", "").replace('|', " "));
        // Preserve headings and table separators, but audit *content rows*
        // too: clinical plans are commonly formatted as Markdown tables.
        if line.trim().is_empty()
            || line.trim_start().starts_with('#')
            || line.trim().chars().all(|c| "|:- ".contains(c))
        {
            kept.push(line.to_owned());
            continue;
        }
        let reason = if mentions_booked_action(&plain) && !transcript_confirms_appointment(&source)
        {
            Some(ReviewReason::UnconfirmedAppointment)
        } else if mandatory_extraction(&plain) && negative_extraction_indication(&source, &plain) {
            Some(ReviewReason::ProcedureContradictsNegativeIndication)
        } else if conflicting_symptom_status(&source, &plain) {
            Some(ReviewReason::ConflictingHistory)
        } else if fdi_laterality_conflict(&plain) {
            Some(ReviewReason::InconsistentDentalLaterality)
        } else if unsupported_anesthesia_type(&source, &plain) {
            Some(ReviewReason::UnverifiedAnesthesiaType)
        } else if unsupported_gingival_anatomy(&source, &plain) {
            Some(ReviewReason::UnverifiedAnatomicalClaim)
        } else if personal_contact_in_body(line) {
            Some(ReviewReason::PersonalContactInBody)
        } else if unverified_clinician_name(&source, line) {
            Some(ReviewReason::UnverifiedClinicianName)
        } else if unconfirmed_performed_care(&source, &plain) {
            Some(ReviewReason::UnconfirmedPerformedCare)
        } else if unverified_future_consultation(&source, &plain) {
            Some(ReviewReason::UnverifiedFutureConsultation)
        } else if conditional_exam_presented_as_mandatory(&source, &plain) {
            Some(ReviewReason::ConditionalExamPresentedAsMandatory)
        } else if contradicted_wisdom_impaction(&source, &plain) {
            Some(ReviewReason::ContradictedWisdomImpaction)
        } else if unsupported_fluoride_only_caries_care(&source, &plain) {
            Some(ReviewReason::UnsupportedFluorideOnlyCariesCare)
        } else if conditional_pulp_care_made_mandatory(&source, &plain) {
            Some(ReviewReason::ConditionalPulpCareMadeMandatory)
        } else if suspicious_pulp_condition(&plain) {
            Some(ReviewReason::UnverifiedDentalCausality)
        } else if unsupported_sector_duration(&source, &plain) {
            Some(ReviewReason::UnverifiedProcedureDuration)
        } else {
            None
        };
        if let Some(reason) = reason {
            findings.push(ReviewFinding {
                line: idx + 1,
                reason,
            });
            // Exclude the unsupported assertion from the patient document.
            // The structured ReviewFinding remains available separately
            // (and a privacy-preserving line/reason is logged by the caller).
            // Never introduce audit instructions or validation notes into
            // patient prose or treatment tables.
        } else {
            kept.push(line.to_owned());
        }
    }
    ScreenResult {
        markdown: kept.join("\n"),
        findings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tentative_slot_cannot_be_turned_into_booked_visit() {
        let source = "Pour le contrôle fictif, les créneaux sont en mai. Si une place se libère, on vous appelle.";
        let report = "## Suivi\nLe praticien a programmé un rendez-vous pour le contrôle fictif.\nD'autres conseils ont été discutés.";
        let checked = screen_clinical_report(source, report);
        assert_eq!(checked.findings.len(), 1);
        assert_eq!(
            checked.findings[0].reason,
            ReviewReason::UnconfirmedAppointment
        );
        assert!(!checked.markdown.contains("a programmé un rendez-vous"));
        assert!(
            checked
                .markdown
                .contains("D'autres conseils ont été discutés")
        );
    }

    #[test]
    fn general_anaesthesia_cannot_be_invented_from_local_anaesthesia() {
        let source = "Pour le soin fictif, une anesthésie locale est possible.";
        let suspect = "Une anesthésie générale sera nécessaire.";
        let result = screen_clinical_report(source, suspect);
        assert_eq!(result.findings.len(), 1);
        assert_eq!(
            result.findings[0].reason,
            ReviewReason::UnverifiedAnesthesiaType
        );
        assert!(!result.markdown.contains(suspect));
        let supported = "Une anesthésie générale a été évoquée pour cette éventualité.";
        assert!(
            screen_clinical_report(supported, supported)
                .findings
                .is_empty()
        );
    }

    #[test]
    fn unsupported_anatomical_assertion_stays_out_of_patient_letter() {
        let source = "Les gencives sont légèrement inflammatoires.";
        let suspect = "Les gencives sont des muqueuses non vascularisées.";
        let result = screen_clinical_report(source, suspect);
        assert_eq!(result.findings.len(), 1);
        assert_eq!(
            result.findings[0].reason,
            ReviewReason::UnverifiedAnatomicalClaim
        );
        assert!(result.markdown.trim().is_empty());
    }

    #[test]
    fn patient_identifiers_go_only_to_controlled_pdf_header() {
        let source = "Consultation fictive sur une carie.";
        for unsafe_note in [
            "Le patient habite au 24 rue Imaginaire.",
            "Contact du patient: exemple@test.invalid.",
        ] {
            let note = format!("## Bilan clinique\n{unsafe_note}\nCarie à surveiller.");
            let result = screen_clinical_report(source, &note);
            assert_eq!(result.findings.len(), 1);
            assert_eq!(
                result.findings[0].reason,
                ReviewReason::PersonalContactInBody
            );
            assert!(!result.markdown.contains(unsafe_note));
            assert!(result.markdown.contains("Carie à surveiller."));
        }
    }

    #[test]
    fn explicitly_confirmed_visit_stays_unmodified() {
        let source = "Le rendez-vous est confirmé pour jeudi.";
        let report = "Le rendez-vous confirmé est prévu jeudi.";
        let checked = screen_clinical_report(source, report);
        assert!(checked.findings.is_empty());
        assert_eq!(checked.markdown, report);
    }

    #[test]
    fn conditional_extraction_is_not_made_mandatory() {
        let source = "Pour ce cas fictif, une intervention n'est pas planifiée. \
Nous n'avons pas besoin de les extraire maintenant. \
Si une pathologie apparaît, une extraction des dents de sagesse sera discutée.";
        let report = "- Les dents de sagesse devront être extraites : il faudra les extraire.";
        let checked = screen_clinical_report(source, report);
        assert_eq!(
            checked.findings[0].reason,
            ReviewReason::ProcedureContradictsNegativeIndication
        );
        assert!(!checked.markdown.contains("faudra les extraire"));
    }

    #[test]
    fn unrelated_tooth_is_not_quarantined_by_wisdom_tooth_discussion() {
        let source = "Pour les dents de sagesse, pas besoin de les extraire. \
La dent 14 devra être extraite en raison d'une fracture fictive.";
        let report = "La dent 14 devra être extraite.";
        assert!(screen_clinical_report(source, report).findings.is_empty());
    }

    #[test]
    fn late_symptoms_trigger_review_of_initial_denial() {
        let source = "Pas de problème au niveau du sommeil ? Plus tard : \
troubles du sommeil depuis trois semaines, c'est important.";
        let report = "Le patient n'a aucun problème au niveau du sommeil.";
        let checked = screen_clinical_report(source, report);
        assert_eq!(checked.findings[0].reason, ReviewReason::ConflictingHistory);
    }

    #[test]
    fn wrong_fdi_side_quarantined_but_right_side_preserved() {
        let source = "Cas fictif : dent 24 et dent 15.";
        let bad = "La dent 24 (prémolaire supérieure droite) nécessite un soin fictif.";
        let good = "La dent 24 (prémolaire supérieure gauche) nécessite un soin fictif.";
        assert_eq!(
            screen_clinical_report(source, bad).findings[0].reason,
            ReviewReason::InconsistentDentalLaterality
        );
        assert!(screen_clinical_report(source, good).findings.is_empty());
    }

    #[test]
    fn fdi_notation_accepts_unicode_degree_and_never_panics() {
        let source = "Consultation fictive : dent 46 et dent 36.";
        let cases = [
            ("La dent n°46 (molaire à gauche) est examinée.", true),
            ("La dent n°46 (molaire à droite) est examinée.", false),
            ("La dent n° 46 (molaire à gauche) est examinée.", true),
            ("La dent numéro 46 (molaire à droite) est examinée.", false),
            ("Dent n°46 sans latéralité explicitée.", false),
            (
                "La dent n°°46 est évoquée. La dent 36 (à gauche) aussi.",
                false,
            ),
        ];
        for (line, expected_flag) in cases {
            let findings = screen_clinical_report(source, line).findings;
            assert_eq!(!findings.is_empty(), expected_flag, "{line}");
        }
    }

    #[test]
    fn interview_question_followed_by_yes_and_duration_flags_a_denial() {
        let source = "Pas de problème au niveau du sommeil ? Des insomnies ?\n\nOui, d'accord, ça fait longtemps ?\n\nNon, trois semaines.\n\nEssayons d'améliorer le sommeil.";
        let report = "Le patient n'a aucun problème au niveau du sommeil.";
        let checked = screen_clinical_report(source, report);
        assert_eq!(checked.findings[0].reason, ReviewReason::ConflictingHistory);
    }

    #[test]
    fn unverified_clinician_names_are_removed_from_patient_draft() {
        let source = "Si l'extraction devient nécessaire, nous verrons un spécialiste.";
        let report = "Je vous adresserai au Dr Inventus si une extraction est nécessaire.";
        let checked = screen_clinical_report(source, report);
        assert_eq!(
            checked.findings[0].reason,
            ReviewReason::UnverifiedClinicianName
        );
        assert!(!checked.markdown.contains("Inventus"));
        let supported = "Je vous adresserai au Dr Exemple si nécessaire.";
        assert!(
            screen_clinical_report("Adressez-vous au Dr Exemple.", supported)
                .findings
                .is_empty()
        );
    }

    #[test]
    fn conditional_treatment_cannot_be_reported_as_completed() {
        let source = "Une dévitalisation pourrait être proposée seulement en cas de complication fictive. Une radiographie panoramique sera envisagée si une chirurgie devait être étudiée.";
        let note =
            "Une dévitalisation a été réalisée.\nUne radiographie panoramique a été réalisée.";
        let result = screen_clinical_report(source, note);
        assert_eq!(result.findings.len(), 2);
        assert!(
            result
                .findings
                .iter()
                .all(|f| f.reason == ReviewReason::UnconfirmedPerformedCare)
        );
    }

    #[test]
    fn performed_radiography_does_not_prove_devitalization_was_done() {
        let source = "Une radiographie a été réalisée, mais une dévitalisation pourrait être proposée en cas de complication.";
        let claimed = "Une dévitalisation a été réalisée.";
        assert_eq!(
            screen_clinical_report(source, claimed).findings[0].reason,
            ReviewReason::UnconfirmedPerformedCare
        );
    }

    #[test]
    fn confirmed_intervention_is_not_quarantined() {
        let source = "La radiographie panoramique a été réalisée hier.";
        let note = "La radiographie panoramique a été réalisée.";
        assert!(screen_clinical_report(source, note).findings.is_empty());
    }

    #[test]
    fn unsafe_clinical_table_rows_are_quarantined_without_breaking_markdown() {
        let source = "Dans ce scénario fictif, les dents de sagesse ne gênent pas ; pas besoin de les extraire. Un rendez-vous en mai serait possible si une place se libère.";
        let report = "| Acte | Statut | Délai |\\n|---|---|---|\\n| Extraction des dents de sagesse | Il faudra les extraire | rapide |\\n| Rendez-vous | Le rendez-vous a été programmé | mai |";
        let report = report.replace("\\n", "\n");
        let screened = screen_clinical_report(source, &report);
        assert_eq!(screened.findings.len(), 2);
        assert!(!screened.markdown.contains("faudra les extraire"));
        assert!(!screened.markdown.contains("a été programmé"));
        assert!(screened.markdown.contains("|---|---|---|"));
        assert_eq!(
            screened
                .markdown
                .lines()
                .filter(|line| line.starts_with('|'))
                .count(),
            2
        );
        assert!(!screened.markdown.contains("Éléments à vérifier"));
    }

    #[test]
    fn procedures_cannot_be_shifted_to_an_invented_future_session() {
        let source = "Je vais vous montrer le geste aujourd'hui. C'est simple.";
        let note = "Le geste sera expliqué à la prochaine consultation.";
        let screened = screen_clinical_report(source, note);
        assert_eq!(
            screened.findings[0].reason,
            ReviewReason::UnverifiedFutureConsultation
        );
        assert!(!screened.markdown.contains("prochaine consultation"));
    }

    #[test]
    fn next_visit_for_another_topic_does_not_prove_future_brushing() {
        let source = "La prochaine consultation permettra de soigner une carie.\\n\\nAujourd'hui je vais vous montrer la technique de brossage.";
        let note = "La technique de brossage sera expliquée à la prochaine consultation.";
        let screened = screen_clinical_report(&source.replace("\\n", "\n"), note);
        assert_eq!(
            screened.findings[0].reason,
            ReviewReason::UnverifiedFutureConsultation
        );
    }

    #[test]
    fn early_deferred_brushing_is_superseded_by_later_demonstration() {
        let source = "La prochaine fois j'expliquerai la technique de brossage.\\n\\nNous passons à autre chose.\\n\\nAujourd'hui, je vais vous montrer comment on se brosse les dents.";
        let note =
            "L'explication de la technique de brossage est prévue pour la prochaine consultation.";
        let source = source.replace("\\n", "\n");
        assert_eq!(
            screen_clinical_report(&source, note).findings[0].reason,
            ReviewReason::UnverifiedFutureConsultation
        );
    }

    #[test]
    fn panoramic_examination_condition_is_not_overstated() {
        let source = "Si une extraction est nécessaire, on fera une radiographie panoramique.";
        let note = "La radiographie panoramique est nécessaire.";
        let screened = screen_clinical_report(source, note);
        assert_eq!(
            screened.findings[0].reason,
            ReviewReason::ConditionalExamPresentedAsMandatory
        );
    }

    #[test]
    fn pulp_conditional_inversion_is_quarantined() {
        let source = "Pour ce cas fictif, si la pulpe est trop endommagée, on discutera un traitement endodontique ; sinon on conserve la vitalité.";
        let suspect = "Si la pulpe non atteinte, on réalisera une dévitalisation.";
        let result = screen_clinical_report(source, suspect);
        assert_eq!(
            result.findings[0].reason,
            ReviewReason::UnverifiedDentalCausality
        );
        assert!(!result.markdown.contains("on réalisera une dévitalisation"));
        let safe = "Le traitement endodontique reste une possibilité conditionnelle.";
        assert!(screen_clinical_report(source, safe).findings.is_empty());
    }

    #[test]
    fn a_total_brushing_time_is_not_silently_multiplied() {
        let source = "Brossez toutes les dents pendant deux minutes au total.";
        let suspect = "Brossez-vous deux minutes par secteur.";
        let checked = screen_clinical_report(source, suspect);
        assert_eq!(
            checked.findings[0].reason,
            ReviewReason::UnverifiedProcedureDuration
        );
        assert!(
            screen_clinical_report("Deux minutes par secteur ont été expliquées.", suspect)
                .findings
                .is_empty()
        );
    }

    #[test]
    fn no_source_only_and_no_leakage_to_generic_notes() {
        let markdown = "## Note\nUne réunion a été programmée.";
        assert!(screen_clinical_report("", markdown).findings.is_empty());
    }

    #[test]
    fn wisdom_impaction_denial_is_quarantined_when_source_confirms_impaction() {
        let source = "Les dents de sagesse sont enclavées et il n'y a pas d'indication d'extraction maintenant.";
        let bad = "Les dents de sagesse ne sont pas enclavées de manière urgente.";
        let screened = screen_clinical_report(source, bad);
        assert_eq!(screened.findings.len(), 1);
        assert_eq!(
            screened.findings[0].reason,
            ReviewReason::ContradictedWisdomImpaction
        );
        assert!(screened.markdown.is_empty());

        let safe = "Les dents de sagesse sont enclavées, sans extraction indiquée à ce stade.";
        assert!(screen_clinical_report(source, safe).findings.is_empty());
        // Explicitly negative source statements must not be reversed.
        let non_impacted = "Les dents de sagesse ne sont pas enclavées.";
        assert!(
            screen_clinical_report(non_impacted, non_impacted)
                .findings
                .is_empty()
        );
    }

    #[test]
    fn fluoride_alone_is_not_a_substitute_for_source_rejected_fissure_care() {
        let source = "Une carie de sillon ne se reminéralise jamais à elle seule : le fluor n'atteint pas le fond.";
        let bad = "| Traitement de la carie dent 36 | Application de fluorure pour reminéralisation et surveillance |";
        let checked = screen_clinical_report(source, bad);
        assert_eq!(checked.findings.len(), 1);
        assert_eq!(
            checked.findings[0].reason,
            ReviewReason::UnsupportedFluorideOnlyCariesCare
        );
        let safe = "Le fluor ne suffit pas à traiter la carie de sillon; un soin restaurateur est envisagé.";
        assert!(screen_clinical_report(source, safe).findings.is_empty());
        let prevention = "Le fluor est discuté pour la prévention et les lésions initiales.";
        assert!(
            screen_clinical_report(source, prevention)
                .findings
                .is_empty()
        );
        let ambiguous = "Nous avons discuté des caries et du fluor.";
        assert!(screen_clinical_report(ambiguous, bad).findings.is_empty());
    }

    #[test]
    fn fluoride_only_is_not_cure_for_established_caries_in_different_words() {
        let source = "Pour les lésions précoces au point de contact, du fluor peut aider. Une carie de sillon ne se reminéralise jamais.";
        let unsafe_note = "Traitement envisagé : Application de fluor pour reminéralisation et arrêt du processus carieux.";
        let screened = screen_clinical_report(source, unsafe_note);
        assert_eq!(screened.findings.len(), 1);
        assert_eq!(
            screened.findings[0].reason,
            ReviewReason::UnsupportedFluorideOnlyCariesCare
        );
        let safe_note = "Le fluor peut aider certaines lésions initiales au point de contact, pas la carie de sillon.";
        assert!(
            screen_clinical_report(source, safe_note)
                .findings
                .is_empty()
        );
    }

    #[test]
    fn source_vitality_option_cannot_become_mandatory_root_canal() {
        let source = "Si une zone limitée du nerf est exposée, on essaie de garder la dent vivante. En cas d'échec, une dévitalisation pourra être envisagée.";
        let unsafe_note = "Si la carie atteint la pulpe, la dent devra être dévitalisée.";
        let checked = screen_clinical_report(source, unsafe_note);
        assert_eq!(checked.findings.len(), 1);
        assert_eq!(
            checked.findings[0].reason,
            ReviewReason::ConditionalPulpCareMadeMandatory
        );
        assert!(checked.markdown.is_empty());
        let safe_note = "Si la carie expose la pulpe sur une petite zone, une tentative de préserver la vitalité est envisagée ; en cas d'échec une dévitalisation pourra être discutée.";
        assert!(
            screen_clinical_report(source, safe_note)
                .findings
                .is_empty()
        );
        let no_preservation_mentioned =
            "Si la pulpe est totalement atteinte, il faudra dévitaliser.";
        assert!(
            screen_clinical_report(no_preservation_mentioned, no_preservation_mentioned)
                .findings
                .is_empty()
        );
    }

    #[test]
    fn private_replay_only_if_explicitly_configured() {
        let Ok(root) = std::env::var("SCRIBEWATCH_PRIVATE_BENCH_ROOT") else {
            return;
        };
        let base = std::path::Path::new(&root);
        let source = std::fs::read_to_string(base.join("source-parakeet-ultra.md")).unwrap();
        let report_path = std::env::var("SCRIBEWATCH_PRIVATE_BENCH_REPORT")
            .unwrap_or_else(|_| "CT700/Qwen4B-Thematic-20261110/thematic-report-private.md".into());
        // Only a relative path beneath the private benchmark root is accepted.
        assert!(!report_path.starts_with('/') && !report_path.contains(".."));
        let report = std::fs::read_to_string(base.join(report_path)).unwrap();
        // A published ScribeWatch note contains both the generated document
        // and the complete original transcript. Review the generated document
        // only; the source transcript must never itself be quarantined.
        let generated = report
            .split_once("## Structured notes")
            .map(|(_, rest)| rest.split("## Transcript").next().unwrap_or(rest))
            .unwrap_or(&report);
        let screened = screen_clinical_report(&source, generated);
        assert!(!screened.findings.is_empty());
        // No private excerpts, patient identity or claims in test output.
        eprintln!(
            "PRIVATE_CLINICAL_GUARD_FINDING_COUNT={}",
            screened.findings.len()
        );
        for finding in &screened.findings {
            eprintln!(
                "PRIVATE_GUARD_REASON={:?}_LINE={}",
                finding.reason, finding.line
            );
        }
    }
}
