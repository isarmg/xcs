//! Out reference ([`&'a out T`](Out)).
#![deny(
    missing_docs,
    clippy::all,
    clippy::cargo,
    clippy::missing_const_for_fn,
    clippy::missing_inline_in_public_items,
    clippy::must_use_candidate
)]
#![cfg_attr(not(test), no_std)]

use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::ptr::{self, NonNull};
use core::slice;

/// Out reference ([`&'a out T`](Out)).
///
/// An out reference is similar to a mutable reference, but it may point to uninitialized memory.
/// An out reference may be used to initialize the pointee or represent a data buffer.
///
/// [`&'a out T`](Out) can be converted from:
/// + [`&'a mut MaybeUninit<T>`](core::mem::MaybeUninit)
///     and [`&'a mut [MaybeUninit<T>]`](core::mem::MaybeUninit),
///     where the `T` may be uninitialized.
/// + [`&'a mut T`](reference) and [`&'a mut [T]`](prim@slice),
///     where the `T` is initialized and [Copy].
///
/// It is not allowed to corrupt or de-initialize the pointee, which may cause unsoundness.
/// It is the main difference between [`&'a out T`](Out)
/// and [`&'a mut MaybeUninit<T>`](core::mem::MaybeUninit)
/// /[`&'a mut [MaybeUninit<T>]`](core::mem::MaybeUninit).
///
/// Any reads through an out reference may read uninitialized value(s) and cause undefined behavior.
///
/// [`AsOut`] provides a shortcut for converting a mutable reference to an out reference.
///
/// # Examples
///
/// ```rust
/// use core::ptr;
/// use core::slice;
/// use core::mem::MaybeUninit;
///
/// use outref::AsOut;
/// use outref::Out;
///
/// fn copy<'d, T: Copy>(src: &[T], mut dst: Out<'d, [T]>) -> &'d mut [T] {
///     assert_eq!(src.len(), dst.len());
///     unsafe {
///         let count = src.len();
///         let src = src.as_ptr();
///         let dst = dst.as_mut_ptr();
///         ptr::copy_nonoverlapping(src, dst, count);
///         slice::from_raw_parts_mut(dst, count)
///     }
/// }
///
/// fn copy_init<'d, T: Copy>(src: &[T], dst: &'d mut [T]) -> &'d mut [T] {
///     copy(src, dst.as_out())
/// }
///
/// fn copy_uninit<'d, T: Copy>(src: &[T], dst: &'d mut [MaybeUninit<T>]) -> &'d mut [T] {
///     copy(src, dst.as_out())
/// }
/// ```
#[repr(transparent)]
pub struct Out<'a, T: 'a + ?Sized> {
    data: NonNull<T>,
    _marker: PhantomData<&'a mut T>,
}

unsafe impl<T: Send> Send for Out<'_, T> {}
unsafe impl<T: Sync> Sync for Out<'_, T> {}
impl<T: Unpin> Unpin for Out<'_, T> {}

impl<'a, T: ?Sized> Out<'a, T> {
    /// Forms an [`Out<'a, T>`](Out)
    ///
    /// # Safety
    ///
    /// * `data` must be valid for writes.
    /// * `data` must be properly aligned.
    #[inline(always)]
    #[must_use]
    pub unsafe fn new(data: *mut T) -> Self {
        Self {
            data: NonNull::new_unchecked(data),
            _marker: PhantomData,
        }
    }

    /// Converts to a mutable (unique) reference to the value.
    ///
    /// # Safety
    /// The referenced value must be initialized when calling this function.
    #[inline(always)]
    #[must_use]
    pub unsafe fn assume_init(mut self) -> &'a mut T {
        self.data.as_mut()
    }

    /// Reborrows the out reference for a shorter lifetime.
    #[inline(always)]
    #[must_use]
    pub fn reborrow<'s>(&'s mut self) -> Out<'s, T>
    where
        'a: 's,
    {
        Self {
            data: self.data,
            _marker: PhantomData,
        }
    }
}

impl<'a, T> Out<'a, T> {
    /// Forms an [`Out<'a, T>`](Out).
    #[inline(always)]
    #[must_use]
    pub fn from_mut(data: &'a mut T) -> Self
    where
        T: Copy,
    {
        unsafe { Self::new(data) }
    }

    /// Forms an [`Out<'a, T>`](Out) from an uninitialized value.
    #[inline(always)]
    #[must_use]
    pub fn from_uninit(data: &'a mut MaybeUninit<T>) -> Self {
        let data: *mut T = MaybeUninit::as_mut_ptr(data);
        unsafe { Self::new(data.cast()) }
    }

    /// Converts to [`&'a mut MaybeUninit<T>`](core::mem::MaybeUninit)
    /// # Safety
    /// It is not allowed to corrupt or de-initialize the pointee.
    #[inline(always)]
    #[must_use]
    pub unsafe fn into_uninit(self) -> &'a mut MaybeUninit<T> {
        &mut *self.data.as_ptr().cast()
    }

    /// Returns an unsafe mutable pointer to the value.
    #[inline(always)]
    #[must_use]
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.data.as_ptr().cast()
    }
}

impl<'a, T> Out<'a, [T]> {
    /// Forms an [`Out<'a, [T]>`](Out).
    #[inline(always)]
    #[must_use]
    pub fn from_slice(slice: &'a mut [T]) -> Self
    where
        T: Copy,
    {
        unsafe { Self::new(slice) }
    }

    /// Forms an [`Out<'a, [T]>`](Out) from an uninitialized slice.
    #[inline(always)]
    #[must_use]
    pub fn from_uninit_slice(slice: &'a mut [MaybeUninit<T>]) -> Self {
        let slice: *mut [T] = {
            let len = slice.len();
            let data = slice.as_mut_ptr().cast();
            ptr::slice_from_raw_parts_mut(data, len)
        };
        unsafe { Self::new(slice) }
    }

    /// Converts to [`&'a mut [MaybeUninit<T>]`](core::mem::MaybeUninit)
    /// # Safety
    /// It is not allowed to corrupt or de-initialize the pointee.
    #[inline(always)]
    #[must_use]
    pub unsafe fn into_uninit_slice(self) -> &'a mut [MaybeUninit<T>] {
        let len = self.len();
        let data = self.data.as_ptr().cast();
        slice::from_raw_parts_mut(data, len)
    }

    /// Returns true if the slice has a length of 0.
    #[inline(always)]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the number of elements in the slice.
    #[inline(always)]
    #[must_use]
    pub const fn len(&self) -> usize {
        NonNull::len(self.data)
    }

    /// Returns an unsafe mutable pointer to the slice's buffer.
    #[inline(always)]
    #[must_use]
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.data.as_ptr().cast()
    }
}

/// Extension trait for converting a mutable reference to an out reference.
///
/// # Safety
/// This trait can be trusted to be implemented correctly for all types.
pub unsafe trait AsOut<T: ?Sized> {
    /// Returns an out reference to self.
    fn as_out(&mut self) -> Out<'_, T>;
}

unsafe impl<T> AsOut<T> for T
where
    T: Copy,
{
    #[inline(always)]
    fn as_out(&mut self) -> Out<'_, T> {
        Out::from_mut(self)
    }
}

unsafe impl<T> AsOut<T> for MaybeUninit<T> {
    #[inline(always)]
    fn as_out(&mut self) -> Out<'_, T> {
        Out::from_uninit(self)
    }
}

unsafe impl<T> AsOut<[T]> for [T]
where
    T: Copy,
{
    #[inline(always)]
    fn as_out(&mut self) -> Out<'_, [T]> {
        Out::from_slice(self)
    }
}

unsafe impl<T> AsOut<[T]> for [MaybeUninit<T>] {
    #[inline(always)]
    fn as_out(&mut self) -> Out<'_, [T]> {
        Out::from_uninit_slice(self)
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/outref/unit/src/lib/tests_252.rs"));
