#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preset {
    pub id: &'static str,
    pub title: &'static str,
    pub category: &'static str,
    pub description: &'static str,
    pub lesson: &'static str,
    pub source: &'static str,
    pub tags: &'static [&'static str],
}

pub const PRESETS: &[Preset] = &[
    Preset {
        id: "basics",
        title: "Basics",
        category: "Basics",
        description: "Two objects, a pair of arrows, and an identity witness.",
        lesson: "Start with named cells. A 0-cell is an object, a 1-cell has a source and target, and a 2-cell relates parallel 1-dimensional diagrams.",
        source: r#"title "Basics";
cell A;
cell B;
cell f: A -> B;
cell g: B -> A;
cell loop: id(A) -> f * g;
show loop;
"#,
        tags: &["0-cells", "1-cells", "identity"],
    },
    Preset {
        id: "adjunction",
        title: "Adjunction",
        category: "Adjunction",
        description: "A reusable structure that creates concrete adjunction data between two 0-cells.",
        lesson: "`struct` creates fresh data each time it is used. This is the right mode for an adjunction, since two adjunctions with the same endpoints need not be identical.",
        source: r#"title "Adjunction";
cell A;
cell B;

struct Adjunction(A: cell<0>, B: cell<0>) {
  cell F: A -> B;
  cell G: B -> A;
  cell unit: id(A) -> F * G;
  cell counit: G * F -> id(B);
}

use Adjunction(A, B) as adj;
show adj.unit;
"#,
        tags: &["struct", "free variables", "2-cells"],
    },
    Preset {
        id: "equivalence",
        title: "Equivalence",
        category: "Equivalence",
        description: "A compact equivalence structure with inverse arrows and witnesses.",
        lesson: "Equivalence data is packaged as a structure: forward and backward arrows, plus witnesses that the composites behave like identities.",
        source: r#"title "Equivalence";
cell X;
cell Y;

struct Equivalence(X: cell<0>, Y: cell<0>) {
  cell forward: X -> Y;
  cell backward: Y -> X;
  cell left: id(X) -> forward * backward;
  cell right: backward * forward -> id(Y);
}

use Equivalence(X, Y) as eq;
show eq.left;
"#,
        tags: &["struct", "equivalence", "2-cells"],
    },
    Preset {
        id: "braids",
        title: "Braids",
        category: "Braids",
        description: "Named objects and arrows arranged as a small braid-like pattern.",
        lesson: "This preset keeps the source deliberately direct. It is a good place to compare the DSL's named cells with point-and-click signature editing.",
        source: r#"title "Braids";
cell A;
cell B;
cell C;
cell over: A -> B;
cell under: B -> C;
cell braid: over * under -> over * under;
show braid;
"#,
        tags: &["composition", "2-cells"],
    },
    Preset {
        id: "property-uniqueness",
        title: "Property Uniqueness",
        category: "Properties",
        description: "Materializes the canonicality of repeated property instances as proof symbols.",
        lesson: "`property` is applicative: the same declaration applied to the same resolved arguments reuses one canonical instance. `unique` exposes that compiler fact as field-wise identity witnesses such as `same.loop`.",
        source: r#"title "Property Uniqueness";
abstract "Repeated property instances are canonical. The `unique` constructor turns that canonicality into visible proof symbols.";

cell A;

property Pointed(X: cell<0>) {
  cell loop: X -> X;
}

use Pointed(A) as first;
use Pointed(A) as second;
unique first, second as same;
show same.loop;
"#,
        tags: &["property", "unique", "canonical", "proof"],
    },
    Preset {
        id: "macro-composition",
        title: "Macro Composition",
        category: "Examples",
        description: "A macro alias for a reusable two-arrow shape.",
        lesson: "`macro` is the lightweight generative form. Parenthesized and unparenthesized composition in `show` both compile to the same diagram.",
        source: r#"title "Macro Composition";
cell A;
cell B;
cell C;

macro Span(A: cell<0>, B: cell<0>) {
  cell left: A -> B;
  cell right: B -> A;
  cell witness: id(A) -> left * right;
}

use Span(A, B) as first;
use Span(B, C) as second;
show first.witness;
"#,
        tags: &["macro", "namespace", "composition"],
    },
    Preset {
        id: "idempotent-split",
        title: "Idempotent Split",
        category: "Structures",
        description: "Packages existing idempotent data, then passes it into a split structure.",
        lesson: "`with` fills a structure from existing cells instead of generating those fields freely. The filled structure can then be passed as a richer argument.",
        source: r#"title "Idempotent Split";
abstract "A structure can be filled from existing data with `with`, and another structure can take that packaged structure as an argument.";

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
"#,
        tags: &["struct", "with", "idempotent", "free variables"],
    },
    Preset {
        id: "uniqueness-of-adjunctions",
        title: "Adjunction Comparison Maps",
        category: "Proofs",
        description:
            "Constructs one comparison map between right adjoints and defines the reverse map as its inverse.",
        lesson: "The comparison map is constructed from adjunction data. The reverse direction is not a separate axiom; it is obtained by applying primitive invertibility to the constructed map.",
        source: r#"title "Adjunction Comparison Maps";
abstract "The comparison maps used in uniqueness of adjunctions are built from the units and counits.";

cell A;
cell B;
cell F: A -> B;

struct RightAdjunction(A: cell<0>, B: cell<0>, F: cell<1>) {
  cell right: B -> A;
  cell unit: id(A) -> F * right;
  cell counit: right * F -> id(B);
  prove right_triangle_path: right -> right {
    attach unit;
    attach counit;
  }
  prove left_triangle_path: F -> F {
    attach unit;
    attach counit;
  }
  cell right_triangle: right_triangle_path <-> id(right);
  cell left_triangle: left_triangle_path <-> id(F);
}

use RightAdjunction(A, B, F) as first;
use RightAdjunction(A, B, F) as second;

prove to_second: first.right <-> second.right {
  attach second.unit;
  attach first.counit;
}
construct to_first: second.right -> first.right {
  attach inv(to_second);
}

show to_second;
"#,
        tags: &["proof", "adjunction", "comparison", "3-cells"],
    },
    Preset {
        id: "eckmann-hilton",
        title: "Eckmann-Hilton",
        category: "Proofs",
        description: "Constructs the commutativity proof for two 2-loops using biased contraction.",
        lesson: "The two products are compared by contracting both to the same horizontal composite, then composing one path with the inverse of the other.",
        source: r#"title "Eckmann-Hilton";
abstract "Two 2-loops on the identity 1-cell commute by comparing two contractions into the same horizontal composite. This is the core pi>=2 abelian argument.";

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
"#,
        tags: &["proof", "Eckmann-Hilton", "2-loops", "abelian"],
    },
    Preset {
        id: "paper-action-replay",
        title: "Paper Action Replay",
        category: "Specification",
        description: "An experimental low-level source form that replays the same proof actions as the default editor modes.",
        lesson: "`actions [...]` is an experimental escape hatch for the underlying proof-action stream. It keeps the DSL visibly grounded in the original homotopy.io specification, but ordinary authoring should prefer cells, structures, properties, macros, and constructed proofs.",
        source: r#"actions [
  "CreateGeneratorZero",
  {"SelectGenerator":{"id":0,"dimension":0}},
  {"SetBoundary":"Source"},
  "CreateGeneratorZero",
  {"SelectGenerator":{"id":1,"dimension":0}},
  {"SetBoundary":"Target"}
]
"#,
        tags: &["paper", "actions", "signature"],
    },
];

pub fn default_preset() -> &'static Preset {
    &PRESETS[1]
}

pub fn get(id: &str) -> Option<&'static Preset> {
    PRESETS.iter().find(|preset| preset.id == id)
}
