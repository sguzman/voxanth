//! Experimental IPA soundboard synthesis.
//!
//! Covers all 28 basic IPA vowel symbols and an explicitly supported subset
//! of English pulmonic consonants and affricates. Formants are approximate
//! educational targets, NOT measured speaker-independent articulations.
//! Unknown IPA symbols fail explicitly instead of being silently replaced.

use super::{Filter, SAMPLE_RATE, TAU, Voice};

pub const VOWELS: [(&str, [f32; 3]); 28] = [
    ("i", [280., 2350., 3000.]), ("y", [300., 1800., 2700.]),
    ("ɨ", [330., 1700., 2600.]), ("ʉ", [330., 1400., 2400.]),
    ("ɯ", [350., 1250., 2400.]), ("u", [310., 830., 2300.]),
    ("ɪ", [390., 2000., 2650.]), ("ʏ", [420., 1680., 2500.]),
    ("ʊ", [410., 1150., 2400.]),
    ("e", [420., 2050., 2900.]), ("ø", [430., 1650., 2650.]),
    ("ɘ", [450., 1700., 2700.]), ("ɵ", [460., 1450., 2500.]),
    ("ɤ", [460., 1300., 2450.]), ("o", [460., 890., 2400.]),
    ("ə", [500., 1500., 2500.]),
    ("ɛ", [600., 1780., 2650.]), ("œ", [610., 1480., 2500.]),
    ("ɜ", [600., 1450., 2500.]), ("ɞ", [600., 1200., 2400.]),
    ("ʌ", [600., 1150., 2500.]), ("ɔ", [600., 870., 2400.]),
    ("æ", [680., 1720., 2600.]), ("ɐ", [700., 1300., 2500.]),
    ("a", [800., 1400., 2600.]), ("ɶ", [800., 1100., 2400.]),
    ("ɑ", [800., 1080., 2450.]), ("ɒ", [760., 930., 2300.]),
];

/// Consonants currently modeled by the source-filter renderer.
/// Stops / affricates are auditioned with a neutral vowel when isolated.
pub const CONSONANTS: [&str; 24] = [
    "p", "b", "t", "d", "k", "ɡ", "m", "n", "ŋ",
    "f", "v", "θ", "ð", "s", "z", "ʃ", "ʒ", "h",
    "ɹ", "j", "w", "l", "tʃ", "dʒ",
];

#[derive(Clone, Copy)]
enum Kind {
    Vowel,
    Stop { voiced: bool },
    Fricative { voiced: bool },
    Nasal,
    Approximant,
    Affricate { voiced: bool },
}

#[derive(Clone, Copy)]
struct Sound {
    first: [f32; 3],
    last: [f32; 3],
    kind: Kind,
}

#[derive(Clone, Copy)]
enum Part { Sound(Sound, f32), Pause }

fn vowel(symbol: &str) -> Option<Sound> {
    VOWELS.iter().find(|(s, _)| *s == symbol).map(|(_, target)| Sound {
        first: *target, last: *target, kind: Kind::Vowel,
    })
}

fn consonant(symbol: &str) -> Option<Sound> {
    use Kind::*;
    let (formants, kind) = match symbol {
        "p" => ([410., 1180., 2450.], Stop { voiced: false }),
        "b" => ([410., 1180., 2450.], Stop { voiced: true }),
        "t" => ([520., 1750., 3250.], Stop { voiced: false }),
        "d" => ([520., 1750., 3250.], Stop { voiced: true }),
        "k" => ([500., 1450., 2700.], Stop { voiced: false }),
        "ɡ" => ([500., 1450., 2700.], Stop { voiced: true }),
        "m" | "n" | "ŋ" => ([310., 1190., 2450.], Nasal),
        "f" => ([450., 2500., 4300.], Fricative { voiced: false }),
        "v" => ([450., 2500., 4300.], Fricative { voiced: true }),
        "θ" => ([510., 2250., 3800.], Fricative { voiced: false }),
        "ð" => ([510., 2250., 3800.], Fricative { voiced: true }),
        "s" => ([700., 4200., 6350.], Fricative { voiced: false }),
        "z" => ([700., 4200., 6350.], Fricative { voiced: true }),
        "ʃ" => ([520., 2600., 4200.], Fricative { voiced: false }),
        "ʒ" => ([520., 2600., 4200.], Fricative { voiced: true }),
        "h" => ([560., 1700., 3000.], Fricative { voiced: false }),
        "ɹ" => ([430., 1170., 1690.], Approximant),
        "j" => ([320., 2250., 2950.], Approximant),
        "w" => ([330., 950., 2420.], Approximant),
        "l" => ([425., 1270., 2630.], Approximant),
        "tʃ" => ([520., 2600., 4200.], Affricate { voiced: false }),
        "dʒ" => ([520., 2600., 4200.], Affricate { voiced: true }),
        _ => return None,
    };
    Some(Sound { first: formants, last: formants, kind })
}

pub fn is_supported(symbol: &str) -> bool {
    vowel(symbol).is_some() || consonant(symbol).is_some()
}

fn split(input: &str) -> Result<Vec<Part>, String> {
    let mut sequence = Vec::new();
    let mut text = input.trim();
    while !text.is_empty() && sequence.len() < 80 {
        if text.starts_with("tʃ") || text.starts_with("dʒ") {
            let symbol = &text[..text.char_indices().nth(2).map_or(text.len(), |(i, _)| i)];
            let atom = consonant(symbol).expect("supported affricate");
            sequence.push(Part::Sound(atom, 1.0));
            text = &text[symbol.len()..];
            continue;
        }
        let symbol = text.chars().next().expect("nonempty string");
        text = &text[symbol.len_utf8()..];
        match symbol {
            '[' | ']' | '/' | 'ˈ' | 'ˌ' | '͡' => {}
            'ː' | 'ˑ' => {
                if let Some(Part::Sound(_, length)) = sequence.last_mut() {
                    *length *= if symbol == 'ː' { 1.7 } else { 1.3 };
                } else {
                    return Err("Length mark must follow an IPA sound.".into());
                }
            }
            ' ' | '\n' | '.' | '|' | '‿' => {
                if !sequence.is_empty() && !matches!(sequence.last(), Some(Part::Pause)) {
                    sequence.push(Part::Pause);
                }
            }
            _ => {
                let s = symbol.to_string();
                let atom = vowel(&s).or_else(|| consonant(&s)).ok_or_else(|| {
                    format!("IPA symbol /{symbol}/ is not modeled yet. No substitution was made.")
                })?;
                sequence.push(Part::Sound(atom, 1.0));
            }
        }
    }
    while matches!(sequence.last(), Some(Part::Pause)) { sequence.pop(); }
    if sequence.len() == 1 {
        if let Part::Sound(sound, _) = sequence[0] {
            if !matches!(sound.kind, Kind::Vowel) {
                // A single stop cannot be heard without a release/vowel cue.
                sequence.push(Part::Sound(vowel("ə").expect("schwa target"), 0.62));
            }
        }
    }
    Ok(sequence)
}

fn transition(a: [f32; 3], b: [f32; 3], fraction: f32) -> [f32; 3] {
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * fraction.clamp(0., 1.))
}

impl Filter {
    fn retune_ipa(&mut self, frequency: f32, bandwidth: f32) {
        let fresh = Self::bandpass(frequency, bandwidth);
        self.b0 = fresh.b0;
        self.b2 = fresh.b2;
        self.a1 = fresh.a1;
        self.a2 = fresh.a2;
    }
}

/// Render approximate IPA phones with a continuous source and resonators.
/// This is a synthesis experiment; it is not a claim of phonetic fidelity.
pub fn render(voice: &Voice, input: &str) -> Result<Vec<f32>, String> {
    let sequence = split(input)?;
    if sequence.is_empty() { return Ok(Vec::new()); }
    let voice = voice.sanitized();
    let mut output = Vec::new();
    let limit = SAMPLE_RATE as usize * 10;
    let mut phase = 0.0f32;
    let mut noise_state = 0x8D69_E4B1u32;
    let mut previous = vowel("ə").expect("schwa target").first;
    let mut bands = [
        Filter::bandpass(500., 95.),
        Filter::bandpass(1500., 150.),
        Filter::bandpass(2500., 240.),
    ];
    let weights: [f32; 14] = std::array::from_fn(|i| {
        1.0 / ((i + 1) as f32).powf(1.42 - 0.38 * voice.brightness)
    });
    let isolated = sequence.len() == 1;
    for (index, part) in sequence.iter().enumerate() {
        if output.len() >= limit { break; }
        if let Part::Pause = part {
            output.extend(std::iter::repeat_n(0.0, (0.065 * SAMPLE_RATE as f32) as usize));
            continue;
        }
        let Part::Sound(sound, multiplier) = part else { continue };
        let (ms, voiced, noise, nasal) = match sound.kind {
            Kind::Vowel => (if isolated { 680. } else { 160. }, 1.0, voice.breathiness * 0.25, 0.0),
            Kind::Stop { voiced } => (95., if voiced { 0.30 } else { 0.0 }, 0.62, 0.0),
            Kind::Fricative { voiced } => (140., if voiced { 0.23 } else { 0.0 }, 0.94, 0.0),
            Kind::Nasal => (120., 0.62, 0.06, 0.45),
            Kind::Approximant => (105., 0.78, 0.05, 0.0),
            Kind::Affricate { voiced } => (135., if voiced { 0.20 } else { 0.0 }, 0.95, 0.0),
        };
        let len = ((ms * multiplier / voice.speed * SAMPLE_RATE as f32 / 1000.) as usize)
            .min(limit - output.len());
        let next = sequence[index + 1..].iter().find_map(|p| match p {
            Part::Sound(s, _) => Some(s.first),
            Part::Pause => None,
        }).unwrap_or(sound.last);
        for n in 0..len {
            let progress = n as f32 / len.max(1) as f32;
            let target = if progress < 0.13 {
                transition(previous, sound.first, progress / 0.13)
            } else if progress > 0.79 {
                transition(sound.last, next, (progress - 0.79) / 0.21 * 0.30)
            } else {
                sound.first
            };
            if n % 64 == 0 {
                for (i, band) in bands.iter_mut().enumerate() {
                    band.retune_ipa(target[i] * voice.formant_scale, [95., 160., 250.][i]);
                }
            }
            let t = output.len() as f32 / SAMPLE_RATE as f32;
            let vibrato = 1. + 0.018 * voice.vibrato * (TAU * 5.2 * t).sin();
            phase = (phase + voice.pitch_hz * vibrato / SAMPLE_RATE as f32).fract();
            let mut harmonic = 0.0f32;
            for (i, weight) in weights.iter().enumerate() {
                harmonic += (TAU * phase * (i + 1) as f32).sin() * weight;
            }
            noise_state ^= noise_state << 13;
            noise_state ^= noise_state >> 17;
            noise_state ^= noise_state << 5;
            let hiss = noise_state as f32 / u32::MAX as f32 * 2. - 1.;
            let (closure, burst) = if matches!(sound.kind, Kind::Stop{..} | Kind::Affricate{..}) {
                (if progress < .50 { 0.0 } else { 1.0 },
                 if progress >= .50 && progress < .72 { 1.0 } else { .10 })
            } else { (1.0, 1.0) };
            let input = harmonic * voiced * closure + hiss * noise * burst;
            let filtered = bands[0].tick(input) * 1.8
                + bands[1].tick(input) * (0.75 + voice.brightness * 0.4)
                + bands[2].tick(input) * (0.33 + voice.brightness * 0.6);
            let airy = if matches!(sound.kind, Kind::Fricative{..} | Kind::Affricate{..}) {
                hiss * noise * .23
            } else { 0.0 };
            let fade_in = (n as f32 / (SAMPLE_RATE as f32 * .008)).min(1.);
            let fade_out = ((len - n) as f32 / (SAMPLE_RATE as f32 * .008)).min(1.);
            let sample = ((filtered * (1. - nasal) + harmonic * nasal * .045 + airy)
                * 1.8 * fade_in * fade_out).tanh();
            output.push(if sample.is_finite() { sample } else { 0.0 });
        }
        previous = sound.last;
    }
    output.truncate(limit);
    let peak = output.iter().fold(0.0_f32, |max, x| max.max(x.abs()));
    if peak > 0.82 {
        for sample in &mut output { *sample *= 0.82 / peak; }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_cardinal_vowel_has_real_finite_sound() {
        assert_eq!(VOWELS.len(), 28);
        for (ipa, _) in VOWELS {
            let audio = render(&Voice::default(), ipa).unwrap();
            assert!(audio.len() > 6000, "{ipa}");
            assert!(audio.iter().all(|v| v.is_finite() && v.abs() <= 1.), "{ipa}");
            assert!(audio.iter().any(|v| v.abs() > .001), "{ipa}");
        }
    }
    #[test]
    fn consonants_have_audible_context() {
        assert_eq!(CONSONANTS.len(), 24);
        for ipa in CONSONANTS {
            assert!(is_supported(ipa), "{ipa}");
            let audio = render(&Voice::default(), ipa).unwrap();
            assert!(audio.iter().any(|v| v.abs() > .001), "{ipa}");
        }
    }
    #[test]
    fn sequences_can_be_rendered_and_invalid_symbols_are_reported() {
        for seq in ["/hɛloʊ/", "[tʃæ t]", "pə", "kɪt", "iː", "dʒə"] {
            assert!(render(&Voice::default(), seq).unwrap().len() > 3000, "{seq}");
        }
        assert!(render(&Voice::default(), "ʙ").unwrap_err().contains("not modeled"));
        assert!(render(&Voice::default(), "ː").is_err());
        assert!(render(&Voice::default(), " ").unwrap().is_empty());
    }
    #[test]
    fn contrasting_vowels_do_not_produce_identical_pcm() {
        assert_ne!(render(&Voice::default(), "i").unwrap(),
                   render(&Voice::default(), "u").unwrap());
    }
}
