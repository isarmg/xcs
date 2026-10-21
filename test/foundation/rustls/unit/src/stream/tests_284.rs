#[cfg(test)]
mod tests {
    use std::net::TcpStream;

    use super::{Stream, StreamOwned};
    use crate::client::ClientConnection;
    use crate::server::ServerConnection;

    #[test]
    fn stream_can_be_created_for_connection_and_tcpstream() {
        type _Test<'a> = Stream<'a, ClientConnection, TcpStream>;
    }

    #[test]
    fn streamowned_can_be_created_for_client_and_tcpstream() {
        type _Test = StreamOwned<ClientConnection, TcpStream>;
    }

    #[test]
    fn streamowned_can_be_created_for_server_and_tcpstream() {
        type _Test = StreamOwned<ServerConnection, TcpStream>;
    }
}
