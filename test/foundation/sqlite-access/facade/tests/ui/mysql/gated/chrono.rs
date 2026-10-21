fn main() {
    let _ = sqlite_access::query!("select CONVERT(now(), DATE) date");

    let _ = sqlite_access::query!("select CONVERT(now(), TIME) time");

    let _ = sqlite_access::query!("select CONVERT(now(), DATETIME) datetime");
}
