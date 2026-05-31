async function loadOptionalHighs() {
  if (window.HOMOTOPY_ENABLE_HIGHS !== true) return;

  try {
    const highs = await import(new URL("./highs.js", import.meta.url).href);
    await highs.default();
  } catch (error) {
    console.warn("HiGHS solver assets were found but could not be loaded.", error);
  }
}

async function boot() {
  try {
    await import(new URL("./editor.js", import.meta.url).href);
    await loadOptionalHighs();
    const wasm = await import(new URL("./homotopy_web.js", import.meta.url).href);
    await wasm.default();
  } catch (error) {
    console.error(error);
  }
}

boot();
