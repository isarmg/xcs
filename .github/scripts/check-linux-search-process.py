#!/usr/bin/env python3
"""Check retained byte-search, Linux process, and Unix IPC consumer APIs."""
from pathlib import Path
import json
import runpy
import sys
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
TARGET = "x86_64-unknown-linux-gnu"
CONSUMER = r'''
#[cfg(test)] mod tests {
#[test] fn byte_and_substring_boundaries() {
 for offset in 0..64 { for len in [0,1,2,3,15,16,17,31,32,33,63,64,65,255,256,257] {
  let storage: Vec<u8>=(0..offset+len).map(|i| ((i*37+i/11)%251) as u8).collect(); let h=&storage[offset..];
  for a in [0u8,1,63,127,250,255] { let b=a.wrapping_add(1); let c=a.wrapping_add(2);
   assert_eq!(memchr::memchr(a,h),h.iter().position(|&v|v==a)); assert_eq!(memchr::memrchr(a,h),h.iter().rposition(|&v|v==a));
   assert_eq!(memchr::memchr2(a,b,h),h.iter().position(|&v|v==a||v==b)); assert_eq!(memchr::memrchr2(a,b,h),h.iter().rposition(|&v|v==a||v==b));
   assert_eq!(memchr::memchr3(a,b,c,h),h.iter().position(|&v|v==a||v==b||v==c)); assert_eq!(memchr::memrchr3(a,b,c,h),h.iter().rposition(|&v|v==a||v==b||v==c));
  }
  for nlen in [0,1,2,3,7,16,32,64,128,258] { let needle = &storage[..nlen.min(storage.len())];
   let expected=if needle.is_empty(){Some(0)}else{h.windows(needle.len()).position(|x|x==needle)};
   let reverse=if needle.is_empty(){Some(h.len())}else{h.windows(needle.len()).rposition(|x|x==needle)};
   assert_eq!(memchr::memmem::find(h,needle),expected); assert_eq!(memchr::memmem::rfind(h,needle),reverse);
   assert_eq!(memchr::memmem::Finder::new(needle).find(h),expected);
  }
 }}
}
#[tokio::test] async fn linux_process_and_unix_ipc() {
 use tokio::io::{AsyncReadExt,AsyncWriteExt};
 let output=tokio::process::Command::new("/bin/sh").args(["-c","printf baseline"]).output().await.unwrap(); assert!(output.status.success()); assert_eq!(output.stdout,b"baseline");
 let mut child=tokio::process::Command::new("/bin/sleep").arg("30").spawn().unwrap(); child.kill().await.unwrap(); assert!(!child.wait().await.unwrap().success());
 let (mut a,mut b)=tokio::net::UnixStream::pair().unwrap(); a.write_all(b"ipc").await.unwrap(); let mut bytes=[0;3]; b.read_exact(&mut bytes).await.unwrap(); assert_eq!(&bytes,b"ipc");
}
}
'''


def check(root):
    root = Path(root).resolve()
    temporary_root = root / "target" / "scope-consumers"
    temporary_root.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="search-process-", dir=temporary_root) as temporary:
        work = Path(temporary)
        (work / "src").mkdir()
        manifest = [
            '[package]',
            'name = "retained-search-process-consumer"',
            'version = "0.0.0"',
            'edition = "2021"',
            '[workspace]',
            '[dependencies]',
            'memchr = { path = ' + json.dumps(str(root / "src/foundation/memchr")) + ' }',
            'tokio = { path = ' + json.dumps(str(root / "src/foundation/tokio")) + ', features = ["full"] }',
        ]
        (work / "Cargo.toml").write_text("\n".join(manifest) + "\n", encoding="utf-8")
        (work / "src/lib.rs").write_text(CONSUMER, encoding="utf-8")
        common = ["--manifest-path", str(work / "Cargo.toml"), "--offline"]
        # Create only this isolated consumer's lockfile, then prohibit resolution changes.
        subprocess.run(["cargo", "generate-lockfile", *common], cwd=root, check=True, timeout=120)
        print("Checking memchr boundaries and Tokio Linux process / Unix IPC APIs", flush=True)
        diagnostics_type = runpy.run_path(
            str(root / ".github/scripts/check-rust-diagnostics.py")
        )["CargoDiagnostics"]
        diagnostics = diagnostics_type(sys.stderr)
        executables = []
        command = [
            "cargo", "test", *common, "--locked", "--no-run", "--message-format=json",
            "--jobs", "2", "--target", TARGET,
            "--target-dir", str(root / "target" / "search-process-consumer"),
        ]
        with subprocess.Popen(command, cwd=root, stdout=subprocess.PIPE,
                              text=True, encoding="utf-8") as process:
            for number, line in enumerate(process.stdout, 1):
                diagnostics.consume(line, number)
                try:
                    event = json.loads(line)
                except ValueError:
                    continue  # CargoDiagnostics rejects malformed JSON.
                if (isinstance(event, dict) and event.get("reason") == "compiler-artifact"
                        and event.get("target", {}).get("name") == "retained_search_process_consumer"
                        and event.get("profile", {}).get("test") and event.get("executable")):
                    executables.append(event["executable"])
            if diagnostics.result(process.wait()):
                raise RuntimeError("Cargo diagnostics rejected the search/process consumer build")
        if len(executables) != 1:
            raise AssertionError("Expected exactly one search/process consumer test executable")
        subprocess.run([executables[0]], cwd=root, check=True, timeout=60)



if __name__ == "__main__":
    check(ROOT)
