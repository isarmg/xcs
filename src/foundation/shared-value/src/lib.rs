//! which is used to store immutable references to values.
//! This is useful for storing, for example, strings.
//!
//! Imagine this as an alternative to [`Cow`] with an additional, reference-counted
//! branch.
//!
//! ```rust
//! use shared_value::SharedValue;
//! use std::sync::Arc;
//!
//! let static_str = "foo";
//! let rc_str: Arc<str> = "bar".into();
//! let owned_str: String = "baz".into();
//!
//! fn uses_shared_value(value: impl Into<SharedValue<'static, str>>) {
//!     let mut value = value.into();
//!
//!     // ensures that the value is either a reference, or reference-counted
//!     // O(n) at worst
//!     let clone1 = value.clone_inplace();
//!
//!     // these subsequent clones are O(1)
//!     let clone2 = value.clone();
//!     let clone3 = value.clone();
//! }
//!
//! uses_shared_value(static_str);
//! uses_shared_value(rc_str);
//! uses_shared_value(owned_str);
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    borrow::{Borrow, Cow},
    ffi::{CStr, OsStr},
    fmt,
    hash::Hash,
    ops::{Add, Deref},
    path::Path,
    sync::Arc,
};

/// "Owned Clones Once": a smart pointer that can be either a reference,
/// an owned value, or a reference-counted pointer. This is useful for
/// storing immutable values, such as strings, in a way that is cheap to
/// clone and pass around.
///
/// The cost of the `Clone` implementation depends on the branch.  Cloning the [`SharedValue::Borrowed`]
/// variant simply copies the references (`O(1)`). Cloning the [`SharedValue::Counted`]
/// variant increments a reference count (`O(1)`). Cloning the [`SharedValue::Owned`]
/// variant requires an `O(n)` clone of the data.
///
/// For an amortized `O(1)` clone, you can use [`SharedValue::clone_inplace()`]. Using this method,
/// [`SharedValue::Borrowed`] and [`SharedValue::Counted`] are still `O(1)`. [`SharedValue::Owned`] does a single `O(n)`
/// clone, but converts the object to the [`SharedValue::Counted`] branch, which means future clones will
/// be `O(1)`.
///
/// In general, you'll either want to call `clone_inplace()` once, before sharing the `SharedValue` with
/// other parts of your application (so that all future clones are `O(1)`), or simply use this as
/// if it is a [`Cow`] with an additional branch for reference-counted values.
pub enum SharedValue<'a, T: ?Sized + ToOwned + 'a> {
    /// A static reference to a value.
    Borrowed(&'a T),
    /// A reference counted pointer to a value.
    Counted(Arc<T>),
    /// An owned value.
    Owned(<T as ToOwned>::Owned),
}

impl<T: ?Sized + ToOwned> SharedValue<'_, T> {
    /// Converts the value into an owned value.
    pub fn into_owned(self) -> <T as ToOwned>::Owned {
        match self {
            SharedValue::Borrowed(v) => v.to_owned(),
            SharedValue::Counted(v) => v.as_ref().to_owned(),
            SharedValue::Owned(v) => v,
        }
    }

    /// Checks if the value is [`SharedValue::Borrowed`].
    /// # Examples
    /// ```
    /// # use std::sync::Arc;
    /// # use shared_value::SharedValue;
    /// assert!(SharedValue::<str>::Borrowed("Hello").is_borrowed());
    /// assert!(!SharedValue::<str>::Counted(Arc::from("Hello")).is_borrowed());
    /// assert!(!SharedValue::<str>::Owned("Hello".to_string()).is_borrowed());
    /// ```
    pub const fn is_borrowed(&self) -> bool {
        matches!(self, SharedValue::Borrowed(_))
    }

    /// Checks if the value is [`SharedValue::Counted`].
    /// # Examples
    /// ```
    /// # use std::sync::Arc;
    /// # use shared_value::SharedValue;
    /// assert!(SharedValue::<str>::Counted(Arc::from("Hello")).is_counted());
    /// assert!(!SharedValue::<str>::Borrowed("Hello").is_counted());
    /// assert!(!SharedValue::<str>::Owned("Hello".to_string()).is_counted());
    /// ```
    pub const fn is_counted(&self) -> bool {
        matches!(self, SharedValue::Counted(_))
    }

    /// Checks if the value is [`SharedValue::Owned`].
    /// # Examples
    /// ```
    /// # use std::sync::Arc;
    /// # use shared_value::SharedValue;
    /// assert!(SharedValue::<str>::Owned("Hello".to_string()).is_owned());
    /// assert!(!SharedValue::<str>::Borrowed("Hello").is_owned());
    /// assert!(!SharedValue::<str>::Counted(Arc::from("Hello")).is_owned());
    /// ```
    pub const fn is_owned(&self) -> bool {
        matches!(self, SharedValue::Owned(_))
    }
}

impl<T: ?Sized + ToOwned> Deref for SharedValue<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        match self {
            SharedValue::Borrowed(v) => v,
            SharedValue::Owned(v) => v.borrow(),
            SharedValue::Counted(v) => v,
        }
    }
}

impl<T: ?Sized + ToOwned> Borrow<T> for SharedValue<'_, T> {
    #[inline(always)]
    fn borrow(&self) -> &T {
        self.deref()
    }
}

impl<T: ?Sized + ToOwned> AsRef<T> for SharedValue<'_, T> {
    #[inline(always)]
    fn as_ref(&self) -> &T {
        self.deref()
    }
}

impl AsRef<Path> for SharedValue<'_, str> {
    #[inline(always)]
    fn as_ref(&self) -> &Path {
        self.as_str().as_ref()
    }
}

impl AsRef<Path> for SharedValue<'_, OsStr> {
    #[inline(always)]
    fn as_ref(&self) -> &Path {
        self.as_os_str().as_ref()
    }
}

// --------------------------------------
// pub fn as_{slice}(&self) -> &{slice}
// --------------------------------------

impl SharedValue<'_, str> {
    /// Returns a `&str` slice of this [`SharedValue`].
    /// # Examples
    /// ```
    /// # use shared_value::SharedValue;
    /// let shared = SharedValue::<str>::Borrowed("Hello");
    /// let s: &str = shared.as_str();
    /// assert_eq!(s, "Hello");
    /// ```
    #[inline(always)]
    pub fn as_str(&self) -> &str {
        self
    }
}

impl SharedValue<'_, CStr> {
    /// Returns a `&CStr` slice of this [`SharedValue`].
    /// # Examples
    /// ```
    /// # use shared_value::SharedValue;
    /// use std::ffi::CStr;
    ///
    /// let shared =
    ///     SharedValue::<CStr>::Borrowed(CStr::from_bytes_with_nul(b"Hello\0").unwrap());
    /// let s: &CStr = shared.as_c_str();
    /// assert_eq!(s, CStr::from_bytes_with_nul(b"Hello\0").unwrap());
    /// ```
    #[inline(always)]
    pub fn as_c_str(&self) -> &CStr {
        self
    }
}

impl SharedValue<'_, OsStr> {
    /// Returns a `&OsStr` slice of this [`SharedValue`].
    /// # Examples
    /// ```
    /// # use shared_value::SharedValue;
    /// use std::ffi::OsStr;
    ///
    /// let shared = SharedValue::<OsStr>::Borrowed(OsStr::new("Hello"));
    /// let s: &OsStr = shared.as_os_str();
    /// assert_eq!(s, OsStr::new("Hello"));
    /// ```
    #[inline(always)]
    pub fn as_os_str(&self) -> &OsStr {
        self
    }
}

impl SharedValue<'_, Path> {
    /// Returns a `&Path` slice of this [`SharedValue`].
    /// # Examples
    /// ```
    /// # use shared_value::SharedValue;
    /// use std::path::Path;
    ///
    /// let shared = SharedValue::<Path>::Borrowed(Path::new("Hello"));
    /// let s: &Path = shared.as_path();
    /// assert_eq!(s, Path::new("Hello"));
    /// ```
    #[inline(always)]
    pub fn as_path(&self) -> &Path {
        self
    }
}

impl<T> SharedValue<'_, [T]>
where
    [T]: ToOwned,
{
    /// Returns a `&[T]` slice of this [`SharedValue`].
    /// # Examples
    /// ```
    /// # use shared_value::SharedValue;
    /// let shared = SharedValue::<[u8]>::Borrowed(b"Hello");
    /// let s: &[u8] = shared.as_slice();
    /// assert_eq!(s, b"Hello");
    /// ```
    #[inline(always)]
    pub fn as_slice(&self) -> &[T] {
        self
    }
}

impl<'a, T> Clone for SharedValue<'a, T>
where
    T: ?Sized + ToOwned + 'a,
    for<'b> Arc<T>: From<&'b T>,
{
    /// Returns a new [`SharedValue`] with the same value as this one.
    /// If the value is [`SharedValue::Owned`], this will convert it into
    /// [`SharedValue::Counted`], so that the next clone will be O(1).
    /// # Examples
    /// [`String`] :
    /// ```
    /// # use shared_value::SharedValue;
    /// let shared = SharedValue::<str>::Owned("Hello".to_string());
    /// let shared2 = shared.clone();
    /// assert_eq!(shared, shared2);
    /// assert!(shared2.is_counted());
    /// ```
    /// [`Vec`] :
    /// ```
    /// # use shared_value::SharedValue;
    /// let shared = SharedValue::<[u8]>::Owned(b"Hello".to_vec());
    /// let shared2 = shared.clone();
    /// assert_eq!(shared, shared2);
    /// assert!(shared2.is_counted());
    /// ```
    fn clone(&self) -> Self {
        match self {
            Self::Borrowed(v) => Self::Borrowed(v),
            Self::Counted(v) => Self::Counted(Arc::clone(v)),
            Self::Owned(v) => Self::Counted(Arc::from(v.borrow())),
        }
    }
}

impl<'a, T> SharedValue<'a, T>
where
    T: ?Sized + ToOwned + 'a,
    for<'b> Arc<T>: From<&'b T>,
{
    /// Upgrades the value in place, by converting into [`SharedValue::Counted`] if it
    /// was previously [`SharedValue::Owned`].
    /// # Examples
    /// ```
    /// # use shared_value::SharedValue;
    /// let mut shared1 = SharedValue::<str>::Owned("Hello".to_string());
    /// assert!(shared1.is_owned());
    /// shared1.upgrade_inplace();
    /// assert!(shared1.is_counted());
    /// ```
    pub fn upgrade_inplace(&mut self) {
        if let Self::Owned(v) = &*self {
            let rc = Arc::from(v.borrow());
            *self = Self::Counted(rc);
        }
    }

    /// Clones the value with inplace conversion into [`SharedValue::Counted`] if it
    /// was previously [`SharedValue::Owned`].
    /// # Examples
    /// ```
    /// # use shared_value::SharedValue;
    /// let mut shared1 = SharedValue::<str>::Owned("Hello".to_string());
    /// let shared2 = shared1.clone_inplace();
    /// assert_eq!(shared1, shared2);
    /// assert!(shared1.is_counted());
    /// assert!(shared2.is_counted());
    /// ```
    pub fn clone_inplace(&mut self) -> Self {
        match &*self {
            Self::Borrowed(v) => Self::Borrowed(v),
            Self::Counted(v) => Self::Counted(Arc::clone(v)),
            Self::Owned(v) => {
                let rc = Arc::from(v.borrow());
                *self = Self::Counted(rc.clone());
                Self::Counted(rc)
            }
        }
    }
}

impl<T: ?Sized> Default for SharedValue<'_, T>
where
    T: ToOwned,
    T::Owned: Default,
{
    fn default() -> Self {
        SharedValue::Owned(T::Owned::default())
    }
}

impl<'b, A: ?Sized, B: ?Sized> PartialEq<SharedValue<'b, B>> for SharedValue<'_, A>
where
    A: PartialEq<B>,
    A: ToOwned,
    B: ToOwned,
{
    fn eq(&self, other: &SharedValue<'b, B>) -> bool {
        **self == **other
    }
}

impl<T: ?Sized + ToOwned + Eq> Eq for SharedValue<'_, T> {}

impl<'b, A: ?Sized, B: ?Sized> PartialOrd<SharedValue<'b, B>> for SharedValue<'_, A>
where
    A: PartialOrd<B>,
    A: ToOwned,
    B: ToOwned,
{
    fn partial_cmp(&self, other: &SharedValue<'b, B>) -> Option<std::cmp::Ordering> {
        (**self).partial_cmp(&**other)
    }
}

impl<T: ?Sized + Ord> Ord for SharedValue<'_, T>
where
    T: ToOwned,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (**self).cmp(&**other)
    }
}

impl<T: ?Sized + Hash> Hash for SharedValue<'_, T>
where
    T: ToOwned,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (**self).hash(state)
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for SharedValue<'_, T>
where
    T: ToOwned,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl<T: ?Sized + fmt::Display> fmt::Display for SharedValue<'_, T>
where
    T: ToOwned,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl<'a, T: ?Sized> From<&'a T> for SharedValue<'a, T>
where
    T: ToOwned,
{
    fn from(v: &'a T) -> Self {
        SharedValue::Borrowed(v)
    }
}

impl<'a, T: ?Sized> From<Cow<'a, T>> for SharedValue<'a, T>
where
    T: ToOwned,
{
    fn from(v: Cow<'a, T>) -> Self {
        match v {
            Cow::Borrowed(v) => SharedValue::Borrowed(v),
            Cow::Owned(v) => SharedValue::Owned(v),
        }
    }
}

impl<'a, T: ?Sized> From<SharedValue<'a, T>> for Cow<'a, T>
where
    T: ToOwned,
{
    fn from(value: SharedValue<'a, T>) -> Self {
        match value {
            SharedValue::Borrowed(v) => Cow::Borrowed(v),
            SharedValue::Owned(v) => Cow::Owned(v),
            SharedValue::Counted(v) => Cow::Owned(v.as_ref().to_owned()),
        }
    }
}

impl<T: ?Sized> From<Arc<T>> for SharedValue<'_, T>
where
    T: ToOwned,
{
    fn from(v: Arc<T>) -> Self {
        SharedValue::Counted(v)
    }
}

impl<T: ?Sized> From<Box<T>> for SharedValue<'_, T>
where
    T: ToOwned,
{
    fn from(v: Box<T>) -> Self {
        SharedValue::Counted(v.into())
    }
}

impl From<String> for SharedValue<'_, str> {
    fn from(v: String) -> Self {
        SharedValue::Owned(v)
    }
}

impl From<SharedValue<'_, str>> for String {
    fn from(v: SharedValue<'_, str>) -> Self {
        match v {
            SharedValue::Borrowed(v) => v.to_owned(),
            SharedValue::Counted(v) => v.as_ref().to_owned(),
            SharedValue::Owned(v) => v,
        }
    }
}

impl<T> From<Vec<T>> for SharedValue<'_, [T]>
where
    [T]: ToOwned<Owned = Vec<T>>,
{
    fn from(v: Vec<T>) -> Self {
        SharedValue::Owned(v)
    }
}

impl<'a, T, const N: usize> From<&'a [T; N]> for SharedValue<'a, [T]>
where
    [T]: ToOwned,
{
    fn from(v: &'a [T; N]) -> Self {
        SharedValue::Borrowed(v)
    }
}

impl<'a> From<SharedValue<'a, str>> for SharedValue<'a, [u8]> {
    fn from(v: SharedValue<'a, str>) -> Self {
        match v {
            SharedValue::Borrowed(v) => SharedValue::Borrowed(v.as_bytes()),
            SharedValue::Owned(v) => SharedValue::Owned(v.into_bytes()),
            SharedValue::Counted(v) => SharedValue::Counted(v.into()),
        }
    }
}

/// Error returned from `SharedValue::try_from` for unsuccessful
/// conversion from `SharedValue<'_, [u8]>` to `SharedValue<'_, str>`.
#[derive(Debug, Clone, thiserror::Error)]
#[error("invalid utf-8 sequence: {_0}")]
pub enum FromUtf8Error {
    /// Error for conversion of [`SharedValue::Borrowed`] and [`SharedValue::Counted`] variants
    /// (`&[u8]` to `&str`).
    #[error("{_0}")]
    StrFromBytes(
        #[source]
        #[from]
        std::str::Utf8Error,
    ),
    /// Error for conversion of [`SharedValue::Owned`] variant (`Vec<u8>` to `String`).
    #[error("{_0}")]
    StringFromBytes(
        #[source]
        #[from]
        std::string::FromUtf8Error,
    ),
}

macro_rules! impl_slice_eq {
    ([$($g:tt)*] $((where $($w:tt)+))?, $lhs:ty, $rhs: ty) => {
        impl<$($g)*> PartialEq<$rhs> for $lhs
        $(where
            $($w)*)?
        {
            #[inline]
            fn eq(&self, other: &$rhs) -> bool {
                PartialEq::eq(&self[..], &other[..])
            }
        }

        impl<$($g)*> PartialEq<$lhs> for $rhs
        $(where
            $($w)*)?
        {
            #[inline]
            fn eq(&self, other: &$lhs) -> bool {
                PartialEq::eq(&self[..], &other[..])
            }
        }
    };
}

impl_slice_eq!([], SharedValue<'_, str>, str);
impl_slice_eq!(['a, 'b], SharedValue<'a, str>, &'b str);
impl_slice_eq!([], SharedValue<'_, str>, String);
impl_slice_eq!(['a, 'b], SharedValue<'a, str>, Cow<'b, str>);

impl_slice_eq!([T: PartialEq] (where [T]: ToOwned), SharedValue<'_, [T]>, [T]);
impl_slice_eq!(['a, 'b, T: PartialEq] (where [T]: ToOwned), SharedValue<'a, [T]>, &'b [T]);
impl_slice_eq!([T: PartialEq] (where [T]: ToOwned), SharedValue<'_, [T]>, Vec<T>);
impl_slice_eq!(['a, 'b, T: PartialEq] (where [T]: ToOwned), SharedValue<'a, [T]>, Cow<'b, [T]>);

impl<'b> Add<&'b str> for SharedValue<'_, str> {
    type Output = SharedValue<'static, str>;

    fn add(self, rhs: &'b str) -> Self::Output {
        SharedValue::Owned(String::from(self) + rhs)
    }
}

impl<'b> Add<Cow<'b, str>> for SharedValue<'_, str> {
    type Output = SharedValue<'static, str>;

    fn add(self, rhs: Cow<'b, str>) -> Self::Output {
        SharedValue::Owned(String::from(self) + rhs.as_ref())
    }
}

impl<'b> Add<SharedValue<'b, str>> for SharedValue<'_, str> {
    type Output = SharedValue<'static, str>;

    fn add(self, rhs: SharedValue<'b, str>) -> Self::Output {
        SharedValue::Owned(String::from(self) + rhs.as_ref())
    }
}

impl<'a> FromIterator<SharedValue<'a, str>> for String {
    fn from_iter<T: IntoIterator<Item = SharedValue<'a, str>>>(iter: T) -> Self {
        iter.into_iter().fold(String::new(), |mut acc, item| {
            acc.push_str(item.as_ref());
            acc
        })
    }
}

impl<'a, T> Deserialize<'a> for SharedValue<'static, T>
where
    T: ?Sized + ToOwned + 'a,
    T::Owned: DeserializeOwned,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'a>,
    {
        <T::Owned>::deserialize(deserializer).map(SharedValue::Owned)
    }
}

impl<'a, T> Serialize for SharedValue<'a, T>
where
    T: ?Sized + ToOwned + 'a,
    for<'b> &'b T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.as_ref().serialize(serializer)
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/shared-value/unit/src/lib/tests_613.rs"));
