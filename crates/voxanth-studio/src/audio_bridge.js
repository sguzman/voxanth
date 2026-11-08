// Small, auditable browser boundary. All synthesis runs in Rust/WASM.
let context = null;
let source = null;
const storageKey = "voxanth-profiles-v1";

function audioContext() {
  if (!context) context = window.__voxanthAudio || new (window.AudioContext || window.webkitAudioContext)();
  return context;
}

export function playAudio(samples, sampleRate) {
  if (!samples || samples.length === 0) return;
  const ctx = audioContext();
  if (ctx.state === "suspended") ctx.resume().catch(() => {});
  if (source) { try { source.stop(); } catch (_) {} source.disconnect(); }
  const buffer = ctx.createBuffer(1, samples.length, sampleRate);
  buffer.copyToChannel(new Float32Array(samples), 0);
  source = ctx.createBufferSource();
  source.buffer = buffer;
  source.connect(ctx.destination);
  source.start();
}

export function downloadAudio(samples, sampleRate, filename) {
  const bytes = new ArrayBuffer(44 + samples.length * 2);
  const view = new DataView(bytes);
  function str(offset, text) { for (let i = 0; i < text.length; i++) view.setUint8(offset + i, text.charCodeAt(i)); }
  str(0, "RIFF"); view.setUint32(4, bytes.byteLength - 8, true);
  str(8, "WAVE"); str(12, "fmt "); view.setUint32(16, 16, true);
  view.setUint16(20, 1, true); view.setUint16(22, 1, true);
  view.setUint32(24, sampleRate, true); view.setUint32(28, sampleRate * 2, true);
  view.setUint16(32, 2, true); view.setUint16(34, 16, true);
  str(36, "data"); view.setUint32(40, samples.length * 2, true);
  for (let i = 0; i < samples.length; i++) {
    const s = Math.max(-1, Math.min(1, samples[i]));
    view.setInt16(44 + i * 2, Math.round(s * (s < 0 ? 32768 : 32767)), true);
  }
  const url = URL.createObjectURL(new Blob([bytes], { type: "audio/wav" }));
  const a = document.createElement("a"); a.href = url; a.download = filename;
  document.body.appendChild(a); a.click(); a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

export function loadProfiles() {
  try { return localStorage.getItem(storageKey) || "[]"; }
  catch (_) { return "[]"; }
}

export function saveProfiles(json) {
  try { localStorage.setItem(storageKey, json); }
  catch (error) { console.warn("Profile storage unavailable", error); }
}

export function isDarkMode() {
  const preference = document.documentElement.dataset.theme || "system";
  return preference === "dark" ||
    (preference === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches);
}

/** Called from the first actual egui frame, not merely the WASM loader. */
export function markStudioReady() {
  document.documentElement.dataset.voxanthStudio = "ready";
  const loading = document.getElementById("studio-loading");
  if (loading) loading.hidden = true;
  const fallback = document.getElementById("studio-fallback");
  if (fallback) fallback.hidden = true;
}

/** Keep startup failures visible and route to GPU-independent Rust controls. */
export function reportStudioError(message) {
  console.error("[Voxanth studio]", message);
  window.dispatchEvent(new CustomEvent("VoxanthStudioError", {
    detail: { message: String(message) }
  }));
}
