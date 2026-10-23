use std::marker::PhantomData;
use std::ptr::NonNull;

/// An erased reference to a value `&'a T`
///
/// Example:
/// ```rust
/// use erased::Erased;
///
/// let mut vec: Vec<Erased> = Vec::new();
/// vec.push((&5u64).into());
/// vec.push((&"Hello World").into());
///
/// // SAFETY: Above we insered a `u64` into an empty vec, therefore converting the element back to a `u64` is sound.
/// assert_eq!(unsafe { *vec[0].get::<u64>() }, 5);
/// // SAFETY: Above we insered a `&'static str` into a vec containing one element, therefore converting the element back to a `&'static str` is sound.
/// assert_eq!(unsafe { *vec[1].get::<&'static str>() }, "Hello World");
/// ```
#[derive(Copy, Clone, Debug)]
pub struct Erased<'a> {
    ptr: NonNull<()>,
    phantom: PhantomData<&'a ()>,
}

impl<'a> Erased<'a> {
    /// Create a new erased reference from a reference to `T`
    pub fn new<T>(t: &'a T) -> Erased<'a> {
        Self {
            ptr: NonNull::from(t).cast(),
            phantom: PhantomData,
        }
    }

    /// Get a reference to `T` back from the erased reference.
    ///
    /// # Safety
    /// The generic argument `T` of this function must match the `T` that was used to create this erased reference in `Erased::new` exactly.
    /// Pay specific attention that any lifetime parameters of `T` match.
    ///
    /// It is **strongly recommended** to provide `T` explicitly, even if it can be inferred. This is to make sure that the value of `T` is not accidentally changed.
    pub unsafe fn get<T>(&self) -> &'a T {
        // Safety: From the safety comment the `T` matches the `T` this erased reference was created with. The reference is still valid since its lifetime `'a` is still alive.
        self.ptr.cast::<T>().as_ref()
    }
}

impl<'a, T> From<&'a T> for Erased<'a> {
    fn from(value: &'a T) -> Self {
        Self::new(value)
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/erased/unit/src/erased_ref/tests_53.rs"));
