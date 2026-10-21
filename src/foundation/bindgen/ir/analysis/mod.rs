//! Fix-point analyses on the IR using the "monotone framework".
//!
//! A lattice is a set with a partial ordering between elements, where there is
//! a single least upper bound and a single greatest least bound for every
//! subset. We are dealing with finite lattices, which means that it has a
//! finite number of elements, and it follows that there exists a single top and
//! a single bottom member of the lattice. For example, the power set of a
//! finite set forms a finite lattice where partial ordering is defined by set
//! inclusion, that is `a <= b` if `a` is a subset of `b`. Here is the finite
//! lattice constructed from the set {0,1,2}:
//!
//! ```text
//!                    .----- Top = {0,1,2} -----.
//!                   /            |              \
//!                  /             |               \
//!                 /              |                \
//!              {0,1} -------.  {0,2}  .--------- {1,2}
//!                |           \ /   \ /             |
//!                |            /     \              |
//!                |           / \   / \             |
//!               {0} --------'   {1}   `---------- {2}
//!                 \              |                /
//!                  \             |               /
//!                   \            |              /
//!                    `------ Bottom = {} ------'
//! ```
//!
//! A monotone function `f` is a function where if `x <= y`, then it holds that
//! `f(x) <= f(y)`. It should be clear that running a monotone function to a
//! fix-point on a finite lattice will always terminate: `f` can only "move"
//! along the lattice in a single direction, and therefore can only either find
//! a fix-point in the middle of the lattice or continue to the top or bottom
//! depending if it is ascending or descending the lattice respectively.
//!
//! For a deeper introduction to the general form of this kind of analysis, see
//! [Static Program Analysis by Anders Møller and Michael I. Schwartzbach][spa].
//!
//! [spa]: https://cs.au.dk/~amoeller/spa/spa.pdf

// Re-export individual analyses.
mod template_params;
pub(crate) use self::template_params::UsedTemplateParameters;
mod derive;
pub use self::derive::DeriveTrait;
pub(crate) use self::derive::{as_cannot_derive_set, CannotDerive};
mod has_vtable;
pub(crate) use self::has_vtable::{
    HasVtable, HasVtableAnalysis, HasVtableResult,
};
mod has_destructor;
pub(crate) use self::has_destructor::HasDestructorAnalysis;
mod has_type_param_in_array;
pub(crate) use self::has_type_param_in_array::HasTypeParameterInArray;
mod has_float;
pub(crate) use self::has_float::HasFloat;
mod sizedness;
pub(crate) use self::sizedness::{
    Sizedness, SizednessAnalysis, SizednessResult,
};

use crate::ir::context::{BindgenContext, ItemId};

use crate::ir::traversal::{EdgeKind, Trace};
use crate::HashMap;
use std::fmt;
use std::ops;

/// An analysis in the monotone framework.
///
/// Implementors of this trait must maintain the following two invariants:
///
/// 1. The concrete data must be a member of a finite-height lattice.
/// 2. The concrete `constrain` method must be monotone: that is,
///    if `x <= y`, then `constrain(x) <= constrain(y)`.
///
/// If these invariants do not hold, iteration to a fix-point might never
/// complete.
///
/// For a simple example analysis, see the `ReachableFrom` type in the `tests`
/// module below.
pub(crate) trait MonotoneFramework: Sized + fmt::Debug {
    /// The type of node in our dependency graph.
    ///
    /// This is just generic (and not `ItemId`) so that we can easily unit test
    /// without constructing real `Item`s and their `ItemId`s.
    type Node: Copy;

    /// Any extra data that is needed during computation.
    ///
    /// Again, this is just generic (and not `&BindgenContext`) so that we can
    /// easily unit test without constructing real `BindgenContext`s full of
    /// real `Item`s and real `ItemId`s.
    type Extra: Sized;

    /// The final output of this analysis. Once we have reached a fix-point, we
    /// convert `self` into this type, and return it as the final result of the
    /// analysis.
    type Output: From<Self> + fmt::Debug;

    /// Construct a new instance of this analysis.
    fn new(extra: Self::Extra) -> Self;

    /// Get the initial set of nodes from which to start the analysis. Unless
    /// you are sure of some domain-specific knowledge, this should be the
    /// complete set of nodes.
    fn initial_worklist(&self) -> Vec<Self::Node>;

    /// Update the analysis for the given node.
    ///
    /// If this results in changing our internal state (ie, we discovered that
    /// we have not reached a fix-point and iteration should continue), return
    /// `ConstrainResult::Changed`. Otherwise, return `ConstrainResult::Same`.
    /// When `constrain` returns `ConstrainResult::Same` for all nodes in the
    /// set, we have reached a fix-point and the analysis is complete.
    fn constrain(&mut self, node: Self::Node) -> ConstrainResult;

    /// For each node `d` that depends on the given `node`'s current answer when
    /// running `constrain(d)`, call `f(d)`. This informs us which new nodes to
    /// queue up in the worklist when `constrain(node)` reports updated
    /// information.
    fn each_depending_on<F>(&self, node: Self::Node, f: F)
    where
        F: FnMut(Self::Node);
}

/// Whether an analysis's `constrain` function modified the incremental results
/// or not.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub(crate) enum ConstrainResult {
    /// The incremental results were updated, and the fix-point computation
    /// should continue.
    Changed,

    /// The incremental results were not updated.
    #[default]
    Same,
}

impl ops::BitOr for ConstrainResult {
    type Output = Self;

    fn bitor(self, rhs: ConstrainResult) -> Self::Output {
        if self == ConstrainResult::Changed || rhs == ConstrainResult::Changed {
            ConstrainResult::Changed
        } else {
            ConstrainResult::Same
        }
    }
}

impl ops::BitOrAssign for ConstrainResult {
    fn bitor_assign(&mut self, rhs: ConstrainResult) {
        *self = *self | rhs;
    }
}

/// Run an analysis in the monotone framework.
pub(crate) fn analyze<Analysis>(extra: Analysis::Extra) -> Analysis::Output
where
    Analysis: MonotoneFramework,
{
    let mut analysis = Analysis::new(extra);
    let mut worklist = analysis.initial_worklist();

    while let Some(node) = worklist.pop() {
        if let ConstrainResult::Changed = analysis.constrain(node) {
            analysis.each_depending_on(node, |needs_work| {
                worklist.push(needs_work);
            });
        }
    }

    analysis.into()
}

/// Generate the dependency map for analysis
pub(crate) fn generate_dependencies<F>(
    ctx: &BindgenContext,
    consider_edge: F,
) -> HashMap<ItemId, Vec<ItemId>>
where
    F: Fn(EdgeKind) -> bool,
{
    let mut dependencies = HashMap::default();

    for &item in ctx.allowlisted_items() {
        dependencies.entry(item).or_insert_with(Vec::new);

        {
            // We reverse our natural IR graph edges to find dependencies
            // between nodes.
            item.trace(
                ctx,
                &mut |sub_item: ItemId, edge_kind| {
                    if ctx.allowlisted_items().contains(&sub_item) &&
                        consider_edge(edge_kind)
                    {
                        dependencies
                            .entry(sub_item)
                            .or_insert_with(Vec::new)
                            .push(item);
                    }
                },
                &(),
            );
        }
    }
    dependencies
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/bindgen/unit/ir/analysis/mod/tests_211.rs"));
