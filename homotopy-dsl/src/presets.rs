#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preset {
    pub id: &'static str,
    pub title: &'static str,
    pub category: &'static str,
    pub description: &'static str,
    pub source: &'static str,
    pub tags: &'static [&'static str],
}

pub const PRESETS: &[Preset] = &[
    Preset {
        id: "basics",
        title: "Basics",
        category: "Basics",
        description: "Two objects, a pair of arrows, and an identity witness.",
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
        description: "A reusable schema that creates a concrete adjunction between two 0-cells.",
        source: r#"title "Adjunction";
cell A;
cell B;

schema Adjunction(A: cell<0>, B: cell<0>) {
  cell F: A -> B;
  cell G: B -> A;
  cell unit: id(A) -> F * G;
  cell counit: G * F -> id(B);
}

use Adjunction(A, B) as adj;
show adj.unit;
"#,
        tags: &["schema", "free variables", "2-cells"],
    },
    Preset {
        id: "equivalence",
        title: "Equivalence",
        category: "Equivalence",
        description: "A compact equivalence schema with inverse arrows and witnesses.",
        source: r#"title "Equivalence";
cell X;
cell Y;

schema Equivalence(X: cell<0>, Y: cell<0>) {
  cell forward: X -> Y;
  cell backward: Y -> X;
  cell left: id(X) -> forward * backward;
  cell right: backward * forward -> id(Y);
}

use Equivalence(X, Y) as eq;
show eq.left;
"#,
        tags: &["schema", "equivalence", "2-cells"],
    },
    Preset {
        id: "braids",
        title: "Braids",
        category: "Braids",
        description: "Named objects and arrows arranged as a small braid-like pattern.",
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
        id: "macro-composition",
        title: "Macro Composition",
        category: "Examples",
        description: "A macro alias for a reusable two-arrow shape.",
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
];

pub fn default_preset() -> &'static Preset {
    &PRESETS[1]
}

pub fn get(id: &str) -> Option<&'static Preset> {
    PRESETS.iter().find(|preset| preset.id == id)
}
