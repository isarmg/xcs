#[test]
fn server_rejects_sni_with_illegal_dns_name() {
    let mut ch = minimal_client_hello();
    ch.extensions.server_name = Some(ServerNamePayload::Invalid);
    std::println!("{:?}", ch.extensions);
    assert_eq!(
        test_process_client_hello(ch),
        Err(PeerMisbehaved::ServerNameMustContainOneHostName.into())
    );
}
