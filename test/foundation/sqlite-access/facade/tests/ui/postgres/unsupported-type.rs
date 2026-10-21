fn main() {
    // we're probably not going to get around to the geometric types anytime soon
    let _ = sqlite_access::query!("select null::circle");
    let _ = sqlite_access::query!("select $1::circle", panic!());
}
