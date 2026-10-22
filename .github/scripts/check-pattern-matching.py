#!/usr/bin/env python3
"""Exercise retained regex capabilities without inventing absent upstream providers."""
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[2]
REMOVED = ("arbitrary", "pattern", "unstable", "use_std", "perf-cache")
WASM = ("", "syntax", "syntax,unicode", "dfa-search", "literal", "literal,literal-perf",
        "nfa-thompson", "nfa-pikevm,syntax", "dfa-build,syntax", "hybrid,syntax",
        "matcher", "matcher,unicode", "automata-full")
BOUNDARIES = [
 ('syntax-positive',['syntax'],'#![no_std]\npub fn parse(s: &str) { let _ = regex::syntax::Parser::new().parse(s); }',True,None),
 ('syntax-no-matcher',['syntax'],'#![no_std]\npub fn denied() { let _ = regex::Regex::new("x"); }',False,'could not find `Regex`'),
 ('syntax-no-engine',['syntax'],'#![no_std]\npub fn denied() { let _ = regex::automata::meta::Regex::new("x"); }',False,'could not find `automata`'),
 ('dfa-noalloc-positive',['dfa-search'],'#![no_std]\npub fn decode(bytes: &[u8]) { let _ = regex::automata::dfa::dense::DFA::from_bytes(bytes); }',True,None),
 ('dfa-no-parser',['dfa-search'],'#![no_std]\npub fn denied() { let _ = regex::syntax::Parser::new(); }',False,'could not find `syntax`'),
 ('literal-no-engine',['literal'],'#![no_std]\npub fn denied() { let _ = regex::automata::meta::Regex::new("x"); }',False,'could not find `automata`'),
 ('literal-positive',['literal'],'#![no_std]\npub fn search() { let a = regex::literal::AhoCorasick::new(["a", "b"]).unwrap(); assert!(a.is_match("b")); }',True,None),
 ('matcher-no-std',['matcher'],'#![no_std]\npub fn search() { let a = regex::Regex::new("[a-z]+").unwrap(); assert!(a.is_match("abc")); }',True,None),
]


def run(command, env, expected=True, needle=None):
    result = subprocess.run(command, cwd=ROOT, env=env, text=True,
                            stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=600)
    print(result.stdout, end="", flush=True)
    if (result.returncode == 0) != expected:
        raise AssertionError(f"unexpected compiler/test result: {command}")
    if needle and needle not in result.stdout:
        raise AssertionError(f"negative failed for unrelated reason: {needle}")
    if expected:
        for line in result.stdout.splitlines():
            if not line.startswith("{"):
                continue
            try:
                event = json.loads(line)
            except json.JSONDecodeError:
                continue
            if event.get("reason") == "compiler-message":
                message = event["message"]
                if message["level"] == "warning" and event.get("target", {}).get("name") == "regex":
                    raise AssertionError(f"regex warning: {message['message']}")
    return result


def fixture(root, work):
    package = work / "src/foundation/regex"
    shutil.copytree(root / "src/foundation/regex", package)
    for relative in ("test/foundation/regex", "test/regex-family"):
        shutil.copytree(root / relative, work / relative)
    manifest = package / "Cargo.toml"
    text = manifest.read_text()
    text = re.sub(r'path = "\.\./([^/"\n]+)"',
                  lambda m: "path = " + json.dumps(str(root / "src/foundation" / m[1])), text)
    manifest.write_text(text + "\n[workspace]\n")
    return manifest


def probe_manifest(probe, dependencies):
    (probe / "src").mkdir(parents=True, exist_ok=True)
    (probe / "Cargo.toml").write_text(
        '[package]\nname = "pattern-boundary-probe"\nversion = "0.0.0"\nedition = "2021"\n'
        '[workspace]\n' + dependencies)
    return probe / "Cargo.toml"


def main():
    target = ROOT / "target/scope-consumers/regex-family"
    target.parent.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ, CARGO_BUILD_JOBS="2", CARGO_INCREMENTAL="0",
               CARGO_PROFILE_DEV_DEBUG="0", CARGO_PROFILE_TEST_DEBUG="0", CARGO_TARGET_DIR=str(target))
    with tempfile.TemporaryDirectory(prefix="regex-family-", dir=target.parent) as temporary:
        work = Path(temporary)
        manifest = fixture(ROOT, work / "retained")
        run(["cargo", "generate-lockfile", "--offline", "--manifest-path", str(manifest)], env)
        common = ["--offline", "--locked", "--manifest-path", str(manifest)]
        features = tomllib.loads(manifest.read_text())["features"]
        matrix = [["--no-default-features"], ["--all-features"]]
        matrix += [["--no-default-features", "--features", f] for f in features if f != "default"]
        matrix += [["--target", "wasm32-unknown-unknown", "--no-default-features"] +
                   (["--features", f] if f else []) for f in WASM]
        matrix += [["--target", "wasm32-unknown-unknown", "--all-features"]]
        for args in matrix:
            run(["cargo", "check", *common, "--message-format=json", *args], env)
        print(f"Regex matrix: {len(matrix)} successful feature/target checks", flush=True)
        for features in ("syntax,std,unicode", "literal,std,literal-perf"):
            run(["cargo", "test", *common, "--no-default-features", "--features", features, "--lib"], env)
        run(["cargo", "test", *common, "--all-features", "--test", "retained-regressions"], env)
        run(["cargo", "test", *common, "--all-features", "--test", "retained-regressions", "regression_fuzz::fuzz1", "--", "--ignored", "--exact"], env)
        run(["cargo", "test", *common, "--all-features", "--doc"], env)
        probe = work / "probe"
        for name, features, source, success, needle in BOUNDARIES:
            dep = '[dependencies.regex]\npath = ' + json.dumps(str(ROOT / "src/foundation/regex"))
            dep += '\ndefault-features = false\nfeatures = ' + json.dumps(features) + '\n'
            probe_manifest(probe, dep)
            (probe / "src/lib.rs").write_text(source + "\n")
            run(["cargo", "check", "--offline", "--manifest-path", str(probe / "Cargo.toml")], env, success, needle)
            print(f"Regex capability boundary: {name}", flush=True)
        for feature in REMOVED:
            dep = '[dependencies.regex]\npath = ' + json.dumps(str(ROOT / "src/foundation/regex"))
            dep += '\ndefault-features = false\nfeatures = ' + json.dumps([feature]) + '\n'
            probe_manifest(probe, dep)
            run(["cargo", "check", "--offline", "--manifest-path", str(probe / "Cargo.toml")], env,
                False, "does not have")
        consumers = work / "consumers"
        deps = ""
        for name in ("regex", "tracing", "fancy-regex", "schema-validation"):
            deps += f"[dependencies.{name}]\npath = " + json.dumps(str(ROOT / "src/foundation" / name)) + "\n"
            if name == "schema-validation":
                deps += 'default-features = false\nfeatures = ["pattern"]\n'
            if name == "tracing":
                deps += 'features = ["env-filter"]\n'
        deps += '[[test]]\nname = "consumers"\npath = ' + json.dumps(str(ROOT / "test/regex-family/consumers.rs")) + "\n"
        probe_manifest(consumers, deps)
        (consumers / "src/lib.rs").write_text("")
        run(["cargo", "test", "--offline", "--manifest-path", str(consumers / "Cargo.toml")], env)
        run(["cargo", "check", "--locked", "--offline", "-p", "tracing", "-p", "fancy-regex",
             "-p", "schema-validation"], env)
        # The 17 retained matcher unit tests run in check-tracing-family.py,
        # under the required async shard; do not duplicate the owned family gate.
    print("Regex retained tests, capabilities, real consumers and removed-feature rejection: PASS")


if __name__ == "__main__":
    main()
