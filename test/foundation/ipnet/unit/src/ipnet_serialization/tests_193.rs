#[cfg(test)]
mod tests {
    extern crate serde_test;

    use crate::{IpNet, Ipv4Net, Ipv6Net};
    use self::serde_test::{assert_tokens, Configure, Token};

    #[test]
    fn test_serialize_ipnet_v4() {
        let net_str = "10.1.1.0/24";
        let net: IpNet = net_str.parse().unwrap();
        assert_tokens(&net.readable(), &[Token::Str(net_str)]);
        assert_tokens(&net.compact(), &[
            Token::NewtypeVariant { name: "IpNet", variant: "V4", },
            Token::Tuple { len: 5 },
            Token::U8(10),
            Token::U8(1),
            Token::U8(1),
            Token::U8(0),
            Token::U8(24),
            Token::TupleEnd,
        ]);
    }

    #[test]
    fn test_serialize_ipnet_v6() {
        let net_str = "fd00::/32";
        let net: IpNet = net_str.parse().unwrap();
        assert_tokens(&net.readable(), &[Token::Str(net_str)]);
        assert_tokens(&net.compact(), &[
            Token::NewtypeVariant { name: "IpNet", variant: "V6", },
            // This is too painful, but Token::Bytes() seems to be
            // an array with a length, which is not what we serialize.
            Token::Tuple { len: 17 },
            Token::U8(253u8),
            Token::U8(0),
            Token::U8(0),
            Token::U8(0),
            Token::U8(0),
            Token::U8(0),
            Token::U8(0),
            Token::U8(0),
            Token::U8(0),
            Token::U8(0),
            Token::U8(0),
            Token::U8(0),
            Token::U8(0),
            Token::U8(0),
            Token::U8(0),
            Token::U8(0),
            Token::U8(32),
            Token::TupleEnd,
        ]);
    }

    #[test]
    fn test_serialize_ipv4_net() {
        let net_str = "10.1.1.0/24";
        let net: Ipv4Net = net_str.parse().unwrap();
        assert_tokens(&net.readable(), &[Token::Str(net_str)]);
        assert_tokens(&net.compact(), &[
            Token::Tuple { len: 5 },
            Token::U8(10),
            Token::U8(1),
            Token::U8(1),
            Token::U8(0),
            Token::U8(24),
            Token::TupleEnd,
        ]);
    }

    #[test]
    fn test_serialize_ipv6_net() {
        let net_str = "fd00::/32";
        let net: Ipv6Net = net_str.parse().unwrap();
        assert_tokens(&net.readable(), &[Token::Str(net_str)]);
        assert_tokens(&net.compact(), &[
            // This is too painful, but Token::Bytes() seems to be
            // an array with a length, which is not what we serialize.
            Token::Tuple { len: 17 },
            Token::U8(253u8),
            Token::U8(0),
            Token::U8(0),
            Token::U8(0),
            Token::U8(0),
            Token::U8(0),
            Token::U8(0),
            Token::U8(0),
            Token::U8(0),
            Token::U8(0),
            Token::U8(0),
            Token::U8(0),
            Token::U8(0),
            Token::U8(0),
            Token::U8(0),
            Token::U8(0),
            Token::U8(32),
            Token::TupleEnd,
        ]);
    }
}
