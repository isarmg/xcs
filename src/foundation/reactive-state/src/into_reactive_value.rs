#[doc(hidden)]
pub struct __IntoReactiveValueMarkerBaseCase;

/// A helper trait that works like `Into<T>` but uses a marker generic
/// to allow more `From` implementations than would be allowed with just `Into<T>`.
pub trait IntoReactiveValue<T, M> {
    /// Converts `self` into a `T`.
    fn into_reactive_value(self) -> T;
}

// The base case, which allows anything which implements .into() to work:
impl<T, I> IntoReactiveValue<T, __IntoReactiveValueMarkerBaseCase> for I
where
    I: Into<T>,
{
    fn into_reactive_value(self) -> T {
        self.into()
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/reactive-state/unit/src/into_reactive_value/tests_21.rs"));
