//! Experimental voice synthesis primitives.
//!
//! This is not a human vocal-tract simulation or a production TTS engine.
//! It is a deterministic, lightweight source-filter/formant synthesizer with
//! a deliberately limited text-to-sound front-end, suitable for exploration.

use serde::{Deserialize, Serialize};

pub mod english;
pub mod ipa;
mod speech_dsp;
use std::f32::consts::PI;

pub const SAMPLE_RATE: u32 = 44_100;
const MAX_SECONDS: f32 = 12.0;
const TAU: f32 = 2.0 * PI;

/// Saved timbre parameters. Slider ranges are enforced by `sanitized`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Voice {
    pub name: String,
    pub pitch_hz: f32,
    /// Spectral formant scale: 1 is nominal; larger is a shorter tract.
    pub formant_scale: f32,
    pub breathiness: f32,
    pub brightness: f32,
    pub nasality: f32,
    pub vibrato: f32,
    pub speed: f32,
}

impl Voice {
    pub fn sanitized(&self) -> Self {
        let mut p = self.clone();
        p.name = p.name.chars().take(60).collect();
        if p.name.is_empty() { p.name = "Untitled voice".into(); }
        fn clamp(value: f32, min: f32, max: f32, default: f32) -> f32 {
            if value.is_finite() { value.clamp(min, max) } else { default }
        }
        p.pitch_hz = clamp(p.pitch_hz, 65.0, 450.0, 145.0);
        p.formant_scale = clamp(p.formant_scale, 0.65, 1.6, 1.0);
        p.breathiness = clamp(p.breathiness, 0.0, 1.0, 0.12);
        p.brightness = clamp(p.brightness, 0.0, 1.0, 0.5);
        p.nasality = clamp(p.nasality, 0.0, 1.0, 0.1);
        p.vibrato = clamp(p.vibrato, 0.0, 1.0, 0.05);
        p.speed = clamp(p.speed, 0.5, 1.6, 1.0);
        p
    }

    pub fn presets() -> Vec<Self> {
        vec![
            Self { name: "Clarity / speech".into(), pitch_hz: 150.0, formant_scale: 1.04, breathiness: 0.06, brightness: 0.65, nasality: 0.08, vibrato: 0.0, speed: 1.0 },
            Self { name: "Ember / warm".into(), pitch_hz: 112.0, formant_scale: 0.83, breathiness: 0.11, brightness: 0.34, nasality: 0.12, vibrato: 0.08, speed: 0.9 },
            Self { name: "Glass / bright".into(), pitch_hz: 230.0, formant_scale: 1.28, breathiness: 0.18, brightness: 0.9, nasality: 0.03, vibrato: 0.07, speed: 1.1 },
            Self { name: "Aether / synthetic".into(), pitch_hz: 195.0, formant_scale: 0.94, breathiness: 0.3, brightness: 0.72, nasality: 0.35, vibrato: 0.42, speed: 0.95 },
        ]
    }
}

impl Default for Voice {
    fn default() -> Self { Self::presets().remove(0) }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Vowel { A, E, I, O, U }

impl Vowel {
    pub const ALL: [Self; 5] = [Self::A, Self::E, Self::I, Self::O, Self::U];
    pub fn label(self) -> &'static str {
        match self { Self::A => "A", Self::E => "E", Self::I => "I", Self::O => "O", Self::U => "U" }
    }
    fn formants(self) -> [f32; 3] {
        // Deliberately approximate average vowel targets in hertz.
        match self {
            Self::A => [800.0, 1200.0, 2800.0],
            Self::E => [470.0, 1950.0, 2700.0],
            Self::I => [300.0, 2350.0, 3100.0],
            Self::O => [500.0, 900.0, 2550.0],
            Self::U => [350.0, 800.0, 2300.0],
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Phone { Vowel(Vowel), Fricative, Nasal, Liquid, Stop, Pause }

#[derive(Clone, Copy)]
struct Filter {
    b0: f32, b2: f32, a1: f32, a2: f32,
    x2: f32, y1: f32, y2: f32,
}

impl Filter {
    fn bandpass(frequency: f32, bandwidth: f32) -> Self {
        let w = TAU * frequency.clamp(60.0, 9500.0) / SAMPLE_RATE as f32;
        let q = (frequency / bandwidth).max(0.3);
        let alpha = w.sin() / (2.0 * q);
        let a0 = 1.0 + alpha;
        Self { b0: alpha / a0, b2: -alpha / a0,
               a1: -2.0 * w.cos() / a0, a2: (1.0 - alpha) / a0,
               x2: 0.0, y1: 0.0, y2: 0.0 }
    }
    fn tick(&mut self, input: f32) -> f32 {
        let y = self.b0 * input + self.b2 * self.x2 - self.a1 * self.y1 - self.a2 * self.y2;
        self.x2 = input;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

/// A waveform. Samples are 32-bit floats in [-1, 1] at SAMPLE_RATE.
pub fn render_vowel(voice: &Voice, vowel: Vowel, seconds: f32) -> Vec<f32> {
    render_sequence(voice, &[(Phone::Vowel(vowel), seconds.clamp(0.05, MAX_SECONDS))])
}

pub fn render_singing(voice: &Voice, vowel: Vowel, seconds: f32) -> Vec<f32> {
    let mut singer = voice.sanitized();
    singer.vibrato = singer.vibrato.max(0.35);
    render_vowel(&singer, vowel, seconds)
}

/// Synthesizes a bounded English phoneme transcription.
///
/// Lexicon coverage is intentionally small. Use `english::transcribe` to
/// inspect heuristic fallback words rather than assuming every pronunciation
/// is correct.
pub fn render_speech(voice: &Voice, input: &str) -> Vec<f32> {
    speech_dsp::render(voice, &english::transcribe(input))
}

fn render_sequence(voice: &Voice, sequence: &[(Phone, f32)]) -> Vec<f32> {
    let p = voice.sanitized();
    let mut out = Vec::new();
    let mut phase = 0.0_f32;
    let mut noise_state = 0xA51C_E235_u32;
    let mut tick = 0_usize;
    for &(phone, seconds) in sequence {
        if out.len() >= SAMPLE_RATE as usize * MAX_SECONDS as usize { break; }
        let len = (seconds.clamp(0.0, MAX_SECONDS) * SAMPLE_RATE as f32) as usize;
        if len == 0 { continue; }
        if matches!(phone, Phone::Pause) {
            out.extend(std::iter::repeat_n(0.0, len)); tick += len;
            continue;
        }
        let formants = match phone {
            Phone::Vowel(v) => v.formants(),
            Phone::Nasal => [320.0, 1200.0, 2400.0],
            Phone::Liquid => [460.0, 1400.0, 2650.0],
            Phone::Stop => [520.0, 1700.0, 2700.0],
            Phone::Fricative => [700.0, 3400.0, 5100.0],
            Phone::Pause => unreachable!(),
        };
        let mut bands = [
            Filter::bandpass(formants[0] * p.formant_scale, 85.0),
            Filter::bandpass(formants[1] * p.formant_scale, 130.0),
            Filter::bandpass(formants[2] * p.formant_scale, 230.0),
        ];
        let harmonic_weights: [f32; 16] = std::array::from_fn(|i| {
            1.0 / ((i + 1) as f32).powf(1.35 - p.brightness * 0.45)
        });
        for n in 0..len {
            let t = tick as f32 / SAMPLE_RATE as f32;
            let vibrato = 1.0 + 0.018 * p.vibrato * (TAU * 5.2 * t).sin();
            phase = (phase + p.pitch_hz * vibrato / SAMPLE_RATE as f32).fract();
            // Compact harmonic source. Each partial is attenuated with frequency.
            let mut harmonic = 0.0;
            for (k, weight) in harmonic_weights.iter().enumerate() {
                harmonic += (TAU * phase * (k + 1) as f32).sin() * weight;
            }
            noise_state ^= noise_state << 13;
            noise_state ^= noise_state >> 17;
            noise_state ^= noise_state << 5;
            let noise = (noise_state as f32 / u32::MAX as f32) * 2.0 - 1.0;
            let (voicing, turbulence) = match phone {
                Phone::Vowel(_) => (1.0, p.breathiness * 0.35),
                Phone::Nasal => (0.65, 0.04),
                Phone::Liquid => (0.75, 0.05),
                Phone::Stop => (0.15, 0.32),
                Phone::Fricative => (0.05, 0.9),
                Phone::Pause => (0.0, 0.0),
            };
            let input = harmonic * voicing + noise * turbulence;
            let filtered = bands[0].tick(input) * 1.8
                + bands[1].tick(input) * (0.9 + p.brightness * 0.5)
                + bands[2].tick(input) * (0.3 + p.brightness * 0.65);
            let nasal = if matches!(phone, Phone::Nasal) { 0.7 } else { p.nasality * 0.22 };
            let mixed = filtered * (1.0 - nasal) + harmonic * nasal * 0.12;
            let fade = (n as f32 / (SAMPLE_RATE as f32 * 0.009)).min(1.0)
                * ((len - n) as f32 / (SAMPLE_RATE as f32 * 0.009)).min(1.0);
            out.push((mixed * fade * 2.1).tanh());
            tick += 1;
        }
    }
    // Avoid harsh clipping; a consistent ceiling makes profile A/B comparison safer.
    let peak = out.iter().fold(0.0_f32, |m, v| m.max(v.abs()));
    if peak > 0.82 {
        let factor = 0.82 / peak;
        for sample in &mut out { *sample *= factor; }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn presets_are_valid() {
        for p in Voice::presets() { assert_eq!(p, p.sanitized()); }
    }
    #[test]
    fn vowels_generate_finite_non_silent_audio() {
        for vowel in Vowel::ALL {
            let pcm = render_vowel(&Voice::default(), vowel, 0.2);
            assert_eq!(pcm.len(), 8820);
            assert!(pcm.iter().all(|s| s.is_finite() && s.abs() <= 1.0));
            assert!(pcm.iter().any(|s| s.abs() > 0.005));
        }
    }
    #[test]
    fn speech_handles_accents_punctuation_and_empty_input() {
        assert!(render_speech(&Voice::default(), "").is_empty());
        let pcm = render_speech(&Voice::default(), "Hello, world!");
        assert!(pcm.len() > SAMPLE_RATE as usize / 2);
        assert!(pcm.iter().all(|s| s.is_finite()));
    }
    #[test]
    fn hostile_parameter_values_are_clamped() {
        let mut p = Voice::default();
        p.pitch_hz = f32::NAN;
        p.vibrato = f32::INFINITY;
        p.formant_scale = -200.0;
        let pcm = render_vowel(&p, Vowel::I, 0.1);
        assert!(pcm.iter().all(|s| s.is_finite()));
    }
    #[test]
    fn distinct_profiles_produce_distinct_audio() {
        let ps = Voice::presets();
        let a = render_vowel(&ps[0], Vowel::A, 0.1);
        let b = render_vowel(&ps[1], Vowel::A, 0.1);
        assert_ne!(a, b);
    }
}
