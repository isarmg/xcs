#![doc = include_str!("../../../../docs/upstream/erased-0.1.2/README.md")]

mod erased_box;
mod erased_mut_ref;
mod erased_ref;

pub use erased_box::ErasedBox;
pub use erased_mut_ref::ErasedMut;
pub use erased_ref::Erased;
