#[test]
fn it_works() {
    let x = errno();
    set_errno(x);
}
