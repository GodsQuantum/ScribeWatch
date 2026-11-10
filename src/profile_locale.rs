//! Language-matched built-in AI profiles. This module never translates or
//! overwrites user-customized prompts and never touches the clinical transcript.
use crate::{domain::StructureProfile, llm::builtin_profiles};

pub const SUPPORTED: &[&str] = &["fr", "en", "zh-CN"];

fn default_prompt(locale: &str, id: &str) -> Option<&'static str> {
    match (locale, id) {
        ("fr", "general-structured-note") => Some(
            "Structure cette transcription en français. Rubriques uniquement lorsqu'elles sont étayées : ## Résumé, ## Points clés, ## Décisions, ## Actions (responsables et échéances seulement s'ils sont cités), ## Questions ouvertes. Reste concis, précis et sans invention.",
        ),
        ("fr", "meeting-minutes") => Some(
            "Rédige un compte rendu de réunion en français à partir des seuls propos transcrits. Rubriques pertinentes : ## Contexte, ## Participants identifiables, ## Discussion par thème, ## Décisions, ## Actions (tableau action | responsable | date uniquement si explicitement mentionnés), ## Questions ouvertes, ## Prochaine réunion. Ne déduis ni identité ni engagement.",
        ),
        ("fr", "patient-consultation-report") => Some(
            "Rédige en français une lettre de consultation au patient, pour relecture obligatoire du praticien. Adresse-toi au patient en « vous », sans inventer de faits médicaux. Distingue symptômes rapportés, constatations explicites, explications données, conseils, possibilités conditionnelles, soins réalisés, prescriptions et suivis confirmés. Préserve négations, dents, mesures et incertitudes. Rubriques courtes, paragraphes clairs et **gras** discret ; pas de coordonnées ni signature : le PDF les ajoute.",
        ),
        ("fr", "medical-consultation") | ("fr", "general-medical-consultation") => Some(
            "Rédige en français un compte rendu clinique à valider par le praticien, sans avis médical ajouté. Rubriques seulement si pertinentes : ## Motif, ## Symptômes rapportés, ## Antécédents, traitements et allergies, ## Examen explicitement décrit, ## Évaluations du praticien, ## Actes réalisés, ## Prescriptions et propositions, ## Suivi, ## Points incertains. Ne devine ni diagnostic, médicament, mesure, personne ni statut des actes ; préserve toutes les négations et conditions.",
        ),
        ("fr", "dental-consultation") => Some(
            "Rédige en français un compte rendu dentaire factuel pour validation clinique. Ne produis aucun odontogramme inventé. Préserve précisément numéros dentaires, surfaces, symptômes rapportés, constats explicitement énoncés, examen radiologique seulement s'il est décrit, alternatives, anesthésie, actes réellement effectués, hygiène et suivi. Une option ou prescription envisagée ne devient jamais un acte réalisé. Adresse-toi au patient en « vous » ; conserve incertitudes et détails. Rubriques seulement si étayées.",
        ),
        ("fr", "marketing-brainstorm") => Some(
            "Organise ce brainstorming en français sans effacer les idées originales. Rubriques pertinentes : ## Objectif, ## Idées par thème, ## Hypothèses et contraintes, ## Objections, ## Décisions, ## Expériences proposées, ## Actions et suites. Ne classe pas les idées et n'invente pas de critères ou échéances.",
        ),
        ("fr", "interview-research") => Some(
            "Structure cet entretien de recherche en français par thèmes, besoins exprimés, exemples réels, objections, décisions et questions à approfondir. Conserve les citations uniquement mot pour mot, sans inventer de paroles ni d'identités. Les silences ne prouvent aucune réponse.",
        ),
        ("en", "medical-dual-audience-report") => Some(
            "Create an English clinical consultation draft readable by the patient and usable in the clinician's record AFTER mandatory clinician review. Use only information explicitly present in the source. Separate patient reports, clinician findings, explanations, decisions, prescriptions, completed procedures and conditional options. Preserve negations, numbers, tooth identifiers and uncertainty. Write concise, accessible professional sections and add no generic medical advice, identity, signature or invented provenance.",
        ),
        ("en", "dental-consultation-complete-fr") => Some(
            "Write an ENGLISH patient-facing dental consultation letter for clinician review, using the entire transcript as the ONLY source of clinical facts. Use respectful second-person address. No invented identities, diagnoses, examinations, decisions, bookings, prescriptions or procedures. Reconcile mentions of the same topic across distant turns, including later corrections; distinguish questions from answers, conditional options from decisions and promised from performed actions. Preserve tooth notation, side, numbers, measurements, durations, risks, negations and uncertainty. Adapt length and the number of ## headings to the actual information density and clinical reasoning: use only as much text as needed to cover every relevant finding, explanation, risk, alternative, condition, practical instruction and follow-up. A short consultation deserves a short note; a complex consultation may need a longer one. Never impose a word or page target, invent padding, repeat facts or omit important details to shorten the document. Use meaningful **bold** and clear paragraphs. Patient and clinician details belong exclusively in the PDF exporter.",
        ),
        ("zh-CN", "general-structured-note") => Some(
            "请用简体中文整理完整转录，只写有证据的内容。可用小标题：## 摘要、## 重点、## 已作出的决定、## 明确的待办事项、## 尚未解决的问题。责任人、日期及数字只有在原话明确出现时才能写入，禁止猜测或补充。",
        ),
        ("zh-CN", "meeting-minutes") => Some(
            "请仅根据原始对话，用简体中文写会议纪要。按主题整理背景、参与者、讨论、明确决策、任务和未决问题；仅在明确提及时填写责任人和截止日期，不能编造姓名或承诺。使用简洁的小标题。",
        ),
        ("zh-CN", "medical-dual-audience-report") => Some(
            "请用简体中文起草适合患者阅读、也可供医生审核的医疗记录。必须由医生审核后才能使用。严格依据转录，区分患者陈述、医生观察、解释、建议、条件性方案、实际完成的操作、已开具的处方及确已确定的复诊。保留否定、数字、剂量、时间及不确定性。不得自行诊断或填充未提及的事实。患者身份由 PDF 表单单独填写。",
        ),
        ("zh-CN", "patient-consultation-report") => Some(
            "请用简体中文写给患者的诊疗说明信，必须经医生审核。语言通俗且准确，严格保留原话中的症状、解释、条件、数字、牙位、操作和真实复诊安排。不能把建议写成处方或把可能发生的操作写成已经完成。不得编造身份或医疗事实，用简洁标题和适度加粗。",
        ),
        ("zh-CN", "medical-consultation") | ("zh-CN", "general-medical-consultation") => Some(
            "请用简体中文整理待医生审核的临床记录。根据转录实际内容分为主诉、病史、药物过敏、检查、医生明确的评估、已实施的操作、明确处方、后续事项和待核实问题。严格区分患者陈述与医生观察，保留时间、数字、否定、不确定性和条件，禁止从常识补充病历。",
        ),
        ("zh-CN", "dental-consultation") => Some(
            "请用简体中文撰写牙科诊疗记录草稿供医生审核。仅记录转录中明确出现的牙位编号、口腔检查、症状、影像、风险、备选方案、已经完成的操作、口腔清洁说明及后续安排。严格区分建议、可能方案和实际实施的治疗。不得自行推断牙位侧别、诊断或编造牙周检查数据。",
        ),
        ("zh-CN", "dental-consultation-complete-fr") => Some(
            "请用简体中文为患者撰写牙科就诊说明信，必须经牙医审核后使用。仅依据完整转录，整合前后相距较远的同类信息，并以最终确认的事实为准。明确区分患者说法、医生观察、解释、建议、条件性备选方案、已经实施的治疗、正式处方和已确认预约。不能凭推测补充诊断、牙位、检查或用药。保留数字、牙位编号、否定、条件及不确定性。篇幅与小标题数量应完全根据本次就诊的信息量和临床解释的复杂程度决定：简单就诊写得简洁，复杂就诊须充分保留明确提到的发现、条件、风险、替代方案、推理和实际操作。不要设定字数或页数目标，不要重复内容，也不能为了缩短文字而漏掉重要事实。用清晰的短段落和适度加粗。姓名日期由PDF导出处理。",
        ),
        ("zh-CN", "marketing-brainstorm") => Some(
            "请用简体中文将头脑风暴内容整理为目标、主题、想法、假设、限制、异议、明确决策、建议的实验和后续事项。保留不同创意，不应擅自排序或补充不存在的数据与结论。",
        ),
        ("zh-CN", "interview-research") => Some(
            "请用简体中文按主题整理访谈，保留受访者真实提出的需求、例子、异议、已明确的决定及待深入问题。只有在原始转录中出现的原话才能加引号，不得编造人物和陈述。",
        ),
        _ => None,
    }
}

/// Identify the user-selected language only for an untouched known built-in
/// profile. Never guess the language of custom instructions.
pub(crate) fn known_builtin_locale(profile: &StructureProfile) -> Option<&'static str> {
    if !profile.built_in {
        return None;
    }
    for locale in SUPPORTED {
        if default_prompt(locale, &profile.id).is_some_and(|prompt| prompt == profile.prompt) {
            return Some(locale);
        }
    }
    let original = builtin_profiles()
        .into_iter()
        .find(|p| p.id == profile.id)?;
    if original.prompt == profile.prompt
        || (profile.id == "dental-consultation-complete-fr"
            && fnv1a(&profile.prompt) == 0x19a8b8ffa45c944a)
    {
        return Some(match profile.id.as_str() {
            "dental-consultation-complete-fr" | "medical-dual-audience-report" => "fr",
            _ => "en",
        });
    }
    None
}

fn fnv1a(text: &str) -> u64 {
    text.as_bytes()
        .iter()
        .fold(0xcbf29ce484222325, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        })
}

/// True for untouched shipped prompts, including an older version of our
/// French dental preset. Custom user prompts are never overwritten.
pub fn replacement_for_locale(
    profile: &StructureProfile,
    locale: &str,
) -> Option<StructureProfile> {
    if !SUPPORTED.contains(&locale) || !profile.built_in {
        return None;
    }
    let builtin = builtin_profiles()
        .into_iter()
        .find(|p| p.id == profile.id)?;
    let recognized = profile.prompt == builtin.prompt
        || SUPPORTED
            .iter()
            .filter_map(|lang| default_prompt(lang, &profile.id))
            .any(|known| profile.prompt == known)
        || (profile.id == "dental-consultation-complete-fr"
            && fnv1a(&profile.prompt) == 0x19a8b8ffa45c944a);
    if !recognized {
        return None;
    }
    let mut result = profile.clone();
    result.prompt = default_prompt(locale, &profile.id)
        .unwrap_or(&builtin.prompt)
        .to_owned();
    // The dual-audience built-in has a French-only optional layout template.
    // It is omitted in English/Chinese, without affecting a user-owned template.
    if profile.id == "medical-dual-audience-report"
        && (profile.output_template == builtin.output_template
            || profile.output_template.is_empty())
    {
        result.output_template = if locale == "fr" {
            builtin.output_template
        } else {
            String::new()
        };
    }
    (result.prompt != profile.prompt || result.output_template != profile.output_template)
        .then_some(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_cover_all_three_interface_languages() {
        for p in builtin_profiles() {
            for locale in SUPPORTED {
                let mut current = p.clone();
                if let Some(updated) = replacement_for_locale(&current, locale) {
                    current = updated;
                }
                assert!(!current.prompt.is_empty());
                assert!(replacement_for_locale(&current, locale).is_none());
            }
        }
    }
    #[test]
    fn never_replace_a_modified_builtin() {
        let mut p = builtin_profiles().remove(0);
        p.prompt = "Mes instructions personnalisées.".into();
        for lang in SUPPORTED {
            assert!(replacement_for_locale(&p, lang).is_none());
        }
    }
    #[test]
    fn unmodified_english_and_chinese_are_reversible() {
        let original = builtin_profiles()
            .into_iter()
            .find(|p| p.id == "dental-consultation-complete-fr")
            .unwrap();
        let chinese = replacement_for_locale(&original, "zh-CN").unwrap();
        assert!(chinese.prompt.contains("简体中文"));
        let english = replacement_for_locale(&chinese, "en").unwrap();
        assert!(english.prompt.contains("ENGLISH"));
        let french = replacement_for_locale(&english, "fr").unwrap();
        assert_eq!(french.prompt, original.prompt);
    }
    #[test]
    fn known_profile_language_is_recovered_without_guessing_user_prompts() {
        let original = builtin_profiles()
            .into_iter()
            .find(|p| p.id == "dental-consultation-complete-fr")
            .unwrap();
        assert_eq!(known_builtin_locale(&original), Some("fr"));
        let english = replacement_for_locale(&original, "en").unwrap();
        assert_eq!(known_builtin_locale(&english), Some("en"));
        let chinese = replacement_for_locale(&english, "zh-CN").unwrap();
        assert_eq!(known_builtin_locale(&chinese), Some("zh-CN"));
        let mut changed = chinese;
        changed.prompt.push_str(" User-only instructions.");
        assert_eq!(known_builtin_locale(&changed), None);
    }

    #[test]
    fn translated_dental_prompts_never_prescribe_fixed_length() {
        let original = builtin_profiles()
            .into_iter()
            .find(|p| p.id == "dental-consultation-complete-fr")
            .unwrap();
        for lang in ["en", "zh-CN"] {
            let translated = replacement_for_locale(&original, lang).unwrap();
            for forbidden in ["1300", "1650", "4–8", "4 pages", "4-page"] {
                assert!(
                    !translated.prompt.contains(forbidden),
                    "{lang}: {forbidden}"
                );
            }
            assert!(if lang == "en" {
                translated.prompt.contains("actual information density")
            } else {
                translated.prompt.contains("实际")
            });
        }
    }

    #[test]
    fn refuses_invalid_locale() {
        let p = builtin_profiles().remove(0);
        assert!(replacement_for_locale(&p, "ru").is_none());
    }
}
