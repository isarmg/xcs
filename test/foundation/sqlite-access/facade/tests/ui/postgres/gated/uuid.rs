fn main() {
    let _ = sqlite_access::query!("select 'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11'::uuid");
    let _ = sqlite_access::query!("select $1::uuid", ());
}
