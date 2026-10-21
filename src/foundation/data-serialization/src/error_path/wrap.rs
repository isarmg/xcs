use crate::error_path::{Chain, Track};

// Wrapper that attaches context to a `Visitor`, `SeqAccess` or `EnumAccess`.
pub struct Wrap<'a, 'b, X> {
    pub(super) delegate: X,
    pub(super) chain: &'a Chain<'a>,
    pub(super) track: &'b Track,
}

// Wrapper that attaches context to a `VariantAccess`.
pub struct WrapVariant<'a, 'b, X> {
    pub(super) delegate: X,
    pub(super) chain: Chain<'a>,
    pub(super) track: &'b Track,
}

impl<'a, 'b, X> Wrap<'a, 'b, X> {
    pub(super) fn new(delegate: X, chain: &'a Chain<'a>, track: &'b Track) -> Self {
        Wrap {
            delegate,
            chain,
            track,
        }
    }
}

impl<'a, 'b, X> WrapVariant<'a, 'b, X> {
    pub(super) fn new(delegate: X, chain: Chain<'a>, track: &'b Track) -> Self {
        WrapVariant {
            delegate,
            chain,
            track,
        }
    }
}
