"""Static regressions for libc's Linux amd64/Web source boundary."""
import importlib.util
from pathlib import Path
import re
import unittest

SPEC = importlib.util.spec_from_file_location("libc_source_manager", (Path(__file__).resolve().parents[3] / "tools/rust-sources" / "manage.py"))
manager = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(manager)
from historical_test_sources import historical_reader
import source_layout

LIBC = manager.ROOT / "src/foundation/platform/libc"
DISPATCH_EDITS = {
    "build.rs", "src/lib.rs", "src/new/mod.rs", "src/unix/mod.rs",
    "src/unix/linux_like/mod.rs", "src/unix/linux_like/linux/arch/mod.rs",
    "src/unix/linux_like/linux/gnu/mod.rs", "src/unix/linux_like/linux/gnu/b64/mod.rs",
    "src/unix/linux_like/linux/gnu/b64/x86_64/mod.rs",
}


class LibcPlatformScope(unittest.TestCase):
    def setUp(self):
        self.enterContext(source_layout.activate(source_layout.load(manager.ROOT)))

    def test_platform_directories_and_dispatch_agree(self):
        expected = {
            "src": {"new", "unix"},
            "src/new": {"common", "glibc", "linux_uapi", "musl", "uclibc"},
            "src/unix": {"linux_like"},
            "src/unix/linux_like": {"linux"},
            "src/unix/linux_like/linux/arch": {"generic"},
            "src/unix/linux_like/linux/gnu": {"b64"},
            "src/unix/linux_like/linux/gnu/b64": {"x86_64"},
        }
        for relative, children in expected.items():
            with self.subTest(relative=relative):
                self.assertEqual({p.name for p in (LIBC / relative).iterdir() if p.is_dir()}, children)
        dispatch = "\n".join((LIBC / path).read_text() for path in DISPATCH_EDITS if path != "build.rs")
        for retired in (
            "aix android apple bionic_libc bsd cygwin dragonfly emscripten espidf freebsd "
            "fuchsia haiku helenos hermit hermit_abi horizon hurd illumos l4re netbsd newlib "
            "nto nuttx openbsd psp qurt redox relibc rtems sgx solid solaris solarish switch "
            "teeos trusty ucrt vita vxworks wasi windows xous b32 x32 aarch64 loongarch64 "
            "mips mips64 powerpc powerpc64 riscv64 s390x sparc sparc64"
        ).split():
            self.assertNotRegex(dispatch, rf"\bmod\s+{re.escape(retired)}\s*;")

    def test_retained_upstream_code_changes_only_at_dispatch_boundaries(self):
        catalog = manager.read_json(manager.ROOT / manager.CATALOG)
        package = catalog["packages"][LIBC.relative_to(manager.ROOT).as_posix()]
        origin = manager.origins(manager.ROOT)[package["origin_directory"]]
        originals = origin["original_files_sha256"]
        read = historical_reader(manager.ROOT)
        retained = []
        for path in LIBC.rglob("*.rs"):
            relative = path.relative_to(LIBC).as_posix()
            if relative in DISPATCH_EDITS:
                continue
            self.assertIn(relative, originals)
            self.assertEqual(manager.digest(read("src/foundation/libc/" + relative)), originals[relative], relative)
            retained.append(relative)
        self.assertGreater(len(retained), 100)
        for required in (
            "src/unix/linux_like/linux_l4re_shared.rs",
            "src/unix/linux_like/linux/arch/generic/mod.rs",
            "src/unix/linux_like/linux/gnu/b64/x86_64/not_x32.rs",
            "src/unix/linux_like/linux/musl/b64/x86_64/mod.rs",
            "src/unix/linux_like/linux/uclibc/x86_64/mod.rs",
            "src/new/linux_uapi/asm/socket.rs",
            "src/new/glibc/sysdeps/x86/nptl/bits/struct_mutex.rs",
        ):
            self.assertIn(required, retained)
        self.assertTrue(origin["central_license_materials"])
        for relative in origin["central_license_materials"]:
            self.assertTrue((manager.ROOT / relative).is_file())

    def test_explicit_target_guard_preserves_only_native_amd64_and_empty_web_surface(self):
        source = (LIBC / "src/lib.rs").read_text()
        self.assertIn('target_os = "linux", target_arch = "x86_64", target_pointer_width = "64"', source)
        self.assertIn('target_arch = "wasm32", target_os = "unknown", target_pointer_width = "32"', source)
        self.assertIn("compile_error!", source)
        self.assertIn('any(target_env = "musl", target_env = "ohos")', (LIBC / "src/new/mod.rs").read_text())
        self.assertFalse((LIBC / "src/unix/linux_like/linux/gnu/b64/x86_64/x32.rs").exists())


if __name__ == "__main__":
    unittest.main()
