# Voxanth

**A laboratory for synthetic voice.** Explore voices by shaping acoustic parameters instead of training large models. Voxanth is an open-source, local-first experiment in vocal source-filter synthesis, voice profile design, and intelligible synthetic speech.

## First prototype

- Pure Rust formant/source-filter synthesizer with five vowels and voice timbre controls
- English-first text-to-speech experiment with a small inspectable pronunciation lexicon, explicit stress, and a labeled fallback for unsupported words
- Continuous phoneme rendering with English vowel/diphthong targets, consonant closures, simple formant transitions, and an elementary question pitch contour
- Singing tones with vibrato, several adjustable voice profiles, local saved profiles
- Rust egui studio compiled to WebAssembly, inside a responsive public website
- Browser audio playback and waveform; no account, remote inference, telemetry, or npm

**Status:** experimental. **English is the primary language target.** The speech mode is *not* a general-purpose, validated TTS engine. Its pronunciation dictionary is intentionally limited, and unknown words use an explicitly labeled approximate fallback; the output is best understood as a baseline for testing intelligibility. This implementation is formant synthesis, not a detailed branching-tube anatomical simulation.

## Repository

- `crates/voxanth-core`: portable synthesis code, including `english.rs` (pronunciation) and `speech_dsp.rs` (experimental phoneme rendering), with tests
- `Cargo.toml` + `crates/voxanth-studio/src`: root egui/WASM application, browser audio bridge and presets
- `index.html` + `styles.css`: public presentation and embedded studio
- `.github/workflows/pages.yml`: tests, WASM build, Pages publication

## Run

Install Rust stable, `wasm32-unknown-unknown`, and [Trunk](https://trunkrs.dev/). No npm is used.

```sh
rustup target add wasm32-unknown-unknown
cargo install --locked trunk
cargo test -p voxanth-core
trunk serve
```

The studio lives in the homepage. Click **Play** to generate audio; browsers require an interaction to enable sound.

## Approach

Voice parameters are serialized locally. The first English lexicon and phone renderer are implemented, but English pronunciation coverage, consonant realism, coarticulation, prosody, and intelligibility benchmarks remain core development goals. Other languages are out of scope for the initial intelligible-speech milestone. We intend to progressively replace simple orthographic parsing with a proper English phonemizer and blind transcription tests. Audio is generated on-device.

## License

MIT. See [LICENSE](LICENSE).
