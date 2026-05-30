# Development

This workspace is a snapshot-based fork of the public `homotopy-io/homotopy-rs`
source.

## Local build

The MVP web build uses the pure-Rust `minilp` solver by default so it can run
without Nix-generated `highs.js`/`highs.wasm` assets.

```powershell
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli
cargo test -p homotopy-dsl
cargo build -p homotopy-web --target wasm32-unknown-unknown
New-Item -ItemType Directory -Force -Path dist
Copy-Item -Path homotopy-web\static\* -Destination dist -Recurse -Force
wasm-bindgen --out-dir dist --no-typescript --target web target\wasm32-unknown-unknown\debug\homotopy_web.wasm
```

Serve `dist/` with COOP/COEP headers. The original `cargo make dist` workflow
also works when `cargo-make` is installed.

## Docker fallback

The local Windows/WSL environment used to create this snapshot initially lacked
`cargo`, `rustc`, `nix`, and `cargo-make`, so this fork includes a simple Docker
fallback:

```bash
docker build -f Dockerfile.dev -t homotopy-editor-dev .
docker run --rm -it -p 8080:8080 -v "$PWD:/workspace" homotopy-editor-dev
```

Inside the container:

```bash
cargo test -p homotopy-dsl
cargo make dist
sfz --render-index --coi dist/
```

The original Nix/devcontainer workflow is still present for contributors who prefer it.

## Optional HiGHS solver

`homotopy-web` keeps an opt-in `highs` feature for builds that also ship the
generated HiGHS web assets:

```bash
cargo build -p homotopy-web --target wasm32-unknown-unknown --features highs
```

When using that feature, provide `dist/highs.js` and `dist/highs.wasm`, and set
`window.HOMOTOPY_ENABLE_HIGHS = true` before `boot.js` runs.
