//! 772 word / number search for dialogue conditions (`strings.cc`).

/// Case-insensitive word-boundary search; trailing `$` requires a boundary after the term.
///
/// Returns the byte index of the match start, or `None`.
///
/// C++ `SearchForWord` — `strings.cc:318-366`.
pub fn search_for_word(pattern: &str, text: &str) -> Option<usize> {
    if pattern.is_empty() {
        return None;
    }
    let (needle, whole_word) = if let Some(rest) = pattern.strip_suffix('$') {
        (rest, true)
    } else {
        (pattern, false)
    };
    if needle.is_empty() {
        return None;
    }
    let text_bytes = text.as_bytes();
    let needle_bytes = needle.as_bytes();
    let mut word_start = true;
    let mut i = 0;
    while i < text_bytes.len() {
        let c = text_bytes[i];
        if !is_alnum(c) {
            word_start = true;
            i += 1;
            continue;
        }
        if word_start {
            if matches_ci_prefix(text_bytes, i, needle_bytes) {
                let end = i + needle_bytes.len();
                if !whole_word || end >= text_bytes.len() || !is_alnum(text_bytes[end]) {
                    return Some(i);
                }
            }
            word_start = false;
        }
        i += 1;
    }
    None
}

/// Find the `count`-th (1-based) numeric word in `text`; returns start index of digits.
///
/// C++ `SearchForNumber` — `strings.cc:368-407`.
pub fn search_for_number(count: u8, text: &str) -> Option<usize> {
    if count < 1 {
        return None;
    }
    let text_bytes = text.as_bytes();
    let mut remaining = count;
    let mut word_start = true;
    let mut i = 0;
    while i < text_bytes.len() {
        let c = text_bytes[i];
        if !is_alnum(c) {
            word_start = true;
            i += 1;
            continue;
        }
        if word_start {
            let mut j = 0;
            while i + j < text_bytes.len() && text_bytes[i + j].is_ascii_digit() {
                j += 1;
            }
            if j > 0 && (i + j >= text_bytes.len() || !is_alpha(text_bytes[i + j])) {
                remaining -= 1;
                if remaining == 0 {
                    return Some(i);
                }
            }
            word_start = false;
        }
        i += 1;
    }
    None
}

fn matches_ci_prefix(hay: &[u8], start: usize, needle: &[u8]) -> bool {
    if start + needle.len() > hay.len() {
        return false;
    }
    for (a, b) in hay[start..start + needle.len()].iter().zip(needle.iter()) {
        if a.to_ascii_lowercase() != b.to_ascii_lowercase() {
            return false;
        }
    }
    true
}

fn is_alnum(c: u8) -> bool {
    c.is_ascii_alphanumeric()
}

fn is_alpha(c: u8) -> bool {
    c.is_ascii_alphabetic()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_boundary_and_dollar() {
        assert_eq!(search_for_word("hi$", "hi there"), Some(0));
        assert_eq!(search_for_word("hi$", "high"), None);
        assert_eq!(search_for_word("hi", "high"), Some(0));
        assert_eq!(search_for_word("bye", "say bye now"), Some(4));
        assert_eq!(search_for_word("Hi$", "HI"), Some(0));
    }

    #[test]
    fn numeric_nth() {
        assert_eq!(search_for_number(1, "bet 12 on 34"), Some(4));
        assert_eq!(search_for_number(2, "bet 12 on 34"), Some(10));
        assert_eq!(search_for_number(3, "bet 12 on 34"), None);
    }
}
