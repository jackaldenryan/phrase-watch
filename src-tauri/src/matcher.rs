use std::collections::HashMap;
use std::time::Instant;

pub fn normalize(text: &str) -> String {
    let lowered = text.to_lowercase().replace('\u{2019}', "'");
    let contracted = regex_im(&lowered);
    let mut out = String::new();
    let mut last_space = true;
    for ch in contracted.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_space = false;
        } else if !last_space {
            out.push(' ');
            last_space = true;
        }
    }
    out.trim().to_string()
}

fn regex_im(text: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if (i == 0 || !chars[i - 1].is_ascii_alphanumeric())
            && chars[i] == 'i'
            && i + 2 < chars.len()
            && (chars[i + 1] == '\'' || chars[i + 1] == '\u{2019}')
            && chars[i + 2] == 'm'
            && (i + 3 == chars.len() || !chars[i + 3].is_ascii_alphanumeric())
        {
            out.push_str("i am");
            i += 3;
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

pub fn match_phrase<'a>(transcript: &str, phrases: &'a [String]) -> Option<&'a str> {
    let hay = format!(" {} ", normalize(transcript));
    if hay.trim().is_empty() {
        return None;
    }
    let mut ordered: Vec<&String> = phrases.iter().collect();
    ordered.sort_by_key(|p| std::cmp::Reverse(normalize(p).len()));
    for phrase in ordered {
        let needle = normalize(phrase);
        if needle.is_empty() {
            continue;
        }
        if hay.contains(&format!(" {needle} ")) {
            return Some(phrase.as_str());
        }
    }
    None
}

pub struct Debouncer {
    seconds: f64,
    last: HashMap<String, Instant>,
}

impl Debouncer {
    pub fn new(seconds: f64) -> Self {
        Self {
            seconds,
            last: HashMap::new(),
        }
    }

    pub fn allow(&mut self, phrase: &str) -> bool {
        let key = normalize(phrase);
        let now = Instant::now();
        if let Some(prev) = self.last.get(&key) {
            if now.duration_since(*prev).as_secs_f64() < self.seconds {
                return false;
            }
        }
        self.last.insert(key, now);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_contraction() {
        assert_eq!(normalize("I'm sorry"), "i am sorry");
        assert_eq!(normalize("I’M SORRY!"), "i am sorry");
    }

    #[test]
    fn matches_variants() {
        let phrases = vec!["i'm sorry".into(), "i am sorry".into()];
        assert_eq!(
            match_phrase("yeah I'm sorry about that", &phrases),
            Some("i'm sorry")
        );
        assert!(match_phrase("the weather is nice today", &phrases).is_none());
    }

    #[test]
    fn prefers_longer() {
        let phrases = vec!["sorry".into(), "i'm sorry".into()];
        assert_eq!(
            match_phrase("I said I'm sorry", &phrases),
            Some("i'm sorry")
        );
    }

    #[test]
    fn many_phrases() {
        let phrases = vec![
            "i'm sorry".into(),
            "i apologize".into(),
            "my bad".into(),
        ];
        assert_eq!(match_phrase("oh my bad dude", &phrases), Some("my bad"));
    }
}
