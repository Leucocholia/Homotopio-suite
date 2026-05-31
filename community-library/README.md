# Homotopio Community Library

This directory is the seed for a small, curated preset library. It is designed
for GitHub pull requests rather than in-app uploads: contributors add one preset
folder, automation validates it, and maintainers review the mathematical and
didactic content.

There are no community submissions yet. The generated catalog is kept empty
until the first reviewed contribution lands.

For full submission instructions, see [Community Library](../COMMUNITY_LIBRARY.md).

## Layout

```text
community-library/
  presets/
    example-id/
      preset.toml
      source.homl
      didactic.md
  generated/
    index.json
```

`generated/index.json` is rebuilt by:

```bash
cargo run -p homotopy-dsl --bin homotopy-library -- community-library
```

CI checks that the generated index is current and that every preset compiles.
