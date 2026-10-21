
#[cfg(test)]
mod tests {
    use std::{io::{Read, Write}, time::Duration};
    #[test]
    fn system_and_future_linux_apis() {
        assert!(sysinfo::IS_SUPPORTED_SYSTEM);
        let mut system = sysinfo::System::new_all();
        system.refresh_all();
        assert!(!system.cpus().is_empty());
        assert!(system.total_memory() > 0);
        assert!(system.process(sysinfo::get_current_pid().unwrap()).is_some());
        let _ = sysinfo::Disks::new_with_refreshed_list();
        let _ = sysinfo::Components::new_with_refreshed_list();
        let _ = sysinfo::Networks::new_with_refreshed_list();
        let _ = sysinfo::Users::new_with_refreshed_list();
        let _ = sysinfo::Groups::new_with_refreshed_list();
    }
    #[test]
    fn socket_abi_tcp_udp_unix() -> std::io::Result<()> {
        use socket2::{Domain, Protocol, SockAddr, SockAddrStorage, Socket, Type};
        assert_eq!(std::mem::size_of::<SockAddrStorage>(), 128);
        assert_eq!(std::mem::align_of::<SockAddrStorage>(), 8);
        let listener = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))?;
        listener.bind(&SockAddr::from("127.0.0.1:0".parse::<std::net::SocketAddr>().unwrap()))?;
        listener.listen(4)?;
        let mut client = std::net::TcpStream::connect(listener.local_addr()?.as_socket().unwrap())?;
        let (accepted, _) = listener.accept()?;
        let mut server: std::net::TcpStream = accepted.into();
        server.set_read_timeout(Some(Duration::from_secs(2)))?;
        client.write_all(b"tcp")?;
        let mut data = [0; 3]; server.read_exact(&mut data)?; assert_eq!(&data,b"tcp");
        let udp = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
        udp.bind(&SockAddr::from("127.0.0.1:0".parse::<std::net::SocketAddr>().unwrap()))?;
        let receiver: std::net::UdpSocket = udp.into(); receiver.set_read_timeout(Some(Duration::from_secs(2)))?;
        let sender = std::net::UdpSocket::bind("127.0.0.1:0")?;
        sender.send_to(b"udp", receiver.local_addr()?)?;
        assert_eq!(receiver.recv(&mut data)?, 3); assert_eq!(&data,b"udp");
        let (mut a, mut b) = std::os::unix::net::UnixStream::pair()?;
        let _ = socket2::SockRef::from(&a).r#type()?;
        b.set_read_timeout(Some(Duration::from_secs(2)))?;
        a.write_all(b"uds")?; b.read_exact(&mut data)?; assert_eq!(&data,b"uds");
        Ok(())
    }
    #[cfg(feature = "poll")]
    #[test]
    fn poll_waker_pipe_and_network() -> std::io::Result<()> {
        use mio::{Events, Interest, Poll, Token, Waker};
        let mut poll = Poll::new()?;
        let mut events = Events::with_capacity(16);
        let wake = Waker::new(poll.registry(), Token(7))?; wake.wake()?;
        poll.poll(&mut events, Some(Duration::from_secs(2)))?;
        assert!(events.iter().any(|event| event.token()==Token(7)));
        let (mut tx, mut rx) = mio::unix::pipe::new()?;
        poll.registry().register(&mut rx, Token(8), Interest::READABLE)?;
        tx.write_all(b"pipe")?;
        poll.poll(&mut events, Some(Duration::from_secs(2)))?;
        assert!(events.iter().any(|event| event.token()==Token(8) && event.is_readable()));
        let mut data=[0;4]; rx.read_exact(&mut data)?; assert_eq!(&data,b"pipe");
        poll.registry().deregister(&mut rx)?;
        let mut tcp=mio::net::TcpListener::bind("127.0.0.1:0".parse().unwrap())?;
        poll.registry().register(&mut tcp,Token(9),Interest::READABLE)?;
        let _client=std::net::TcpStream::connect(tcp.local_addr()?)?;
        poll.poll(&mut events,Some(Duration::from_secs(2)))?;
        assert!(events.iter().any(|event|event.token()==Token(9)));
        tcp.accept()?;
        Ok(())
    }
    #[cfg(not(feature = "poll"))]
    #[test]
    fn shell_keeps_common_api() {
        let events = mio::Events::with_capacity(8);
        assert!(events.is_empty());
        assert!(mio::Interest::READABLE.is_readable());
    }
}
