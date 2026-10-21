// Adapted from RustCrypto digest 0.10.7 core_api/rt_variable.rs and src/lib.rs.
// Original license and source provenance remain under compliance/provenance.
#[cfg(feature = "alloc")]
use alloc::{boxed::Box, vec};
use core::fmt;
use digest::{
    block_api::{AlgorithmName, TruncSide, UpdateCore, VariableOutputCore},
    block_buffer::BlockBuffer,
    typenum::Unsigned,
    HashMarker, InvalidOutputSize, MacMarker, Reset, Update,
};
/// Trait for hash functions with variable-size output.
pub trait VariableOutput: Sized + Update {
    /// Maximum size of output hash.
    const MAX_OUTPUT_SIZE: usize;

    /// Create new hasher instance with the given output size.
    ///
    /// It will return `Err(InvalidOutputSize)` in case if hasher can not return
    /// hash of the specified output size.
    fn new(output_size: usize) -> Result<Self, InvalidOutputSize>;

    /// Get output size of the hasher instance provided to the `new` method
    fn output_size(&self) -> usize;

    /// Write result into the output buffer.
    ///
    /// Returns `Err(InvalidBufferSize)` if `out` size is not equal to
    /// `self.output_size()`.
    fn finalize_variable(self, out: &mut [u8]) -> Result<(), InvalidBufferSize>;

    /// Compute hash of `data` and write it to `output`.
    ///
    /// Length of the output hash is determined by `output`. If `output` is
    /// bigger than `Self::MAX_OUTPUT_SIZE`, this method returns
    /// `InvalidOutputSize`.
    fn digest_variable(
        input: impl AsRef<[u8]>,
        output: &mut [u8],
    ) -> Result<(), InvalidOutputSize> {
        let mut hasher = Self::new(output.len())?;
        hasher.update(input.as_ref());
        hasher
            .finalize_variable(output)
            .map_err(|_| InvalidOutputSize)
    }

    /// Retrieve result into a boxed slice and consume hasher.
    ///
    /// `Box<[u8]>` is used instead of `Vec<u8>` to save stack space, since
    /// they have size of 2 and 3 words respectively.
    #[cfg(feature = "alloc")]
    #[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
    fn finalize_boxed(self) -> Box<[u8]> {
        let n = self.output_size();
        let mut buf = vec![0u8; n].into_boxed_slice();
        self.finalize_variable(&mut buf)
            .expect("buf length is equal to output_size");
        buf
    }
}

/// Trait for hash functions with variable-size output able to reset themselves.
pub trait VariableOutputReset: VariableOutput + Reset {
    /// Write result into the output buffer and reset the hasher state.
    ///
    /// Returns `Err(InvalidBufferSize)` if `out` size is not equal to
    /// `self.output_size()`.
    fn finalize_variable_reset(&mut self, out: &mut [u8]) -> Result<(), InvalidBufferSize>;

    /// Retrieve result into a boxed slice and reset the hasher state.
    ///
    /// `Box<[u8]>` is used instead of `Vec<u8>` to save stack space, since
    /// they have size of 2 and 3 words respectively.
    #[cfg(feature = "alloc")]
    #[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
    fn finalize_boxed_reset(&mut self) -> Box<[u8]> {
        let n = self.output_size();
        let mut buf = vec![0u8; n].into_boxed_slice();
        self.finalize_variable_reset(&mut buf)
            .expect("buf length is equal to output_size");
        buf
    }
}

/// Buffer length is not equal to hash output size.
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub struct InvalidBufferSize;

impl fmt::Display for InvalidBufferSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid buffer length")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for InvalidBufferSize {}
/// Wrapper around [`VariableOutputCore`] which selects output size
/// at run time.
#[derive(Clone)]
pub struct RuntimeOutputHash<T>
where
    T: VariableOutputCore + UpdateCore,
{
    core: T,
    buffer: BlockBuffer<T::BlockSize, T::BufferKind>,
    output_size: usize,
}

impl<T> RuntimeOutputHash<T>
where
    T: VariableOutputCore,
{
    #[inline]
    fn finalize_dirty(&mut self, out: &mut [u8]) -> Result<(), InvalidBufferSize> {
        let Self {
            core,
            buffer,
            output_size,
        } = self;
        if out.len() != *output_size || out.len() > Self::MAX_OUTPUT_SIZE {
            return Err(InvalidBufferSize);
        }
        let mut full_res = Default::default();
        core.finalize_variable_core(buffer, &mut full_res);
        let n = out.len();
        let m = full_res.len() - n;
        match T::TRUNC_SIDE {
            TruncSide::Left => out.copy_from_slice(&full_res[..n]),
            TruncSide::Right => out.copy_from_slice(&full_res[m..]),
        }
        Ok(())
    }
}

impl<T> HashMarker for RuntimeOutputHash<T> where T: VariableOutputCore + HashMarker {}

impl<T> MacMarker for RuntimeOutputHash<T> where T: VariableOutputCore + MacMarker {}

impl<T> Reset for RuntimeOutputHash<T>
where
    T: VariableOutputCore + UpdateCore + Reset,
{
    #[inline]
    fn reset(&mut self) {
        self.buffer.reset();
        self.core.reset();
    }
}

impl<T> Update for RuntimeOutputHash<T>
where
    T: VariableOutputCore + UpdateCore,
{
    #[inline]
    fn update(&mut self, input: &[u8]) {
        let Self { core, buffer, .. } = self;
        buffer.digest_blocks(input, |blocks| core.update_blocks(blocks));
    }
}

impl<T> VariableOutput for RuntimeOutputHash<T>
where
    T: VariableOutputCore + UpdateCore,
{
    const MAX_OUTPUT_SIZE: usize = T::OutputSize::USIZE;

    fn new(output_size: usize) -> Result<Self, InvalidOutputSize> {
        let buffer = Default::default();
        T::new(output_size).map(|core| Self {
            core,
            buffer,
            output_size,
        })
    }

    fn output_size(&self) -> usize {
        self.output_size
    }

    fn finalize_variable(mut self, out: &mut [u8]) -> Result<(), InvalidBufferSize> {
        self.finalize_dirty(out)
    }
}

impl<T> VariableOutputReset for RuntimeOutputHash<T>
where
    T: VariableOutputCore + UpdateCore + Reset,
{
    fn finalize_variable_reset(&mut self, out: &mut [u8]) -> Result<(), InvalidBufferSize> {
        self.finalize_dirty(out)?;
        self.core.reset();
        self.buffer.reset();
        Ok(())
    }
}

impl<T> fmt::Debug for RuntimeOutputHash<T>
where
    T: VariableOutputCore + UpdateCore + AlgorithmName,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        T::write_alg_name(f)?;
        f.write_str(" { .. }")
    }
}

#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
impl<T> std::io::Write for RuntimeOutputHash<T>
where
    T: VariableOutputCore + UpdateCore,
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
