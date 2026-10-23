// Adapted from RustCrypto digest 0.10.7 core_api/wrapper.rs.
// Original license and source provenance remain under compliance/provenance.
use core::fmt;
use cryptographic_core::{
    block_api::{AlgorithmName, Buffer, BufferKindUser, FixedOutputCore, UpdateCore},
    buffer::BlockBuffer,
    common_types::{BlockSizeUser, InvalidLength, Key, KeyInit, KeySizeUser},
    FixedOutput, FixedOutputReset, HashMarker, MacMarker, Output, OutputSizeUser, Reset, Update,
};
/// Wrapper around [`BufferKindUser`].
///
/// It handles data buffering and implements the slice-based traits.
#[derive(Clone, Default)]
pub struct BufferedHash<T>
where
    T: BufferKindUser,
{
    core: T,
    buffer: BlockBuffer<T::BlockSize, T::BufferKind>,
}

impl<T> HashMarker for BufferedHash<T> where T: BufferKindUser + HashMarker {}

impl<T> MacMarker for BufferedHash<T> where T: BufferKindUser + MacMarker {}

// this blanket impl is needed for HMAC
impl<T> BlockSizeUser for BufferedHash<T>
where
    T: BufferKindUser + HashMarker,
{
    type BlockSize = T::BlockSize;
}

impl<T> BufferedHash<T>
where
    T: BufferKindUser,
{
    /// Create new wrapper from `core`.
    #[inline]
    pub fn from_core(core: T) -> Self {
        let buffer = Default::default();
        Self { core, buffer }
    }

    /// Decompose wrapper into inner parts.
    #[inline]
    pub fn decompose(self) -> (T, Buffer<T>) {
        let Self { core, buffer } = self;
        (core, buffer)
    }
}

impl<T> KeySizeUser for BufferedHash<T>
where
    T: BufferKindUser + KeySizeUser,
{
    type KeySize = T::KeySize;
}

impl<T> KeyInit for BufferedHash<T>
where
    T: BufferKindUser + KeyInit,
{
    #[inline]
    fn new(key: &Key<Self>) -> Self {
        Self {
            core: T::new(key),
            buffer: Default::default(),
        }
    }

    #[inline]
    fn new_from_slice(key: &[u8]) -> Result<Self, InvalidLength> {
        Ok(Self {
            core: T::new_from_slice(key)?,
            buffer: Default::default(),
        })
    }
}

impl<T> fmt::Debug for BufferedHash<T>
where
    T: BufferKindUser + AlgorithmName,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        T::write_alg_name(f)?;
        f.write_str(" { .. }")
    }
}

impl<T> Reset for BufferedHash<T>
where
    T: BufferKindUser + Reset,
{
    #[inline]
    fn reset(&mut self) {
        self.core.reset();
        self.buffer.reset();
    }
}

impl<T> Update for BufferedHash<T>
where
    T: BufferKindUser + UpdateCore,
{
    #[inline]
    fn update(&mut self, input: &[u8]) {
        let Self { core, buffer } = self;
        buffer.digest_blocks(input, |blocks| core.update_blocks(blocks));
    }
}

impl<T> OutputSizeUser for BufferedHash<T>
where
    T: BufferKindUser + OutputSizeUser,
{
    type OutputSize = T::OutputSize;
}

impl<T> FixedOutput for BufferedHash<T>
where
    T: FixedOutputCore,
{
    #[inline]
    fn finalize_into(mut self, out: &mut Output<Self>) {
        let Self { core, buffer } = &mut self;
        core.finalize_fixed_core(buffer, out);
    }
}

impl<T> FixedOutputReset for BufferedHash<T>
where
    T: FixedOutputCore + Reset,
{
    #[inline]
    fn finalize_into_reset(&mut self, out: &mut Output<Self>) {
        let Self { core, buffer } = self;
        core.finalize_fixed_core(buffer, out);
        core.reset();
        buffer.reset();
    }
}

#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
impl<T> std::io::Write for BufferedHash<T>
where
    T: BufferKindUser + UpdateCore,
{
    #[inline]
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Update::update(self, buf);
        Ok(buf.len())
    }

    #[inline]
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<T: BufferKindUser> cryptographic_core::block_api::CoreProxy for BufferedHash<T> {
    type Core = T;
    fn compose(core: T, buffer: Buffer<T>) -> Self {
        Self { core, buffer }
    }
    fn decompose(self) -> (T, Buffer<T>) {
        (self.core, self.buffer)
    }
}
