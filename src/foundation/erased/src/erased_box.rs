use std::ptr::NonNull;

/// A box with an erased type.
///
/// # Warning
/// This type **leaks** the Box when it is dropped.
/// To ensure that the Box is not leaked, call `into_inner` on it before it is dropped.
///
/// Example:
/// ```rust
/// use erased::ErasedBox;
///
/// let b: Box<usize> = Box::new(5usize);
/// let erased: ErasedBox = ErasedBox::new(b);
///
/// /// Safety: The type given to `into_inner` matches the give of `b`.
/// let v: Box<usize> = unsafe { erased.into_inner::<usize>() };
/// assert_eq!(*v, 5usize);
/// ```
#[derive(Debug)]
pub struct ErasedBox {
    ptr: NonNull<()>,
}

impl ErasedBox {
    /// Create a new erased box from a `Box<T>`
    pub fn new<T>(t: Box<T>) -> ErasedBox {
        Self {
            ptr: NonNull::from(Box::leak(t)).cast(),
        }
    }

    /// Get a normal box `Box<T>` back from the erased box.
    ///
    /// # Safety
    /// The generic argument `T` of this function must match the `T` that was used to create this erased box in `ErasedBox::new` exactly.
    /// Pay specific attention that any lifetime parameters of `T` match.
    ///
    /// It is **strongly recommended** to provide `T` explicitly, even if it can be inferred. This is to make sure that the value of `T` is not accidentally changed.
    pub unsafe fn into_inner<T>(self) -> Box<T> {
        // Safety: From the safety comment the `T` matches the `T` this erased box was created with. The reference is unique since we consume `self`.
        Box::from_raw(self.ptr.cast::<T>().as_mut())
    }

    /// Get a reference to the value in this box.
    ///
    /// # Safety
    /// The generic argument `T` of this function must match the `T` that was used to create this erased box in `ErasedBox::new` exactly.
    /// Pay specific attention that any lifetime parameters of `T` match.
    ///
    /// It is **strongly recommended** to provide `T` explicitly, even if it can be inferred. This is to make sure that the value of `T` is not accidentally changed.
    pub unsafe fn get_ref<T>(&self) -> &T {
        // Safety: From the safety comment the `T` matches the `T` this box was created with. The reference borrows self which owns the pointer, so its lifetime is valid.
        self.ptr.cast::<T>().as_ref()
    }

    /// Get a mutable reference to the value in this box.
    ///
    /// # Safety
    /// The generic argument `T` of this function must match the `T` that was used to create this erased box in `ErasedBox::new` exactly.
    /// Pay specific attention that any lifetime parameters of `T` match.
    ///
    /// It is **strongly recommended** to provide `T` explicitly, even if it can be inferred. This is to make sure that the value of `T` is not accidentally changed.
    pub unsafe fn get_mut<T>(&mut self) -> &mut T {
        // Safety: From the safety comment the `T` matches the `T` this box was created with. The reference borrows self which owns the pointer, so its lifetime is valid.
        // Self is borrowed mutably ensuring exclusive access.
        self.ptr.cast::<T>().as_mut()
    }
}

impl<T> From<Box<T>> for ErasedBox {
    fn from(value: Box<T>) -> Self {
        Self::new(value)
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/erased/unit/src/erased_box/tests_77.rs"));
