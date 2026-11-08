#![cfg(target_arch = "wasm32")]
//! Browser voice studio. Acoustic synthesis is implemented in voxanth-core.

use eframe::egui::{self, Color32, RichText, Stroke};
use voxanth_core::{render_singing, render_speech, render_vowel, Voice, Vowel, SAMPLE_RATE};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[wasm_bindgen(module = "/crates/voxanth-studio/src/audio_bridge.js")]
extern "C" {
    #[wasm_bindgen(js_name = playAudio)]
    fn play_audio(samples: &[f32], sample_rate: u32);
    #[wasm_bindgen(js_name = downloadAudio)]
    fn download_audio(samples: &[f32], sample_rate: u32, filename: &str);
    #[wasm_bindgen(js_name = loadProfiles)]
    fn load_profiles() -> String;
    #[wasm_bindgen(js_name = saveProfiles)]
    fn save_profiles(profiles: &str);
    #[wasm_bindgen(js_name = isDarkMode)]
    fn is_dark_mode() -> bool;
}

#[wasm_bindgen(start)]
pub fn start() {
    wasm_bindgen_futures::spawn_local(async {
        let Some(window) = web_sys::window() else { return };
        let Some(document) = window.document() else { return };
        let Some(canvas) = document
            .get_element_by_id("voxanth-canvas")
            .and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok())
        else { return };
        let runner = eframe::WebRunner::new();
        if let Err(error) = runner
            .start(canvas, eframe::WebOptions::default(), Box::new(|_| Ok(Box::new(Studio::new()))))
            .await
        {
            web_sys::console::error_1(&error);
        }
        // The browser owns the frame lifecycle after initialization.
        std::mem::forget(runner);
    });
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode { Formant, Speech, Singing }

impl Mode {
    fn name(self) -> &'static str {
        match self { Self::Formant => "FORMANT", Self::Speech => "SPEECH / EXPERIMENTAL", Self::Singing => "SINGING" }
    }
}

struct Studio {
    voice: Voice,
    mode: Mode,
    vowel: Vowel,
    text: String,
    saved: Vec<Voice>,
    samples: Vec<f32>,
    status: String,
    json_editor: String,
    show_json: bool,
    continuous_preview: bool,
}

impl Studio {
    fn new() -> Self {
        let saved = serde_json::from_str::<Vec<Voice>>(&load_profiles())
            .unwrap_or_default()
            .into_iter()
            .take(30)
            .map(|v| v.sanitized())
            .collect();
        Self {
            voice: Voice::default(), mode: Mode::Formant, vowel: Vowel::A,
            text: "Hello world".into(), saved, samples: Vec::new(),
            status: "Choose a sound, then select Render & play.".into(),
            json_editor: String::new(), show_json: false, continuous_preview: false,
        }
    }

    fn render(&mut self, duration: f32) {
        self.voice = self.voice.sanitized();
        self.samples = match self.mode {
            Mode::Formant => render_vowel(&self.voice, self.vowel, duration),
            Mode::Speech => render_speech(&self.voice, &self.text),
            Mode::Singing => render_singing(&self.voice, self.vowel, duration),
        };
        if self.samples.is_empty() {
            self.status = "Nothing to render. Enter a phrase or choose a vowel.".into();
        } else {
            play_audio(&self.samples, SAMPLE_RATE);
            self.status = format!(
                "Generated {:.2} seconds · {} Hz · local synthesis",
                self.samples.len() as f32 / SAMPLE_RATE as f32,
                SAMPLE_RATE
            );
        }
    }

    fn persist(&self) {
        if let Ok(json) = serde_json::to_string(&self.saved) { save_profiles(&json); }
    }

    fn waveform(&self, ui: &mut egui::Ui) {
        let width = ui.available_width().max(170.0);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 116.0), egui::Sense::hover());
        let painter = ui.painter_at(rect);
        let bg = if is_dark_mode() { Color32::from_rgb(18, 29, 33) } else { Color32::from_rgb(234, 241, 241) };
        let fg = Color32::from_rgb(34, 169, 156);
        painter.rect_filled(rect, 8.0, bg);
        painter.line_segment(
            [egui::pos2(rect.left() + 8.0, rect.center().y), egui::pos2(rect.right() - 8.0, rect.center().y)],
            Stroke::new(1.0, fg.gamma_multiply(0.24))
        );
        if self.samples.is_empty() {
            painter.text(rect.center(), egui::Align2::CENTER_CENTER, "Waveform appears after rendering",
                egui::FontId::proportional(13.0), fg.gamma_multiply(0.7));
            return;
        }
        let bins = (width as usize).min(420).max(32);
        for i in 0..bins {
            let first = i * self.samples.len() / bins;
            let end = ((i + 1) * self.samples.len() / bins).max(first + 1).min(self.samples.len());
            let peak = self.samples[first..end].iter().fold(0.0_f32, |a, b| a.max(b.abs()));
            let x = rect.left() + 6.0 + (i as f32 / bins as f32) * (width - 12.0);
            let height = peak * 47.0;
            painter.line_segment(
                [egui::pos2(x, rect.center().y - height), egui::pos2(x, rect.center().y + height)],
                Stroke::new(1.0, fg)
            );
        }
    }

    fn profile_section(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("VOICE PROFILES").strong().color(Color32::from_rgb(34, 169, 156)));
        ui.add_space(5.0);
        ui.horizontal_wrapped(|ui| {
            for preset in Voice::presets() {
                let chosen = self.voice.name == preset.name;
                if ui.selectable_label(chosen, &preset.name).clicked() {
                    self.voice = preset;
                    self.status = "Preset loaded. Render to audition.".into();
                }
            }
        });
        if !self.saved.is_empty() {
            ui.add_space(4.0);
            ui.label(RichText::new("Your locally saved voices").small());
            ui.horizontal_wrapped(|ui| {
                for preset in &self.saved {
                    if ui.button(&preset.name).clicked() { self.voice = preset.clone(); }
                }
            });
        }
        ui.add_space(7.0);
        ui.horizontal(|ui| {
            ui.label("Name:");
            ui.text_edit_singleline(&mut self.voice.name);
            if ui.button("Save profile").clicked() {
                let clean = self.voice.sanitized();
                if let Some(existing) = self.saved.iter_mut().find(|v| v.name == clean.name) {
                    *existing = clean;
                } else if self.saved.len() < 30 {
                    self.saved.push(clean);
                } else {
                    self.status = "Local profile limit reached (30).".into();
                    return;
                }
                self.persist();
                self.status = "Voice saved to this browser.".into();
            }
        });
        ui.horizontal(|ui| {
            if ui.button("Profile JSON").clicked() {
                self.show_json = !self.show_json;
                self.json_editor = serde_json::to_string_pretty(&self.voice).unwrap_or_default();
            }
            if ui.button("Delete saved matching name").clicked() {
                let before = self.saved.len();
                self.saved.retain(|v| v.name != self.voice.name);
                self.persist();
                self.status = if before == self.saved.len() { "No saved voice with this name.".into() }
                    else { "Removed locally saved profile.".into() };
            }
        });
        if self.show_json {
            ui.label(RichText::new("Edit, copy, or paste a voice definition").small());
            ui.add(egui::TextEdit::multiline(&mut self.json_editor).desired_rows(7).code_editor());
            ui.horizontal(|ui| {
                if ui.button("Copy JSON").clicked() {
                    ui.ctx().copy_text(self.json_editor.clone());
                }
                if ui.button("Load JSON").clicked() {
                    match serde_json::from_str::<Voice>(&self.json_editor) {
                        Ok(p) => { self.voice = p.sanitized(); self.status = "Profile imported.".into(); }
                        Err(e) => self.status = format!("Invalid profile: {e}"),
                    }
                }
            });
        }
    }

    fn controls(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("ACOUSTIC PARAMETERS").strong().color(Color32::from_rgb(34, 169, 156)));
        let mut changed = false;
        changed |= ui.add(egui::Slider::new(&mut self.voice.pitch_hz, 65.0..=450.0).text("Pitch (Hz)")).changed();
        changed |= ui.add(egui::Slider::new(&mut self.voice.formant_scale, 0.65..=1.6).text("Formant scale")).changed();
        changed |= ui.add(egui::Slider::new(&mut self.voice.brightness, 0.0..=1.0).text("Brightness")).changed();
        changed |= ui.add(egui::Slider::new(&mut self.voice.breathiness, 0.0..=1.0).text("Breathiness")).changed();
        changed |= ui.add(egui::Slider::new(&mut self.voice.nasality, 0.0..=1.0).text("Nasality")).changed();
        changed |= ui.add(egui::Slider::new(&mut self.voice.vibrato, 0.0..=1.0).text("Vibrato")).changed();
        changed |= ui.add(egui::Slider::new(&mut self.voice.speed, 0.5..=1.6).text("Speech speed")).changed();
        ui.checkbox(&mut self.continuous_preview, "Preview while adjusting sliders");
        if changed && self.continuous_preview && self.mode != Mode::Speech {
            self.render(0.24);
        }
        ui.label(RichText::new("Formants model resonances, not literal vocal-tract geometry.").small().weak());
    }
}

impl eframe::App for Studio {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let visuals = if is_dark_mode() { egui::Visuals::dark() } else { egui::Visuals::light() };
        ui.ctx().set_visuals(visuals);
        egui::ScrollArea::vertical().id_salt("voxanth-studio-scroll").show(ui, |ui| {
            ui.add_space(12.0);
            ui.heading(RichText::new("Design a voice. Hear every decision.").size(24.0));
            ui.label(RichText::new("On-device Rust DSP · no accounts · no pretrained voice model").weak());
            ui.add_space(15.0);
            ui.horizontal_wrapped(|ui| {
                for candidate in [Mode::Formant, Mode::Speech, Mode::Singing] {
                    ui.selectable_value(&mut self.mode, candidate, candidate.name());
                }
            });
            ui.add_space(10.0);
            match self.mode {
                Mode::Speech => {
                    ui.label("Text-to-sound / phonetic demonstrator");
                    ui.add(egui::TextEdit::multiline(&mut self.text)
                        .desired_rows(2).hint_text("Try: Hello world / voice / hello"));
                    ui.label(RichText::new("Experimental English text-to-sound rules. Not yet reliable intelligible speech, especially for consonants.").small().color(Color32::from_rgb(197, 137, 74)));
                }
                Mode::Formant | Mode::Singing => {
                    ui.label("Select a sustained vowel");
                    ui.horizontal_wrapped(|ui| {
                        for vowel in Vowel::ALL {
                            ui.selectable_value(&mut self.vowel, vowel, vowel.label());
                        }
                    });
                }
            }
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button(RichText::new("▶  Render & play").strong().size(16.0)).clicked() {
                    self.render(if self.mode == Mode::Singing { 1.6 } else { 0.85 });
                }
                if ui.add_enabled(!self.samples.is_empty(), egui::Button::new("↓  Export WAV")).clicked() {
                    download_audio(&self.samples, SAMPLE_RATE, "voxanth-sample.wav");
                }
            });
            ui.add_space(5.0);
            ui.label(RichText::new(&self.status).small().weak());
            ui.add_space(10.0);
            self.waveform(ui);
            ui.add_space(14.0);
            ui.separator();
            ui.add_space(8.0);
            self.profile_section(ui);
            ui.add_space(15.0);
            ui.separator();
            ui.add_space(8.0);
            self.controls(ui);
            ui.add_space(20.0);
        });
    }
}
