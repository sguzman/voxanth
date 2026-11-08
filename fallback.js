// Accessible DOM fallback for browsers where egui cannot initialize WebGL2.
// It never synthesizes sound in JavaScript: audio samples come from Voxanth Rust/WASM.
(() => {
  "use strict";
  const $ = id => document.getElementById(id);
  const root = document.documentElement;
  const fallback = $("studio-fallback");
  const status = $("compat-status");
  const fieldIds = {
    pitch_hz: "compat-pitch", formant_scale: "compat-formant",
    brightness: "compat-brightness", breathiness: "compat-breath",
    nasality: "compat-nasal", vibrato: "compat-vibrato", speed: "compat-speed"
  };
  const outputs = {
    pitch_hz: "compat-pitch-value", formant_scale: "compat-formant-value",
    brightness: "compat-brightness-value", breathiness: "compat-breath-value",
    nasality: "compat-nasal-value", vibrato: "compat-vibrato-value",
    speed: "compat-speed-value"
  };
  const storageKey = "voxanth-profiles-v1";
  const stockProfiles = [
    { name: "Clarity / speech", pitch_hz: 150, formant_scale: 1.04, breathiness: .06, brightness: .65, nasality: .08, vibrato: 0, speed: 1 },
    { name: "Ember / warm", pitch_hz: 112, formant_scale: .83, breathiness: .11, brightness: .34, nasality: .12, vibrato: .08, speed: .9 },
    { name: "Glass / bright", pitch_hz: 230, formant_scale: 1.28, breathiness: .18, brightness: .9, nasality: .03, vibrato: .07, speed: 1.1 },
    { name: "Aether / synthetic", pitch_hz: 195, formant_scale: .94, breathiness: .3, brightness: .72, nasality: .35, vibrato: .42, speed: .95 }
  ];
  let source = null;
  let rendered = null;

  function currentProfile() {
    const profile = { name: $("compat-name").value.trim() || "Untitled voice" };
    for (const [key, id] of Object.entries(fieldIds)) {
      profile[key] = Number($(id).value);
    }
    return profile;
  }

  function updateOutputs() {
    for (const [key, id] of Object.entries(outputs)) {
      const value = Number($(fieldIds[key]).value);
      $(id).textContent = key === "pitch_hz"
        ? Math.round(value) + " Hz" : value.toFixed(2);
    }
  }

  function savedProfiles() {
    try {
      const value = JSON.parse(localStorage.getItem(storageKey) || "[]");
      return Array.isArray(value)
        ? value.filter(p => p && typeof p.name === "string").slice(0, 30)
        : [];
    } catch (_) { return []; }
  }

  function refreshProfiles() {
    const select = $("compat-profile");
    select.replaceChildren();
    const placeholder = document.createElement("option");
    placeholder.value = "";
    placeholder.textContent = "Choose preset or saved voice";
    select.append(placeholder);
    for (const p of [...stockProfiles, ...savedProfiles()]) {
      const option = document.createElement("option");
      option.value = JSON.stringify(p);
      option.textContent = p.name;
      select.append(option);
    }
  }

  function applyProfile(profile) {
    $("compat-name").value = profile.name || "Untitled voice";
    for (const [key, id] of Object.entries(fieldIds)) {
      if (typeof profile[key] === "number" && Number.isFinite(profile[key])) {
        $(id).value = profile[key];
      }
    }
    updateOutputs();
  }

  function showFallback(message) {
    if (root.dataset.voxanthStudio === "ready") return;
    root.dataset.voxanthStudio = "compatibility";
    fallback.hidden = false;
    $("voxanth-canvas").hidden = true;
    $("studio-loading").hidden = true;
    $("compat-reason").textContent = message ||
      "The graphics interface could not start. These controls use the same Rust voice synthesis engine.";
    status.textContent = typeof window.wasmBindings?.render_preview === "function"
      ? "Rust synthesizer ready. Select a mode and press Render & play."
      : "The Rust module is not available. Check WebAssembly support or reload the page.";
  }

  function renderSamples() {
    const render = window.wasmBindings?.render_preview;
    if (typeof render !== "function") {
      throw new Error("The Rust/WebAssembly synthesizer has not loaded.");
    }
    const p = currentProfile();
    const samples = render(
      $("compat-mode").value, $("compat-vowel").value, $("compat-text").value,
      p.pitch_hz, p.formant_scale, p.brightness, p.breathiness,
      p.nasality, p.vibrato, p.speed
    );
    if (!samples || samples.length === 0) {
      throw new Error("No audio was generated. Enter some text or choose a vowel.");
    }
    return samples;
  }

  function playSamples(samples) {
    const Context = window.AudioContext || window.webkitAudioContext;
    if (!Context) throw new Error("Web Audio is unavailable.");
    const context = window.__voxanthAudio ||= new Context();
    if (context.state === "suspended") context.resume();
    if (source) { try { source.stop(); } catch (_) {} source.disconnect(); }
    const buffer = context.createBuffer(1, samples.length, 44100);
    buffer.copyToChannel(new Float32Array(samples), 0);
    source = context.createBufferSource();
    source.buffer = buffer;
    source.connect(context.destination);
    source.start();
  }

  function wavBlob(samples) {
    const bytes = new ArrayBuffer(44 + samples.length * 2);
    const view = new DataView(bytes);
    function str(offset, value) {
      for (let i = 0; i < value.length; i++) {
        view.setUint8(offset + i, value.charCodeAt(i));
      }
    }
    str(0, "RIFF"); view.setUint32(4, bytes.byteLength - 8, true);
    str(8, "WAVE"); str(12, "fmt "); view.setUint32(16, 16, true);
    view.setUint16(20, 1, true); view.setUint16(22, 1, true);
    view.setUint32(24, 44100, true); view.setUint32(28, 88200, true);
    view.setUint16(32, 2, true); view.setUint16(34, 16, true);
    str(36, "data"); view.setUint32(40, samples.length * 2, true);
    for (let i = 0; i < samples.length; i++) {
      const s = Math.max(-1, Math.min(1, samples[i]));
      view.setInt16(44 + i * 2, Math.round(s * (s < 0 ? 32768 : 32767)), true);
    }
    return new Blob([bytes], { type: "audio/wav" });
  }

  $("compat-profile").addEventListener("change", event => {
    if (event.target.value) applyProfile(JSON.parse(event.target.value));
  });
  for (const id of Object.values(fieldIds)) {
    $(id).addEventListener("input", updateOutputs);
  }
  $("compat-play").addEventListener("click", () => {
    try {
      rendered = renderSamples();
      playSamples(rendered);
      $("compat-export").disabled = false;
      status.textContent = "Generated " + (rendered.length / 44100).toFixed(2)
        + " seconds using the Rust engine.";
    } catch (error) {
      status.textContent = "Audio error: " + error.message;
    }
  });
  $("compat-export").addEventListener("click", () => {
    if (!rendered) return;
    const url = URL.createObjectURL(wavBlob(rendered));
    const link = document.createElement("a");
    link.href = url; link.download = "voxanth-voice.wav";
    document.body.append(link);
    link.click();
    link.remove();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  });
  $("compat-save").addEventListener("click", () => {
    try {
      const profiles = savedProfiles();
      const profile = currentProfile();
      const existing = profiles.findIndex(p => p.name === profile.name);
      if (existing >= 0) profiles[existing] = profile;
      else if (profiles.length < 30) profiles.push(profile);
      else throw new Error("The 30-profile storage limit has been reached.");
      localStorage.setItem(storageKey, JSON.stringify(profiles));
      refreshProfiles();
      status.textContent = "Saved '" + profile.name + "' in this browser.";
    } catch (error) {
      status.textContent = "Save failed: " + error.message;
    }
  });

  refreshProfiles();
  updateOutputs();
  window.addEventListener("VoxanthStudioError", event => {
    showFallback(event.detail?.message || "The graphics studio could not initialize.");
  });

  window.addEventListener("TrunkApplicationStarted", () => {
    // Query-triggered smoke test used by CI; normal visitors do not run it.
    if (new URLSearchParams(location.search).has("voxanth-smoke")) {
      try {
        const samples = window.wasmBindings.render_preview(
          "vowel", "A", "", 150, 1, .65, .06, .08, 0, 1
        );
        root.dataset.voxanthSmoke = samples.length > 3000 &&
          samples.some(value => Math.abs(value) > .001)
            ? "passed" : "failed";
      } catch (error) {
        root.dataset.voxanthSmoke = "failed";
        console.error("Rust engine smoke test:", error);
      }
    }
    if (root.dataset.voxanthStudio === "compatibility") {
      showFallback($("compat-reason").textContent);
    }
    setTimeout(() => {
      if (root.dataset.voxanthStudio !== "ready") {
        showFallback("The graphics interface did not start. The Rust synthesis engine is available in compatibility mode.");
      }
    }, 4500);
  });

  window.addEventListener("error", event => {
    if (root.dataset.voxanthStudio !== "ready") {
      showFallback("JavaScript startup error: " + (event.message || "A required script could not load."));
    }
  });
  window.addEventListener("unhandledrejection", event => {
    if (root.dataset.voxanthStudio !== "ready") {
      showFallback("WebAssembly startup error: " + String(event.reason));
    }
  });

  // Never leave visitors looking at a blank dark canvas indefinitely.
  setTimeout(() => {
    if (!root.dataset.voxanthStudio) {
      showFallback("The WebAssembly studio did not finish loading. Check browser support or reload.");
    }
  }, 11000);
})();
