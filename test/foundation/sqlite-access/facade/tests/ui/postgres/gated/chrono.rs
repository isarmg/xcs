fn main() {
    let _ = sqlite_access::query!("select now()::date");

    let _ = sqlite_access::query!("select now()::time");

    let _ = sqlite_access::query!("select now()::timestamp");

    let _ = sqlite_access::query!("select now()::timestamptz");

    let _ = sqlite_access::query!("select $1::date", ());

    let _ = sqlite_access::query!("select $1::time", ());

    let _ = sqlite_access::query!("select $1::timestamp", ());

    let _ = sqlite_access::query!("select $1::timestamptz", ());
}
