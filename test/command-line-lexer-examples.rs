// In-crate copies of lexer documentation examples. The lexer stays private.
// Original lexer mod.rs, example 1; flags: rust.
#[test]
fn lexer_documentation_example_1() {
use std::path::PathBuf;
use std::ffi::OsStr;

type BoxedError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug)]
struct Args {
    paths: Vec<PathBuf>,
    color: Color,
    verbosity: usize,
}

#[derive(Debug)]
enum Color {
    Always,
    Auto,
    Never,
}

impl Color {
    fn parse(s: Option<&OsStr>) -> Result<Self, BoxedError> {
        let s = s.map(|s| s.to_str().ok_or(s));
        match s {
            Some(Ok("always")) | Some(Ok("")) | None => {
                Ok(Color::Always)
            }
            Some(Ok("auto")) => {
                Ok(Color::Auto)
            }
            Some(Ok("never")) => {
                Ok(Color::Never)
            }
            Some(invalid) => {
                Err(format!("Invalid value for `--color`, {invalid:?}").into())
            }
        }
    }
}

fn parse_args(
    raw: impl IntoIterator<Item=impl Into<std::ffi::OsString>>
) -> Result<Args, BoxedError> {
    let mut args = Args {
        paths: Vec::new(),
        color: Color::Auto,
        verbosity: 0,
    };

    let raw = crate::lexer::RawArgs::new(raw);
    let mut cursor = raw.cursor();
    raw.next(&mut cursor);  // Skip the bin
    while let Some(arg) = raw.next(&mut cursor) {
        if arg.is_escape() {
            args.paths.extend(raw.remaining(&mut cursor).map(PathBuf::from));
        } else if arg.is_stdio() {
            args.paths.push(PathBuf::from("-"));
        } else if let Some((long, value)) = arg.to_long() {
            match long {
                Ok("verbose") => {
                    if let Some(value) = value {
                        return Err(format!("`--verbose` does not take a value, got `{value:?}`").into());
                    }
                    args.verbosity += 1;
                }
                Ok("color") => {
                    args.color = Color::parse(value)?;
                }
                _ => {
                    return Err(
                        format!("Unexpected flag: --{}", arg.display()).into()
                    );
                }
            }
        } else if let Some(mut shorts) = arg.to_short() {
            while let Some(short) = shorts.next_flag() {
                match short {
                    Ok('v') => {
                        args.verbosity += 1;
                    }
                    Ok('c') => {
                        let value = shorts.next_value_os();
                        args.color = Color::parse(value)?;
                    }
                    Ok(c) => {
                        return Err(format!("Unexpected flag: -{c}").into());
                    }
                    Err(e) => {
                        return Err(format!("Unexpected flag: -{}", e.to_string_lossy()).into());
                    }
                }
            }
        } else {
            args.paths.push(PathBuf::from(arg.to_value_os().to_owned()));
        }
    }

    Ok(args)
}

let args = parse_args(["bin", "--hello", "world"]);
println!("{args:?}");
}

// Original lexer mod.rs, example 2; flags: rust,no_run.
#[allow(dead_code)]
fn lexer_documentation_example_2() {
use std::path::PathBuf;
let raw = crate::lexer::RawArgs::from_args();
let mut cursor = raw.cursor();
let _bin = raw.next_os(&mut cursor);

let mut paths = raw.remaining(&mut cursor).map(PathBuf::from).collect::<Vec<_>>();
println!("{paths:?}");
}

// Original lexer mod.rs, example 3; flags: rust,no_run.
#[allow(dead_code)]
fn lexer_documentation_example_3() {
use std::path::PathBuf;
let raw = crate::lexer::RawArgs::new(["bin", "foo.txt"]);
let mut cursor = raw.cursor();
let _bin = raw.next_os(&mut cursor);

let mut paths = raw.remaining(&mut cursor).map(PathBuf::from).collect::<Vec<_>>();
println!("{paths:?}");
}

// Original lexer mod.rs, example 4; flags: rust,no_run.
#[allow(dead_code)]
fn lexer_documentation_example_4() {
use std::path::PathBuf;
let raw = crate::lexer::RawArgs::new(["bin", "foo.txt"]);
let mut cursor = raw.cursor();
let _bin = raw.next_os(&mut cursor);

let mut paths = raw.remaining(&mut cursor).map(PathBuf::from).collect::<Vec<_>>();
println!("{paths:?}");
}

// Original lexer mod.rs, example 5; flags: rust,no_run.
#[allow(dead_code)]
fn lexer_documentation_example_5() {
use std::path::PathBuf;
let raw = crate::lexer::RawArgs::new(["bin", "foo.txt"]);
let mut cursor = raw.cursor();
let _bin = raw.next_os(&mut cursor);

let mut paths = raw.remaining(&mut cursor).map(PathBuf::from).collect::<Vec<_>>();
println!("{paths:?}");
}

// Original lexer ext.rs, example 6; flags: rust.
#[test]
fn lexer_documentation_example_6() {
use crate::lexer::OsStrExt as _;
let bananas = std::ffi::OsStr::new("bananas");

assert!(bananas.contains("nana"));
assert!(!bananas.contains("apples"));
}

// Original lexer ext.rs, example 7; flags: rust.
#[test]
fn lexer_documentation_example_7() {
use crate::lexer::OsStrExt as _;
let s = std::ffi::OsStr::new("Löwe 老虎 Léopard Gepardi");

assert_eq!(s.find("L"), Some(0));
assert_eq!(s.find("é"), Some(14));
assert_eq!(s.find("par"), Some(17));
}

// Original lexer ext.rs, example 8; flags: rust.
#[test]
fn lexer_documentation_example_8() {
use crate::lexer::OsStrExt as _;
let s = std::ffi::OsStr::new("Löwe 老虎 Léopard");

assert_eq!(s.find("1"), None);
}

// Original lexer ext.rs, example 9; flags: rust.
#[test]
fn lexer_documentation_example_9() {
use std::ffi::OsStr;
use crate::lexer::OsStrExt as _;
assert_eq!(OsStr::new("foo:bar").strip_prefix("foo:"), Some(OsStr::new("bar")));
assert_eq!(OsStr::new("foo:bar").strip_prefix("bar"), None);
assert_eq!(OsStr::new("foofoo").strip_prefix("foo"), Some(OsStr::new("foo")));
}

// Original lexer ext.rs, example 10; flags: rust.
#[test]
fn lexer_documentation_example_10() {
use crate::lexer::OsStrExt as _;
let bananas = std::ffi::OsStr::new("bananas");

assert!(bananas.starts_with("bana"));
assert!(!bananas.starts_with("nana"));
}

// Original lexer ext.rs, example 11; flags: rust.
#[test]
fn lexer_documentation_example_11() {
use std::ffi::OsStr;
use crate::lexer::OsStrExt as _;
let v: Vec<_> = OsStr::new("Mary had a little lamb").split(" ").collect();
assert_eq!(v, [OsStr::new("Mary"), OsStr::new("had"), OsStr::new("a"), OsStr::new("little"), OsStr::new("lamb")]);

let v: Vec<_> = OsStr::new("").split("X").collect();
assert_eq!(v, [OsStr::new("")]);

let v: Vec<_> = OsStr::new("lionXXtigerXleopard").split("X").collect();
assert_eq!(v, [OsStr::new("lion"), OsStr::new(""), OsStr::new("tiger"), OsStr::new("leopard")]);

let v: Vec<_> = OsStr::new("lion::tiger::leopard").split("::").collect();
assert_eq!(v, [OsStr::new("lion"), OsStr::new("tiger"), OsStr::new("leopard")]);
}

// Original lexer ext.rs, example 12; flags: rust.
#[test]
fn lexer_documentation_example_12() {
use std::ffi::OsStr;
use crate::lexer::OsStrExt as _;
let x = OsStr::new("||||a||b|c");
let d: Vec<_> = x.split("|").collect();

assert_eq!(d, &[OsStr::new(""), OsStr::new(""), OsStr::new(""), OsStr::new(""), OsStr::new("a"), OsStr::new(""), OsStr::new("b"), OsStr::new("c")]);
}

// Original lexer ext.rs, example 13; flags: rust.
#[test]
fn lexer_documentation_example_13() {
use std::ffi::OsStr;
use crate::lexer::OsStrExt as _;
let x = OsStr::new("(///)");
let d: Vec<_> = x.split("/").collect();

assert_eq!(d, &[OsStr::new("("), OsStr::new(""), OsStr::new(""), OsStr::new(")")]);
}

// Original lexer ext.rs, example 14; flags: rust.
#[test]
fn lexer_documentation_example_14() {
use std::ffi::OsStr;
use crate::lexer::OsStrExt as _;
let d: Vec<_> = OsStr::new("010").split("0").collect();
assert_eq!(d, &[OsStr::new(""), OsStr::new("1"), OsStr::new("")]);
}

// Original lexer ext.rs, example 15; flags: should_panic.
#[test]
#[should_panic]
fn lexer_documentation_example_15() {
use std::ffi::OsStr;
use crate::lexer::OsStrExt as _;
let f: Vec<_> = OsStr::new("rust").split("").collect();
assert_eq!(f, &[OsStr::new(""), OsStr::new("r"), OsStr::new("u"), OsStr::new("s"), OsStr::new("t"), OsStr::new("")]);
}

// Original lexer ext.rs, example 16; flags: rust.
#[test]
fn lexer_documentation_example_16() {
use std::ffi::OsStr;
use crate::lexer::OsStrExt as _;
let x = OsStr::new("    a  b c");
let d: Vec<_> = x.split(" ").collect();

assert_eq!(d, &[OsStr::new(""), OsStr::new(""), OsStr::new(""), OsStr::new(""), OsStr::new("a"), OsStr::new(""), OsStr::new("b"), OsStr::new("c")]);
}

// Original lexer ext.rs, example 17; flags: ,ignore.
#[test]
#[ignore = "upstream documentation marks this example ignored"]
fn lexer_documentation_example_17() {
// assert_eq!(d, &[OsStr::new("a"), OsStr::new("b"), OsStr::new("c")]);
}

// Original lexer ext.rs, example 18; flags: rust.
#[test]
fn lexer_documentation_example_18() {
use std::ffi::OsStr;
use crate::lexer::OsStrExt as _;
assert_eq!(OsStr::new("cfg").split_once("="), None);
assert_eq!(OsStr::new("cfg=").split_once("="), Some((OsStr::new("cfg"), OsStr::new(""))));
assert_eq!(OsStr::new("cfg=foo").split_once("="), Some((OsStr::new("cfg"), OsStr::new("foo"))));
assert_eq!(OsStr::new("cfg=foo=bar").split_once("="), Some((OsStr::new("cfg"), OsStr::new("foo=bar"))));
}
