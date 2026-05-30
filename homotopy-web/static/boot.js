async function loadOptionalHighs() {
  if (window.HOMOTOPY_ENABLE_HIGHS !== true) return;

  try {
    const highs = await import("/highs.js");
    await highs.default();
  } catch (error) {
    console.warn("HiGHS solver assets were found but could not be loaded.", error);
  }
}

async function boot() {
  try {
    await import("/editor.js");
    await loadOptionalHighs();
    const wasm = await import("/homotopy_web.js");
    await wasm.default();
  } catch (error) {
    console.error(error);
  }
}

boot();
