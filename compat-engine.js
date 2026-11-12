// Loads the independent Rust audio WASM; deliberately has no egui or WebGL imports.
// Errors are displayed by fallback.js instead of leaving the controls inert.
(async () => {
  try {
    const module = await import("./engine/voxanth_web_audio.js");
    await module.default(new URL("./engine/voxanth_web_audio_bg.wasm", import.meta.url));
    window.voxanthAudioRender = module.render_preview;
    window.voxanthIpaRender = module.render_ipa;
    window.voxanthAudioRate = module.sample_rate();
    if (new URLSearchParams(location.search).has("voxanth-smoke")) {
      try {
        const samples = module.render_preview(
          "vowel", "A", "", 150, 1.0, 0.65, 0.06, 0.08, 0, 1
        );
        document.documentElement.dataset.voxanthSmoke =
          samples.length > 3000 && samples.some(n => Math.abs(n) > 0.001)
            ? "passed" : "failed";
      } catch (error) {
        document.documentElement.dataset.voxanthSmoke = "failed";
        console.error("[Voxanth] Audio self-test failed:", error);
      }
    }
    window.dispatchEvent(new CustomEvent("VoxanthAudioReady"));
  } catch (error) {
    const message = String(error && (error.stack || error.message || error));
    console.error("[Voxanth] Independent Rust audio module failed:", error);
    window.dispatchEvent(new CustomEvent("VoxanthAudioError", {
      detail: { message }
    }));
  }
})();
