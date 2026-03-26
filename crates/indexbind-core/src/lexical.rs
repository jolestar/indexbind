pub const LEXICAL_TOKENIZER_VERSION: &str = "mixed-cjk-bigram-v2";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScriptClass {
    Alnum,
    Cjk,
}

pub fn tokenize(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut current_class: Option<ScriptClass> = None;

    let flush =
        |tokens: &mut Vec<String>, current: &mut String, current_class: Option<ScriptClass>| {
            if current.is_empty() {
                return;
            }
            match current_class {
                Some(ScriptClass::Alnum) => tokens.push(current.to_lowercase()),
                Some(ScriptClass::Cjk) => push_cjk_tokens(tokens, current),
                None => {}
            }
            current.clear();
        };

    for ch in input.chars() {
        let Some(class) = classify_char(ch) else {
            flush(&mut tokens, &mut current, current_class);
            current_class = None;
            continue;
        };

        if current_class != Some(class) {
            flush(&mut tokens, &mut current, current_class);
            current_class = Some(class);
        }

        if class == ScriptClass::Alnum {
            for lower in ch.to_lowercase() {
                current.push(lower);
            }
        } else {
            current.push(ch);
        }
    }

    flush(&mut tokens, &mut current, current_class);
    tokens
}

pub fn tokenize_for_storage(input: &str) -> String {
    tokenize(input).join(" ")
}

pub fn estimate_token_count(input: &str) -> usize {
    let mut count = 0usize;
    let mut current_len = 0usize;
    let mut current_class: Option<ScriptClass> = None;

    let flush = |count: &mut usize, current_len: &mut usize, current_class: Option<ScriptClass>| {
        if *current_len == 0 {
            return;
        }
        match current_class {
            Some(ScriptClass::Alnum) => *count += 1,
            Some(ScriptClass::Cjk) => *count += cjk_token_count(*current_len),
            None => {}
        }
        *current_len = 0;
    };

    for ch in input.chars() {
        let Some(class) = classify_char(ch) else {
            flush(&mut count, &mut current_len, current_class);
            current_class = None;
            continue;
        };

        if current_class != Some(class) {
            flush(&mut count, &mut current_len, current_class);
            current_class = Some(class);
        }
        current_len += 1;
    }

    flush(&mut count, &mut current_len, current_class);
    count.max(1)
}

pub fn normalize_for_heuristic(input: &str) -> String {
    input
        .chars()
        .map(|ch| {
            if classify_char(ch).is_some() {
                ch.to_lowercase().collect::<String>()
            } else {
                " ".to_string()
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn push_cjk_tokens(tokens: &mut Vec<String>, text: &str) {
    let chars = text.chars().collect::<Vec<_>>();
    match chars.len() {
        0 => {}
        1 => tokens.push(chars[0].to_string()),
        2 => tokens.push(chars.iter().collect()),
        _ => {
            for window in chars.windows(2) {
                tokens.push(window.iter().collect());
            }
        }
    }
}

fn cjk_token_count(char_len: usize) -> usize {
    match char_len {
        0 => 0,
        1 | 2 => 1,
        len => len - 1,
    }
}

fn classify_char(ch: char) -> Option<ScriptClass> {
    if is_cjk(ch) {
        Some(ScriptClass::Cjk)
    } else if ch.is_alphanumeric() {
        Some(ScriptClass::Alnum)
    } else {
        None
    }
}

fn is_cjk(ch: char) -> bool {
    matches!(
        ch as u32,
        0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xF900..=0xFAFF
            | 0x20000..=0x2A6DF
            | 0x2A700..=0x2B73F
            | 0x2B740..=0x2B81F
            | 0x2B820..=0x2CEAF
            | 0x2CEB0..=0x2EBEF
            | 0x2EBF0..=0x2EE5F
            | 0x30000..=0x3134F
            | 0x31350..=0x323AF
            | 0x323B0..=0x3347F
    )
}

#[cfg(test)]
mod tests {
    use super::{
        estimate_token_count, normalize_for_heuristic, tokenize, tokenize_for_storage,
        LEXICAL_TOKENIZER_VERSION,
    };

    #[test]
    fn tokenizes_mixed_cjk_and_latin_queries() {
        assert_eq!(tokenize("比特币 Layer2"), vec!["比特", "特币", "layer2"]);
        assert_eq!(
            tokenize("模块化区块链"),
            vec!["模块", "块化", "化区", "区块", "块链"]
        );
    }

    #[test]
    fn tokenizes_short_cjk_terms_without_dropping_exact_terms() {
        assert_eq!(tokenize("调用层"), vec!["调用", "用层"]);
        assert_eq!(tokenize("链"), vec!["链"]);
        assert_eq!(tokenize("L2"), vec!["l2"]);
    }

    #[test]
    fn normalizes_heuristic_text_consistently() {
        assert_eq!(
            normalize_for_heuristic("比特币 Layer2 / 调用层"),
            "比特币 layer2 调用层"
        );
        assert_eq!(
            tokenize_for_storage("模块化区块链"),
            "模块 块化 化区 区块 块链"
        );
        assert_eq!(estimate_token_count("模块化区块链"), 5);
        assert_eq!(LEXICAL_TOKENIZER_VERSION, "mixed-cjk-bigram-v2");
    }

    #[test]
    fn counts_newer_cjk_extensions_as_cjk() {
        assert_eq!(tokenize("\u{31350}\u{31351}\u{31352}"), vec!["𱍐𱍑", "𱍑𱍒"]);
        assert_eq!(estimate_token_count("\u{31350}\u{31351}\u{31352}"), 2);
    }
}
