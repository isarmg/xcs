#!/usr/bin/env python3
"""Check Linux parking timeout and wake-token behavior in a locked consumer."""
from pathlib import Path
import json
import os
import runpy
import sys
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
TARGET = "x86_64-unknown-linux-gnu"
CONSUMER = r'''
#[cfg(test)]
mod tests {
    use parking_lot_core::{park, unpark_one, ParkResult, ParkToken, UnparkToken};
    use std::time::{Duration, Instant};
    #[test]
    fn timeout_and_validation() {
        let owned = Box::new(0u8);
        let key = (&*owned) as *const u8 as usize;
        unsafe {
            assert_eq!(
                park(
                    key,
                    || false,
                    || {},
                    |_, _| panic!("invalid must not timeout"),
                    ParkToken(1),
                    None
                ),
                ParkResult::Invalid
            );
            let mut called = false;
            let started = Instant::now();
            assert_eq!(
                park(
                    key,
                    || true,
                    || {},
                    |k, last| {
                        assert_eq!(k, key);
                        assert!(last);
                        called = true;
                    },
                    ParkToken(2),
                    Some(started + Duration::from_millis(10))
                ),
                ParkResult::TimedOut
            );
            assert!(called);
            assert!(started.elapsed() >= Duration::from_millis(10));
            assert_eq!(unpark_one(key, |_| UnparkToken(0)).unparked_threads, 0);
        }
    }
    #[test]
    fn wake_before_sleep_delivers_token() {
        let owned = Box::new(0u8);
        let key = (&*owned) as *const u8 as usize;
        unsafe {
            assert_eq!(
                park(
                    key,
                    || true,
                    || {
                        assert_eq!(
                            unpark_one(key, |r| {
                                assert_eq!(r.unparked_threads, 1);
                                UnparkToken(7)
                            })
                            .unparked_threads,
                            1
                        );
                    },
                    |_, _| panic!("unexpected timeout"),
                    ParkToken(3),
                    Some(Instant::now() + Duration::from_secs(2))
                ),
                ParkResult::Unparked(UnparkToken(7))
            );
        }
    }
}
'''

def check(root):
    (root / 'target').mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='parking-regression-', dir=root / 'target') as temporary:
        work = Path(temporary)
        (work / 'src').mkdir()
        foundation = root / 'src/foundation'
        (work / 'Cargo.toml').write_text(f'''[package]
name = "parking-runtime-regression"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
parking_lot_core = {{ path = "{foundation / 'parking_lot_core'}" }}
''')
        (work / 'src/lib.rs').write_text(CONSUMER)
        subprocess.run(["cargo", "generate-lockfile", "--offline", "--manifest-path",
                        str(work / "Cargo.toml")], cwd=root, check=True)
        diagnostics_type = runpy.run_path(
            str(root / ".github/scripts/check-rust-diagnostics.py")
        )["CargoDiagnostics"]
        for name, flags in [("parking", "")]:
            print(f"Checking Linux amd64 {name}", flush=True)
            command = ["cargo", "test", "--locked", "--offline", "--no-run",
                       "--message-format=json", "--manifest-path", str(work / "Cargo.toml"),
                       "--target", TARGET, "--target-dir", str(work / "target"), "-j2"]
            diagnostics = diagnostics_type(sys.stderr)
            executables = []
            with subprocess.Popen(command, cwd=root, env=os.environ | {"RUSTFLAGS": flags},
                                  stdout=subprocess.PIPE, text=True, encoding="utf-8") as process:
                for number, line in enumerate(process.stdout, 1):
                    diagnostics.consume(line, number)
                    try:
                        event = json.loads(line)
                    except ValueError:
                        continue  # The strict diagnostic consumer rejects malformed JSON.
                    if (isinstance(event, dict) and event.get("reason") == "compiler-artifact"
                            and event.get("target", {}).get("name") == "parking_runtime_regression"
                            and event.get("profile", {}).get("test") and event.get("executable")):
                        executables.append(event["executable"])
                if diagnostics.result(process.wait()):
                    raise RuntimeError(f"{name}: Cargo diagnostics rejected the regression build")
            if len(executables) != 1:
                raise AssertionError(f"{name}: expected exactly one consumer test executable")
            subprocess.run([executables[0]], cwd=root, check=True)

if __name__ == "__main__":
    check(ROOT)
