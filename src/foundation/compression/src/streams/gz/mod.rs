use crate::streams::io::{BufRead, Error, ErrorKind, Read, Result, Write};
use alloc::boxed::Box;
use alloc::ffi::CString;
use alloc::vec::Vec;
use core::convert::TryFrom;
use core::time;

use crate::streams::bufreader::BufReader;
use crate::streams::{Compression, Crc};

pub static FHCRC: u8 = 1 << 1;
pub static FEXTRA: u8 = 1 << 2;
pub static FNAME: u8 = 1 << 3;
pub static FCOMMENT: u8 = 1 << 4;
pub static FRESERVED: u8 = 1 << 5 | 1 << 6 | 1 << 7;

pub mod bufread;
pub mod read;
pub mod write;

// The maximum length of the header filename and comment fields. More than
// enough for these fields in reasonable use, but prevents possible attacks.
const MAX_HEADER_BUF: usize = 65535;

/// A structure representing the header of a gzip stream.
///
/// The header can contain metadata about the file that was compressed, if
/// present.
#[derive(PartialEq, Clone, Debug, Default)]
pub struct GzHeader {
    extra: Option<Vec<u8>>,
    filename: Option<Vec<u8>>,
    comment: Option<Vec<u8>>,
    operating_system: u8,
    mtime: u32,
}

impl GzHeader {
    /// Returns the `filename` field of this gzip stream's header, if present.
    pub fn filename(&self) -> Option<&[u8]> {
        self.filename.as_ref().map(|s| &s[..])
    }

    /// Returns the `extra` field of this gzip stream's header, if present.
    pub fn extra(&self) -> Option<&[u8]> {
        self.extra.as_ref().map(|s| &s[..])
    }

    /// Returns the `comment` field of this gzip stream's header, if present.
    pub fn comment(&self) -> Option<&[u8]> {
        self.comment.as_ref().map(|s| &s[..])
    }

    /// Returns the `operating_system` field of this gzip stream's header.
    ///
    /// There are predefined values for various operating systems.
    /// 255 means that the value is unknown.
    pub fn operating_system(&self) -> u8 {
        self.operating_system
    }

    /// This gives the most recent modification time of the original file being compressed.
    ///
    /// The time is in Unix format, i.e., seconds since 00:00:00 GMT, Jan. 1, 1970.
    /// (Note that this may cause problems for MS-DOS and other systems that use local
    /// rather than Universal time.) If the compressed data did not come from a file,
    /// `mtime` is set to the time at which compression started.
    /// `mtime` = 0 means no time stamp is available.
    ///
    /// The usage of `mtime` is discouraged because of Year 2038 problem.
    pub fn mtime(&self) -> u32 {
        self.mtime
    }

    /// Returns the most recent modification time represented by a date-time type.
    /// Returns `None` if the value of the underlying counter is 0,
    /// indicating no time stamp is available.
    ///
    ///
    /// The time is measured as seconds since 00:00:00 GMT, Jan. 1 1970.
    /// See [`mtime`](#method.mtime) for more detail.
    pub fn mtime_as_datetime(&self) -> Option<std::time::SystemTime> {
        self.mtime_as_duration().map(|d| std::time::UNIX_EPOCH + d)
    }

    /// Returns the [`Duration`](time::Duration) between the most recent modification
    /// time and 00:00:00 GMT, Jan. 1 1970, also known as Unix epoch.
    /// See [`mtime`](#method.mtime) for more detail.
    /// Returns `None` if the value of the underlying counter is 0,
    /// indicating no time stamp is available.
    pub fn mtime_as_duration(&self) -> Option<time::Duration> {
        if self.mtime == 0 {
            None
        } else {
            let duration = time::Duration::new(u64::from(self.mtime), 0);
            Some(duration)
        }
    }
}

#[derive(Debug, Default)]
pub enum GzHeaderState {
    Start(u8, [u8; 10]),
    Xlen(Option<Box<Crc>>, u8, [u8; 2]),
    Extra(Option<Box<Crc>>, u16),
    Filename(Option<Box<Crc>>),
    Comment(Option<Box<Crc>>),
    Crc(Option<Box<Crc>>, u8, [u8; 2]),
    #[default]
    Complete,
}

#[derive(Debug, Default)]
pub struct GzHeaderParser {
    state: GzHeaderState,
    flags: u8,
    header: GzHeader,
}

impl GzHeaderParser {
    fn new() -> Self {
        GzHeaderParser {
            state: GzHeaderState::Start(0, [0; 10]),
            flags: 0,
            header: GzHeader::default(),
        }
    }

    fn parse<R: BufRead>(&mut self, r: &mut R) -> Result<()> {
        loop {
            match &mut self.state {
                GzHeaderState::Start(count, buffer) => {
                    while (*count as usize) < buffer.len() {
                        *count += read_into(r, &mut buffer[*count as usize..])? as u8;
                    }
                    // Gzip identification bytes
                    if buffer[0] != 0x1f || buffer[1] != 0x8b {
                        return Err(bad_header());
                    }
                    // Gzip compression method (8 = deflate)
                    if buffer[2] != 8 {
                        return Err(bad_header());
                    }
                    self.flags = buffer[3];
                    // RFC1952: "must give an error indication if any reserved bit is non-zero"
                    if self.flags & FRESERVED != 0 {
                        return Err(bad_header());
                    }
                    self.header.mtime = (buffer[4] as u32)
                        | ((buffer[5] as u32) << 8)
                        | ((buffer[6] as u32) << 16)
                        | ((buffer[7] as u32) << 24);
                    let _xfl = buffer[8];
                    self.header.operating_system = buffer[9];
                    let crc = if self.flags & FHCRC != 0 {
                        let mut crc = Box::new(Crc::new());
                        crc.update(buffer);
                        Some(crc)
                    } else {
                        None
                    };
                    self.state = GzHeaderState::Xlen(crc, 0, [0; 2]);
                }
                GzHeaderState::Xlen(crc, count, buffer) => {
                    if self.flags & FEXTRA != 0 {
                        while (*count as usize) < buffer.len() {
                            *count += read_into(r, &mut buffer[*count as usize..])? as u8;
                        }
                        if let Some(crc) = crc {
                            crc.update(buffer);
                        }
                        let xlen = parse_le_u16(buffer);
                        self.header.extra = Some(vec![0; xlen as usize]);
                        self.state = GzHeaderState::Extra(crc.take(), 0);
                    } else {
                        self.state = GzHeaderState::Filename(crc.take());
                    }
                }
                GzHeaderState::Extra(crc, count) => {
                    debug_assert!(self.header.extra.is_some());
                    let extra = self.header.extra.as_mut().unwrap();
                    while (*count as usize) < extra.len() {
                        *count += read_into(r, &mut extra[*count as usize..])? as u16;
                    }
                    if let Some(crc) = crc {
                        crc.update(extra);
                    }
                    self.state = GzHeaderState::Filename(crc.take());
                }
                GzHeaderState::Filename(crc) => {
                    if self.flags & FNAME != 0 {
                        let filename = self.header.filename.get_or_insert_with(Vec::new);
                        read_to_nul(r, filename)?;
                        if let Some(crc) = crc {
                            crc.update(filename);
                            crc.update(b"\0");
                        }
                    }
                    self.state = GzHeaderState::Comment(crc.take());
                }
                GzHeaderState::Comment(crc) => {
                    if self.flags & FCOMMENT != 0 {
                        let comment = self.header.comment.get_or_insert_with(Vec::new);
                        read_to_nul(r, comment)?;
                        if let Some(crc) = crc {
                            crc.update(comment);
                            crc.update(b"\0");
                        }
                    }
                    self.state = GzHeaderState::Crc(crc.take(), 0, [0; 2]);
                }
                GzHeaderState::Crc(crc, count, buffer) => {
                    if let Some(crc) = crc {
                        debug_assert!(self.flags & FHCRC != 0);
                        while (*count as usize) < buffer.len() {
                            *count += read_into(r, &mut buffer[*count as usize..])? as u8;
                        }
                        let stored_crc = parse_le_u16(buffer);
                        let calced_crc = crc.sum() as u16;
                        if stored_crc != calced_crc {
                            return Err(corrupt());
                        }
                    }
                    self.state = GzHeaderState::Complete;
                }
                GzHeaderState::Complete => {
                    return Ok(());
                }
            }
        }
    }

    fn header(&self) -> Option<&GzHeader> {
        match self.state {
            GzHeaderState::Complete => Some(&self.header),
            _ => None,
        }
    }
}

impl From<GzHeaderParser> for GzHeader {
    fn from(parser: GzHeaderParser) -> Self {
        debug_assert!(matches!(parser.state, GzHeaderState::Complete));
        parser.header
    }
}

// Attempt to fill the `buffer` from `r`. Return the number of bytes read.
// Return an error if EOF is read before the buffer is full.  This differs
// from `read` in that Ok(0) means that more data may be available.
fn read_into<R: Read>(r: &mut R, buffer: &mut [u8]) -> Result<usize> {
    debug_assert!(!buffer.is_empty());
    match r.read(buffer) {
        Ok(0) => Err(ErrorKind::UnexpectedEof.into()),
        Ok(n) => Ok(n),
        Err(ref e) if e.kind() == ErrorKind::Interrupted => Ok(0),
        Err(e) => Err(e),
    }
}

// Read `r` up to the first nul byte, pushing non-nul bytes to `buffer`.
fn read_to_nul<R: BufRead>(r: &mut R, buffer: &mut Vec<u8>) -> Result<()> {
    let mut bytes = r.bytes();
    loop {
        match bytes.next().transpose()? {
            Some(0) => return Ok(()),
            Some(_) if buffer.len() == MAX_HEADER_BUF => {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "gzip header field too long",
                ));
            }
            Some(byte) => {
                buffer.push(byte);
            }
            None => {
                return Err(ErrorKind::UnexpectedEof.into());
            }
        }
    }
}

fn parse_le_u16(buffer: &[u8; 2]) -> u16 {
    u16::from_le_bytes(*buffer)
}

fn bad_header() -> Error {
    Error::new(ErrorKind::InvalidInput, "invalid gzip header")
}

fn corrupt() -> Error {
    Error::new(
        ErrorKind::InvalidInput,
        "corrupt gzip stream does not have a matching checksum",
    )
}

/// A builder structure to create a new gzip Encoder.
///
/// This structure controls header configuration options such as the filename.
///
/// # Examples
///
/// ```
/// use std::io::prelude::*;
/// # use std::io;
/// use std::fs::File;
/// use compression::streams::GzBuilder;
/// use compression::streams::Compression;
///
/// // GzBuilder opens a file and writes a sample string using GzBuilder pattern
///
/// # fn sample_builder() -> Result<(), io::Error> {
/// let f = File::create("examples/hello_world.gz")?;
/// let mut gz = GzBuilder::new()
///                 .filename("hello_world.txt")
///                 .comment("test file, please delete")
///                 .write(f, Compression::default());
/// gz.write_all(b"hello world")?;
/// gz.finish()?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Default)]
pub struct GzBuilder {
    extra: Option<Vec<u8>>,
    filename: Option<CString>,
    comment: Option<CString>,
    operating_system: Option<u8>,
    mtime: u32,
}

impl GzBuilder {
    /// Create a new blank builder with no header by default.
    pub fn new() -> GzBuilder {
        Self::default()
    }

    /// Configure the `mtime` field in the gzip header.
    pub fn mtime(mut self, mtime: u32) -> GzBuilder {
        self.mtime = mtime;
        self
    }

    /// Configure the `operating_system` field in the gzip header.
    pub fn operating_system(mut self, os: u8) -> GzBuilder {
        self.operating_system = Some(os);
        self
    }

    /// Configure the `extra` field in the gzip header.
    ///
    /// # Panics
    ///
    /// Panics if `extra` is longer than [`u16::MAX`].
    pub fn extra<T: Into<Vec<u8>>>(mut self, extra: T) -> GzBuilder {
        let extra = extra.into();
        assert!(
            extra.len() <= u16::MAX as usize,
            "gzip extra field length cannot exceed u16::MAX"
        );
        self.extra = Some(extra);
        self
    }

    /// Configure the `filename` field in the gzip header.
    ///
    /// # Panics
    ///
    /// Panics if the `filename` slice contains a zero.
    pub fn filename<T: Into<Vec<u8>>>(mut self, filename: T) -> GzBuilder {
        self.filename = Some(CString::new(filename.into()).unwrap());
        self
    }

    /// Configure the `comment` field in the gzip header.
    ///
    /// # Panics
    ///
    /// Panics if the `comment` slice contains a zero.
    pub fn comment<T: Into<Vec<u8>>>(mut self, comment: T) -> GzBuilder {
        self.comment = Some(CString::new(comment.into()).unwrap());
        self
    }

    /// Consume this builder, creating a writer encoder in the process.
    ///
    /// The data written to the returned encoder will be compressed and then
    /// written out to the supplied parameter `w`.
    pub fn write<W: Write>(self, w: W, lvl: Compression) -> write::GzEncoder<W> {
        write::gz_encoder(self.into_header(lvl), w, lvl)
    }

    /// Consume this builder, creating a reader encoder in the process.
    ///
    /// Data read from the returned encoder will be the compressed version of
    /// the data read from the given reader.
    pub fn read<R: Read>(self, r: R, lvl: Compression) -> read::GzEncoder<R> {
        read::gz_encoder(self.buf_read(BufReader::new(r), lvl))
    }

    /// Consume this builder, creating a reader encoder in the process.
    ///
    /// Data read from the returned encoder will be the compressed version of
    /// the data read from the given reader.
    pub fn buf_read<R>(self, r: R, lvl: Compression) -> bufread::GzEncoder<R>
    where
        R: BufRead,
    {
        bufread::gz_encoder(self.into_header(lvl), r, lvl)
    }

    fn into_header(self, lvl: Compression) -> Vec<u8> {
        let GzBuilder {
            extra,
            filename,
            comment,
            operating_system,
            mtime,
        } = self;
        let mut flg = 0;
        let mut header = vec![0u8; 10];
        if let Some(v) = extra {
            flg |= FEXTRA;
            header.extend(
                (u16::try_from(v.len()).expect(
                    "`extra` can only be created from `extra()` which would have panicked on len > u16::MAX",
                ))
                .to_le_bytes(),
            );
            header.extend(v);
        }
        if let Some(filename) = filename {
            flg |= FNAME;
            header.extend(filename.as_bytes_with_nul().iter().copied());
        }
        if let Some(comment) = comment {
            flg |= FCOMMENT;
            header.extend(comment.as_bytes_with_nul().iter().copied());
        }
        header[0] = 0x1f;
        header[1] = 0x8b;
        header[2] = 8;
        header[3] = flg;
        header[4] = mtime as u8;
        header[5] = (mtime >> 8) as u8;
        header[6] = (mtime >> 16) as u8;
        header[7] = (mtime >> 24) as u8;
        header[8] = if lvl.0 >= Compression::best().0 {
            2
        } else if lvl.0 <= Compression::fast().0 {
            4
        } else {
            0
        };

        // Typically this byte indicates what OS the gz stream was created on,
        // but in an effort to have cross-platform reproducible streams just
        // default this value to 255. I'm not sure that if we "correctly" set
        // this it'd do anything anyway...
        header[9] = operating_system.unwrap_or(255);
        header
    }
}

// Upstream randomized tests retained externally; their providers are not vendored.
