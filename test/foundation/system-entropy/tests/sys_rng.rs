//! Tests for `SysRng`
#![cfg(feature = "sys_rng")]

use system_entropy::SysRng;
use system_entropy::rand_core::TryRng;

#[test]
fn test_sys_rng() {
    let x = SysRng.try_next_u64().unwrap();
    let y = SysRng.try_next_u64().unwrap();
    assert!(x != 0);
    assert!(x != y);
}

#[test]
fn test_construction() {
    assert!(SysRng.try_next_u64().unwrap() != 0);
}
