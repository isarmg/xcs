// This file is part of ICU4X. For terms of use, please see the file
// called LICENSE at the top level of the ICU4X source tree
// (online at: https://github.com/unicode-org/icu4x/blob/main/LICENSE ).

// This way we can copy-paste Yokeable impls
#![allow(unknown_lints)] // forgetting_copy_types
#![allow(renamed_and_removed_lints)] // forgetting_copy_types
#![allow(forgetting_copy_types)]
#![allow(clippy::forget_copy)]
#![allow(clippy::forget_non_drop)]

#[cfg(feature = "alloc")]
use crate::map::ZeroMapBorrowed;
#[cfg(feature = "alloc")]
use crate::map::ZeroMapKV;
#[cfg(feature = "alloc")]
use crate::map2d::ZeroMap2dBorrowed;
use crate::ule::*;
use crate::{VarZeroCow, VarZeroVec, ZeroVec};
#[cfg(feature = "alloc")]
use crate::{ZeroMap, ZeroMap2d};
use core::{mem, ptr};
use yoke::*;

// This impl is similar to the impl on Cow and is safe for the same reasons
/// This impl requires enabling the optional `yoke` Cargo feature of the `zerovec` crate
unsafe impl<'a, T: 'static + AsULE> Yokeable<'a> for ZeroVec<'static, T> {
    type Output = ZeroVec<'a, T>;
    #[inline]
    fn transform(&'a self) -> &'a Self::Output {
        self
    }
    #[inline]
    fn transform_owned(self) -> Self::Output {
        self
    }
    #[inline]
    unsafe fn make(from: Self::Output) -> Self {
        debug_assert!(size_of::<Self::Output>() == size_of::<Self>());
        let from = mem::ManuallyDrop::new(from);
        let ptr: *const Self = (&*from as *const Self::Output).cast();
        ptr::read(ptr)
    }
    #[inline]
    fn transform_mut<F>(&'a mut self, f: F)
    where
        F: 'static + for<'b> FnOnce(&'b mut Self::Output),
    {
        unsafe { f(mem::transmute::<&mut Self, &mut Self::Output>(self)) }
    }
}

// This impl is similar to the impl on Cow and is safe for the same reasons
/// This impl requires enabling the optional `yoke` Cargo feature of the `zerovec` crate
unsafe impl<'a, T: 'static + VarULE + ?Sized> Yokeable<'a> for VarZeroVec<'static, T> {
    type Output = VarZeroVec<'a, T>;
    #[inline]
    fn transform(&'a self) -> &'a Self::Output {
        self
    }
    #[inline]
    fn transform_owned(self) -> Self::Output {
        self
    }
    #[inline]
    unsafe fn make(from: Self::Output) -> Self {
        debug_assert!(size_of::<Self::Output>() == size_of::<Self>());
        let from = mem::ManuallyDrop::new(from);
        let ptr: *const Self = (&*from as *const Self::Output).cast();
        ptr::read(ptr)
    }
    #[inline]
    fn transform_mut<F>(&'a mut self, f: F)
    where
        F: 'static + for<'b> FnOnce(&'b mut Self::Output),
    {
        unsafe { f(mem::transmute::<&mut Self, &mut Self::Output>(self)) }
    }
}

// This impl is similar to the impl on Cow and is safe for the same reasons
/// This impl requires enabling the optional `yoke` Cargo feature of the `zerovec` crate
unsafe impl<'a, T: 'static + ?Sized> Yokeable<'a> for VarZeroCow<'static, T> {
    type Output = VarZeroCow<'a, T>;
    #[inline]
    fn transform(&'a self) -> &'a Self::Output {
        self
    }
    #[inline]
    fn transform_owned(self) -> Self::Output {
        self
    }
    #[inline]
    unsafe fn make(from: Self::Output) -> Self {
        debug_assert!(size_of::<Self::Output>() == size_of::<Self>());
        let from = mem::ManuallyDrop::new(from);
        let ptr: *const Self = (&*from as *const Self::Output).cast();
        ptr::read(ptr)
    }
    #[inline]
    fn transform_mut<F>(&'a mut self, f: F)
    where
        F: 'static + for<'b> FnOnce(&'b mut Self::Output),
    {
        unsafe { f(mem::transmute::<&mut Self, &mut Self::Output>(self)) }
    }
}

/// This impl requires enabling the optional `yoke` Cargo feature of the `zerovec` crate
#[cfg(feature = "alloc")]
unsafe impl<'a, K, V> Yokeable<'a> for ZeroMap<'static, K, V>
where
    K: 'static + for<'b> ZeroMapKV<'b> + ?Sized,
    V: 'static + for<'b> ZeroMapKV<'b> + ?Sized,
    <K as ZeroMapKV<'static>>::Container: for<'b> Yokeable<'b>,
    <V as ZeroMapKV<'static>>::Container: for<'b> Yokeable<'b>,
{
    type Output = ZeroMap<'a, K, V>;
    #[inline]
    fn transform(&'a self) -> &'a Self::Output {
        unsafe {
            // Unfortunately, because K and V are generic, rustc is
            // unaware that these are covariant types, and cannot perform this cast automatically.
            // We transmute it instead, and enforce the lack of a lifetime with the `K, V: 'static` bound
            mem::transmute::<&Self, &Self::Output>(self)
        }
    }
    #[inline]
    fn transform_owned(self) -> Self::Output {
        debug_assert!(size_of::<Self::Output>() == size_of::<Self>());
        unsafe {
            // Similar problem as transform(), but we need to use ptr::read since
            // the compiler isn't sure of the sizes
            let this = mem::ManuallyDrop::new(self);
            let ptr: *const Self::Output = (&*this as *const Self).cast();
            ptr::read(ptr)
        }
    }
    #[inline]
    unsafe fn make(from: Self::Output) -> Self {
        debug_assert!(size_of::<Self::Output>() == size_of::<Self>());
        let from = mem::ManuallyDrop::new(from);
        let ptr: *const Self = (&*from as *const Self::Output).cast();
        ptr::read(ptr)
    }
    #[inline]
    fn transform_mut<F>(&'a mut self, f: F)
    where
        F: 'static + for<'b> FnOnce(&'b mut Self::Output),
    {
        unsafe { f(mem::transmute::<&mut Self, &mut Self::Output>(self)) }
    }
}

/// This impl requires enabling the optional `yoke` Cargo feature of the `zerovec` crate
#[cfg(feature = "alloc")]
unsafe impl<'a, K, V> Yokeable<'a> for ZeroMapBorrowed<'static, K, V>
where
    K: 'static + for<'b> ZeroMapKV<'b> + ?Sized,
    V: 'static + for<'b> ZeroMapKV<'b> + ?Sized,
    &'static <K as ZeroMapKV<'static>>::Slice: for<'b> Yokeable<'b>,
    &'static <V as ZeroMapKV<'static>>::Slice: for<'b> Yokeable<'b>,
{
    type Output = ZeroMapBorrowed<'a, K, V>;
    #[inline]
    fn transform(&'a self) -> &'a Self::Output {
        unsafe {
            // Unfortunately, because K and V are generic, rustc is
            // unaware that these are covariant types, and cannot perform this cast automatically.
            // We transmute it instead, and enforce the lack of a lifetime with the `K, V: 'static` bound
            mem::transmute::<&Self, &Self::Output>(self)
        }
    }
    #[inline]
    fn transform_owned(self) -> Self::Output {
        debug_assert!(size_of::<Self::Output>() == size_of::<Self>());
        unsafe {
            // Similar problem as transform(), but we need to use ptr::read since
            // the compiler isn't sure of the sizes
            let this = mem::ManuallyDrop::new(self);
            let ptr: *const Self::Output = (&*this as *const Self).cast();
            ptr::read(ptr)
        }
    }
    #[inline]
    unsafe fn make(from: Self::Output) -> Self {
        debug_assert!(size_of::<Self::Output>() == size_of::<Self>());
        let from = mem::ManuallyDrop::new(from);
        let ptr: *const Self = (&*from as *const Self::Output).cast();
        ptr::read(ptr)
    }
    #[inline]
    fn transform_mut<F>(&'a mut self, f: F)
    where
        F: 'static + for<'b> FnOnce(&'b mut Self::Output),
    {
        unsafe { f(mem::transmute::<&mut Self, &mut Self::Output>(self)) }
    }
}

/// This impl requires enabling the optional `yoke` Cargo feature of the `zerovec` crate
#[cfg(feature = "alloc")]
unsafe impl<'a, K0, K1, V> Yokeable<'a> for ZeroMap2d<'static, K0, K1, V>
where
    K0: 'static + for<'b> ZeroMapKV<'b> + ?Sized,
    K1: 'static + for<'b> ZeroMapKV<'b> + ?Sized,
    V: 'static + for<'b> ZeroMapKV<'b> + ?Sized,
    <K0 as ZeroMapKV<'static>>::Container: for<'b> Yokeable<'b>,
    <K1 as ZeroMapKV<'static>>::Container: for<'b> Yokeable<'b>,
    <V as ZeroMapKV<'static>>::Container: for<'b> Yokeable<'b>,
{
    type Output = ZeroMap2d<'a, K0, K1, V>;
    #[inline]
    fn transform(&'a self) -> &'a Self::Output {
        unsafe {
            // Unfortunately, because K and V are generic, rustc is
            // unaware that these are covariant types, and cannot perform this cast automatically.
            // We transmute it instead, and enforce the lack of a lifetime with the `K0, K1, V: 'static` bound
            mem::transmute::<&Self, &Self::Output>(self)
        }
    }
    #[inline]
    fn transform_owned(self) -> Self::Output {
        debug_assert!(size_of::<Self::Output>() == size_of::<Self>());
        unsafe {
            // Similar problem as transform(), but we need to use ptr::read since
            // the compiler isn't sure of the sizes
            let this = mem::ManuallyDrop::new(self);
            let ptr: *const Self::Output = (&*this as *const Self).cast();
            ptr::read(ptr)
        }
    }
    #[inline]
    unsafe fn make(from: Self::Output) -> Self {
        debug_assert!(size_of::<Self::Output>() == size_of::<Self>());
        let from = mem::ManuallyDrop::new(from);
        let ptr: *const Self = (&*from as *const Self::Output).cast();
        ptr::read(ptr)
    }
    #[inline]
    fn transform_mut<F>(&'a mut self, f: F)
    where
        F: 'static + for<'b> FnOnce(&'b mut Self::Output),
    {
        unsafe { f(mem::transmute::<&mut Self, &mut Self::Output>(self)) }
    }
}

/// This impl requires enabling the optional `yoke` Cargo feature of the `zerovec` crate
#[cfg(feature = "alloc")]
unsafe impl<'a, K0, K1, V> Yokeable<'a> for ZeroMap2dBorrowed<'static, K0, K1, V>
where
    K0: 'static + for<'b> ZeroMapKV<'b> + ?Sized,
    K1: 'static + for<'b> ZeroMapKV<'b> + ?Sized,
    V: 'static + for<'b> ZeroMapKV<'b> + ?Sized,
    &'static <K0 as ZeroMapKV<'static>>::Slice: for<'b> Yokeable<'b>,
    &'static <K1 as ZeroMapKV<'static>>::Slice: for<'b> Yokeable<'b>,
    &'static <V as ZeroMapKV<'static>>::Slice: for<'b> Yokeable<'b>,
{
    type Output = ZeroMap2dBorrowed<'a, K0, K1, V>;
    #[inline]
    fn transform(&'a self) -> &'a Self::Output {
        unsafe {
            // Unfortunately, because K and V are generic, rustc is
            // unaware that these are covariant types, and cannot perform this cast automatically.
            // We transmute it instead, and enforce the lack of a lifetime with the `K0, K1, V: 'static` bound
            mem::transmute::<&Self, &Self::Output>(self)
        }
    }
    #[inline]
    fn transform_owned(self) -> Self::Output {
        debug_assert!(size_of::<Self::Output>() == size_of::<Self>());
        unsafe {
            // Similar problem as transform(), but we need to use ptr::read since
            // the compiler isn't sure of the sizes
            let this = mem::ManuallyDrop::new(self);
            let ptr: *const Self::Output = (&*this as *const Self).cast();
            ptr::read(ptr)
        }
    }
    #[inline]
    unsafe fn make(from: Self::Output) -> Self {
        debug_assert!(size_of::<Self::Output>() == size_of::<Self>());
        let from = mem::ManuallyDrop::new(from);
        let ptr: *const Self = (&*from as *const Self::Output).cast();
        ptr::read(ptr)
    }
    #[inline]
    fn transform_mut<F>(&'a mut self, f: F)
    where
        F: 'static + for<'b> FnOnce(&'b mut Self::Output),
    {
        unsafe { f(mem::transmute::<&mut Self, &mut Self::Output>(self)) }
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/zerovec/unit/src/yoke_impls/test_297.rs"));
