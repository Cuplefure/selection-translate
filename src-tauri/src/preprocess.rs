use crate::error::{AppError, AppResult};

const MAX_CHARS: usize = 2000;

pub struct Prepared {
    pub text: String,
    pub source_lang: String,
}

pub fn normalize(raw: &str) -> AppResult<Prepared> {
    let cleaned: String = raw
        .chars()
        .filter(|c| !matches!(c, '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{FEFF}' | '\u{00A0}'))
        .collect();
    let text = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let text = text.trim().to_string();

    if text.is_empty() {
        return Err(AppError::NoSelection);
    }
    let count = text.chars().count();
    if count > MAX_CHARS {
        return Err(AppError::TooLong(count));
    }

    let cjk = text
        .chars()
        .filter(|c| matches!(c, '\u{4e00}'..='\u{9fff}' | '\u{3040}'..='\u{30ff}' | '\u{ac00}'..='\u{d7af}'))
        .count();
    let source_lang = if cjk as f64 / count as f64 > 0.08 {
        "zh".to_string()
    } else {
        "auto".to_string()
    };

    Ok(Prepared { text, source_lang })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_and_detects_chinese() {
        let prepared = normalize("  你好\u{200b}世界  \n  hello ").unwrap();
        assert_eq!(prepared.text, "你好世界 hello");
        assert_eq!(prepared.source_lang, "zh");
    }

    #[test]
    fn rejects_empty_selection() {
        assert!(matches!(normalize("  \n "), Err(AppError::NoSelection)));
    }
}
