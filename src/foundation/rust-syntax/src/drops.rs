use core::iter;
use core::mem::ManuallyDrop;
use core::ops::{Deref, DerefMut};
use core::option;
use core::slice;

#[repr(transparent)]
pub(crate) struct NoDrop<T: ?Sized>(ManuallyDrop<T>);

impl<T> NoDrop<T> {
    pub(crate) fn new(value: T) -> Self
    where
        T: TrivialDrop,
    {
        NoDrop(ManuallyDrop::new(value))
    }
}

impl<T: ?Sized> Deref for NoDrop<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: ?Sized> DerefMut for NoDrop<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub(crate) trait TrivialDrop {}

impl<T> TrivialDrop for iter::Empty<T> {}
impl<T> TrivialDrop for slice::Iter<'_, T> {}
impl<T> TrivialDrop for slice::IterMut<'_, T> {}
impl<T> TrivialDrop for option::IntoIter<&T> {}
impl<T> TrivialDrop for option::IntoIter<&mut T> {}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rust-syntax/functions/src/drops/test_needs_drop_40.rs"));
