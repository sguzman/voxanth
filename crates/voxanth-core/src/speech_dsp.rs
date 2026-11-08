//! Deterministic English phoneme renderer (experimental acoustic baseline).
//!
//! Maintains a continuous oscillator and resonator state across phones rather
//! than reinitializing an isolated vowel on every character. This still lacks
//! a physiological tract model and should not be marketed as natural speech.

use super::{Filter, MAX_SECONDS, SAMPLE_RATE, TAU, Voice};
use crate::english::{Phone, Segment, Transcription};

#[derive(Clone, Copy)]
struct Resonances {
    start: [f32; 3],
    end: [f32; 3],
}

impl Resonances {
    const fn fixed(v: [f32; 3]) -> Self { Self { start: v, end: v } }
}

fn resonances(phone: Phone) -> Resonances {
    use Phone::*;
    match phone {
        AA => Resonances::fixed([750., 1120., 2600.]),
        AE => Resonances::fixed([660., 1690., 2450.]),
        AH => Resonances::fixed([590., 1270., 2550.]),
        AO => Resonances::fixed([580., 900., 2600.]),
        EH => Resonances::fixed([520., 1840., 2600.]),
        ER => Resonances::fixed([470., 1380., 1700.]),
        IH => Resonances::fixed([410., 2040., 2660.]),
        IY => Resonances::fixed([290., 2340., 3100.]),
        UH => Resonances::fixed([420., 1020., 2360.]),
        UW => Resonances::fixed([310., 890., 2400.]),
        AW => Resonances { start: [750., 1120., 2600.], end: [420., 920., 2380.] },
        AY => Resonances { start: [760., 1190., 2600.], end: [300., 2250., 2950.] },
        EY => Resonances { start: [525., 1850., 2600.], end: [310., 2280., 2960.] },
        OW => Resonances { start: [590., 880., 2520.], end: [340., 910., 2450.] },
        OY => Resonances { start: [570., 900., 2550.], end: [340., 2160., 2850.] },
        M | N | NG => Resonances::fixed([310., 1190., 2450.]),
        L => Resonances::fixed([425., 1270., 2630.]),
        R => Resonances::fixed([430., 1170., 1690.]),
        W => Resonances::fixed([330., 950., 2420.]),
        Y => Resonances::fixed([320., 2250., 2950.]),
        S | Z => Resonances::fixed([700., 4200., 6350.]),
        SH | ZH | CH | JH => Resonances::fixed([520., 2600., 4200.]),
        F | V => Resonances::fixed([450., 2500., 4300.]),
        TH | DH => Resonances::fixed([510., 2250., 3800.]),
        HH => Resonances::fixed([560., 1700., 3000.]),
        B | P => Resonances::fixed([410., 1180., 2450.]),
        D | T => Resonances::fixed([520., 1750., 3250.]),
        G | K => Resonances::fixed([500., 1450., 2700.]),
        ShortPause | LongPause => Resonances::fixed([480., 1400., 2550.]),
    }
}

fn lerp(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t)
}

fn duration(segment: Segment, speed: f32) -> usize {
    let ms: f32 = if segment.phone == Phone::ShortPause { 42.0 }
        else if segment.phone == Phone::LongPause { 145.0 }
        else if segment.phone.is_vowel() { if segment.stress > 0 { 172.0 } else { 112.0 } }
        else if segment.phone.is_stop() { 88.0 }
        else if segment.phone.is_fricative() { 112.0 }
        else if segment.phone.is_nasal() { 110.0 }
        else { 88.0 };
    ((ms / speed) * SAMPLE_RATE as f32 / 1000.0) as usize
}

impl Filter {
    /// Update coefficients without discarding resonator history.
    fn retune(&mut self, frequency: f32, bandwidth: f32) {
        let mut fresh = Self::bandpass(frequency, bandwidth);
        fresh.x2 = self.x2;
        fresh.y1 = self.y1;
        fresh.y2 = self.y2;
        *self = fresh;
    }
}

/// Render a transcribed English phrase as mono floating point samples.
pub fn render(voice: &Voice, transcription: &Transcription) -> Vec<f32> {
    if transcription.segments.is_empty() { return Vec::new(); }
    let voice = voice.sanitized();
    let max_samples = SAMPLE_RATE as usize * MAX_SECONDS as usize;
    let expected: usize = transcription.segments.iter()
        .map(|segment| duration(*segment, voice.speed)).sum();
    let mut samples = Vec::with_capacity(expected.min(max_samples));
    let mut filters = [
        Filter::bandpass(520., 100.),
        Filter::bandpass(1450., 160.),
        Filter::bandpass(2550., 250.),
    ];
    let weights: [f32; 12] = std::array::from_fn(|i| {
        1.0 / ((i + 1) as f32).powf(1.45 - voice.brightness * 0.48)
    });
    let mut phase = 0.0f32;
    let mut rng = 0x6A92_43F1_u32;
    let total_samples = expected.min(max_samples).max(1);
    let mut previous = resonances(Phone::AH).start;

    for (index, segment) in transcription.segments.iter().enumerate() {
        if samples.len() >= max_samples { break; }
        let phone = segment.phone;
        let length = duration(*segment, voice.speed).min(max_samples - samples.len());
        if phone.is_pause() {
            samples.extend(std::iter::repeat_n(0.0f32, length));
            continue;
        }
        let resonant = resonances(phone);
        let next = transcription.segments[index + 1..].iter()
            .find(|s| !s.phone.is_pause())
            .map(|s| resonances(s.phone).start)
            .unwrap_or(resonant.end);
        let noise_amount = if phone.is_fricative() {
            if phone == Phone::HH { 0.28 } else { 0.85 }
        } else if phone.is_stop() { 0.7 }
        else if phone.is_vowel() { voice.breathiness * 0.26 }
        else { 0.07 + voice.breathiness * 0.08 };
        let voiced = if phone.is_vowel() { 1.0 }
            else if phone.is_nasal() { 0.57 }
            else if phone.is_voiced() && phone.is_fricative() { 0.25 }
            else if phone.is_voiced() { 0.58 }
            else { 0.0 };
        let segment_start = samples.len();
        let length_f = length.max(1) as f32;
        for n in 0..length {
            let progress = n as f32 / length_f;
            let diphthong = lerp(resonant.start, resonant.end,
                ((progress - 0.30) / 0.55).clamp(0.0, 1.0));
            let target = if progress < 0.20 {
                lerp(previous, diphthong, (progress / 0.20).clamp(0.0, 1.0))
            } else if progress > 0.85 {
                lerp(diphthong, next, ((progress - 0.85) / 0.15) * 0.28)
            } else { diphthong };
            if n % 64 == 0 {
                for (i, filter) in filters.iter_mut().enumerate() {
                    filter.retune(target[i] * voice.formant_scale, [100., 155., 255.][i]);
                }
            }
            let t = samples.len() as f32 / SAMPLE_RATE as f32;
            let phrase_progress = samples.len() as f32 / total_samples as f32;
            let phrase_contour = if transcription.is_question && phrase_progress > 0.78 {
                0.98 + (phrase_progress - 0.78) * 0.8
            } else { 1.05 - 0.12 * phrase_progress };
            let stress_pitch = if phone.is_vowel() { 1.0 + 0.05 * segment.stress as f32 } else { 1.0 };
            let vibrato = 1.0 + (TAU * 5.3 * t).sin() * voice.vibrato * 0.018;
            phase = (phase + voice.pitch_hz * vibrato * stress_pitch * phrase_contour
                / SAMPLE_RATE as f32).fract();
            let mut harmonic = 0.0;
            for (i, weight) in weights.iter().enumerate() {
                harmonic += (TAU * phase * (i + 1) as f32).sin() * weight;
            }
            rng ^= rng << 13;
            rng ^= rng >> 17;
            rng ^= rng << 5;
            let noise = (rng as f32 / u32::MAX as f32) * 2.0 - 1.0;

            // Stop phones: closure followed by a short release burst.
            let (closure, burst) = if phone.is_stop() {
                (if progress < 0.50 { 0.0 } else { 1.0 },
                 if progress > 0.50 && progress < 0.66 { 1.0 } else { 0.08 })
            } else { (1.0, 1.0) };
            let excite = harmonic * voiced * closure
                + noise * noise_amount * burst;
            let mut resonated = filters[0].tick(excite) * 1.75
                + filters[1].tick(excite) * (0.70 + voice.brightness * 0.5)
                + filters[2].tick(excite) * (0.34 + voice.brightness * 0.5);
            if phone.is_nasal() {
                resonated = resonated * (0.48 + voice.nasality * 0.1)
                    + harmonic * voiced * 0.045;
            }
            // Sibilant noise is only partially filtered through vocal resonances.
            let airy = if phone.is_fricative() { noise * noise_amount * 0.2 }
                else { 0.0 };
            let soft_start = (samples.len() as f32
                / (SAMPLE_RATE as f32 * 0.012)).min(1.0);
            let soft_end = ((total_samples - samples.len()).max(1) as f32
                / (SAMPLE_RATE as f32 * 0.012)).min(1.0);
            let envelope = soft_start * soft_end;
            let result = ((resonated * 1.8 + airy) * envelope).tanh();
            samples.push(if result.is_finite() { result } else { 0.0 });
        }
        // The last sound's target becomes the next sound's transition source.
        previous = if length > 0 { resonant.end } else { previous };
        debug_assert_eq!(samples.len(), segment_start + length);
    }
    let peak = samples.iter().fold(0.0_f32, |m, s| m.max(s.abs()));
    if peak > 0.80 {
        let scale = 0.80 / peak;
        for sample in &mut samples { *sample *= scale; }
    }
    samples
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phrases_generate_bounded_audio() {
        for phrase in [
            "Hello world", "Can you hear my voice?", "We are making sound.",
            "English speech synthesis", "",
        ] {
            let pcm = render(&Voice::default(), &crate::english::transcribe(phrase));
            assert!(pcm.len() <= SAMPLE_RATE as usize * MAX_SECONDS as usize);
            assert!(pcm.iter().all(|v| v.is_finite() && v.abs() <= 1.0));
            if !phrase.is_empty() { assert!(pcm.iter().any(|v| v.abs() > 0.005)); }
        }
    }

    #[test]
    fn repeated_words_do_not_reset_oscillator() {
        let pcm = render(&Voice::default(), &crate::english::transcribe("voice voice"));
        let midpoint = pcm.len() / 2;
        assert!(pcm[..midpoint].iter().any(|v| v.abs() > 0.005));
        assert!(pcm[midpoint..].iter().any(|v| v.abs() > 0.005));
    }
}
