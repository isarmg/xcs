/// Three-valued verdict for "does this schema admit this value?". `Unknown` marks a constraint no
/// checker can decide; resolving it belongs to the caller alone, because the safe reading differs
/// by direction: an unresolved verdict must never narrow the schema.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Verdict {
    Admits,
    Rejects,
    Unknown,
}

/// How a facet no checker covers reads. A validator without a checker skips the keyword, so a
/// demanded facet passes the value and a barred one rejects it; a value set carries no facet of its
/// own, so intersection can express only that reading. Union keeps such a facet undecided instead,
/// because absorbing a branch on a guess drops values a checker would have kept.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UncheckableFacet {
    Skipped,
    Undecided,
}

impl Verdict {
    pub(crate) fn from_bool(admitted: bool) -> Self {
        if admitted {
            Verdict::Admits
        } else {
            Verdict::Rejects
        }
    }

    /// Logical AND over three values: a definite rejection wins, and one undecided input leaves
    /// the whole answer undecided.
    pub(crate) fn and(self, other: Self) -> Self {
        match (self, other) {
            (Verdict::Rejects, Verdict::Admits | Verdict::Rejects | Verdict::Unknown)
            | (Verdict::Admits | Verdict::Unknown, Verdict::Rejects) => Verdict::Rejects,
            (Verdict::Unknown, Verdict::Admits | Verdict::Unknown)
            | (Verdict::Admits, Verdict::Unknown) => Verdict::Unknown,
            (Verdict::Admits, Verdict::Admits) => Verdict::Admits,
        }
    }

    pub(crate) fn all(verdicts: impl IntoIterator<Item = Self>) -> Self {
        let mut result = Verdict::Admits;
        for verdict in verdicts {
            result = result.and(verdict);
            if result == Verdict::Rejects {
                return Verdict::Rejects;
            }
        }
        result
    }

    /// Logical OR over three values: a definite admission wins, and one undecided input leaves
    /// the whole answer undecided.
    pub(crate) fn any(verdicts: impl IntoIterator<Item = Self>) -> Self {
        let mut result = Verdict::Rejects;
        for verdict in verdicts {
            match verdict {
                Verdict::Admits => return Verdict::Admits,
                Verdict::Unknown => result = Verdict::Unknown,
                Verdict::Rejects => {}
            }
        }
        result
    }
}

#[cfg(test)]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../test/foundation/schema-validation/upstream/jsonschema/unit/src/canonical/ir/verdict/tests_68.rs"
));
