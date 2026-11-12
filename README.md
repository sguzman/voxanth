# Voxanth

**A laboratory for synthetic voice.** Explore voices by shaping acoustic parameters instead of training large models. Voxanth is an open-source, local-first experiment in vocal source-filter synthesis, voice profile design, and intelligible synthetic speech.

## First prototype

- Pure Rust formant/source-filter synthesizer with five vowels and voice timbre controls
- English-first text-to-speech experiment with a small inspectable pronunciation lexicon, explicit stress, and a labeled fallback for unsupported words
- Continuous phoneme rendering with English vowel/diphthong targets, consonant closures, simple formant transitions, and an elementary question pitch contour
- Singing tones with vibrato, several adjustable voice profiles, local saved profiles
- Rust egui studio compiled to WebAssembly, inside a responsive public website
- Browser audio playback and waveform; no account, remote inference, telemetry, or npm
- High-contrast light/dark studio and a separate, keyboard-accessible IPA soundboard
- **All 28 basic IPA monophthong symbols** with distinct approximate formant targets; 24 modeled English pulmonic consonants and affricates, with unsupported symbols explicitly marked
- Compose IPA sequences such as `/hɛloʊ/`, `/kæt/`, or `/iː/`, play them with the standalone Rust/WASM audio engine, and export WAV (not a phonetic reference recording)
- A functional, accessible HTML control surface backed by the same Rust/WASM engine when egui/WebGL2 cannot initialize
- Explicit loading errors and a GitHub Actions browser smoke test that disables WebGL and verifies actual Rust audio output

**Status:** experimental. **English is the primary language target.** The speech mode is *not* a general-purpose, validated TTS engine. Its pronunciation dictionary is intentionally limited, and unknown words use an explicitly labeled approximate fallback; the output is best understood as a baseline for testing intelligibility. This implementation is formant synthesis, not a detailed branching-tube anatomical simulation.

## Repository

- `crates/voxanth-core`: portable synthesis code, including `english.rs` (English orthography), `speech_dsp.rs` (phoneme rendering), and `ipa.rs` (28 IPA vowels plus 24 consonants), with tests
- `Cargo.toml` + `crates/voxanth-studio/src`: root egui/WASM application, browser audio bridge and presets
- `index.html` + `styles.css` + `ipa-widget.js`: accessible IPA chart, audio workbench, and embedded egui studio
- `.github/workflows/pages.yml`: tests, WASM build, Pages publication

## Run

Install Rust stable, `wasm32-unknown-unknown`, and [Trunk](https://trunkrs.dev/). No npm is used.

```sh
rustup target add wasm32-unknown-unknown
cargo install --locked trunk
cargo test -p voxanth-core
trunk serve
```

The studio lives on the homepage. Click **Enable audio** and then **Render & play**; browsers require an interaction to enable sound. If WebGL2 is blocked or egui cannot initialize, the page exposes compatibility controls using the *same* Rust synthesizer, rather than leaving the user on a blank canvas.

## Approach

Voice parameters are serialized locally. The first English lexicon and phone renderer are implemented, but English pronunciation coverage, consonant realism, coarticulation, prosody, and intelligibility benchmarks remain core development goals. Other languages are out of scope for the initial intelligible-speech milestone. We intend to progressively replace simple orthographic parsing with a proper English phonemizer and blind transcription tests. Audio is generated on-device.

## License

MIT. See [LICENSE](LICENSE).

## IPA accuracy and limitations

The IPA soundboard references the [official IPA chart](https://www.internationalphoneticassociation.org/content/chart). Its 28 basic vowel targets are **approximate acoustic formants**; their actual sounds depend on pitch, speaker anatomy, and language. They are not calibrated reference recordings. The first 24 consonant/affricate sounds are also approximations from a simplified source-filter model, and isolated consonants are auditioned with /ə/ afterward for audibility. Unmodeled symbols are visibly disabled or produce errors in the IPA input; there is no silent substitution. The implementation does not yet model all IPA consonants, clicks, ejectives, tone, diacritics, or allophonic detail.
