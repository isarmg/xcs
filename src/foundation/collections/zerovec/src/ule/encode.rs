// This file is part of ICU4X. For terms of use, please see the file
// called LICENSE at the top level of the ICU4X source tree
// (online at: https://github.com/unicode-org/icu4x/blob/main/LICENSE ).

use crate::ule::*;
use crate::varzerovec::VarZeroVecFormat;
use crate::{VarZeroSlice, VarZeroVec, ZeroSlice, ZeroVec};
#[cfg(feature = "alloc")]
use alloc::borrow::{Cow, ToOwned};
#[cfg(feature = "alloc")]
use alloc::boxed::Box;
#[cfg(feature = "alloc")]
use alloc::string::String;
#[cfg(feature = "alloc")]
use alloc::{vec, vec::Vec};

/// Allows types to be encoded as [`VarULE`]s. This is highly useful for implementing [`VarULE`] on
/// custom DSTs where the type cannot be obtained as a reference to some other type.
///
/// [`Self::encode_var_ule_as_slices()`] should be implemented by providing an encoded slice for each field
/// of the [`VarULE`] type to the callback, in order. For an implementation to be safe, the slices
/// to the callback must, when concatenated, be a valid instance of the [`VarULE`] type.
///
/// See the [custom `VarULEdocumentation`](crate::ule::custom) for examples.
///
/// [`Self::encode_var_ule_as_slices()`] is only used to provide default implementations for [`Self::encode_var_ule_write()`]
/// and [`Self::encode_var_ule_len()`]. If you override the default implementations it is totally valid to
/// replace [`Self::encode_var_ule_as_slices()`]'s body with `unreachable!()`. This can be done for cases where
/// it is not possible to implement [`Self::encode_var_ule_as_slices()`] but the other methods still work.
///
/// A typical implementation will take each field in the order found in the [`VarULE`] type,
/// convert it to ULE, call [`ULE::slice_as_bytes()`] on them, and pass the slices to `cb` in order.
/// A trailing [`ZeroVec`](crate::ZeroVec) or [`VarZeroVec`](crate::VarZeroVec) can have their underlying
/// byte representation passed through.
///
/// In case the compiler is not optimizing [`Self::encode_var_ule_len()`], it can be overridden. A typical
/// implementation will add up the sizes of each field on the [`VarULE`] type and then add in the byte length of the
/// dynamically-sized part.
///
/// # Reverse-encoding [`VarULE`]
///
/// This trait maps a struct to its bytes representation ("serialization"), and
/// [`ZeroFrom`](zerofrom::ZeroFrom) performs the opposite operation, taking those bytes and
/// creating a struct from them ("deserialization").
///
/// # Safety
///
/// The safety invariants of [`Self::encode_var_ule_as_slices()`] are:
/// - It must call `cb` (only once)
/// - The slices passed to `cb`, if concatenated, should be a valid instance of the `T` [`VarULE`] type
///   (i.e. if fed to [`VarULE::validate_bytes()`] they must produce a successful result)
/// - It must return the return value of `cb` to the caller
///
/// One or more of [`Self::encode_var_ule_len()`] and [`Self::encode_var_ule_write()`] may be provided.
/// If both are, then `zerovec` code is guaranteed to not call [`Self::encode_var_ule_as_slices()`], and it may be replaced
/// with `unreachable!()`.
///
/// The safety invariants of [`Self::encode_var_ule_len()`] are:
/// - It must return the length of the corresponding [`VarULE`] type
///
/// The safety invariants of [`Self::encode_var_ule_write()`] are:
/// - The slice written to `dst` must be a valid instance of the `T` [`VarULE`] type
pub unsafe trait EncodeAsVarULE<T: VarULE + ?Sized> {
    /// Calls `cb` with a piecewise list of byte slices that when concatenated
    /// produce the memory pattern of the corresponding instance of `T`.
    ///
    /// Do not call this function directly; instead use the other two. Some implementors
    /// may define this function to panic.
    fn encode_var_ule_as_slices<R>(&self, cb: impl FnOnce(&[&[u8]]) -> R) -> R;

    /// Return the length, in bytes, of the corresponding [`VarULE`] type
    fn encode_var_ule_len(&self) -> usize {
        self.encode_var_ule_as_slices(|slices| slices.iter().map(|s| s.len()).sum())
    }

    /// Write the corresponding [`VarULE`] type to the `dst` buffer. `dst` should
    /// be the size of [`Self::encode_var_ule_len()`]
    fn encode_var_ule_write(&self, mut dst: &mut [u8]) {
        debug_assert_eq!(self.encode_var_ule_len(), dst.len());
        self.encode_var_ule_as_slices(move |slices| {
            #[expect(clippy::indexing_slicing)] // by debug_assert
            for slice in slices {
                dst[..slice.len()].copy_from_slice(slice);
                dst = &mut dst[slice.len()..];
            }
        });
    }
}

/// Given an [`EncodeAsVarULE`] type `S`, encode it into a `Box<T>`
///
/// This is primarily useful for generating `Deserialize` impls for [`VarULE`] types
#[cfg(feature = "alloc")]
pub fn encode_varule_to_box<S: EncodeAsVarULE<T> + ?Sized, T: VarULE + ?Sized>(x: &S) -> Box<T> {
    // zero-fill the vector to avoid uninitialized data UB
    let mut vec: Vec<u8> = vec![0; x.encode_var_ule_len()];
    x.encode_var_ule_write(&mut vec);
    // SAFETY:
    // - T::from_bytes_unchecked is safe because the bytes were written by x.encode_var_ule_write
    //   which guarantees a valid representation of T.
    unsafe { cast_box(vec.into_boxed_slice()) }
}

unsafe impl<T: VarULE + ?Sized> EncodeAsVarULE<T> for T {
    fn encode_var_ule_as_slices<R>(&self, cb: impl FnOnce(&[&[u8]]) -> R) -> R {
        cb(&[T::as_bytes(self)])
    }
}

unsafe impl<T: VarULE + ?Sized> EncodeAsVarULE<T> for &'_ T {
    fn encode_var_ule_as_slices<R>(&self, cb: impl FnOnce(&[&[u8]]) -> R) -> R {
        cb(&[T::as_bytes(self)])
    }
}

unsafe impl<T: VarULE + ?Sized> EncodeAsVarULE<T> for &'_ &'_ T {
    fn encode_var_ule_as_slices<R>(&self, cb: impl FnOnce(&[&[u8]]) -> R) -> R {
        cb(&[T::as_bytes(self)])
    }
}

#[cfg(feature = "alloc")]
unsafe impl<T: VarULE + ?Sized> EncodeAsVarULE<T> for Cow<'_, T>
where
    T: ToOwned,
{
    fn encode_var_ule_as_slices<R>(&self, cb: impl FnOnce(&[&[u8]]) -> R) -> R {
        cb(&[T::as_bytes(self.as_ref())])
    }
}

#[cfg(feature = "alloc")]
unsafe impl<T: VarULE + ?Sized> EncodeAsVarULE<T> for Box<T> {
    fn encode_var_ule_as_slices<R>(&self, cb: impl FnOnce(&[&[u8]]) -> R) -> R {
        cb(&[T::as_bytes(self)])
    }
}

#[cfg(feature = "alloc")]
unsafe impl<T: VarULE + ?Sized> EncodeAsVarULE<T> for &'_ Box<T> {
    fn encode_var_ule_as_slices<R>(&self, cb: impl FnOnce(&[&[u8]]) -> R) -> R {
        cb(&[T::as_bytes(self)])
    }
}

#[cfg(feature = "alloc")]
unsafe impl EncodeAsVarULE<str> for String {
    fn encode_var_ule_as_slices<R>(&self, cb: impl FnOnce(&[&[u8]]) -> R) -> R {
        cb(&[self.as_bytes()])
    }
}

#[cfg(feature = "alloc")]
unsafe impl EncodeAsVarULE<str> for &'_ String {
    fn encode_var_ule_as_slices<R>(&self, cb: impl FnOnce(&[&[u8]]) -> R) -> R {
        cb(&[self.as_bytes()])
    }
}

// Note: This impl could technically use `T: AsULE`, but we want users to prefer `ZeroSlice<T>`
// for cases where T is not a ULE. Therefore, we can use the more efficient `memcpy` impl here.
#[cfg(feature = "alloc")]
unsafe impl<T> EncodeAsVarULE<[T]> for Vec<T>
where
    T: ULE,
{
    fn encode_var_ule_as_slices<R>(&self, cb: impl FnOnce(&[&[u8]]) -> R) -> R {
        cb(&[<[T] as VarULE>::as_bytes(self)])
    }
}

unsafe impl<T> EncodeAsVarULE<ZeroSlice<T>> for &'_ [T]
where
    T: AsULE + 'static,
{
    fn encode_var_ule_as_slices<R>(&self, _: impl FnOnce(&[&[u8]]) -> R) -> R {
        // unnecessary if the other two are implemented
        unreachable!()
    }

    #[inline]
    fn encode_var_ule_len(&self) -> usize {
        self.len() * size_of::<T::ULE>()
    }

    fn encode_var_ule_write(&self, dst: &mut [u8]) {
        #[allow(non_snake_case)]
        let S = size_of::<T::ULE>();
        debug_assert_eq!(self.len() * S, dst.len());
        for (item, ref mut chunk) in self.iter().zip(dst.chunks_mut(S)) {
            let ule = item.to_unaligned();
            chunk.copy_from_slice(ULE::slice_as_bytes(slice::from_ref(&ule)));
        }
    }
}

#[cfg(feature = "alloc")]
unsafe impl<T> EncodeAsVarULE<ZeroSlice<T>> for Vec<T>
where
    T: AsULE + 'static,
{
    fn encode_var_ule_as_slices<R>(&self, _: impl FnOnce(&[&[u8]]) -> R) -> R {
        // unnecessary if the other two are implemented
        unreachable!()
    }

    #[inline]
    fn encode_var_ule_len(&self) -> usize {
        self.as_slice().encode_var_ule_len()
    }

    #[inline]
    fn encode_var_ule_write(&self, dst: &mut [u8]) {
        self.as_slice().encode_var_ule_write(dst)
    }
}

unsafe impl<T> EncodeAsVarULE<ZeroSlice<T>> for ZeroVec<'_, T>
where
    T: AsULE + 'static,
{
    fn encode_var_ule_as_slices<R>(&self, _: impl FnOnce(&[&[u8]]) -> R) -> R {
        // unnecessary if the other two are implemented
        unreachable!()
    }

    #[inline]
    fn encode_var_ule_len(&self) -> usize {
        self.as_bytes().len()
    }

    fn encode_var_ule_write(&self, dst: &mut [u8]) {
        debug_assert_eq!(self.as_bytes().len(), dst.len());
        dst.copy_from_slice(self.as_bytes());
    }
}

unsafe impl<T, E, F> EncodeAsVarULE<VarZeroSlice<T, F>> for &'_ [E]
where
    T: VarULE + ?Sized,
    E: EncodeAsVarULE<T>,
    F: VarZeroVecFormat,
{
    fn encode_var_ule_as_slices<R>(&self, _: impl FnOnce(&[&[u8]]) -> R) -> R {
        // unnecessary if the other two are implemented
        unimplemented!()
    }

    #[expect(clippy::unwrap_used)] // TODO(#1410): Rethink length errors in VZV.
    fn encode_var_ule_len(&self) -> usize {
        crate::varzerovec::components::compute_serializable_len::<T, E, F>(self).unwrap() as usize
    }

    fn encode_var_ule_write(&self, dst: &mut [u8]) {
        crate::varzerovec::components::write_serializable_bytes::<T, E, F>(self, dst)
    }
}

#[cfg(feature = "alloc")]
unsafe impl<T, E, F> EncodeAsVarULE<VarZeroSlice<T, F>> for Vec<E>
where
    T: VarULE + ?Sized,
    E: EncodeAsVarULE<T>,
    F: VarZeroVecFormat,
{
    fn encode_var_ule_as_slices<R>(&self, _: impl FnOnce(&[&[u8]]) -> R) -> R {
        // unnecessary if the other two are implemented
        unreachable!()
    }

    #[inline]
    fn encode_var_ule_len(&self) -> usize {
        <_ as EncodeAsVarULE<VarZeroSlice<T, F>>>::encode_var_ule_len(&self.as_slice())
    }

    #[inline]
    fn encode_var_ule_write(&self, dst: &mut [u8]) {
        <_ as EncodeAsVarULE<VarZeroSlice<T, F>>>::encode_var_ule_write(&self.as_slice(), dst)
    }
}

unsafe impl<T, F> EncodeAsVarULE<VarZeroSlice<T, F>> for VarZeroVec<'_, T, F>
where
    T: VarULE + ?Sized,
    F: VarZeroVecFormat,
{
    fn encode_var_ule_as_slices<R>(&self, _: impl FnOnce(&[&[u8]]) -> R) -> R {
        // unnecessary if the other two are implemented
        unreachable!()
    }

    #[inline]
    fn encode_var_ule_len(&self) -> usize {
        self.as_bytes().len()
    }

    #[inline]
    fn encode_var_ule_write(&self, dst: &mut [u8]) {
        debug_assert_eq!(self.as_bytes().len(), dst.len());
        dst.copy_from_slice(self.as_bytes());
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/zerovec/unit/src/ule/encode/test_304.rs"));
