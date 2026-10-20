use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::{
    io::{self, Read, Write},
    mem,
    os::{
        fd::{AsRawFd, BorrowedFd, FromRawFd, OwnedFd},
        unix::net::UnixStream,
    },
    time::Duration,
};

pub const PROTOCOL: &str = "smb-protocol-rpc";
pub const MAX_FRAME_BYTES: usize = 8 * 1024 * 1024;

pub fn effective_uid() -> u32 {
    // SAFETY: geteuid has no arguments or side effects and cannot fail.
    unsafe { libc::geteuid() }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    Hello {
        protocol: String,
        username: String,
        repository_id: String,
    },
    Open {
        path: String,
        writable: bool,
        create: bool,
        truncate: bool,
        exclusive: bool,
        write_through: bool,
    },
    Dirty {
        #[serde(with = "storage_contract::u64_string")]
        handle: u64,
    },
    AuthorizeRead {
        #[serde(with = "storage_contract::u64_string")]
        handle: u64,
    },
    Flush {
        #[serde(with = "storage_contract::u64_string")]
        handle: u64,
    },
    Close {
        #[serde(with = "storage_contract::u64_string")]
        handle: u64,
    },
    Abort {
        #[serde(with = "storage_contract::u64_string")]
        handle: u64,
    },
    Stat {
        path: String,
    },
    List {
        path: String,
    },
    Namespace {
        kind: String,
        path: String,
        destination: Option<String>,
        replace: bool,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum Response {
    Ok {
        result: Value,
    },
    Error {
        code: String,
        message: String,
        retryable: bool,
        operation_id: Option<String>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenResult {
    pub entry_id: String,
    pub kind: String,
    pub path: String,
    #[serde(with = "storage_contract::u64_string")]
    pub handle: u64,
    #[serde(with = "storage_contract::u64_string")]
    pub size: u64,
    #[serde(with = "storage_contract::u64_string")]
    pub max_file_bytes: u64,
    pub version_id: Option<String>,
    pub workspace_id: Option<String>,
}

/// One serialized request/response stream. The descriptor, when present, is
/// attached to the four-byte frame header using SCM_RIGHTS, never a JSON path.
pub struct FrameStream(pub UnixStream);

impl FrameStream {
    pub fn new(stream: UnixStream) -> io::Result<Self> {
        stream.set_read_timeout(Some(Duration::from_secs(120)))?;
        stream.set_write_timeout(Some(Duration::from_secs(120)))?;
        Ok(Self(stream))
    }

    pub fn send<T: Serialize>(&mut self, value: &T, fd: Option<BorrowedFd<'_>>) -> io::Result<()> {
        let payload = serde_json::to_vec(value).map_err(io::Error::other)?;
        if payload.is_empty() || payload.len() > MAX_FRAME_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "RPC frame size exceeded",
            ));
        }
        let header = (payload.len() as u32).to_be_bytes();
        let mut vector = libc::iovec {
            iov_base: header.as_ptr().cast_mut().cast(),
            iov_len: header.len(),
        };
        // usize provides the alignment required by cmsghdr. Space for one int
        // plus padding fits this fixed, initialized 64-byte control buffer.
        let mut control = [0usize; 8];
        // SAFETY: zero is the initialized empty msghdr representation; all used
        // pointers below refer to live header/control storage until sendmsg returns.
        let mut message: libc::msghdr = unsafe { mem::zeroed() };
        message.msg_iov = &mut vector;
        message.msg_iovlen = 1;
        if let Some(fd) = fd {
            message.msg_control = control.as_mut_ptr().cast();
            // SAFETY: libc computes the native ancillary buffer layout for one fd.
            message.msg_controllen =
                unsafe { libc::CMSG_SPACE(mem::size_of::<i32>() as u32) as usize };
            // SAFETY: control is aligned, zeroed, and large enough for this header
            // and one descriptor; the ancillary pointers do not escape this call.
            unsafe {
                let cmsg = libc::CMSG_FIRSTHDR(&message);
                (*cmsg).cmsg_level = libc::SOL_SOCKET;
                (*cmsg).cmsg_type = libc::SCM_RIGHTS;
                (*cmsg).cmsg_len = libc::CMSG_LEN(mem::size_of::<i32>() as u32) as usize;
                std::ptr::write_unaligned(libc::CMSG_DATA(cmsg).cast::<i32>(), fd.as_raw_fd());
            }
        }
        let sent = loop {
            // SAFETY: all message pointers and lengths were initialized above;
            // the socket is a live owned Unix stream. MSG_NOSIGNAL avoids SIGPIPE.
            let result = unsafe { libc::sendmsg(self.0.as_raw_fd(), &message, libc::MSG_NOSIGNAL) };
            if result >= 0 {
                break result as usize;
            }
            let error = io::Error::last_os_error();
            if error.kind() != io::ErrorKind::Interrupted {
                return Err(error);
            }
        };
        if sent == 0 {
            return Err(io::ErrorKind::WriteZero.into());
        }
        self.0.write_all(&header[sent..])?;
        self.0.write_all(&payload)
    }

    pub fn receive<T: DeserializeOwned>(&mut self) -> io::Result<(T, Option<OwnedFd>)> {
        let mut header = [0u8; 4];
        let mut received = 0;
        let mut descriptor = None;
        while received < header.len() {
            let mut vector = libc::iovec {
                iov_base: header[received..].as_mut_ptr().cast(),
                iov_len: header.len() - received,
            };
            let mut control = [0usize; 32];
            // SAFETY: a zeroed msghdr has no dangling allocations; its buffers are
            // fully initialized below and remain live through recvmsg.
            let mut message: libc::msghdr = unsafe { mem::zeroed() };
            message.msg_iov = &mut vector;
            message.msg_iovlen = 1;
            message.msg_control = control.as_mut_ptr().cast();
            message.msg_controllen = mem::size_of_val(&control);
            // SAFETY: all buffers are writable, bounded, and aligned. Received
            // descriptors are CLOEXEC immediately, before any other thread can fork.
            let count =
                unsafe { libc::recvmsg(self.0.as_raw_fd(), &mut message, libc::MSG_CMSG_CLOEXEC) };
            if count < 0 {
                let error = io::Error::last_os_error();
                if error.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(error);
            }
            if count == 0 {
                return Err(io::ErrorKind::UnexpectedEof.into());
            }
            let mut extra_descriptors = false;
            // SAFETY: libc created this ancillary sequence within the supplied
            // aligned buffer. Every SCM_RIGHTS descriptor is immediately owned,
            // including unwanted extras, so errors cannot leak received fds.
            unsafe {
                let mut cmsg = libc::CMSG_FIRSTHDR(&message);
                while !cmsg.is_null() {
                    if (*cmsg).cmsg_level == libc::SOL_SOCKET
                        && (*cmsg).cmsg_type == libc::SCM_RIGHTS
                    {
                        let base = libc::CMSG_LEN(0) as usize;
                        if (*cmsg).cmsg_len < base {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "invalid ancillary header",
                            ));
                        }
                        let bytes = (*cmsg).cmsg_len - base;
                        for offset in (0..bytes).step_by(mem::size_of::<i32>()) {
                            if offset + mem::size_of::<i32>() > bytes {
                                break;
                            }
                            let raw = std::ptr::read_unaligned(
                                libc::CMSG_DATA(cmsg).add(offset).cast::<i32>(),
                            );
                            let fd = OwnedFd::from_raw_fd(raw);
                            if descriptor.is_some() {
                                extra_descriptors = true;
                            } else {
                                descriptor = Some(fd);
                            }
                        }
                    }
                    cmsg = libc::CMSG_NXTHDR(&message, cmsg);
                }
            }
            if extra_descriptors || message.msg_flags & libc::MSG_CTRUNC != 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "too many RPC descriptors",
                ));
            }
            received += count as usize;
        }
        let length = u32::from_be_bytes(header) as usize;
        if length == 0 || length > MAX_FRAME_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "RPC frame size exceeded",
            ));
        }
        let mut payload = vec![0u8; length];
        self.0.read_exact(&mut payload)?;
        let value = storage_contract::decode_strict(&payload).map_err(io::Error::other)?;
        Ok((value, descriptor))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs::File, io::Seek, os::fd::AsFd};

    #[test]
    fn framing_transfers_an_owned_cloexec_descriptor_without_a_path() {
        let (one, two) = UnixStream::pair().unwrap();
        let mut sender = FrameStream::new(one).unwrap();
        let mut receiver = FrameStream::new(two).unwrap();
        let source = File::open("/etc/hostname").unwrap();
        sender
            .send(
                &Response::Ok {
                    result: serde_json::json!({"handle":"18446744073709551615"}),
                },
                Some(source.as_fd()),
            )
            .unwrap();
        let (result, fd) = receiver.receive::<Response>().unwrap();
        assert!(matches!(result, Response::Ok { .. }));
        let fd = fd.unwrap();
        // SAFETY: F_GETFD only reads the flag word of this live owned descriptor.
        assert_ne!(
            unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_GETFD) } & libc::FD_CLOEXEC,
            0
        );
        let mut transferred = File::from(fd);
        transferred.rewind().unwrap();
        assert!(transferred.metadata().unwrap().is_file());
    }

    #[test]
    fn oversized_length_is_rejected_before_allocating_payload() {
        let (mut one, two) = UnixStream::pair().unwrap();
        one.write_all(&u32::MAX.to_be_bytes()).unwrap();
        let error = FrameStream::new(two)
            .unwrap()
            .receive::<Request>()
            .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }
}
