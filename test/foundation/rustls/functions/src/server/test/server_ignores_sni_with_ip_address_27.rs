#[test]
fn server_ignores_sni_with_ip_address() {
    let mut ch = minimal_client_hello();
    ch.extensions.server_name = Some(ServerNamePayload::IpAddress);
    std::println!("{:?}", ch.extensions);
    assert_eq!(test_process_client_hello(ch), Ok(()));
}
