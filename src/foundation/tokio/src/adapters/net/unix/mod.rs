//! Unix domain socket helpers.

use super::Listener;
use std::io::Result;
use std::task::{Context, Poll};

impl Listener for crate::net::UnixListener {
    type Io = crate::net::UnixStream;
    type Addr = crate::net::unix::SocketAddr;

    fn poll_accept(&mut self, cx: &mut Context<'_>) -> Poll<Result<(Self::Io, Self::Addr)>> {
        Self::poll_accept(self, cx)
    }

    fn local_addr(&self) -> Result<Self::Addr> {
        self.local_addr()
    }
}
