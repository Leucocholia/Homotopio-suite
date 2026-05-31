#![allow(
    clippy::manual_is_multiple_of,
    clippy::mem_replace_option_with_some,
    clippy::mutable_key_type,
    clippy::unnecessary_map_or,
    clippy::unnecessary_sort_by
)]

pub use common::{Boundary, Direction, Generator, Height, Orientation, SliceIndex};
pub use contraction::Bias;
pub use diagram::{Diagram, Diagram0, DiagramN};
pub use rewrite::{Cospan, Rewrite, Rewrite0, RewriteN};

pub mod antipushout;
pub mod attach;
pub mod bubble;
pub mod check;
pub mod collapse;
pub mod common;
pub mod complex;
pub mod contraction;
pub mod diagram;
pub mod examples;
pub mod expansion;
pub mod factorization;
pub mod layout;
pub mod manifold;
pub mod mesh;
pub mod migration;
pub mod monotone;
pub mod projection;
pub mod rewrite;
pub mod scaffold;
pub mod serialize;
pub mod signature;
pub mod typecheck;

pub fn collect_garbage() {
    DiagramN::collect_garbage();
    RewriteN::collect_garbage();
    rewrite::Cone::collect_garbage();
    common::Label::collect_garbage();
}
