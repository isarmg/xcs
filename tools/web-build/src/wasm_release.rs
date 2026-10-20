//! Keep browser builds independent of the native workspace release profile.
use std::process::Command;

pub fn configure(command: &mut Command) {
    command
        .env("CARGO_PROFILE_RELEASE_OPT_LEVEL", "s")
        .env("CARGO_PROFILE_RELEASE_LTO", "fat")
        .env("CARGO_PROFILE_RELEASE_STRIP", "symbols");
}
