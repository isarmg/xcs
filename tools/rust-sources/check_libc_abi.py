#!/usr/bin/env python3
"""Check the retained libc dispatch and GNU amd64 ABI without network or sockets.

Run with the repository's Rust toolchain and a GNU/Linux amd64 C compiler:
    python3 tools/rust-sources/check_libc_abi.py

Optionally prove selected Rust code/data equivalence against a pre-pruning commit:
    python3 tools/rust-sources/check_libc_abi.py --compare-revision REV

Alternate Linux amd64 libcs are preservation scope, not validated product ABIs.
Accepting their build-script target gate is not a musl/ohos/uclibc build/runtime test.
"""
if not __debug__:
    raise RuntimeError('Validation requires assertions; do not use Python optimization (-O/-OO or PYTHONOPTIMIZE).')

import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import platform
import shlex
import subprocess
import tarfile
import tempfile

ROOT = Path(__file__).resolve().parents[2]
LIBC = ROOT / "src/foundation/platform/libc"
NATIVE = "x86_64-unknown-linux-gnu"
WEB = "wasm32-unknown-unknown"
VARIANTS = {"default": ["std"], "no_std": [], "extra_traits": ["std", "extra_traits"]}

# C spelling, Rust spelling, public fields whose offsets must match host headers.
LAYOUTS = [
    ("long", "c_long", []),
    ("size_t", "size_t", []),
    ("ssize_t", "ssize_t", []),
    ("off_t", "off_t", []),
    ("time_t", "time_t", []),
    ("dev_t", "dev_t", []),
    ("ino_t", "ino_t", []),
    ("nlink_t", "nlink_t", []),
    ("pthread_t", "pthread_t", []),
    ("struct stat", "stat", ["st_dev", "st_ino", "st_nlink", "st_mode", "st_uid",
                            "st_gid", "st_rdev", "st_size", "st_blksize", "st_blocks"]),
    ("struct stat64", "stat64", ["st_ino", "st_size", "st_blocks"]),
    ("struct statfs", "statfs", ["f_type", "f_bsize", "f_blocks", "f_fsid", "f_namelen"]),
    ("struct statfs64", "statfs64", ["f_blocks", "f_bavail", "f_fsid"]),
    ("struct statvfs", "statvfs", ["f_bsize", "f_blocks", "f_fsid", "f_flag", "f_namemax"]),
    ("struct statvfs64", "statvfs64", ["f_bsize", "f_blocks", "f_flag"]),
    ("struct statx", "statx", ["stx_mask", "stx_ino", "stx_size", "stx_atime", "stx_mnt_id"]),
    ("struct timespec", "timespec", ["tv_sec", "tv_nsec"]),
    ("struct timeval", "timeval", ["tv_sec", "tv_usec"]),
    ("struct itimerspec", "itimerspec", ["it_interval", "it_value"]),
    ("struct tm", "tm", ["tm_sec", "tm_year", "tm_isdst", "tm_gmtoff", "tm_zone"]),
    ("struct dirent", "dirent", ["d_ino", "d_off", "d_reclen", "d_type", "d_name"]),
    ("struct dirent64", "dirent64", ["d_ino", "d_off", "d_name"]),
    ("struct flock", "flock", ["l_type", "l_whence", "l_start", "l_len", "l_pid"]),
    ("struct iovec", "iovec", ["iov_base", "iov_len"]),
    ("struct msghdr", "msghdr", ["msg_name", "msg_namelen", "msg_iov", "msg_iovlen",
                                "msg_control", "msg_controllen", "msg_flags"]),
    ("struct cmsghdr", "cmsghdr", ["cmsg_len", "cmsg_level", "cmsg_type"]),
    ("struct sockaddr", "sockaddr", ["sa_family", "sa_data"]),
    ("struct sockaddr_in", "sockaddr_in", ["sin_family", "sin_port", "sin_addr"]),
    ("struct sockaddr_in6", "sockaddr_in6", ["sin6_family", "sin6_port", "sin6_addr"]),
    ("struct sockaddr_un", "sockaddr_un", ["sun_family", "sun_path"]),
    ("struct sockaddr_storage", "sockaddr_storage", ["ss_family"]),
    ("struct epoll_event", "epoll_event", ["events"]),
    ("struct pollfd", "pollfd", ["fd", "events", "revents"]),
    ("pthread_mutex_t", "pthread_mutex_t", []),
    ("pthread_cond_t", "pthread_cond_t", []),
    ("pthread_rwlock_t", "pthread_rwlock_t", []),
    ("pthread_attr_t", "pthread_attr_t", []),
    ("sigset_t", "sigset_t", []),
    ("struct sigaction", "sigaction", ["sa_mask", "sa_flags", "sa_restorer"]),
    ("siginfo_t", "siginfo_t", ["si_signo", "si_errno", "si_code"]),
    ("ucontext_t", "ucontext_t", ["uc_flags", "uc_link", "uc_stack", "uc_mcontext", "uc_sigmask"]),
    ("struct utsname", "utsname", ["sysname", "nodename", "release", "machine"]),
    ("struct rlimit", "rlimit", ["rlim_cur", "rlim_max"]),
    ("struct rlimit64", "rlimit64", ["rlim_cur", "rlim_max"]),
    ("struct passwd", "passwd", ["pw_name", "pw_uid", "pw_gid", "pw_dir", "pw_shell"]),
    ("struct group", "group", ["gr_name", "gr_gid", "gr_mem"]),
    ("struct addrinfo", "addrinfo", ["ai_flags", "ai_family", "ai_addrlen", "ai_addr", "ai_next"]),
    ("struct termios", "termios", ["c_iflag", "c_cflag", "c_line", "c_cc"]),
    ("struct msqid_ds", "msqid_ds", ["msg_perm", "msg_stime", "msg_qnum", "msg_lspid"]),
    ("struct shmid_ds", "shmid_ds", ["shm_perm", "shm_segsz", "shm_atime", "shm_nattch"]),
    ("struct semid_ds", "semid_ds", ["sem_perm", "sem_otime", "sem_ctime", "sem_nsems"]),
]
CONSTANTS = """
O_RDONLY O_WRONLY O_RDWR O_CREAT O_EXCL O_TRUNC O_APPEND O_NONBLOCK O_CLOEXEC
O_DIRECTORY O_NOFOLLOW O_PATH O_TMPFILE O_LARGEFILE AT_FDCWD AT_SYMLINK_NOFOLLOW
AT_EMPTY_PATH F_GETFD F_SETFD F_GETFL F_SETFL F_DUPFD_CLOEXEC FD_CLOEXEC
SEEK_SET SEEK_CUR SEEK_END SEEK_DATA SEEK_HOLE
S_IFREG S_IFDIR S_IFLNK S_IRUSR S_IWUSR EINVAL ENOENT EEXIST EACCES EAGAIN EINTR
AF_UNIX AF_INET AF_INET6 SOCK_STREAM SOCK_DGRAM SOCK_CLOEXEC SOCK_NONBLOCK
SOL_SOCKET SO_REUSEADDR SO_REUSEPORT SO_ERROR SO_RCVTIMEO SO_SNDTIMEO SCM_RIGHTS
MSG_CMSG_CLOEXEC MSG_NOSIGNAL EPOLLIN EPOLLOUT EPOLLERR EPOLLHUP EPOLLET EPOLL_CLOEXEC
SIGINT SIGTERM SIGKILL SIGCHLD SIGPIPE SA_RESTART SA_SIGINFO CLOCK_REALTIME CLOCK_MONOTONIC
PROT_READ PROT_WRITE MAP_PRIVATE MAP_ANONYMOUS MS_SYNC MFD_CLOEXEC
RLIMIT_NOFILE RLIM_INFINITY PR_SET_NAME CLONE_NEWNS STATX_BASIC_STATS STATX_MNT_ID
SYS_openat SYS_statx SYS_copy_file_range SYS_renameat2 SYS_getrandom SYS_futex
""".split()
HEADERS = """
stddef.h stdio.h stdint.h fcntl.h errno.h unistd.h pthread.h signal.h time.h dirent.h
poll.h sched.h ucontext.h pwd.h grp.h netdb.h termios.h netinet/in.h sys/stat.h
sys/statfs.h sys/statvfs.h sys/time.h sys/uio.h sys/socket.h sys/un.h sys/epoll.h
sys/utsname.h sys/resource.h sys/msg.h sys/shm.h sys/sem.h sys/mman.h sys/prctl.h
sys/syscall.h sys/wait.h
""".split()


def run(args, *, cwd=ROOT, env=None, manifest_dir=None):
    if manifest_dir is not None:
        env = (os.environ.copy() if env is None else env.copy())
        env["CARGO_MANIFEST_DIR"] = str(Path(manifest_dir).resolve())
    result = subprocess.run(args, cwd=cwd, env=env, text=True, capture_output=True)
    if result.returncode:
        raise RuntimeError(f"{shlex.join(map(str, args))}\n{result.stdout}\n{result.stderr}")
    return result.stdout


def build_environment(os_name, arch, width, libc_env):
    result = os.environ.copy()
    # No user-specific unstable configuration should leak into the ABI baseline.
    for key in list(result):
        if key.startswith(("CARGO_CFG_", "CARGO_FEATURE_", "RUST_LIBC_")) or key == "LIBC_CI":
            del result[key]
    result.update(CARGO_CFG_TARGET_OS=os_name, CARGO_CFG_TARGET_ARCH=arch,
                  CARGO_CFG_TARGET_POINTER_WIDTH=width, CARGO_CFG_TARGET_ENV=libc_env,
                  CARGO_CFG_TARGET_ABI="", RUSTC="rustc")
    return result


def check_target_gate(build_script):
    accepted = [
        ("linux", "x86_64", "64", "gnu"), ("linux", "x86_64", "64", "musl"),
        ("linux", "x86_64", "64", "uclibc"), ("linux", "x86_64", "64", "ohos"),
        ("linux", "x86_64", "64", ""), ("unknown", "wasm32", "32", ""),
    ]
    rejected = [
        ("linux", "x86_64", "32", "gnu"), ("linux", "aarch64", "64", "gnu"),
        ("linux", "x86", "32", "gnu"), ("linux", "riscv64", "64", "gnu"),
        ("windows", "x86_64", "64", "gnu"), ("windows", "x86_64", "64", "msvc"),
        ("macos", "x86_64", "64", ""), ("android", "x86_64", "64", ""),
        ("wasi", "wasm32", "32", "p1"), ("emscripten", "wasm32", "32", ""),
        ("unknown", "wasm64", "64", ""),
    ]
    for target in accepted:
        run([str(build_script)], env=build_environment(*target))
    for target in rejected:
        result = subprocess.run([str(build_script)], env=build_environment(*target),
                                text=True, capture_output=True)
        assert result.returncode and "foundation libc supports Linux amd64" in result.stderr, target
    print(f"Target gate: {len(accepted)} accepted configurations; {len(rejected)} rejected.")


def probes():
    c = ["#define _GNU_SOURCE 1", *[f"#include <{h}>" for h in HEADERS], "int main(void) {"]
    rust = ["use std::mem::{align_of, offset_of, size_of};", "fn main() {"]

    def emit(name, c_expr, rust_expr):
        c.append(f'printf("{name}=%llu\\n", (unsigned long long)({c_expr}));')
        rust.append(f'println!("{name}={{}}", ({rust_expr}) as u64);')

    for c_type, rust_type, fields in LAYOUTS:
        emit(f"size.{rust_type}", f"sizeof({c_type})", f"size_of::<libc::{rust_type}>()")
        emit(f"align.{rust_type}", f"_Alignof({c_type})", f"align_of::<libc::{rust_type}>()")
        for field in fields:
            emit(f"offset.{rust_type}.{field}", f"offsetof({c_type}, {field})",
                 f"offset_of!(libc::{rust_type}, {field})")
    # libc exposes nanoseconds as flattened fields, while glibc embeds timespec.
    for field, c_field in [("st_atime", "st_atim.tv_sec"), ("st_atime_nsec", "st_atim.tv_nsec"),
                           ("st_mtime", "st_mtim.tv_sec"), ("st_ctime_nsec", "st_ctim.tv_nsec")]:
        emit(f"offset.stat.{field}", f"offsetof(struct stat, {c_field})", f"offset_of!(libc::stat, {field})")
    emit("offset.epoll_event.u64", "offsetof(struct epoll_event, data)", "offset_of!(libc::epoll_event, u64)")
    for value in CONSTANTS:
        # Linux flags are 32-bit bit patterns. C's EPOLLET enum is unsigned,
        # while libc exposes c_int, so compare at ABI width, not after promotion.
        width = 64 if value == "RLIM_INFINITY" else 32
        emit(f"constant.{value}", f"(uint{width}_t)({value})", f"libc::{value} as u{width}")
    for size in (0, 1, 4, 8, 17, 64):
        emit(f"CMSG_SPACE.{size}", f"CMSG_SPACE({size})", f"unsafe {{ libc::CMSG_SPACE({size}) }}")
        emit(f"CMSG_LEN.{size}", f"CMSG_LEN({size})", f"unsafe {{ libc::CMSG_LEN({size}) }}")
    c.extend(["fd_set fds; FD_ZERO(&fds); FD_SET(7, &fds);",
              "cpu_set_t cpus; CPU_ZERO(&cpus); CPU_SET(5, &cpus);"])
    rust.extend(["let mut fds: libc::fd_set = unsafe { std::mem::zeroed() };",
                 "let mut cpus: libc::cpu_set_t = unsafe { std::mem::zeroed() };",
                 "unsafe { libc::FD_ZERO(&mut fds); libc::FD_SET(7, &mut fds); }",
                 "unsafe { libc::CPU_ZERO(&mut cpus); libc::CPU_SET(5, &mut cpus); }"])
    emit("FD_ISSET.7", "FD_ISSET(7, &fds)", "unsafe { libc::FD_ISSET(7, &fds) }")
    emit("FD_ISSET.8", "FD_ISSET(8, &fds)", "unsafe { libc::FD_ISSET(8, &fds) }")
    emit("CPU_ISSET.5", "CPU_ISSET(5, &cpus)", "unsafe { libc::CPU_ISSET(5, &cpus) }")
    emit("CPU_COUNT", "CPU_COUNT(&cpus)", "unsafe { libc::CPU_COUNT(&cpus) }")
    emit("WEXITSTATUS", "WEXITSTATUS(7 << 8)", "libc::WEXITSTATUS(7 << 8)")
    c.append("return 0; }")
    rust.append("}")
    return "\n".join(c) + "\n", "\n".join(rust) + "\n"


def rustc_args(features):
    args = ["rustc", "--edition=2021", "--deny=warnings", "--target", NATIVE,
            "--cfg", "linux_time_bits64"]
    for feature in features:
        args += ["--cfg", f'feature="{feature}"']
    return args


def check_host_abi(temp):
    c_source, rust_source = probes()
    (temp / "probe.c").write_text(c_source)
    (temp / "probe.rs").write_text(rust_source)
    c_probe = temp / "c-probe"
    run([*shlex.split(os.environ.get("CC", "cc")), "-std=gnu11", "-Wall", "-Wextra",
         "-Werror", str(temp / "probe.c"), "-o", str(c_probe)])
    expected = run([str(c_probe)])
    for variant, features in VARIANTS.items():
        rlib = temp / f"liblibc_{variant}.rlib"
        run([*rustc_args(features), "--crate-type=rlib", "src/lib.rs", "-o", str(rlib)], cwd=LIBC, manifest_dir=LIBC)
        binary = temp / f"rust-probe-{variant}"
        run(["rustc", "--edition=2021", "--deny=warnings", "--target", NATIVE, str(temp / "probe.rs"),
             "--extern", f"libc={rlib}", "-o", str(binary)], manifest_dir=ROOT)
        actual = run([str(binary)])
        if actual != expected:
            import difflib
            raise AssertionError("".join(difflib.unified_diff(
                expected.splitlines(True), actual.splitlines(True), fromfile="C headers", tofile=variant)))
        print(f"GNU amd64 {variant}: {len(actual.splitlines())} layout/constant/helper checks match C headers.")
    # The supported Web surface is intentionally empty, but must still compile.
    run(["rustc", "--edition=2021", "--deny=warnings", "--target", WEB, "--cfg", 'feature="std"',
         "--emit=metadata", "src/lib.rs", "-o", str(temp / "web.rmeta")], cwd=LIBC, manifest_dir=LIBC)
    print("wasm32-unknown-unknown: empty libc surface compiles.")


def expanded(source, features, target=NATIVE):
    args = rustc_args(features)
    args[args.index(NATIVE)] = target
    if target == WEB:
        args.remove("linux_time_bits64")
        args.remove("--cfg")
    env = os.environ.copy()
    env["RUSTC_BOOTSTRAP"] = "1"  # Inspection only, not the product build.
    output = run([*args, "-Zunpretty=expanded", "src/lib.rs"], cwd=source, env=env, manifest_dir=source)
    # Only blank lines and full-line comments are excluded. All Rust definitions,
    # attributes, constant data, signatures, and function bodies remain compared.
    return "\n".join(line for line in output.splitlines()
                     if line.strip() and not line.lstrip().startswith("//")).encode()


def compare_revision(revision, temp):
    # Resolve once so an option-like input or moving ref cannot alter the archive.
    commit = run(["git", "rev-parse", "--verify", "--end-of-options", f"{revision}^{{commit}}"]).strip()
    archive = subprocess.check_output(["git", "archive", commit, "--", "src/foundation/libc"], cwd=ROOT)
    old = temp / "original"
    old.mkdir()
    with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
        # Read only regular source files, never links or archive-selected paths
        # outside the requested subtree. This also works on Python 3.11.
        for member in tar:
            path = Path(member.name)
            assert not path.is_absolute() and ".." not in path.parts
            if member.isdir():
                continue
            assert member.isfile() and path.parts[:3] == ("src", "foundation", "libc")
            destination = old / path
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(tar.extractfile(member).read())
    old_libc = old / "src/foundation/libc"
    build_scripts = []
    for label, source in (("old", old_libc), ("new", LIBC)):
        build = temp / f"build-script-{label}"
        run(["rustc", "--edition=2021", "--deny=warnings", str(source / "build.rs"), "-o", str(build)], manifest_dir=source)
        build_scripts.append(build)
    targets = [("linux", "x86_64", "64", libc_env) for libc_env in ("gnu", "musl", "ohos", "uclibc", "")]
    targets.append(("unknown", "wasm32", "32", ""))
    options = [
        {}, {"CARGO_CFG_LIBC_UNSTABLE_TIME64": "1"},
        {"CARGO_CFG_LIBC_UNSTABLE_GNU_TIME_BITS": "32"},
        {"CARGO_CFG_LIBC_UNSTABLE_GNU_TIME_BITS": "64"},
        {"CARGO_CFG_LIBC_UNSTABLE_MUSL_V1_2": "1"},
        {"CARGO_CFG_LIBC_UNSTABLE_UCLIBC_TIME64": "1"},
        {"CARGO_FEATURE_RUSTC_DEP_OF_STD": "1"},
        {"LIBC_CI": "1"},
    ]
    for target in targets:
        for option in options:
            env = build_environment(*target)
            env.update(option)
            before, after = [run([str(build)], env=env) for build in build_scripts]
            assert before == after, f"Build selection changed for {target}, {option}"
    print(f"Build-script selection matches for {len(targets) * len(options)} retained target/configuration inputs.")
    evidence = {}
    for variant, features in {**VARIANTS, "wasm": ["std"]}.items():
        target = WEB if variant == "wasm" else NATIVE
        before = expanded(old_libc, features, target)
        after = expanded(LIBC, features, target)
        assert before == after, f"Selected code/data changed for {variant} versus {commit}"
        evidence[variant] = {"bytes": len(after), "sha256": hashlib.sha256(after).hexdigest()}
    print(f"Selected code/data matches {commit} (only blank/full-comment lines ignored):")
    print(json.dumps(evidence, indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--compare-revision")
    args = parser.parse_args()
    if platform.system() != "Linux" or platform.machine() != "x86_64" or platform.libc_ver()[0] != "glibc":
        raise SystemExit("This check needs a GNU/Linux amd64 host; other ABIs are not validated.")
    with tempfile.TemporaryDirectory(prefix="zxs-libc-abi-") as directory:
        temp = Path(directory)
        build = temp / "build-script"
        run(["rustc", "--edition=2021", "--deny=warnings", str(LIBC / "build.rs"), "-o", str(build)], manifest_dir=LIBC)
        check_target_gate(build)
        check_host_abi(temp)
        if args.compare_revision:
            compare_revision(args.compare_revision, temp)


if __name__ == "__main__":
    main()
