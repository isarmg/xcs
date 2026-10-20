#!/usr/bin/env python3
"""Exercise retained system/socket/poll APIs without upstream dev dependencies."""
from pathlib import Path
import json
import os
import re
import runpy
import sys
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
TARGET = "x86_64-unknown-linux-gnu"
CONSUMER = r'''
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
'''


def check_target_contract(root, work):
    for crate in ("sysinfo", "socket2", "mio"):
        source = (root / "src/foundation" / crate / "src/lib.rs").read_text()
        guard = re.search(r"// Maintained native target contract;.*?compile_error!\(.*?\);", source, re.S)
        if guard is None:
            raise AssertionError(f"{crate}: missing native target contract")
        # Evaluate the exact production predicate using independent synthetic cfg
        # names, not conflicting built-in target flags on the host compiler.
        probe = re.sub(r"\btarget_(os|arch|pointer_width)\b", r"probe_\1", guard.group())
        probe_path = work / "target-contract.rs"
        probe_path.write_text(probe + "\nfn main() {}\n")
        for target_os, arch, width, allowed in [
            ("linux", "x86_64", "64", True),
            ("linux", "x86_64", "32", False),
            ("linux", "aarch64", "64", False),
            ("linux", "x86", "32", False),
            ("windows", "x86_64", "64", False),
            ("macos", "x86_64", "64", False),
            ("freebsd", "x86_64", "64", False),
            ("unknown", "wasm32", "32", crate == "sysinfo"),
            ("wasi", "wasm32", "32", False),
        ]:
            command = ["rustc", str(probe_path), "--deny=warnings", "--emit=metadata", "-o", str(work / "probe.rmeta")]
            for key, value in [("os", target_os), ("arch", arch), ("pointer_width", width)]:
                command += ["--cfg", f'probe_{key}="{value}"']
            result = subprocess.run(command, capture_output=True, text=True)
            if (result.returncode == 0) != allowed:
                raise AssertionError(f"{crate}: wrong target contract for {target_os}/{arch}/{width}: {result.stderr}")
    for path in ("sysinfo/src/windows", "sysinfo/src/unix/apple", "sysinfo/src/unix/bsd", "socket2/src/sys/windows.rs", "mio/src/sys/windows"):
        if (root / "src/foundation" / path).exists():
            raise AssertionError(f"retired backend present: {path}")
    for path in ("mio/src/sys/unix/selector/poll.rs", "mio/src/sys/unix/waker/pipe.rs", "mio/src/sys/unix/pipe.rs", "mio/src/sys/shell/mod.rs", "sysinfo/src/unknown/mod.rs"):
        if not (root / "src/foundation" / path).is_file():
            raise AssertionError(f"required fallback missing: {path}")

def check(root):
    with tempfile.TemporaryDirectory(prefix="zxs-system-backends-") as temporary:
        work = Path(temporary)
        (work / "src").mkdir()
        foundation = root / "src/foundation"
        check_target_contract(root, work)
        (work / "Cargo.toml").write_text(f'''[package]
name = "linux-system-backends-regression"
version = "0.0.0"
edition = "2021"
[workspace]
[features]
poll = ["mio/net", "mio/os-ext", "mio/os-poll"]
[dependencies]
sysinfo = {{ path = "{foundation / 'sysinfo'}", features = ["c-interface", "serde", "multithread", "linux-netdevs", "linux-tmpfs", "debug"] }}
socket2 = {{ path = "{foundation / 'socket2'}", features = ["all"] }}
mio = {{ path = "{foundation / 'mio'}", default-features = false }}
''')
        (work / "src/lib.rs").write_text(CONSUMER)
        subprocess.run(["cargo", "generate-lockfile", "--offline", "--manifest-path",
                        str(work / "Cargo.toml")], cwd=root, check=True)
        diagnostics_type = runpy.run_path(
            str(root / ".github/scripts/check-rust-diagnostics.py")
        )["CargoDiagnostics"]
        for name, flags in [("epoll", ""), ("poll", "--cfg mio_unsupported_force_poll_poll"), ("pipe", "--cfg mio_unsupported_force_waker_pipe"), ("poll-pipe", "--cfg mio_unsupported_force_poll_poll --cfg mio_unsupported_force_waker_pipe"), ("shell", "")]:
            print(f"Checking Linux amd64 system/socket/mio {name}", flush=True)
            command = ["cargo", "test", "--locked", "--offline", "--no-run",
                       "--message-format=json", "--manifest-path", str(work / "Cargo.toml"),
                       "--target", TARGET, "--target-dir", str(work / "target"), "-j2"]
            if name != "shell":
                command += ["--features", "poll"]
            diagnostics = diagnostics_type(sys.stderr)
            executables = []
            with subprocess.Popen(command, cwd=root, env=os.environ | {"RUSTFLAGS": flags},
                                  stdout=subprocess.PIPE, text=True, encoding="utf-8") as process:
                for number, line in enumerate(process.stdout, 1):
                    diagnostics.consume(line, number)
                    try:
                        event = json.loads(line)
                    except ValueError:
                        continue  # The strict diagnostic consumer rejects malformed JSON.
                    if (isinstance(event, dict) and event.get("reason") == "compiler-artifact"
                            and event.get("target", {}).get("name") == "linux_system_backends_regression"
                            and event.get("profile", {}).get("test") and event.get("executable")):
                        executables.append(event["executable"])
                if diagnostics.result(process.wait()):
                    raise RuntimeError(f"{name}: Cargo diagnostics rejected the regression build")
            if len(executables) != 1:
                raise AssertionError(f"{name}: expected exactly one consumer test executable")
            subprocess.run([executables[0]], cwd=root, check=True)

if __name__ == "__main__":
    check(ROOT)
