# Community Library

Homotopio Suite keeps a small, curated community preset lane for future pull
requests. There are no third-party submissions yet; the generated catalog may
therefore be empty.

The project is independent from homotopy.io. Community presets here are not
official homotopy.io examples.

## What To Submit

Add one folder under `community-library/presets/`:

```text
community-library/presets/my-example/
  preset.toml
  source.homl
  didactic.md
  proof.hom optional
```

Use a lowercase, hyphenated id for the folder name. The `id` field in
`preset.toml` must match the folder name.

## Metadata

`preset.toml` must include:

```toml
id = "my-example"
title = "My Example"
category = "Proofs"
description = "A short one-sentence description."
author = "Your Name"
license = "CC-BY-4.0"
min_app_version = "0.1.2"
tags = ["proof", "example"]
axioms = ["A", "B", "f"]
constructed = ["main_result"]
source = "source.homl"
didactic = "didactic.md"
```

The `axioms` list is the exact list of cells that may appear in the compiled
signature. The `constructed` list is the exact list of non-axiom DSL symbols
produced by `prove` or `construct`. This keeps submissions mathematically
honest: a proof should not quietly become a new axiom.

Use `struct` for freely generated packaged data, such as an adjunction or
idempotent structure. Use `schema` or its alias `property` when repeated uses
with the same arguments should refer to the same canonical instance. A `use`
may include a `with { field = existing_symbol; }` block to fill part of a
structure from pre-existing data instead of generating that field freely.

If you include `proof.hom`, add:

```toml
proof = "proof.hom"
```

The validator checks that the `.hom` file can be imported. The source remains
the authoritative object for review.

## Local Validation

Run this before opening a pull request:

```bash
cargo run -p homotopy-dsl --bin homotopy-library -- community-library
cargo run -p homotopy-dsl --bin homotopy-library -- community-library --check
```

The first command validates every preset and regenerates
`community-library/generated/index.json`. The second command is the CI check: it
fails if the generated index is stale.

The validator checks:

- `preset.toml` parses and has required fields.
- The preset id is a lowercase slug and matches its folder name.
- `source.homl` parses and compiles through `homotopy-dsl`.
- Signature and workspace diagrams pass `diagram.check(true)`.
- The compiled proof round-trips through existing `.hom` serialization.
- Declared `axioms` exactly match compiled signature cells.
- Declared `constructed` symbols exactly match non-axiom DSL symbols.
- Optional `proof.hom` files deserialize as legacy proof exports.

## Review Guidelines

A good preset should:

- Teach one clear idea.
- Keep the source readable.
- Prefer `construct`/`prove` over adding proof conclusions as `cell` axioms.
- State any intentional axioms in `preset.toml`.
- Include didactic text for nontrivial examples.
- Use a permissive content license, preferably `CC-BY-4.0`.

The maintainer may ask for source cleanup, a clearer explanation, or a smaller
example before merging.
