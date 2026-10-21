// Copyright 2026 The Fancy Regex Authors.
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

//! Seek pre-filter: build a simplified approximation of a pattern used to
//! skip to plausible match positions before handing off to the backtracking VM.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

#[cfg(not(feature = "std"))]
use alloc::collections::BTreeMap as Map;
#[cfg(feature = "std")]
use std::collections::HashMap as Map;

use crate::analyze::Info;
use crate::compile::MAX_SUBROUTINE_RECURSION_DEPTH;
use crate::{write_quantifier, Absent, Assertion, Expr};

/// Returns `true` if the expression tree contains a `StartText` or `EndText` assertion anywhere.
///
/// This is used to decide whether it is safe to inline a group body when approximating a backref:
/// text-anchor assertions in an inlined body would be evaluated at the wrong string position and
/// could cause false negatives (missing valid match positions).
pub(crate) fn expr_contains_positional_anchor(expr: &Expr) -> bool {
    match expr {
        // StartText (^), EndText ($), StartLine ((?m)^), EndLine ((?m)$) are positional anchors
        // that depend on absolute position in the string.  When a group containing such anchors
        // is inlined for a backref approximation, the anchors would be evaluated at the wrong
        // position (the backref position rather than the original capture position), potentially
        // producing false negatives.
        Expr::Assertion(
            Assertion::StartText
            | Assertion::EndText
            | Assertion::EndTextIgnoreTrailingNewlines { .. }
            | Assertion::StartLine { .. }
            | Assertion::StartLineOniguruma { .. }
            | Assertion::EndLine { .. },
        ) => true,
        _ => expr.children_iter().any(expr_contains_positional_anchor),
    }
}

/// Cap on the seek-pattern buffer length. Inlining a backref/subroutine
/// substitutes the referenced group's body, and when that body transitively
/// references the same group (e.g. `(\s+((\3|\4|\5)))?` where `\4`/`\5` point
/// at ancestors of the backref) the substitution branches at every level and,
/// bounded only by the recursion depth, expands the buffer exponentially — so
/// a short pattern can produce a multi-megabyte seek string. Once the buffer
/// is already this large, further inlining stops and a permissive placeholder
/// is emitted instead: the seek pattern only narrows candidate start positions
/// and never decides matches, so a less-selective approximation stays correct.
pub(crate) const MAX_SEEK_PATTERN_LEN: usize = 4096;

/// Emit a permissive size placeholder — `(?s:.)+` or `(?s:.){N,}` — into `buf`.
///
/// Used when a hard node cannot be represented in the seek pattern but has a known minimum
/// size.  The placeholder is an over-approximation that avoids false negatives: it admits any
/// run of characters of at least `min_size` length.
pub(crate) fn emit_min_size_placeholder(buf: &mut String, min_size: usize, precedence: u8) {
    if min_size == 0 {
        return; // zero width: safe to drop without emitting anything
    }
    if precedence > 2 {
        buf.push_str("(?:");
    }
    buf.push_str("(?s:.)");
    match min_size {
        1 => buf.push('+'),
        n => {
            buf.push('{');
            crate::push_usize(buf, n);
            buf.push_str(",}");
        }
    }
    if precedence > 2 {
        buf.push(')');
    }
}

/// Write the seek-pattern approximation for `info` into `buf`.
///
/// Easy subtrees without capture groups are serialised verbatim via [`Expr::to_str`].
/// Hard nodes are handled as follows:
/// - `Backref` / `SubroutineCall`: inline the referenced group's body (up to
///   `MAX_SUBROUTINE_RECURSION_DEPTH`). For backrefs, positional anchors (`^`, `$`, `\b`) etc.
///   are dropped from the inlined body because backreferences only capture text, not positions.
///   If inlining is not possible, a permissive `(?s:.){min_size,}` placeholder is emitted
///   instead of silently dropping, to maintain correctness.
/// - `LookAround`, `KeepOut`, `ContinueFromPreviousMatchEnd`, `BacktrackingControlVerb`,
///   `BackrefExistsCondition`, `Absent`: dropped (emit nothing — safe over-approximation,
///   their `min_size` is 0).
/// - `AtomicGroup`: keep the contents, discard the atomicity wrapper.
/// - `Group`: keep the contents, discard the capture group wrapper.
/// - `Conditional`: emit the union of the true and false branches.
/// - `GeneralNewline`: replaced with an explicit alternation.
/// - `Assertion`: anchors and word-boundary assertions are emitted; half-boundaries are
///   dropped (over-approximation — `\b` would be too restrictive).
pub(crate) fn build_seek_pattern<'a>(
    info: &Info<'a>,
    group_info_map: &Map<usize, &'a Info<'a>>,
    depth: usize,
    buf: &mut String,
    precedence: u8,
) {
    let mut inlined_groups = Vec::new();
    build_seek_pattern_impl(
        info,
        group_info_map,
        depth,
        buf,
        precedence,
        false,
        &mut inlined_groups,
    );
}

pub(crate) fn build_seek_pattern_impl<'a>(
    info: &Info<'a>,
    group_info_map: &Map<usize, &'a Info<'a>>,
    depth: usize,
    buf: &mut String,
    precedence: u8,
    drop_positional_anchors: bool,
    inlined_groups: &mut Vec<usize>,
) {
    // Drop positional anchors at this node when requested (used when inlining for a backref).
    if drop_positional_anchors {
        if let Expr::Assertion(
            Assertion::StartText
            | Assertion::EndText
            | Assertion::StartLine { .. }
            | Assertion::StartLineOniguruma { .. }
            | Assertion::EndLine { .. },
        ) = info.expr
        {
            return;
        }
        // \Z is an odd one out, although it is a zero-width assertion, for the purposes of seeking
        // it still needs to consume the newlines
        if let Expr::Assertion(Assertion::EndTextIgnoreTrailingNewlines { crlf }) = info.expr {
            if *crlf {
                buf.push_str(r"[\r\n]*");
            } else {
                buf.push_str(r"\n*");
            }
            return;
        }
    }

    let has_capture_groups = info.start_group() != info.end_group();
    if !info.hard && !has_capture_groups {
        // Easy subtree — use to_str directly when no positional-anchor dropping is needed,
        // or when the subtree contains no positional anchors (so dropping is a no-op).
        if !drop_positional_anchors || !expr_contains_positional_anchor(info.expr) {
            info.expr.to_str(buf, precedence);
            return;
        }
        // The easy subtree contains positional anchors that need to be stripped.
        // Fall through to the match below, which recurses via build_seek_pattern_impl.
    }

    match info.expr {
        Expr::Empty | Expr::DefineGroup { .. } => {}
        Expr::Assertion(assertion) => {
            // Emit all assertion types, approximating half-word-boundaries with \b.
            // Note: easy positional assertions (StartText, EndText, StartLine, EndLine) are
            // handled by the to_str early return above (drop_positional_anchors=false) or by
            // the early drop at the top of this function (drop_positional_anchors=true).
            // Only hard assertions reach this arm.
            match assertion {
                Assertion::EndTextIgnoreTrailingNewlines { crlf: false } => buf.push_str(r"\n*$"),
                Assertion::EndTextIgnoreTrailingNewlines { crlf: true } => {
                    buf.push_str(r"[\r\n]*$")
                }
                Assertion::WordBoundary => buf.push_str(r"\b"),
                Assertion::NotWordBoundary => buf.push_str(r"\B"),
                // Full word boundaries: \< and \> — overapproximate with \b.
                Assertion::LeftWordBoundary | Assertion::RightWordBoundary => buf.push_str(r"\b"),
                // Half-boundaries (\b{start-half}, \b{end-half}) can match in positions where
                // \b does not (e.g. before punctuation), so \b would be an under-approximation.
                // Drop them (emit nothing) which is a safe over-approximation.
                Assertion::LeftWordHalfBoundary | Assertion::RightWordHalfBoundary => {}
                // Easy positional anchors — handled by early return / early drop above.
                // They can only reach here when drop_positional_anchors=false and the subtree
                // is easy (handled by to_str) but the Group/Concat parent fell through to match.
                // Emit them faithfully.
                Assertion::StartText => buf.push('^'),
                Assertion::EndText => buf.push('$'),
                Assertion::StartLine { crlf: false }
                | Assertion::StartLineOniguruma { crlf: false } => buf.push_str("(?m:^)"),
                Assertion::StartLine { crlf: true }
                | Assertion::StartLineOniguruma { crlf: true } => buf.push_str("(?Rm:^)"),
                Assertion::EndLine { crlf: false } => buf.push_str("(?m:$)"),
                Assertion::EndLine { crlf: true } => buf.push_str("(?Rm:$)"),
            }
        }
        Expr::Concat(_) => {
            if precedence > 1 {
                buf.push_str("(?:");
            }
            for child in &info.children {
                build_seek_pattern_impl(
                    child,
                    group_info_map,
                    depth,
                    buf,
                    2,
                    drop_positional_anchors,
                    inlined_groups,
                );
            }
            if precedence > 1 {
                buf.push(')');
            }
        }
        Expr::Alt(_) => {
            if precedence > 0 {
                buf.push_str("(?:");
            }
            let mut first = true;
            for child in &info.children {
                if !first {
                    buf.push('|');
                }
                build_seek_pattern_impl(
                    child,
                    group_info_map,
                    depth,
                    buf,
                    1,
                    drop_positional_anchors,
                    inlined_groups,
                );
                first = false;
            }
            if precedence > 0 {
                buf.push(')');
            }
        }
        Expr::Group(_) => {
            // Drop the capture group wrapper; keep the contents.
            if !info.children.is_empty() {
                build_seek_pattern_impl(
                    &info.children[0],
                    group_info_map,
                    depth,
                    buf,
                    precedence,
                    drop_positional_anchors,
                    inlined_groups,
                );
            }
        }
        Expr::Repeat { lo, hi, greedy, .. } => {
            if precedence > 2 {
                buf.push_str("(?:");
            }
            if !info.children.is_empty() {
                build_seek_pattern_impl(
                    &info.children[0],
                    group_info_map,
                    depth,
                    buf,
                    3,
                    drop_positional_anchors,
                    inlined_groups,
                );
            }
            write_quantifier(buf, *lo, *hi, *greedy);
            if precedence > 2 {
                buf.push(')');
            }
        }
        Expr::Backref { group, casei }
        | Expr::BackrefWithRelativeRecursionLevel { group, casei, .. } => {
            // Inline the body of the referenced capture group, wrapping with (?i:...) when
            // the backref is case-insensitive so the approximation remains correct.
            //
            // Positional anchors are dropped from the inlined body: a backref matches the
            // captured text, not a position, so anchors from the group definition do not hold
            // at the backref's position.
            //
            // For BackrefWithRelativeRecursionLevel, the relative_level is ignored here:
            // for seeking purposes, the group body is the same regardless of recursion level.
            //
            // If inlining is not possible (depth limit, group not in map, or recursive cycle), emit a permissive
            // placeholder so that no match positions are incorrectly skipped.
            if depth < MAX_SUBROUTINE_RECURSION_DEPTH && buf.len() < MAX_SEEK_PATTERN_LEN {
                if let Some(group_info) = group_info_map.get(group) {
                    if inlined_groups.contains(group) {
                        emit_min_size_placeholder(buf, group_info.min_size, precedence);
                        return;
                    }
                    if !group_info.children.is_empty() {
                        let child = &group_info.children[0];
                        inlined_groups.push(*group);
                        if *casei {
                            let mut inner = String::new();
                            build_seek_pattern_impl(
                                child,
                                group_info_map,
                                depth + 1,
                                &mut inner,
                                // Precedence 0 (alternation) so content inside (?i:...) is unambiguous.
                                0,
                                true,
                                inlined_groups,
                            );
                            inlined_groups.pop();
                            if !inner.is_empty() {
                                buf.push_str("(?i:");
                                buf.push_str(&inner);
                                buf.push(')');
                            }
                        } else {
                            build_seek_pattern_impl(
                                child,
                                group_info_map,
                                depth + 1,
                                buf,
                                precedence,
                                true,
                                inlined_groups,
                            );
                            inlined_groups.pop();
                        }
                        return;
                    }
                    // Empty group (no children): min_size=0, nothing to emit.
                    return;
                }
            }
            // Could not inline (depth limit or group not found): emit a permissive placeholder
            // so we don't falsely skip positions where the backref's target could have matched.
            emit_min_size_placeholder(buf, info.min_size, precedence);
        }
        Expr::SubroutineCall(target_group) => {
            // Inline the body of the target group, honouring the recursion depth limit.
            if depth < MAX_SUBROUTINE_RECURSION_DEPTH && buf.len() < MAX_SEEK_PATTERN_LEN {
                if let Some(group_info) = group_info_map.get(target_group) {
                    if !group_info.children.is_empty() {
                        build_seek_pattern_impl(
                            &group_info.children[0],
                            group_info_map,
                            depth + 1,
                            buf,
                            precedence,
                            drop_positional_anchors,
                            inlined_groups,
                        );
                        return;
                    }
                    return;
                }
            }
            emit_min_size_placeholder(buf, info.min_size, precedence);
        }
        // LookAround is zero-width — drop it.
        Expr::LookAround(_, _) => {}
        Expr::AtomicGroup(_) => {
            // Keep the contents; atomicity is invisible to the seek approximation.
            if !info.children.is_empty() {
                build_seek_pattern_impl(
                    &info.children[0],
                    group_info_map,
                    depth,
                    buf,
                    precedence,
                    drop_positional_anchors,
                    inlined_groups,
                );
            }
        }
        Expr::GeneralNewline { unicode } => {
            // Replace \R with an explicit alternation accepted by regex-automata.
            if *unicode {
                buf.push_str(r"(?:\r\n|[\n\x0B\x0C\r\x85\u{2028}\u{2029}])");
            } else {
                buf.push_str(r"(?:\r\n|[\n\x0B\x0C\r])");
            }
        }
        Expr::Conditional { .. } => {
            // Emit the union of (condition + true_branch) and (false_branch).
            //
            // Including the condition prefix in the true alternative ensures that consuming
            // conditions (e.g. `(?(a)b|c)` where `a` is matched and consumed) are
            // over-approximated correctly.  For zero-width conditions (e.g. backref-exists
            // checks) the condition emits nothing and the result degenerates to
            // `true_branch | false_branch`, preserving the original behaviour.
            let mut cond_pat = String::new();
            let mut true_pat = String::new();
            let mut false_pat = String::new();
            if !info.children.is_empty() {
                build_seek_pattern_impl(
                    &info.children[0],
                    group_info_map,
                    depth,
                    &mut cond_pat,
                    2,
                    drop_positional_anchors,
                    inlined_groups,
                );
            }
            if info.children.len() >= 2 {
                build_seek_pattern_impl(
                    &info.children[1],
                    group_info_map,
                    depth,
                    &mut true_pat,
                    2,
                    drop_positional_anchors,
                    inlined_groups,
                );
            }
            if info.children.len() >= 3 {
                build_seek_pattern_impl(
                    &info.children[2],
                    group_info_map,
                    depth,
                    &mut false_pat,
                    1,
                    drop_positional_anchors,
                    inlined_groups,
                );
            }
            // Build the "condition then true-branch" alternative.
            let cond_true = if cond_pat.is_empty() {
                true_pat.clone()
            } else if true_pat.is_empty() {
                cond_pat.clone()
            } else {
                format!("(?:{}{})", cond_pat, true_pat)
            };
            match (cond_true.is_empty(), false_pat.is_empty()) {
                (true, true) => {}
                (true, false) => buf.push_str(&false_pat),
                (false, true) => buf.push_str(&cond_true),
                (false, false) => {
                    if precedence > 0 {
                        buf.push_str("(?:");
                    }
                    buf.push_str(&cond_true);
                    buf.push('|');
                    buf.push_str(&false_pat);
                    if precedence > 0 {
                        buf.push(')');
                    }
                }
            }
        }
        // Absent repeater `(?~expr)` matches any content not containing `expr`.
        // Approximate with `(?s:.*)` (any characters, including newline) since the repeater
        // can consume an arbitrary number of characters.
        Expr::Absent(Absent::Repeater(_)) => buf.push_str("(?s:.*)"),
        // Absent expression `(?~|absent|exp)` matches `exp` subject to the absent constraint.
        // Approximate by seeking only for `exp`, dropping the constraint — this is a safe
        // over-approximation since any position where `exp` matches (ignoring the constraint)
        // is a superset of positions where the full expression matches.
        Expr::Absent(crate::Absent::Expression { .. }) => {
            if info.children.len() >= 2 {
                build_seek_pattern_impl(
                    &info.children[1],
                    group_info_map,
                    depth,
                    buf,
                    precedence,
                    drop_positional_anchors,
                    inlined_groups,
                );
            }
        }
        // Zero-width / control nodes — drop them (all have min_size = 0).
        Expr::KeepOut
        | Expr::ContinueFromPreviousMatchEnd
        | Expr::BacktrackingControlVerb(_)
        | Expr::BackrefExistsCondition { .. }
        | Expr::Absent(_) => {}
        // Easy leaf nodes (Literal, Any, Delegate) are always handled by the easy no-captures
        // early return above and never reach here. Listed explicitly so that adding a new
        // Expr variant produces a compile error until the seek-pattern case is handled.
        Expr::Literal { .. } | Expr::Any { .. } | Expr::Delegate { .. } => {
            info.expr.to_str(buf, precedence)
        }
        // These variants cause a compile error during analysis and are therefore unreachable
        // after a successful `analyze()` call.
        Expr::AstNode(..) => {
            unreachable!("unexpected expr variant after analysis")
        }
    }
}

/// Returns `true` if `pattern` contains at least one character that provides useful filtering
/// (i.e. a literal character, character class, or anchor rather than just wildcards/quantifiers).
///
/// This is a conservative approximation: any pattern containing `[`, `\`, `^`, `$`, or an
/// alphanumeric character is considered useful. The primary goal is to reject fully
/// unconstrained patterns such as `.*`.
pub fn seek_pattern_is_useful(pattern: &str) -> bool {
    pattern
        .bytes()
        .any(|b| matches!(b, b'[' | b'\\' | b'^' | b'$' | b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9'))
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/fancy-regex/unit/src/seek/tests_522.rs"));
