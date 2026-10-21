//! Defining custom `Reader`s quickly.

use alloc::borrow::Cow;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::sync::Arc;
use core::fmt::Debug;
use core::hash::{Hash, Hasher};
use core::ops::{Deref, Index, Range, RangeFrom, RangeTo};
use core::slice;
use core::str;
use stable_deref_trait::CloneStableDeref;

use crate::endianity::Endianity;
use crate::read::{Error, Reader, ReaderOffsetId, Result};

/// A reference counted, non-thread-safe slice of bytes and associated
/// endianity.
///
/// ```
/// # #[cfg(feature = "std")] {
/// use std::rc::Rc;
///
/// let buf = Rc::from(&[1, 2, 3, 4][..]);
/// let reader = gimli::EndianRcSlice::new(buf, gimli::NativeEndian);
/// # let _ = reader;
/// # }
/// ```
pub type EndianRcSlice<Endian> = EndianReader<Endian, Rc<[u8]>>;

/// An atomically reference counted, thread-safe slice of bytes and associated
/// endianity.
///
/// ```
/// # #[cfg(feature = "std")] {
/// use std::sync::Arc;
///
/// let buf = Arc::from(&[1, 2, 3, 4][..]);
/// let reader = gimli::EndianArcSlice::new(buf, gimli::NativeEndian);
/// # let _ = reader;
/// # }
/// ```
pub type EndianArcSlice<Endian> = EndianReader<Endian, Arc<[u8]>>;

/// An easy way to define a custom `Reader` implementation with a reference to a
/// generic buffer of bytes and an associated endianity.
///
/// Note that the whole original buffer is kept alive in memory even if there is
/// only one reader that references only a handful of bytes from that original
/// buffer. That is, `EndianReader` will not do any copying, moving, or
/// compacting in order to free up unused regions of the original buffer. If you
/// require this kind of behavior, it is up to you to implement `Reader`
/// directly by-hand.
///
/// # Example
///
/// Say you have an `mmap`ed file that you want to serve as a `gimli::Reader`.
/// You can wrap that `mmap`ed file up in a `MmapFile` type and use
/// `EndianReader<Rc<MmapFile>>` or `EndianReader<Arc<MmapFile>>` as readers as
/// long as `MmapFile` dereferences to the underlying `[u8]` data.
///
/// ```
/// use std::io;
/// use std::ops::Deref;
/// use std::path::Path;
/// use std::slice;
/// use std::sync::Arc;
///
/// /// A type that represents an `mmap`ed file.
/// #[derive(Debug)]
/// pub struct MmapFile {
///     ptr: *const u8,
///     len: usize,
/// }
///
/// impl MmapFile {
///     pub fn new(path: &Path) -> io::Result<MmapFile> {
///         // Call `mmap` and check for errors and all that...
/// #       unimplemented!()
///     }
/// }
///
/// impl Drop for MmapFile {
///     fn drop(&mut self) {
///         // Call `munmap` to clean up after ourselves...
/// #       unimplemented!()
///     }
/// }
///
/// // And `MmapFile` can deref to a slice of the `mmap`ed region of memory.
/// impl Deref for MmapFile {
///     type Target = [u8];
///     fn deref(&self) -> &[u8] {
///         unsafe {
///             slice::from_raw_parts(self.ptr, self.len)
///         }
///     }
/// }
///
/// /// A type that represents a shared `mmap`ed file.
/// #[derive(Debug, Clone)]
/// pub struct ArcMmapFile(Arc<MmapFile>);
///
/// // And `ArcMmapFile` can deref to a slice of the `mmap`ed region of memory.
/// impl Deref for ArcMmapFile {
///     type Target = [u8];
///     fn deref(&self) -> &[u8] {
///         &self.0
///     }
/// }
///
/// // These are both valid for any `Rc` or `Arc`.
/// unsafe impl gimli::StableDeref for ArcMmapFile {}
/// unsafe impl gimli::CloneStableDeref for ArcMmapFile {}
///
/// /// A `gimli::Reader` that is backed by an `mmap`ed file!
/// pub type MmapFileReader<Endian> = gimli::EndianReader<Endian, ArcMmapFile>;
/// # fn test(_: &MmapFileReader<gimli::NativeEndian>) { }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct EndianReader<Endian, T>
where
    Endian: Endianity,
    T: CloneStableDeref<Target = [u8]> + Debug,
{
    range: SubRange<T>,
    endian: Endian,
}

impl<Endian, T1, T2> PartialEq<EndianReader<Endian, T2>> for EndianReader<Endian, T1>
where
    Endian: Endianity,
    T1: CloneStableDeref<Target = [u8]> + Debug,
    T2: CloneStableDeref<Target = [u8]> + Debug,
{
    fn eq(&self, rhs: &EndianReader<Endian, T2>) -> bool {
        self.bytes() == rhs.bytes()
    }
}

impl<Endian, T> Eq for EndianReader<Endian, T>
where
    Endian: Endianity,
    T: CloneStableDeref<Target = [u8]> + Debug,
{
}

impl<Endian, T> Hash for EndianReader<Endian, T>
where
    Endian: Endianity,
    T: CloneStableDeref<Target = [u8]> + Debug,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        // This must match the `PartialEq` implementation.
        self.bytes().hash(state);
    }
}

// This is separated out from `EndianReader` so that we can avoid running afoul
// of borrowck. We need to `read_slice(&mut self, ...) -> &[u8]` and then call
// `self.endian.read_whatever` on the result. The problem is that the returned
// slice keeps the `&mut self` borrow active, so we wouldn't be able to access
// `self.endian`. Splitting the sub-range out from the endian lets us work
// around this, making it so that only the `self.range` borrow is held active,
// not all of `self`.
//
// This also serves to encapsulate the unsafe code concerning `CloneStableDeref`.
// The `bytes` member is held so that the bytes live long enough, and the
// `CloneStableDeref` ensures these bytes never move.  The `ptr` and `len`
// members point inside `bytes`, and are updated during read operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct SubRange<T>
where
    T: CloneStableDeref<Target = [u8]> + Debug,
{
    bytes: T,
    ptr: *const u8,
    len: usize,
}

unsafe impl<T> Send for SubRange<T> where T: CloneStableDeref<Target = [u8]> + Debug + Send {}

unsafe impl<T> Sync for SubRange<T> where T: CloneStableDeref<Target = [u8]> + Debug + Sync {}

impl<T> SubRange<T>
where
    T: CloneStableDeref<Target = [u8]> + Debug,
{
    #[inline]
    fn new(bytes: T) -> Self {
        let ptr = bytes.as_ptr();
        let len = bytes.len();
        SubRange { bytes, ptr, len }
    }

    #[inline]
    fn bytes(&self) -> &[u8] {
        // Safe because `T` implements `CloneStableDeref`, `bytes` can't be modified,
        // and all operations that modify `ptr` and `len` ensure they stay in range.
        unsafe { slice::from_raw_parts(self.ptr, self.len) }
    }

    #[inline]
    fn len(&self) -> usize {
        self.len
    }

    #[inline]
    fn truncate(&mut self, len: usize) {
        assert!(len <= self.len);
        self.len = len;
    }

    #[inline]
    fn skip(&mut self, len: usize) {
        assert!(len <= self.len);
        self.ptr = unsafe { self.ptr.add(len) };
        self.len -= len;
    }

    #[inline]
    fn read_slice(&mut self, len: usize) -> Option<&[u8]> {
        if self.len() < len {
            None
        } else {
            // Same as for `bytes()`.
            let bytes = unsafe { slice::from_raw_parts(self.ptr, len) };
            self.skip(len);
            Some(bytes)
        }
    }
}

impl<Endian, T> EndianReader<Endian, T>
where
    Endian: Endianity,
    T: CloneStableDeref<Target = [u8]> + Debug,
{
    /// Construct a new `EndianReader` with the given bytes.
    #[inline]
    pub fn new(bytes: T, endian: Endian) -> EndianReader<Endian, T> {
        EndianReader {
            range: SubRange::new(bytes),
            endian,
        }
    }

    /// Return a reference to the raw bytes underlying this reader.
    #[inline]
    pub fn bytes(&self) -> &[u8] {
        self.range.bytes()
    }
}

/// # Range Methods
///
/// Unfortunately, `std::ops::Index` *must* return a reference, so we can't
/// implement `Index<Range<usize>>` to return a new `EndianReader` the way we
/// would like to. Instead, we abandon fancy indexing operators and have these
/// plain old methods.
impl<Endian, T> EndianReader<Endian, T>
where
    Endian: Endianity,
    T: CloneStableDeref<Target = [u8]> + Debug,
{
    /// Take the given `start..end` range of the underlying buffer and return a
    /// new `EndianReader`.
    ///
    /// ```
    /// # #[cfg(feature = "std")] {
    /// use gimli::{EndianReader, LittleEndian};
    /// use std::sync::Arc;
    ///
    /// let buf = Arc::<[u8]>::from(&[0x01, 0x02, 0x03, 0x04][..]);
    /// let reader = EndianReader::new(buf.clone(), LittleEndian);
    /// assert_eq!(reader.range(1..3),
    ///            EndianReader::new(&buf[1..3], LittleEndian));
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the range is out of bounds.
    pub fn range(&self, idx: Range<usize>) -> EndianReader<Endian, T> {
        let mut r = self.clone();
        r.range.skip(idx.start);
        r.range.truncate(idx.len());
        r
    }

    /// Take the given `start..` range of the underlying buffer and return a new
    /// `EndianReader`.
    ///
    /// ```
    /// # #[cfg(feature = "std")] {
    /// use gimli::{EndianReader, LittleEndian};
    /// use std::sync::Arc;
    ///
    /// let buf = Arc::<[u8]>::from(&[0x01, 0x02, 0x03, 0x04][..]);
    /// let reader = EndianReader::new(buf.clone(), LittleEndian);
    /// assert_eq!(reader.range_from(2..),
    ///            EndianReader::new(&buf[2..], LittleEndian));
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the range is out of bounds.
    pub fn range_from(&self, idx: RangeFrom<usize>) -> EndianReader<Endian, T> {
        let mut r = self.clone();
        r.range.skip(idx.start);
        r
    }

    /// Take the given `..end` range of the underlying buffer and return a new
    /// `EndianReader`.
    ///
    /// ```
    /// # #[cfg(feature = "std")] {
    /// use gimli::{EndianReader, LittleEndian};
    /// use std::sync::Arc;
    ///
    /// let buf = Arc::<[u8]>::from(&[0x01, 0x02, 0x03, 0x04][..]);
    /// let reader = EndianReader::new(buf.clone(), LittleEndian);
    /// assert_eq!(reader.range_to(..3),
    ///            EndianReader::new(&buf[..3], LittleEndian));
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the range is out of bounds.
    pub fn range_to(&self, idx: RangeTo<usize>) -> EndianReader<Endian, T> {
        let mut r = self.clone();
        r.range.truncate(idx.end);
        r
    }
}

impl<Endian, T> Index<usize> for EndianReader<Endian, T>
where
    Endian: Endianity,
    T: CloneStableDeref<Target = [u8]> + Debug,
{
    type Output = u8;
    fn index(&self, idx: usize) -> &Self::Output {
        &self.bytes()[idx]
    }
}

impl<Endian, T> Index<RangeFrom<usize>> for EndianReader<Endian, T>
where
    Endian: Endianity,
    T: CloneStableDeref<Target = [u8]> + Debug,
{
    type Output = [u8];
    fn index(&self, idx: RangeFrom<usize>) -> &Self::Output {
        &self.bytes()[idx]
    }
}

impl<Endian, T> Deref for EndianReader<Endian, T>
where
    Endian: Endianity,
    T: CloneStableDeref<Target = [u8]> + Debug,
{
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        self.bytes()
    }
}

impl<Endian, T> Reader for EndianReader<Endian, T>
where
    Endian: Endianity,
    T: CloneStableDeref<Target = [u8]> + Debug,
{
    type Endian = Endian;
    type Offset = usize;

    #[inline]
    fn endian(&self) -> Endian {
        self.endian
    }

    #[inline]
    fn len(&self) -> usize {
        self.range.len()
    }

    #[inline]
    fn empty(&mut self) {
        self.range.truncate(0);
    }

    #[inline]
    fn truncate(&mut self, len: usize) -> Result<()> {
        if self.len() < len {
            Err(Error::UnexpectedEof(self.offset_id()))
        } else {
            self.range.truncate(len);
            Ok(())
        }
    }

    #[inline]
    fn offset_from(&self, base: &EndianReader<Endian, T>) -> usize {
        let base_ptr = base.bytes().as_ptr() as usize;
        let ptr = self.bytes().as_ptr() as usize;
        debug_assert!(base_ptr <= ptr);
        debug_assert!(ptr + self.bytes().len() <= base_ptr + base.bytes().len());
        ptr - base_ptr
    }

    #[inline]
    fn offset_id(&self) -> ReaderOffsetId {
        ReaderOffsetId(self.bytes().as_ptr() as u64)
    }

    #[inline]
    fn lookup_offset_id(&self, id: ReaderOffsetId) -> Option<Self::Offset> {
        let id = id.0;
        let self_id = self.bytes().as_ptr() as u64;
        let self_len = self.bytes().len() as u64;
        if id >= self_id && id <= self_id + self_len {
            Some((id - self_id) as usize)
        } else {
            None
        }
    }

    #[inline]
    fn find(&self, byte: u8) -> Result<usize> {
        self.bytes()
            .iter()
            .position(|x| *x == byte)
            .ok_or_else(|| Error::UnexpectedEof(self.offset_id()))
    }

    #[inline]
    fn skip(&mut self, len: usize) -> Result<()> {
        if self.len() < len {
            Err(Error::UnexpectedEof(self.offset_id()))
        } else {
            self.range.skip(len);
            Ok(())
        }
    }

    #[inline]
    fn split(&mut self, len: usize) -> Result<Self> {
        if self.len() < len {
            Err(Error::UnexpectedEof(self.offset_id()))
        } else {
            let mut r = self.clone();
            r.range.truncate(len);
            self.range.skip(len);
            Ok(r)
        }
    }

    #[inline]
    fn to_slice(&self) -> Result<Cow<'_, [u8]>> {
        Ok(self.bytes().into())
    }

    #[inline]
    fn to_string(&self) -> Result<Cow<'_, str>> {
        match str::from_utf8(self.bytes()) {
            Ok(s) => Ok(s.into()),
            _ => Err(Error::BadUtf8),
        }
    }

    #[inline]
    fn to_string_lossy(&self) -> Result<Cow<'_, str>> {
        Ok(String::from_utf8_lossy(self.bytes()))
    }

    #[inline]
    fn read_slice(&mut self, buf: &mut [u8]) -> Result<()> {
        match self.range.read_slice(buf.len()) {
            Some(slice) => {
                buf.copy_from_slice(slice);
                Ok(())
            }
            None => Err(Error::UnexpectedEof(self.offset_id())),
        }
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/gimli/unit/src/read/endian_reader/tests_492.rs"));
