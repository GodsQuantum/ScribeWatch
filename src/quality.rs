//! Generic, source-indexed recall helpers.
//! No clinical benchmark terms, user identities or profile-specific facts.
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Passage {
    pub id: usize,
    pub text: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Addition {
    pub source_id: usize,
    pub sentence: String,
}
pub fn source_groups(transcript: &str, max_chars: usize) -> Vec<Vec<Passage>> {
    let cap = max_chars.max(2048);
    let mut items = Vec::new();
    for paragraph in transcript
        .split("\n\n")
        .map(str::trim)
        .filter(|p| !p.is_empty())
    {
        if paragraph.chars().count() <= cap {
            items.push(paragraph.to_owned());
        } else {
            // Preserve all words; oversized ASR paragraphs are not truncated.
            let mut current = String::new();
            for word in paragraph.split_whitespace() {
                let word_len = word.chars().count();
                if !current.is_empty() && current.chars().count() + word_len + 1 > cap {
                    items.push(std::mem::take(&mut current));
                }
                if !current.is_empty() {
                    current.push(' ');
                }
                current.push_str(word);
            }
            if !current.is_empty() {
                items.push(current);
            }
        }
    }
    let mut result = Vec::new();
    let mut current = Vec::new();
    let mut chars = 0;
    for (index, content) in items.into_iter().enumerate() {
        let length = content.chars().count() + 15;
        if !current.is_empty() && chars + length > cap {
            result.push(std::mem::take(&mut current));
            chars = 0;
        }
        current.push(Passage {
            id: index + 1,
            text: content,
        });
        chars += length;
    }
    if !current.is_empty() {
        result.push(current);
    }
    result
}
pub fn format_group(passages: &[Passage]) -> String {
    passages
        .iter()
        .map(|p| format!("[P{:04}] {}", p.id, p.text))
        .collect::<Vec<_>>()
        .join("\n\n")
}
fn id_from_value(value: &Value) -> Option<usize> {
    if let Some(i) = value.as_u64() {
        return usize::try_from(i).ok();
    }
    let text = value.as_str()?.trim();
    text.trim_start_matches(['P', 'p']).parse::<usize>().ok()
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
pub fn parse_source_bound_additions(
    response: &str,
    passages: &[Passage],
    existing_note: &str,
) -> Vec<Addition> {
    let Ok(document) = serde_json::from_str::<Value>(response.trim()) else {
        return vec![];
    };
    let Some(items) = document.get("additions").and_then(Value::as_array) else {
        return vec![];
    };
    let mut additions = Vec::new();
    let existing = fingerprint(existing_note);
    for item in items.iter().take(12) {
        let Some(source_id) = item.get("pid").and_then(id_from_value) else {
            continue;
        };
        let Some(source) = passages.iter().find(|p| p.id == source_id) else {
            continue;
        };
        let Some(sentence) = item.get("sentence").and_then(Value::as_str).map(str::trim) else {
            continue;
        };
        if !(28..=360).contains(&sentence.chars().count()) || sentence.contains(['\n', '\r']) {
            continue;
        }
        if sentence.contains('<')
            || sentence.contains('>')
            || sentence.contains("http:")
            || sentence.contains("https:")
            || sentence.contains('@')
        {
            continue;
        }
        if additions
            .iter()
            .any(|a: &Addition| a.source_id == source_id)
        {
            continue;
        }
        let mut invalid_number = false;
        let source_numbers = numerical_tokens(&source.text);
        for number in numerical_tokens(sentence) {
            if !source_numbers.contains(&number) {
                invalid_number = true;
                break;
            }
        }
        if invalid_number {
            continue;
        }
        let normalized = fingerprint(sentence);
        if normalized.len() < 24 || existing.contains(&normalized) {
            continue;
        }
        // Avoid promoting an explicit source negation into a bare affirmative.
        // The gate is conservative and intentionally loses some recall.
        let origin = fingerprint(&source.text);
        let negated = [
            " pas ",
            " aucun ",
            " aucune ",
            " non ",
            " no ",
            " not ",
            " without ",
            " never ",
            " kein ",
            " nicht ",
            "没有",
            "不是",
            "无",
        ]
        .iter()
        .any(|word| origin.contains(word))
            || origin.starts_with("pas ");

        let new_negated = [
            " pas ", " aucun ", " aucune ", " non ", " no ", " not ", " never ", " kein ",
            " nicht ", "没有", "不是", "无",
        ]
        .iter()
        .any(|word| normalized.contains(word))
            || normalized.starts_with("pas ");

        if negated && !new_negated {
            continue;
        }
        additions.push(Addition {
            source_id,
            sentence: sentence.to_owned(),
        });
        if additions.len() >= 3 {
            break;
        }
    }
    additions
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_source_word_including_last_segment_is_retained() {
        let sample=(0..96).map(|i|format!("Conversation paragraph {i}: question, answer, possible action and a stated date."))
            .collect::<Vec<_>>().join("\n\n");
        let groups = source_groups(&sample, 2400);
        assert!(groups.len() >= 3);
        let all = groups.iter().flatten().collect::<Vec<_>>();
        assert_eq!(all.len(), 96);
        assert_eq!(all[0].id, 1);
        assert_eq!(all.last().unwrap().id, 96);
        assert!(all.last().unwrap().text.contains("paragraph 95"));
        assert!(groups.iter().all(|g| !g.is_empty()));
    }
    #[test]
    fn groups_do_not_truncate_long_unsegmented_text() {
        let input = (0..2400)
            .map(|i| format!("word{i}"))
            .collect::<Vec<_>>()
            .join(" ");
        let all = source_groups(&input, 2200)
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        assert!(all.len() > 3);
        assert!(all.first().unwrap().text.starts_with("word0"));
        assert!(all.last().unwrap().text.ends_with("word2399"));
    }
    #[test]
    fn source_id_numeric_and_negation_gates_apply_across_profiles() {
        let p = vec![
            Passage {
                id: 1,
                text: "The client asked for 3 blue reports.".into(),
            },
            Passage {
                id: 2,
                text: "There is no approval for the contract.".into(),
            },
        ];
        let s = r#"{"additions":[
          {"pid":"P0001","sentence":"The client explicitly requested 3 blue reports."},
          {"pid":1,"sentence":"The client requested 999 blue reports."},
          {"pid":2,"sentence":"The contract was approved without further discussion."}
        ]}"#;
        let result = parse_source_bound_additions(s, &p, "## Summary");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].source_id, 1);
        assert!(parse_source_bound_additions("{invalid", &p, "").is_empty());
    }
}
