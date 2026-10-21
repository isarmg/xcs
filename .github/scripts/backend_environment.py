"""Keep backend matrix selection independent of inherited Cargo rustflags."""
import os


def matrix_environment():
    environment = os.environ.copy()
    # Cargo gives this variable priority even when its value is empty. Leaving
    # it set silently disables the RUSTFLAGS used to select each matrix case.
    if "CARGO_ENCODED_RUSTFLAGS" in environment:
        raise RuntimeError("unset CARGO_ENCODED_RUSTFLAGS before this backend matrix")
    # A caller could otherwise force rustix_use_libc, getrandom_backend or mio
    # overrides, making supposedly distinct cases exercise the same backend.
    if environment.get("RUSTFLAGS"):
        raise RuntimeError("unset RUSTFLAGS before this backend matrix; case flags are controlled by the checker")
    # An explicit value also takes priority over Cargo config/target rustflags.
    environment["RUSTFLAGS"] = ""
    return environment
