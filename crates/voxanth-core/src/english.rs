//! English-first pronunciation front-end for Voxanth.
//!
//! A deliberately small, inspectable pronunciation lexicon with a conservative
//! deterministic fallback. This is NOT a complete English grapheme-to-phoneme
//! model. Fallback words are always reported, never silently treated as correct.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phone {
    AA, AE, AH, AO, AW, AY, EH, ER, EY, IH, IY, OW, OY, UH, UW,
    B, CH, D, DH, F, G, HH, JH, K, L, M, N, NG, P, R, S, SH,
    T, TH, V, W, Y, Z, ZH,
    ShortPause, LongPause,
}

impl Phone {
    pub fn is_vowel(self) -> bool {
        matches!(self,
            Self::AA | Self::AE | Self::AH | Self::AO | Self::AW |
            Self::AY | Self::EH | Self::ER | Self::EY | Self::IH |
            Self::IY | Self::OW | Self::OY | Self::UH | Self::UW)
    }

    pub fn is_stop(self) -> bool {
        matches!(self, Self::B | Self::D | Self::G | Self::K |
            Self::P | Self::T | Self::CH | Self::JH)
    }

    pub fn is_fricative(self) -> bool {
        matches!(self, Self::DH | Self::F | Self::HH | Self::S |
            Self::SH | Self::TH | Self::V | Self::Z | Self::ZH)
    }

    pub fn is_nasal(self) -> bool {
        matches!(self, Self::M | Self::N | Self::NG)
    }

    pub fn is_voiced(self) -> bool {
        self.is_vowel() || matches!(self, Self::B | Self::D | Self::DH |
            Self::G | Self::JH | Self::L | Self::M | Self::N | Self::NG |
            Self::R | Self::V | Self::W | Self::Y | Self::Z | Self::ZH)
    }

    pub fn is_pause(self) -> bool {
        matches!(self, Self::ShortPause | Self::LongPause)
    }

    pub fn label(self) -> String {
        match self {
            Self::ShortPause => "·".into(),
            Self::LongPause => "/".into(),
            _ => format!("{self:?}"),
        }
    }

    fn from_arpabet(code: &str) -> Option<Self> {
        let base = code.trim_end_matches(|c: char| c.is_ascii_digit());
        Some(match base {
            "AA" => Self::AA, "AE" => Self::AE, "AH" => Self::AH,
            "AO" => Self::AO, "AW" => Self::AW, "AY" => Self::AY,
            "EH" => Self::EH, "ER" => Self::ER, "EY" => Self::EY,
            "IH" => Self::IH, "IY" => Self::IY, "OW" => Self::OW,
            "OY" => Self::OY, "UH" => Self::UH, "UW" => Self::UW,
            "B" => Self::B, "CH" => Self::CH, "D" => Self::D,
            "DH" => Self::DH, "F" => Self::F, "G" => Self::G,
            "HH" => Self::HH, "JH" => Self::JH, "K" => Self::K,
            "L" => Self::L, "M" => Self::M, "N" => Self::N,
            "NG" => Self::NG, "P" => Self::P, "R" => Self::R,
            "S" => Self::S, "SH" => Self::SH, "T" => Self::T,
            "TH" => Self::TH, "V" => Self::V, "W" => Self::W,
            "Y" => Self::Y, "Z" => Self::Z, "ZH" => Self::ZH,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Segment {
    pub phone: Phone,
    /// 0 = unstressed; 1 = primary stress; 2 = secondary stress.
    pub stress: u8,
}

impl Segment {
    fn new(phone: Phone, stress: u8) -> Self { Self { phone, stress } }
}

#[derive(Clone, Debug, Default)]
pub struct Transcription {
    pub segments: Vec<Segment>,
    /// These words use Voxanth's heuristic fallback and may be mispronounced.
    pub fallback_words: Vec<String>,
    pub is_question: bool,
}

impl Transcription {
    pub fn readable(&self) -> String {
        self.segments.iter().map(|s| {
            let mut label = s.phone.label();
            if s.phone.is_vowel() {
                label.push(char::from(b'0' + s.stress.min(2)));
            }
            label
        }).collect::<Vec<_>>().join(" ")
    }
}

fn lexicon(word: &str) -> Option<&'static str> {
    Some(match word {
        "a" => "AH0",
        "about" => "AH0 B AW1 T",
        "all" => "AO1 L",
        "and" => "AE1 N D",
        "are" => "AA1 R",
        "artificial" => "AA2 R T AH0 F IH1 SH AH0 L",
        "beautiful" => "B Y UW1 T AH0 F AH0 L",
        "black" => "B L AE1 K",
        "blue" => "B L UW1",
        "can" => "K AE1 N",
        "cat" => "K AE1 T",
        "clear" => "K L IH1 R",
        "computer" => "K AH0 M P Y UW1 T ER0",
        "could" => "K UH1 D",
        "create" => "K R IY0 EY1 T",
        "day" => "D EY1",
        "design" => "D IH0 Z AY1 N",
        "do" => "D UW1",
        "dog" => "D AO1 G",
        "eight" => "EY1 T",
        "english" => "IH1 NG G L IH0 SH",
        "every" => "EH1 V R IY0",
        "five" => "F AY1 V",
        "for" => "F AO1 R",
        "four" => "F AO1 R",
        "from" => "F R AH1 M",
        "good" => "G UH1 D",
        "green" => "G R IY1 N",
        "have" => "HH AE1 V",
        "hear" => "HH IH1 R",
        "hello" => "HH AH0 L OW1",
        "here" => "HH IH1 R",
        "how" => "HH AW1",
        "human" => "HH Y UW1 M AH0 N",
        "i" => "AY1",
        "i'm" => "AY1 M",
        "in" => "IH1 N",
        "intelligence" => "IH0 N T EH1 L AH0 JH AH0 N S",
        "is" => "IH1 Z",
        "it" => "IH1 T",
        "like" => "L AY1 K",
        "little" => "L IH1 T AH0 L",
        "machine" => "M AH0 SH IY1 N",
        "made" => "M EY1 D",
        "make" => "M EY1 K",
        "making" => "M EY1 K IH0 NG",
        "me" => "M IY1",
        "model" => "M AA1 D AH0 L",
        "morning" => "M AO1 R N IH0 NG",
        "music" => "M Y UW1 Z IH0 K",
        "my" => "M AY1",
        "new" => "N UW1",
        "nine" => "N AY1 N",
        "no" => "N OW1",
        "not" => "N AA1 T",
        "of" => "AH1 V",
        "on" => "AA1 N",
        "one" => "W AH1 N",
        "open" => "OW1 P AH0 N",
        "our" => "AW1 ER0",
        "phoneme" => "F OW1 N IY0 M",
        "pleasant" => "P L EH1 Z AH0 N T",
        "please" => "P L IY1 Z",
        "red" => "R EH1 D",
        "say" => "S EY1",
        "seven" => "S EH1 V AH0 N",
        "should" => "SH UH1 D",
        "sing" => "S IH1 NG",
        "singer" => "S IH1 NG ER0",
        "singing" => "S IH1 NG IH0 NG",
        "six" => "S IH1 K S",
        "slow" => "S L OW1",
        "sound" => "S AW1 N D",
        "sounds" => "S AW1 N D Z",
        "source" => "S AO1 R S",
        "speech" => "S P IY1 CH",
        "synthetic" => "S IH0 N TH EH1 T IH0 K",
        "synthesis" => "S IH1 N TH AH0 S IH0 S",
        "ten" => "T EH1 N",
        "test" => "T EH1 S T",
        "testing" => "T EH1 S T IH0 NG",
        "thank" => "TH AE1 NG K",
        "thanks" => "TH AE1 NG K S",
        "that" => "DH AE1 T",
        "the" => "DH AH0",
        "there" => "DH EH1 R",
        "these" => "DH IY1 Z",
        "this" => "DH IH1 S",
        "three" => "TH R IY1",
        "time" => "T AY1 M",
        "to" => "T UW0",
        "today" => "T AH0 D EY1",
        "tomorrow" => "T AH0 M AA1 R OW0",
        "two" => "T UW1",
        "voice" => "V OY1 S",
        "voices" => "V OY1 S IH0 Z",
        "vowel" => "V AW1 AH0 L",
        "we" => "W IY1",
        "welcome" => "W EH1 L K AH0 M",
        "what" => "W AH1 T",
        "when" => "W EH1 N",
        "where" => "W EH1 R",
        "white" => "W AY1 T",
        "who" => "HH UW1",
        "why" => "W AY1",
        "will" => "W IH1 L",
        "world" => "W ER1 L D",
        "would" => "W UH1 D",
        "yes" => "Y EH1 S",
        "you" => "Y UW1",
        "your" => "Y AO1 R",
        _ => return None,
    })
}

fn append_arpabet(code: &str, target: &mut Vec<Segment>) {
    for token in code.split_whitespace() {
        if let Some(phone) = Phone::from_arpabet(token) {
            let stress = token.as_bytes().last().and_then(|b| {
                if b.is_ascii_digit() { Some(b - b'0') } else { None }
            }).unwrap_or(0).min(2);
            target.push(Segment::new(phone, stress));
        }
    }
}

fn fallback(word: &str, target: &mut Vec<Segment>) {
    // Longest common grapheme-first matching, then one-letter fallback.
    let mut rest = word;
    let mut stressed = false;
    while !rest.is_empty() {
        if rest == "e" && !target.is_empty() { break; } // basic silent-final-e heuristic
        let patterns: &[(&str, &str)] = &[
            ("tion", "SH AH0 N"), ("tch", "CH"), ("igh", "AY1"),
            ("ough", "OW1"), ("ch", "CH"), ("sh", "SH"), ("th", "TH"),
            ("ng", "NG"), ("ph", "F"), ("wh", "W"), ("ck", "K"),
            ("wr", "R"), ("qu", "K W"), ("ee", "IY1"), ("ea", "IY1"),
            ("oo", "UW1"), ("oa", "OW1"), ("ai", "EY1"), ("ay", "EY1"),
            ("ow", "AW1"), ("ou", "AW1"), ("oi", "OY1"), ("oy", "OY1"),
            ("er", "ER0"), ("ir", "ER1"), ("ur", "ER1"),
        ];
        if let Some((grapheme, phones)) = patterns.iter().find(|(g,_)| rest.starts_with(g)) {
            append_arpabet(phones, target);
            if phones.chars().any(|c| c == '1') { stressed = true; }
            rest = &rest[grapheme.len()..];
            continue;
        }
        let ch = rest.as_bytes()[0] as char;
        let phones = match ch {
            'a' => "AE", 'e' => "EH", 'i' => "IH", 'o' => "AA",
            'u' => "AH", 'y' => "IY", 'b' => "B", 'c' => "K",
            'd' => "D", 'f' => "F", 'g' => "G", 'h' => "HH",
            'j' => "JH", 'k' => "K", 'l' => "L", 'm' => "M",
            'n' => "N", 'p' => "P", 'q' => "K", 'r' => "R",
            's' => "S", 't' => "T", 'v' => "V", 'w' => "W",
            'x' => "K S", 'z' => "Z",
            _ => "",
        };
        for phone in phones.split_whitespace().filter_map(Phone::from_arpabet) {
            let stress = if phone.is_vowel() && !stressed { stressed = true; 1 } else { 0 };
            target.push(Segment::new(phone, stress));
        }
        rest = &rest[1..];
    }
}

fn add_word(word: &str, transcription: &mut Transcription) {
    if word.is_empty() { return; }
    if transcription.segments.len() >= 180 { return; }
    if let Some(phonemes) = lexicon(word) {
        append_arpabet(phonemes, &mut transcription.segments);
    } else {
        if !transcription.fallback_words.iter().any(|w| w == word) {
            transcription.fallback_words.push(word.to_owned());
        }
        fallback(word, &mut transcription.segments);
    }
}

/// Transcribes a bounded English text input to a mixed lexicon/fallback sequence.
/// Question punctuation informs a simple rising phrase-contour in the renderer.
pub fn transcribe(text: &str) -> Transcription {
    let mut result = Transcription::default();
    let mut word = String::new();
    for ch in text.chars().take(160) {
        if ch.is_ascii_alphabetic() || (ch == '\'' && !word.is_empty()) {
            word.push(ch.to_ascii_lowercase());
            continue;
        }
        if !word.is_empty() {
            add_word(&word, &mut result);
            word.clear();
        }
        if ch == '?' { result.is_question = true; }
        if ch.is_whitespace() || matches!(ch, ',' | ';' | ':' | '.' | '!' | '?' | '-') {
            let pause = if matches!(ch, ',' | ';' | ':' | '.' | '!' | '?') {
                Phone::LongPause
            } else { Phone::ShortPause };
            if let Some(last) = result.segments.last_mut() {
                if last.phone.is_pause() {
                    if pause == Phone::LongPause { last.phone = pause; }
                } else {
                    result.segments.push(Segment::new(pause, 0));
                }
            }
        }
    }
    add_word(&word, &mut result);
    while result.segments.last().is_some_and(|s| s.phone.is_pause()) {
        result.segments.pop();
    }
    result.segments.truncate(200);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_words_produce_stressed_english_phonemes() {
        let result = transcribe("Hello world!");
        assert_eq!(result.fallback_words, Vec::<String>::new());
        assert_eq!(result.readable(),
            "HH AH0 L OW1 · W ER1 L D");
    }

    #[test]
    fn english_digraphs_are_not_split_into_letters() {
        let result = transcribe("shack");
        assert_eq!(result.fallback_words, vec!["shack"]);
        let phones: Vec<_> = result.segments.iter().map(|s| s.phone).collect();
        assert_eq!(phones, vec![Phone::SH, Phone::AE, Phone::K]);
    }

    #[test]
    fn fallback_words_are_visible_and_deduplicated() {
        let result = transcribe("Voxanth voxanth ?");
        assert_eq!(result.fallback_words, vec!["voxanth"]);
        assert!(result.is_question);
        assert!(!result.segments.is_empty());
    }

    #[test]
    fn punctuation_does_not_erase_word_boundaries() {
        let result = transcribe("Hello, voice!");
        assert!(result.segments.iter().any(|s| s.phone == Phone::LongPause));
        assert!(result.segments.iter().any(|s| s.phone == Phone::OY));
    }

    #[test]
    fn output_is_bounded_and_empty_is_valid() {
        assert!(transcribe("").segments.is_empty());
        assert!(transcribe(&"voice ".repeat(1000)).segments.len() <= 200);
    }

    #[test]
    fn all_lexicon_tokens_resolve_to_phone_codes() {
        for word in ["hello","world","synthetic","english","voice","synthesis","phoneme",
            "computer","beautiful","sound","intelligence","vowel","you","we"] {
            let sequence = lexicon(word).unwrap();
            for token in sequence.split_whitespace() {
                assert!(Phone::from_arpabet(token).is_some(), "{word}: {token}");
            }
        }
    }
}
