# Homotopio DSL v0.1

This document is the v0.1 reference for the source language used by Homotopio
Suite. The language compiles to ordinary homotopy.io proof state: a signature,
an optional workspace diagram, and metadata. Visual edits in the point-and-click
editor remain supported, but v0.1 does not regenerate source from those edits.

The canonical applicative declaration keyword is `property`. The parser also
accepts `schema` as an alias for compatibility with older examples. `struct`
and `macro` remain generative.

## Program Shape

A source file is a sequence of statements. Whitespace is insignificant, and
line comments start with `//`.

```homl compile
title "Basics";
author "Homotopio Suite";
abstract "A minimal source-driven proof state.";

cell A;
cell B;
cell f: A -> B;
show f;
```

## Cells

Use `cell` for generators. A bare cell is a 0-cell. A bounded cell has a source,
a relation marker, and a target.

```homl compile
cell A;
cell B;
cell f: A -> B;
cell g: B -> A;
cell witness: id(A) -> f * g;
show witness;
```

`->` creates directed data. `<->` creates invertible data. Invertibility is a
primitive of the underlying signature mode, and invertible diagrams are closed
under composition.

```homl compile
cell A;
cell B;
cell C;
cell f: A <-> B;
cell g: B <-> C;
show inv(f * g);
```

The built-in expression forms are:

- `name` for a cell or constructed proof.
- `id(expr)` for an identity.
- `inv(expr)` for the inverse of an invertible positive-dimensional diagram.
- `left * right` for composition.
- `contract(expr)` or `contract(expr, lower|higher|same)` for a selected core
  contraction.
- `unique first, second as same;` to materialize field-wise witnesses that two
  aliases name the same canonical property instance.

## Folders

Folders organize generated signature cells. Constructed proofs are source
symbols, not signature axioms, so they do not appear as folder children unless
they are declared as cells.

```homl compile
cell A;
cell B;
cell f: A <-> B;

folder Equivalences {
  construct cancel: f * inv(f) <-> id(A);
  cell witness: id(A) -> f * inv(f);
}

show cancel;
```

## Structs

`struct` packages data and generates fresh cells every time it is used.
Parameters may be cells, written `cell<n>`, or previously declared structures.

```homl compile
cell A;

struct Pointed(X: cell<0>) {
  cell loop: X -> X;
}

use Pointed(A) as first;
use Pointed(A) as second;
show second.loop;
```

`use ... with { ... }` fills directly declared fields from existing cells
instead of generating those fields freely. The provided data must match the
declared boundary and invertibility.

```homl compile
cell A;
cell existing: A -> A;

struct Endomorphism(X: cell<0>) {
  cell map: X -> X;
}

use Endomorphism(A) as endo with {
  map = existing;
}

show endo.map;
```

Structures can be passed as richer parameters.

```homl compile
cell A;
cell e: A -> A;
cell idem: e * e <-> e;

struct Idempotent(X: cell<0>) {
  cell map: X -> X;
  cell square: map * map <-> map;
}

struct Split(I: Idempotent) {
  cell section: I.X -> I.X;
  cell retract: I.X -> I.X;
  cell factor: I.map <-> section * retract;
}

use Idempotent(A) as given with {
  map = e;
  square = idem;
}

use Split(given) as split;
show split.factor;
```

## Properties

`property` is the applicative declaration form. A property instance is
canonical for its declaration and resolved arguments: using the same property
with the same arguments reuses the first instance instead of creating fresh
cells.

```homl compile
cell A;

property Pointed(X: cell<0>) {
  cell loop: X -> X;
}

use Pointed(A) as first;
use Pointed(A) as second;
show second.loop;
```

In this example, `second.loop` resolves to the canonical cell `first.loop`.
The keyword `schema` is accepted as an alias for `property`. Use `unique` when
you want the compiler's canonicality rule to become visible as proof symbols.

```homl compile
cell A;

property Pointed(X: cell<0>) {
  cell loop: X -> X;
}

use Pointed(A) as first;
use Pointed(A) as second;
unique first, second as same;
show same.loop;
```

This creates `same.loop`, the identity witness for the shared canonical
projection. The witness is a DSL symbol, not a new signature axiom.

```homl compile
cell A;

schema HasLoop(X: cell<0>) {
  cell loop: X -> X;
}

use HasLoop(A) as looped;
show looped.loop;
```

## Macros

`macro` is a lightweight generative declaration. It is useful for reusable
source patterns that should create fresh names on every use.

```homl compile
cell A;
cell B;
cell C;

macro Span(A: cell<0>, B: cell<0>) {
  cell left: A -> B;
  cell right: B -> A;
}

use Span(A, B) as first;
use Span(B, C) as second;
show first.left * second.left;
```

Recursive property, struct, and macro expansion is rejected in v0.1.

## Proofs

`prove` and `construct` are aliases. They create DSL symbols by constructing
proof diagrams from existing data and core contraction rules. They do not add
new signature axioms.

```homl compile
cell A;
cell B;
cell f: A <-> B;

construct cancel: f * inv(f) <-> id(A);
construct undo: id(A) -> f * inv(f) {
  attach inv(cancel);
}

show undo;
```

A proof body is a small script. `attach expr;` attaches a positive-dimensional
diagram to the current proof target. `contract;` or `contract lower;` applies a
core contraction to the current proof target.

```homl compile
cell X;
cell alpha: id(X) <-> id(X);
cell beta: id(X) <-> id(X);

construct alpha_beta_to_horizontal: alpha * beta <-> contract(alpha * beta, lower) {
  contract lower;
}

construct beta_alpha_to_horizontal: beta * alpha <-> contract(alpha * beta, lower) {
  contract higher;
}

construct commute: alpha * beta -> beta * alpha {
  attach alpha_beta_to_horizontal;
  attach inv(beta_alpha_to_horizontal);
}

show commute;
```

## Show

`show expr;` selects the workspace diagram after compilation.

```homl compile
cell A;
cell B;
cell f: A -> B;
show id(f);
```

## Actions Escape Hatch

`actions [...]` is experimental in v0.1. It is a low-level escape hatch that
replays the underlying `proof::Action` stream used by the original editor and
the homotopy.io paper specification. Prefer `cell`, `property`, `struct`,
`macro`, and `construct` for stable authoring.

```homl compile
actions [
  "CreateGeneratorZero",
  {"SelectGenerator":{"id":0,"dimension":0}},
  {"SetBoundary":"Source"},
  "CreateGeneratorZero",
  {"SelectGenerator":{"id":1,"dimension":0}},
  {"SetBoundary":"Target"}
]
```

## Diagnostics

Diagnostics use `{ severity, message, span }`, where `span` is a byte range in
the source. v0.1 reports the following stable classes of errors:

- Duplicate names in the declaration or symbol namespace.
- Unknown symbols, declarations, provided fields, or structure arguments.
- Dimension mismatches in cells, proofs, and composition.
- Invalid `with` bindings, including non-name expressions and missing fields.
- Invertibility mismatches when directed data is provided for invertible fields.
- Property uniqueness mismatches, such as using `unique` on structs or on
  different canonical property instances.
- Recursive property, struct, or macro expansion.
- Proof construction failures.
- Invalid experimental `actions [...]` payloads.

```homl error duplicate symbol
cell A;
cell A;
```

```homl error unknown symbol
cell A;
show missing;
```

```homl error dimension mismatch
cell A;
cell B;
cell f: A -> B;
cell bad: A -> f;
```

```homl error invalid with
cell A;
cell existing: A -> A;

struct Idempotent(X: cell<0>) {
  cell map: X -> X;
  cell square: map * map <-> map;
}

use Idempotent(A) as bad with {
  map = existing;
  square = existing;
}
```

```homl error unique on struct
cell A;

struct Pointed(X: cell<0>) {
  cell loop: X -> X;
}

use Pointed(A) as first;
use Pointed(A) as second;
unique first, second as same;
```

```homl error recursive declaration
cell A;
property Loop(X: cell<0>) {
  use Loop(X) as next;
}
use Loop(A) as loop;
```

```homl error actions
actions [{"DefinitelyNotAnAction":true}]
```

## Limits

v0.1 deliberately keeps the kernel symbolic-free. Free variables exist inside
properties, structs, and macros, and instantiation generates or reuses concrete
named cells. Proof search is bounded and intentionally small; use explicit proof
bodies with `attach` and `contract` when automatic construction is not enough.

The language currently has no type inference for richer structures, no hosted
remote library catalog, and no source regeneration from point-and-click edits.
