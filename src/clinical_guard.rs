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
}

impl ReviewReason {
    fn description(self) -> &'static str {
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
    // Accept only a very local expression such as "dent 24 (prémolaire droite)".
    // Do not silently convert unspecified numbering systems or the source.
    let words = line.as_bytes();
    let mut position = 0;
    while let Some(offset) = line[position..].find("dent ") {
        let at = position + offset + "dent ".len();
        if at + 2 > words.len() {
            break;
        }
        let number = &line[at..at + 2];
        let bytes = number.as_bytes();
        if bytes.iter().all(u8::is_ascii_digit) {
            let quadrant = bytes[0] - b'0';
            let tooth = bytes[1] - b'0';
            if (1..=8).contains(&tooth) && (1..=8).contains(&quadrant) {
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
        position = at + 2;
        if position >= line.len() {
            break;
        }
    }
    false
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
        } else if unverified_clinician_name(&source, line) {
            Some(ReviewReason::UnverifiedClinicianName)
        } else if unconfirmed_performed_care(&source, &plain) {
            Some(ReviewReason::UnconfirmedPerformedCare)
        } else if unverified_future_consultation(&source, &plain) {
            Some(ReviewReason::UnverifiedFutureConsultation)
        } else if conditional_exam_presented_as_mandatory(&source, &plain) {
            Some(ReviewReason::ConditionalExamPresentedAsMandatory)
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
            // Never publish the unverified claim as a patient-facing fact.
            if line.trim().starts_with('|') {
                let columns = line.trim().trim_matches('|').split('|').count();
                let mut replacement = vec![
                    "À confirmer".to_string(),
                    format!("Information écartée : {}", reason.description()),
                ];
                replacement.resize(columns.max(2), "—".into());
                kept.push(format!("| {} |", replacement.join(" | ")));
            } else {
                kept.push(format!(
                    "**À confirmer avant validation** — formulation clinique écartée ({}).",
                    reason.description()
                ));
            }
        } else {
            kept.push(line.to_owned());
        }
    }
    if !findings.is_empty() {
        kept.push(String::new());
        kept.push("## Éléments à vérifier par le praticien".to_owned());
        kept.push(format!(
            "{} passage(s) à risque ont été écartés du brouillon. \
La transcription originale reste disponible pour vérifier le contexte et restituer \
l'information correcte. **Ne pas utiliser ce document sans relecture clinique.**",
            findings.len()
        ));
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
                .filter(|line| line.starts_with("| À confirmer"))
                .count(),
            2
        );
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
        let screened = screen_clinical_report(source.replace("\\n", "\n").as_str(), note);
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
