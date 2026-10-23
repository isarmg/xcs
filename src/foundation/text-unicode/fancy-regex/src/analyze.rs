// Copyright 2016 The Fancy Regex Authors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

//! Analysis of regex expressions.

use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::cmp::{max, min};

use bit_set::BitSet;

use crate::alloc::string::ToString;
use crate::parse::ExprTree;
use crate::vm::CaptureGroupRange;
use crate::{Assertion, AstNode, CaptureGroupTarget, CompileError, Error, Expr, Result};

#[cfg(not(feature = "std"))]
use alloc::collections::BTreeMap as Map;
#[cfg(feature = "std")]
use std::collections::HashMap as Map;

/// Analysis information for a regex expression.
///
/// This structure contains the results of analyzing a regex expression,
/// including size information, whether it requires backtracking features,
/// and references to child expressions in the tree.
#[derive(Debug)]
pub struct Info<'a> {
    pub(crate) capture_groups: CaptureGroupRange,
    /// The minimum number of characters this expression will match
    pub min_size: usize,
    /// The maximum number of characters this expression can match
    pub max_size: usize,
    /// Whether this expression always matches the same number of characters
    pub const_size: bool,
    /// Tracks the minimum number of characters that would be consumed in the innermost capture group
    /// before this expression is matched.
    pub(crate) min_pos_in_group: usize,
    /// Whether this expression requires backtracking features (lookaround, backrefs, etc.)
    pub hard: bool,
    /// The expression being analyzed
    pub expr: &'a Expr,
    /// Analysis information for child expressions
    pub children: Vec<Info<'a>>,
}

impl<'a> Info<'a> {
    /// Returns the start (first) group number for this expression.
    pub fn start_group(&self) -> usize {
        self.capture_groups.start()
    }

    /// Returns the end (last) group number for this expression.
    pub fn end_group(&self) -> usize {
        self.capture_groups.end()
    }

    pub(crate) fn push_literal(&self, buf: &mut String) {
        match *self.expr {
            // could be more paranoid about checking casei
            Expr::Literal { ref val, .. } => buf.push_str(val),
            Expr::Concat(_) => {
                for child in &self.children {
                    child.push_literal(buf);
                }
            }
            _ => panic!("push_literal called on non-literal"),
        }
    }

    /// `None` if this is not a literal; `Some(casei)` if it is, where `casei`
    /// says whether any character is case-insensitive.
    pub(crate) fn is_literal_get_casei(&self) -> Option<bool> {
        match *self.expr {
            Expr::Literal { casei, .. } => Some(casei),
            Expr::Concat(_) => self.children.iter().try_fold(false, |any, child| {
                child.is_literal_get_casei().map(|c| any || c)
            }),
            _ => None,
        }
    }

    /// Pushes each character of the literal along with its case-insensitivity.
    pub(crate) fn push_literal_chars(&self, buf: &mut Vec<(char, bool)>) {
        match *self.expr {
            Expr::Literal { ref val, casei } => buf.extend(val.chars().map(|c| (c, casei))),
            Expr::Concat(_) => {
                for child in &self.children {
                    child.push_literal_chars(buf);
                }
            }
            _ => panic!("push_literal_chars called on non-literal"),
        }
    }
}

struct SizeInfo {
    min_size: usize,
    const_size: bool,
    max_size: usize,
}

/// Represents a subroutine call and its minimum position within a group
#[derive(Debug, Clone)]
struct SubroutineCallInfo {
    /// The group being called
    target_group: usize,
    /// The minimum number of characters consumed in the haystack by the capture group in which this call occurs
    min_pos: usize,
}

struct Analyzer<'a> {
    backrefs: &'a BitSet,
    /// The next group number to assign when a new capture group is encountered.
    /// Starts at 0 or 1 depending on whether explicit capture group 0 is enabled,
    /// and increments for each capture group found during traversal.
    next_group_number: usize,
    /// Stores the analysis info for each group by group number
    // NOTE: uses a Map instead of a Vec because sometimes we start from capture group 1, other times 0
    group_info: Map<usize, SizeInfo>,
    /// Tracks subroutine calls: maps from a group to the subroutines it calls
    subroutine_calls: Map<usize, Vec<SubroutineCallInfo>>,
    /// Groups that are directly executed from root (not inside {0})
    root_groups: BitSet,
    /// Pre-populated map of capture group index to the inner expression of that group.
    /// Only pre-populated/used when there are subroutine calls, to enable correct resolution
    /// of forward references, to detect whether they are const-size etc.
    group_exprs: Map<usize, &'a Expr>,
    /// Track groups currently being analyzed to prevent infinite recursion
    analyzing_groups: BitSet,
    /// When true, nodes that could produce empty matches (min size 0 and not const size)
    /// are promoted to hard so the VM can backtrack past them to find a non-empty match.
    find_not_empty: bool,
    /// When true, nodes that could produce empty matches (min size 0 and not const size)
    /// are promoted to hard so the VM can ensure the whole match at EOF isn't empty.
    disallow_empty_match_at_eof_after_newline: bool,
    /// When true, assertions with runtime input suppression overrides are promoted to hard
    /// so they run on the VM.
    allow_input_assertion_overrides: bool,
    /// When true, leftmost-longest match semantics are enabled.
    #[cfg(feature = "leftmost_longest")]
    leftmost_longest: bool,
    /// Oniguruma's `^` rejects the empty match at the absolute end of the haystack after a trailing
    /// newline when it anchors the match itself. When used as a sub-assertion inside a lookaround,
    /// it behaves as a plain line start. So we need to track when we are analyzing inside a lookaround.
    in_lookaround: bool,
}

impl<'a> Analyzer<'a> {
    // enclosing_group: The group that currently contains the expression being analyzed.
    // This is used to track which group "owns" any subroutine calls found,
    // and to determine if we're at the root level (group 0).
    fn visit(
        &mut self,
        expr: &'a Expr,
        min_pos_in_group: usize,
        inside_zero_rep: bool,
        enclosing_group: usize,
    ) -> Result<Info<'a>> {
        let start_group = self.next_group_number;
        let mut children = match *expr {
            Expr::Concat(ref v) | Expr::Alt(ref v) => Vec::with_capacity(v.len()),
            _ => Vec::new(),
        };
        let mut min_size: usize = 0;
        let mut const_size = false;
        let mut max_size: usize = 0;
        let mut hard = false;
        match *expr {
            Expr::Assertion(assertion) if assertion.is_always_hard() => {
                const_size = true;
                hard = true;
            }
            Expr::Assertion(Assertion::StartLineOniguruma { .. }) if !self.in_lookaround => {
                const_size = true;
                hard = true;
            }
            Expr::Assertion(Assertion::StartText | Assertion::EndText)
                if self.allow_input_assertion_overrides =>
            {
                const_size = true;
                hard = true;
            }
            Expr::Assertion(Assertion::EndText)
                if self.disallow_empty_match_at_eof_after_newline =>
            {
                const_size = true;
                hard = true; // NOTE: \Z is already considered hard and covered in the branch above
            }
            Expr::Assertion(Assertion::EndText) if self.find_not_empty && !self.in_lookaround => {
                const_size = true;
                hard = true;
            }
            Expr::Empty | Expr::Assertion(_) => {
                const_size = true;
            }
            Expr::Any { .. } => {
                min_size = 1;
                const_size = true;
                max_size = 1;
            }
            Expr::GeneralNewline { .. } => {
                // \R matches either \r\n (2 chars) or single newline chars (1 char)
                // So it has min_size = 1, but is not const_size
                min_size = 1;
                const_size = false;
                hard = true; // requires backtracking to handle \r\n without backtracking to \r
                max_size = 2;
            }
            Expr::Literal { ref val, casei } => {
                // right now each character in a literal gets its own node, that might change
                min_size = 1;
                const_size = literal_const_size(val, casei);
                max_size = 1;
            }
            Expr::Concat(ref v) => {
                const_size = true;
                let mut pos_in_group = min_pos_in_group;
                for child in v {
                    let child_info =
                        self.visit(child, pos_in_group, inside_zero_rep, enclosing_group)?;
                    min_size = min_size.saturating_add(child_info.min_size);
                    max_size = if max_size == usize::MAX || child_info.max_size == usize::MAX {
                        usize::MAX
                    } else {
                        max_size.saturating_add(child_info.max_size)
                    };
                    const_size &= child_info.const_size;
                    hard |= child_info.hard;
                    pos_in_group = pos_in_group.saturating_add(child_info.min_size);
                    children.push(child_info);
                }
            }
            Expr::Alt(ref v) => {
                let child_info =
                    self.visit(&v[0], min_pos_in_group, inside_zero_rep, enclosing_group)?;
                min_size = child_info.min_size;
                max_size = child_info.max_size;
                const_size = child_info.const_size;
                hard = child_info.hard;
                children.push(child_info);
                for child in &v[1..] {
                    let child_info =
                        self.visit(child, min_pos_in_group, inside_zero_rep, enclosing_group)?;
                    const_size &= child_info.const_size && min_size == child_info.min_size;
                    min_size = min(min_size, child_info.min_size);
                    max_size = max(max_size, child_info.max_size);
                    hard |= child_info.hard;
                    children.push(child_info);
                }
            }
            Expr::Group(ref child) => {
                let group = self.next_group_number;
                self.next_group_number += 1;
                self.analyzing_groups.insert(group);

                // Track if this group is executed from root (not inside {0})
                if enclosing_group == 0 && !inside_zero_rep {
                    self.root_groups.insert(group);
                }

                let child_info = self.visit(child, 0, inside_zero_rep, group)?;
                self.analyzing_groups.remove(group);
                min_size = child_info.min_size;
                max_size = child_info.max_size;
                const_size = child_info.const_size;
                // Store the group info for use by backrefs
                self.group_info.insert(
                    group,
                    SizeInfo {
                        min_size,
                        const_size,
                        max_size,
                    },
                );
                // If there's a backref to this group, we potentially have to backtrack within the
                // group. E.g. with `(x|xy)\1` and input `xyxy`, `x` matches but then the backref
                // doesn't, so we have to backtrack and try `xy`.
                hard = child_info.hard | self.backrefs.contains(group);
                children.push(child_info);
            }
            Expr::LookAround(ref child, _) => {
                let was_in_lookaround = self.in_lookaround;
                self.in_lookaround = true;
                // NOTE: min_pos_in_group might seem weird for lookbehinds
                let child_info =
                    self.visit(child, min_pos_in_group, inside_zero_rep, enclosing_group)?;
                self.in_lookaround = was_in_lookaround;
                // min_size = 0
                const_size = true;
                hard = true;
                children.push(child_info);
            }
            Expr::Repeat {
                ref child,
                lo,
                hi,
                #[cfg(feature = "leftmost_longest")]
                greedy,
                ..
            } => {
                #[cfg(feature = "leftmost_longest")]
                if !greedy && self.leftmost_longest {
                    return Err(Error::CompileError(Box::new(
                        CompileError::FeatureNotYetSupported(
                            "non-greedy quantifiers are not supported in leftmost-longest mode"
                                .to_string(),
                        ),
                    )));
                }
                // If lo and hi are both 0, we're in a zero-repetition (unreachable)
                let child_inside_zero_rep = if lo == 0 && hi == 0 {
                    true
                } else {
                    inside_zero_rep
                };
                let child_info = self.visit(
                    child,
                    min_pos_in_group,
                    child_inside_zero_rep,
                    enclosing_group,
                )?;
                min_size = child_info.min_size * lo;
                max_size = if hi == usize::MAX || child_info.max_size == usize::MAX {
                    usize::MAX
                } else {
                    child_info.max_size.saturating_mul(hi)
                };
                const_size = child_info.const_size && lo == hi;
                hard = child_info.hard;
                children.push(child_info);
            }
            Expr::Delegate { .. } => {
                // Delegate expressions always match exactly 1 character.
                // This constraint ensures consistency in the AST representation.
                min_size = 1;
                const_size = true;
                max_size = 1;
            }
            Expr::Backref { group, .. } => {
                if group == 0 {
                    return Err(Error::CompileError(Box::new(CompileError::InvalidBackref(
                        group,
                    ))));
                }

                // Check if this backref is inside a recursed group that it refers to
                // This happens when:
                // 1. The backref refers to a group we're currently analyzing
                // 2. That group has a subroutine call to itself
                if self.analyzing_groups.contains(group) {
                    // Check if the group has a recursive subroutine call to itself
                    if let Some(calls) = self.subroutine_calls.get(&group) {
                        if calls.iter().any(|call| call.target_group == group) {
                            return Err(Error::CompileError(Box::new(
                                CompileError::FeatureNotYetSupported(
                                    "Backreference to a capture group from within the same group when it's being recursed".to_string()
                                )
                            )));
                        }
                    }
                }

                // Look up the referenced group's size information
                if let Some(&SizeInfo {
                    min_size: group_min_size,
                    max_size: group_max_size,
                    const_size: group_const_size,
                    ..
                }) = self.group_info.get(&group)
                {
                    min_size = group_min_size;
                    max_size = group_max_size;
                    const_size = group_const_size;
                }
                hard = true;
            }
            Expr::AtomicGroup(ref child) => {
                let child_info =
                    self.visit(child, min_pos_in_group, inside_zero_rep, enclosing_group)?;
                min_size = child_info.min_size;
                max_size = child_info.max_size;
                const_size = child_info.const_size;
                hard = true; // TODO: possibly could weaken
                children.push(child_info);
            }
            Expr::KeepOut => {
                hard = true;
                const_size = true;
            }
            Expr::ContinueFromPreviousMatchEnd => {
                hard = true;
                const_size = true;
            }
            Expr::BackrefExistsCondition { .. } => {
                hard = true;
                const_size = true;
            }
            Expr::BacktrackingControlVerb(_) => {
                hard = true;
                const_size = true;
            }
            Expr::Conditional {
                ref condition,
                ref true_branch,
                ref false_branch,
            } => {
                hard = true;

                let child_info_condition = self.visit(
                    condition,
                    min_pos_in_group,
                    inside_zero_rep,
                    enclosing_group,
                )?;
                let child_info_truth = self.visit(
                    true_branch,
                    min_pos_in_group + child_info_condition.min_size,
                    inside_zero_rep,
                    enclosing_group,
                )?;
                let child_info_false = self.visit(
                    false_branch,
                    min_pos_in_group,
                    inside_zero_rep,
                    enclosing_group,
                )?;

                min_size = child_info_condition.min_size
                    + min(child_info_truth.min_size, child_info_false.min_size);
                max_size = max(child_info_truth.max_size, child_info_false.max_size);
                const_size = child_info_condition.const_size
                    && child_info_truth.const_size
                    && child_info_false.const_size
                    // if the condition's size plus the truth branch's size is equal to the false branch's size then it's const size
                    && child_info_condition.min_size + child_info_truth.min_size == child_info_false.min_size;

                children.push(child_info_condition);
                children.push(child_info_truth);
                children.push(child_info_false);
            }
            Expr::SubroutineCall(target_group) => {
                // Track this subroutine call
                // Only skip tracking if we're in unreachable code at the root level
                // Calls inside groups should always be tracked, even if the group is inside {0} at root,
                // because the group can be called as a subroutine from elsewhere
                if !inside_zero_rep || enclosing_group != 0 {
                    self.subroutine_calls
                        .entry(enclosing_group)
                        .or_default()
                        .push(SubroutineCallInfo {
                            target_group,
                            min_pos: min_pos_in_group,
                        });
                }

                // Look up the target group's min_size if available (similar to backrefs)
                // This is important for accurate left recursion detection
                if let Some(&SizeInfo {
                    min_size: group_min_size,
                    const_size: group_const_size,
                    max_size: group_max_size,
                }) = self.group_info.get(&target_group)
                {
                    min_size = group_min_size;
                    max_size = group_max_size;
                    const_size = group_const_size;
                } else if self.analyzing_groups.contains(target_group) {
                    // Currently analyzing this group - circular reference
                    // Use conservative defaults to avoid infinite recursion
                    min_size = 0;
                    max_size = usize::MAX;
                    const_size = false;
                } else if let Some(&group_expr) = self.group_exprs.get(&target_group) {
                    // If the group hasn't been seen yet (forward reference),
                    // directly analyze it now
                    self.analyzing_groups.insert(target_group);

                    // Save and restore next_group_number around the recursive call
                    let prev_next_group_number = self.next_group_number;
                    self.next_group_number = target_group + 1;
                    let group_info = self.visit(group_expr, 0, inside_zero_rep, target_group)?;
                    self.next_group_number = prev_next_group_number;

                    self.analyzing_groups.remove(target_group);

                    min_size = group_info.min_size;
                    max_size = group_info.max_size;
                    const_size = group_info.const_size;
                    // Store the analysis result for future lookups
                    self.group_info.insert(
                        target_group,
                        SizeInfo {
                            min_size,
                            const_size,
                            max_size,
                        },
                    );
                } else {
                    // Group doesn't exist - this shouldn't happen as the parser would have caught it
                    min_size = 0;
                    max_size = usize::MAX;
                    const_size = false;
                }
                hard = true;
            }
            Expr::AstNode(ref astnode, ix) => {
                // An unresolved AstNode means the resolver couldn't find the target (e.g. a
                // subroutine call to an unknown name). Match each SubroutineCall variant to
                // produce an accurate error message; any other AstNode variant is an internal
                // error and gets a distinct UnresolvedAstNode error.
                match astnode {
                    AstNode::SubroutineCall(CaptureGroupTarget::ByName(name)) => {
                        return Err(Error::CompileError(Box::new(
                            CompileError::SubroutineCallTargetNotFound(
                                format!("named group '{}'", name),
                                ix,
                            ),
                        )));
                    }
                    AstNode::SubroutineCall(CaptureGroupTarget::ByNumber(n)) => {
                        return Err(Error::CompileError(Box::new(
                            CompileError::SubroutineCallTargetNotFound(
                                format!("group number {}", n),
                                ix,
                            ),
                        )));
                    }
                    AstNode::SubroutineCall(CaptureGroupTarget::Relative(n)) => {
                        return Err(Error::CompileError(Box::new(
                            CompileError::SubroutineCallTargetNotFound(
                                format!("relative group {}{}", if *n >= 0 { "+" } else { "" }, n),
                                ix,
                            ),
                        )));
                    }
                    AstNode::AstGroup { .. }
                    | AstNode::Backref { .. }
                    | AstNode::BackrefExistsCondition { .. } => {
                        return Err(Error::CompileError(Box::new(
                            CompileError::UnresolvedAstNode(ix, format!("{:?}", astnode)),
                        )));
                    }
                }
            }
            Expr::BackrefWithRelativeRecursionLevel { .. } => {
                return Err(Error::CompileError(Box::new(
                    CompileError::FeatureNotYetSupported("Backref at recursion level".to_string()),
                )));
            }
            Expr::Absent(ref absent) => {
                use crate::Absent::*;
                match absent {
                    Repeater(ref child) => {
                        let child_info =
                            self.visit(child, min_pos_in_group, inside_zero_rep, enclosing_group)?;
                        min_size = 0;
                        max_size = usize::MAX;
                        const_size = false;
                        hard = true;
                        children.push(child_info);
                    }
                    Expression {
                        ref absent,
                        ref exp,
                    } => {
                        let absent_info =
                            self.visit(absent, min_pos_in_group, inside_zero_rep, enclosing_group)?;
                        let exp_info =
                            self.visit(exp, min_pos_in_group, inside_zero_rep, enclosing_group)?;
                        min_size = exp_info.min_size;
                        max_size = exp_info.max_size;
                        const_size = false;
                        hard = true;
                        children.push(absent_info);
                        children.push(exp_info);
                    }
                    Stopper(ref child) => {
                        let child_info =
                            self.visit(child, min_pos_in_group, inside_zero_rep, enclosing_group)?;
                        // Absent stopper doesn't consume any characters itself
                        min_size = 0;
                        max_size = 0;
                        const_size = true;
                        hard = true;
                        children.push(child_info);
                    }
                    Clear => {
                        // Range clear doesn't consume any characters
                        min_size = 0;
                        max_size = 0;
                        const_size = true;
                        hard = true;
                    }
                }
            }
            Expr::DefineGroup { ref definitions } => {
                // DEFINE groups don't match anything themselves, but we still need to
                // visit the definitions to assign group numbers and analyze any nested content.
                // The DEFINE block itself is not hard - it matches nothing, so it can be
                // delegated (as an empty string) to the underlying engine.
                let def_info = self.visit(definitions, 0, inside_zero_rep, enclosing_group)?;
                min_size = 0;
                max_size = 0;
                const_size = true;
                children.push(def_info);
            }
        };

        // When find_not_empty is active, any node that could produce an empty match
        // must be handled by the VM so it can backtrack past it to find a non-empty match.
        if self.find_not_empty && min_size == 0 && !const_size {
            hard = true;
        }
        #[cfg(feature = "leftmost_longest")]
        if self.leftmost_longest && !const_size {
            hard = true;
        }

        Ok(Info {
            expr,
            children,
            capture_groups: CaptureGroupRange(start_group, self.next_group_number),
            min_size,
            max_size,
            const_size,
            hard,
            min_pos_in_group,
        })
    }

    /// Check for left-recursive subroutine calls using depth-first search
    fn check_left_recursion(&self, named_groups: &Map<String, usize>) -> Result<()> {
        // Compute which groups are reachable from the root (group 0)
        let reachable_groups = self.compute_reachable_groups();

        // Check each reachable group for left recursion
        for &start_group in self.subroutine_calls.keys() {
            if !reachable_groups.contains(start_group) {
                // Skip unreachable groups
                continue;
            }

            let mut visited = BitSet::new();
            let mut recursion_stack = BitSet::new();
            if self.dfs_check_left_recursion(start_group, &mut visited, &mut recursion_stack)? {
                // Found left recursion
                // Build reverse mapping from group number to group name (if any)
                // so we can give friendly error messages
                let mut group_names: Map<usize, String> = Map::new();
                for (name, &group_num) in named_groups.iter() {
                    group_names.insert(group_num, name.clone());
                }

                let group_desc = if let Some(name) = group_names.get(&start_group) {
                    format!("group '{}' ({})", name, start_group)
                } else {
                    format!("group {}", start_group)
                };
                return Err(Error::CompileError(Box::new(
                    CompileError::LeftRecursiveSubroutineCall(group_desc),
                )));
            }
        }
        Ok(())
    }

    /// Check for recursive subroutine calls that can never terminate
    fn check_unbounded_recursion(&self, root_expr: &'a Expr) -> Result<()> {
        let mut memo = Map::new();
        if self.group_can_terminate(0, root_expr, &mut BitSet::new(), &mut memo) {
            return Ok(());
        }

        Err(Error::CompileError(Box::new(
            CompileError::NeverEndingRecursion,
        )))
    }

    fn group_can_terminate(
        &self,
        group: usize,
        root_expr: &'a Expr,
        recursion_stack: &mut BitSet,
        memo: &mut Map<usize, bool>,
    ) -> bool {
        if let Some(&can_terminate) = memo.get(&group) {
            return can_terminate;
        }

        if recursion_stack.contains(group) {
            return false;
        }

        let expr = if group == 0 {
            root_expr
        } else if let Some(&group_expr) = self.group_exprs.get(&group) {
            group_expr
        } else {
            return true;
        };

        recursion_stack.insert(group);
        let can_terminate = self.expr_can_terminate(expr, root_expr, recursion_stack, memo);
        recursion_stack.remove(group);
        memo.insert(group, can_terminate);
        can_terminate
    }

    fn expr_can_terminate(
        &self,
        expr: &'a Expr,
        root_expr: &'a Expr,
        recursion_stack: &mut BitSet,
        memo: &mut Map<usize, bool>,
    ) -> bool {
        match expr {
            Expr::Concat(children) => children
                .iter()
                .all(|child| self.expr_can_terminate(child, root_expr, recursion_stack, memo)),
            Expr::Alt(children) => children
                .iter()
                .any(|child| self.expr_can_terminate(child, root_expr, recursion_stack, memo)),
            Expr::Group(child) => self.expr_can_terminate(child, root_expr, recursion_stack, memo),
            Expr::LookAround(child, _) => {
                self.expr_can_terminate(child, root_expr, recursion_stack, memo)
            }
            Expr::AtomicGroup(child) => {
                self.expr_can_terminate(child, root_expr, recursion_stack, memo)
            }
            Expr::Repeat { child, lo, .. } => {
                *lo == 0 || self.expr_can_terminate(child, root_expr, recursion_stack, memo)
            }
            Expr::Conditional {
                condition,
                true_branch,
                false_branch,
            } => {
                self.expr_can_terminate(false_branch, root_expr, recursion_stack, memo)
                    || (self.expr_can_terminate(condition, root_expr, recursion_stack, memo)
                        && self.expr_can_terminate(true_branch, root_expr, recursion_stack, memo))
            }
            Expr::SubroutineCall(target_group) => {
                self.group_can_terminate(*target_group, root_expr, recursion_stack, memo)
            }
            Expr::Absent(absent) => {
                use crate::Absent::*;
                match absent {
                    Repeater(child) | Stopper(child) => {
                        self.expr_can_terminate(child, root_expr, recursion_stack, memo)
                    }
                    Expression { absent, exp } => {
                        self.expr_can_terminate(absent, root_expr, recursion_stack, memo)
                            && self.expr_can_terminate(exp, root_expr, recursion_stack, memo)
                    }
                    Clear => true,
                }
            }
            Expr::BackrefWithRelativeRecursionLevel { .. } => true,
            Expr::DefineGroup { definitions } => {
                self.expr_can_terminate(definitions, root_expr, recursion_stack, memo)
            }
            _ => true,
        }
    }

    /// A group is reachable if it's executed from root (not inside {0}) or called from a reachable group
    fn compute_reachable_groups(&self) -> BitSet {
        let mut reachable = BitSet::new();
        let mut to_visit = Vec::new();

        // Start from root (group 0)
        // Group 0 is always reachable
        reachable.insert(0);
        to_visit.push(0);

        // Also mark groups that are directly executed from root (not inside {0})
        for group in self.root_groups.iter() {
            if !reachable.contains(group) {
                reachable.insert(group);
                to_visit.push(group);
            }
        }

        // Propagate reachability through subroutine calls
        while let Some(group) = to_visit.pop() {
            if let Some(calls) = self.subroutine_calls.get(&group) {
                for call_info in calls {
                    if !reachable.contains(call_info.target_group) {
                        reachable.insert(call_info.target_group);
                        to_visit.push(call_info.target_group);
                    }
                }
            }
        }

        reachable
    }

    /// Depth-first search to detect left recursion
    /// Returns true if left recursion is detected
    fn dfs_check_left_recursion(
        &self,
        group: usize,
        visited: &mut BitSet,
        recursion_stack: &mut BitSet,
    ) -> Result<bool> {
        if recursion_stack.contains(group) {
            // We found a cycle. Since we only follow calls at position 0 (see below),
            // reaching a group already in the recursion stack means we have a left-recursive cycle.
            return Ok(true);
        }

        if visited.contains(group) {
            return Ok(false);
        }

        visited.insert(group);
        recursion_stack.insert(group);

        // Check all subroutine calls from this group
        if let Some(calls) = self.subroutine_calls.get(&group) {
            for call_info in calls {
                // Only consider calls at position 0 (potential left recursion)
                if call_info.min_pos == 0
                    && self.dfs_check_left_recursion(
                        call_info.target_group,
                        visited,
                        recursion_stack,
                    )?
                {
                    return Ok(true);
                }
            }
        }

        recursion_stack.remove(group);
        Ok(false)
    }
}

fn literal_const_size(_: &str, _: bool) -> bool {
    // Right now, regex doesn't do sophisticated case folding,
    // test below will fail when that changes, then we need to
    // do something fancier here.
    true
}

/// Recursively collect all Group expressions and their capture group indices
fn collect_groups<'a>(
    expr: &'a Expr,
    next_group_number: &mut usize,
    groups: &mut Map<usize, &'a Expr>,
) {
    match expr {
        Expr::Group(inner) => {
            let current_group = *next_group_number;
            *next_group_number += 1;
            groups.insert(current_group, inner.as_ref());
            // Continue recursing to find nested groups
            collect_groups(inner.as_ref(), next_group_number, groups);
        }
        _ => {
            // Recurse into all children
            for child in expr.children_iter() {
                collect_groups(child, next_group_number, groups);
            }
        }
    }
}

/// Context options for [`analyze`].
#[derive(Debug, Clone, Default)]
pub struct AnalyzeContext {
    /// When `true`, capture group numbering starts at 0 rather than 1 (i.e. there is an explicit
    /// outer capture group 0 in the pattern rather than an implicit one added by the compiler).
    pub explicit_capture_group_0: bool,
    /// When `true`, sub-expressions that could produce an empty match (`min_size == 0` and not
    /// `const_size`) are promoted to `hard` so that the VM can backtrack past them to find a
    /// non-empty match.
    pub find_not_empty: bool,
    /// To match Oniguruma behavior where only \z can match at EOF if preceeded by a newline character
    pub disallow_empty_match_at_eof_after_newline: bool,
    /// When true, treat assertions that support runtime input suppression overrides (`\A`, `\z`)
    /// as hard so that they execute on the VM.
    pub allow_input_assertion_overrides: bool,
    /// When true, leftmost-longest match semantics are enabled.
    #[cfg(feature = "leftmost_longest")]
    pub leftmost_longest: bool,
}

/// Analyze the parsed expression to determine whether it requires fancy features.
pub fn analyze<'a>(tree: &'a ExprTree, ctx: AnalyzeContext) -> Result<Info<'a>> {
    let explicit_capture_group_0 = ctx.explicit_capture_group_0;
    let find_not_empty = ctx.find_not_empty;
    let disallow_empty_match_at_eof_after_newline = ctx.disallow_empty_match_at_eof_after_newline;
    let allow_input_assertion_overrides = ctx.allow_input_assertion_overrides;
    #[cfg(feature = "leftmost_longest")]
    let leftmost_longest = ctx.leftmost_longest;

    // Check that numeric capture group references (backrefs and subroutine calls) and named groups are not mixed
    if tree.numbered_groups_ignored
        && tree.numeric_capture_group_references
        && !tree.named_groups.is_empty()
    {
        return Err(Error::CompileError(Box::new(
            CompileError::NamedBackrefOnly,
        )));
    }

    let start_group = if explicit_capture_group_0 { 0 } else { 1 };

    // pre-populate groups if subroutines are present to handle forward references
    let group_exprs = if tree.contains_subroutines {
        let mut groups = Map::new();
        let mut next_group_number = start_group;
        collect_groups(&tree.expr, &mut next_group_number, &mut groups);
        groups
    } else {
        Map::new()
    };

    let mut analyzer = Analyzer {
        backrefs: &tree.backrefs,
        next_group_number: start_group,
        group_info: Map::new(),
        subroutine_calls: Map::new(),
        root_groups: BitSet::new(),
        group_exprs,
        analyzing_groups: BitSet::new(),
        find_not_empty,
        disallow_empty_match_at_eof_after_newline,
        allow_input_assertion_overrides,
        #[cfg(feature = "leftmost_longest")]
        leftmost_longest,
        in_lookaround: false,
    };

    let mut analyzed = analyzer.visit(&tree.expr, 0, false, 0)?;
    // With start_group == 1 (no explicit group 0) the valid backref range is 1..=total_groups.
    // With start_group == 0 (explicit group 0) group 0 is the whole match and inner groups are
    // numbered 1..=total_groups-1, so the valid range is 0..=total_groups-1.
    let max_valid_group = if explicit_capture_group_0 {
        tree.total_groups.saturating_sub(1)
    } else {
        tree.total_groups
    };
    // Out-of-range group numbers are not in the BitSet (to avoid huge allocations), so check the
    // dedicated field first.
    if let Some(group) = tree.out_of_range_backref {
        return Err(Error::CompileError(Box::new(CompileError::InvalidBackref(
            group,
        ))));
    }
    for group in tree.backrefs.iter() {
        if group < start_group || group > max_valid_group {
            return Err(Error::CompileError(Box::new(CompileError::InvalidBackref(
                group,
            ))));
        }
    }

    // Check for left-recursive and unterminating subroutine calls (only if subroutines are present)
    if tree.contains_subroutines {
        analyzer.check_left_recursion(&tree.named_groups)?;
        analyzer.check_unbounded_recursion(&tree.expr)?;
    }

    if analyzed.min_size == 0 && disallow_empty_match_at_eof_after_newline {
        analyzed.hard = true;
    }

    Ok(analyzed)
}

/// Determine if the expression will always only ever match at position 0.
/// Note that false negatives are possible - it can return false even if it could be anchored.
/// This should therefore only be treated as an optimization.
pub fn can_compile_as_anchored(root_expr: &Expr) -> bool {
    match root_expr {
        Expr::Concat(children) => match children[0] {
            Expr::Assertion(assertion) => assertion == Assertion::StartText,
            Expr::ContinueFromPreviousMatchEnd => true,
            _ => false,
        },
        Expr::Assertion(assertion) => *assertion == Assertion::StartText,
        _ => false,
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/fancy-regex/unit/src/analyze/tests_1010.rs"));
