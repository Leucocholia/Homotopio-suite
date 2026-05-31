# Homotopio Suite

Homotopio Suite is an independent experiment based on a snapshot of the public
[`homotopy-io/homotopy-rs`](https://github.com/homotopy-io/homotopy-rs) source.
It is not an official homotopy.io project, is not maintained by the homotopy.io
team, and should not be read as representing `beta.homotopy.io`.

The goal of this fork is to explore a more source-driven and didactic diagram
editor for larger, reusable homotopy.io-style diagrams.

## What This Adds

- A readable DSL for declaring cells, generative `struct`s, applicative
  `schema`/`property` declarations, instantiations, and the diagram to show.
- A browser source editor powered by CodeMirror 6, with diagnostics and syntax
  highlighting.
- Hygienic expansion for reusable diagram shapes. `struct` creates fresh
  packaged data, while `schema`/`property` gives canonical instances for the
  same arguments.
- A built-in reference library with presets for basics, adjunctions,
  equivalences, braids, idempotent structures, proofs, and macro composition.
- Per-preset didactic notes in the Library drawer, with an empty curated
  community preset lane ready for future reviewed examples.
- A `.hio` project bundle format containing `manifest.json`, `proof.hom`,
  `source.homl`, and optional didactic metadata.
- A local-first browser app: no Firebase account, publishing, or remote project
  sync surface is included in this fork.

## Relationship To homotopy.io

The original homotopy.io proof assistant lets users construct composite
morphisms in finitely generated semistrict n-categories through a point-and-click
interface. It renders composites as 2D and 3D geometries, supports 4D
visualisation as movies of 3D geometries, and can export 2D diagrams to
LaTeX/TikZ and SVG.

This repository preserves the upstream BSD-3-Clause source license and keeps
the upstream citation information below. The changes here are independent fork
work layered on top of a source snapshot, not upstream history or an official
deployment.

For background on homotopy.io itself, see the
[arXiv paper](https://arxiv.org/abs/2402.13179), the
[nLab page](https://ncatlab.org/nlab/show/homotopy.io), and the included
[tutorial](./TUTORIAL.md).

## Development

See [DEVELOPMENT.md](./DEVELOPMENT.md) for local, Docker, and optional HiGHS
solver build notes.

See [COMMUNITY_LIBRARY.md](./COMMUNITY_LIBRARY.md) for public preset submission
instructions.

Common local checks:

```bash
cargo test -p homotopy-dsl
cargo run -p homotopy-dsl --bin homotopy-library -- community-library --check
cargo check -p homotopy-web --target wasm32-unknown-unknown
cargo check -p homotopy-web --tests --target wasm32-unknown-unknown
```

To build the browser app locally:

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli
cargo build -p homotopy-web --target wasm32-unknown-unknown
mkdir -p dist
cp -r homotopy-web/static/* dist/
wasm-bindgen --out-dir dist --no-typescript --target web \
  target/wasm32-unknown-unknown/debug/homotopy_web.wasm
```

Serve `dist/` with COOP/COEP headers. The original Nix, devcontainer, and
`cargo make` workflows from upstream are still present for contributors who want
to use them.

## Citing The Upstream Project

The upstream tool should be cited as follows:

```bibtex
@article{hio,
  title={homotopy.io: a proof assistant for finitely-presented globular $n$-categories},
  author={Corbyn, Nathan and Heidemann, Lukas and Hu, Nick and Sarti, Chiara and Tataru, Calin and Vicary, Jamie},
  journal={arXiv preprint arXiv:2402.13179},
  year={2024}
}
```

## License

Unless explicitly stated otherwise, source code in this repository is licensed
under the [BSD 3-Clause License](LICENSE), inherited from the upstream
homotopy.io source snapshot.

The upstream documentation is licensed under a
[Creative Commons Attribution 4.0 International License](http://creativecommons.org/licenses/by/4.0/).

## Dependencies

The upstream project uses the HiGHS linear programming solver for the layout
algorithm:

Parallelizing the dual revised simplex method, Q. Huangfu and J. A. J. Hall,
Mathematical Programming Computation, 10 (1), 119-142, 2018.
DOI: 10.1007/s12532-017-0130-5

The project also uses the [keyboard-css](https://github.com/shhdharmen/keyboard-css)
library.
