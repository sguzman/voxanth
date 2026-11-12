//! Graphics-independent Rust/WASM synthesis bridge.
//!
//! This package intentionally does not depend on egui, winit, WebGL or a
//! JavaScript synthesizer. It can load when the visual studio cannot.

use wasm_bindgen::prelude::*;
use voxanth_core::{render_singing, render_speech, render_vowel, Voice, Vowel, SAMPLE_RATE};

/// Synthesize the supported IPA vowels and consonants using Rust DSP.
/// Returns an explicit error for symbols the model cannot produce.
#[wasm_bindgen]
pub fn render_ipa(
    ipa: &str,
    pitch_hz: f32,
    formant_scale: f32,
    brightness: f32,
    breathiness: f32,
    nasality: f32,
    vibrato: f32,
    speed: f32,
) -> Result<Vec<f32>, JsValue> {
    let mut voice = Voice::default();
    voice.pitch_hz = pitch_hz;
    voice.formant_scale = formant_scale;
    voice.brightness = brightness;
    voice.breathiness = breathiness;
    voice.nasality = nasality;
    voice.vibrato = vibrato;
    voice.speed = speed;
    voxanth_core::ipa::render(&voice, ipa)
        .map_err(|message| JsValue::from_str(&message))
}

/// The sample rate for returned mono f32 audio.
#[wasm_bindgen]
pub fn sample_rate() -> u32 {
    SAMPLE_RATE
}

/// Render an actual Rust DSP waveform, independent of the UI canvas.
#[wasm_bindgen]
pub fn render_preview(
    mode: &str,
    vowel: &str,
    text: &str,
    pitch_hz: f32,
    formant_scale: f32,
    brightness: f32,
    breathiness: f32,
    nasality: f32,
    vibrato: f32,
    speed: f32,
) -> Vec<f32> {
    let mut voice = Voice::default();
    voice.pitch_hz = pitch_hz;
    voice.formant_scale = formant_scale;
    voice.brightness = brightness;
    voice.breathiness = breathiness;
    voice.nasality = nasality;
    voice.vibrato = vibrato;
    voice.speed = speed;
    let voice = voice.sanitized();

    let vowel = match vowel.to_ascii_lowercase().as_str() {
        "e" => Vowel::E,
        "i" => Vowel::I,
        "o" => Vowel::O,
        "u" => Vowel::U,
        _ => Vowel::A,
    };

    match mode {
        "speech" => render_speech(&voice, text),
        "singing" => render_singing(&voice, vowel, 1.4),
        _ => render_vowel(&voice, vowel, 0.9),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn voice_synthesis_is_available_without_egui() {
        let pcm = render_preview("speech", "A", "Hello world!",
            150.0, 1.04, 0.65, 0.06, 0.08, 0.0, 1.0);
        assert!(pcm.len() > 1000);
        assert!(pcm.iter().all(|sample| sample.is_finite() && sample.abs() <= 1.0));
        assert!(pcm.iter().any(|sample| sample.abs() > 0.001));
    }
}
