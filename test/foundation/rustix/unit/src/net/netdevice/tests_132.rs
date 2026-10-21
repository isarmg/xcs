#[cfg(test)]
mod tests {
    use super::{index_to_name, index_to_name_inlined, name_to_index};
    use crate::fd::AsFd;
    use crate::net::{AddressFamily, SocketFlags, SocketType};

    #[test]
    fn test_name_to_index() {
        let fd = crate::net::socket_with(
            AddressFamily::INET,
            SocketType::DGRAM,
            SocketFlags::CLOEXEC,
            None,
        )
        .unwrap();

        let loopback_index = std::fs::read_to_string("/sys/class/net/lo/ifindex")
            .unwrap()
            .as_str()
            .split_at(1)
            .0
            .parse::<u32>()
            .unwrap();
        assert_eq!(Ok(loopback_index), name_to_index(fd.as_fd(), "lo"));
    }

    #[test]
    fn test_index_to_name_inlined() {
        let fd = crate::net::socket_with(
            AddressFamily::INET,
            SocketType::DGRAM,
            SocketFlags::CLOEXEC,
            None,
        )
        .unwrap();

        let loopback_index = std::fs::read_to_string("/sys/class/net/lo/ifindex")
            .unwrap()
            .as_str()
            .split_at(1)
            .0
            .parse::<u32>()
            .unwrap();
        assert_eq!(
            "lo",
            index_to_name_inlined(fd.as_fd(), loopback_index)
                .unwrap()
                .as_str(),
        );
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn test_index_to_name() {
        let fd = crate::net::socket_with(
            AddressFamily::INET,
            SocketType::DGRAM,
            SocketFlags::CLOEXEC,
            None,
        )
        .unwrap();

        let loopback_index = std::fs::read_to_string("/sys/class/net/lo/ifindex")
            .unwrap()
            .as_str()
            .split_at(1)
            .0
            .parse::<u32>()
            .unwrap();
        assert_eq!(
            Ok("lo".to_owned()),
            index_to_name(fd.as_fd(), loopback_index)
        );
    }
}
