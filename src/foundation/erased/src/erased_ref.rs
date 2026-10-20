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
mod tests {
    use crate::Erased;

    #[test]
    fn basic_test() {
        let r1 = &5usize;
        let erased = Erased::new(r1);
        let r2 = unsafe { erased.get::<usize>() };
        assert_eq!(r1, r2);
    }

    #[test]
    fn lifetimed_test() {
        let r1 = &5usize;
        let r2 = &r1;
        let erased = Erased::new(r2);
        let r3 = unsafe { erased.get::<&usize>() };
        assert_eq!(r2, r3);
    }

    #[test]
    fn heterogeneous_test() {
        let mut vec: Vec<Erased> = Vec::new();
        vec.push((&5u64).into());
        vec.push((&"Hello World").into());

        assert_eq!(unsafe { *vec[0].get::<u64>() }, 5);
        assert_eq!(unsafe { *vec[1].get::<&'static str>() }, "Hello World");
    }
}
