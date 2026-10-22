use std::{
    any::Any,
    fs,
    path::Path,
    process::Command,
    sync::{Arc, Mutex},
};

#[derive(Default)]
struct Messages(Mutex<Vec<String>>);
impl cc::BuildMessageLogger for Messages {
    fn log(&self, kind: cc::BuildMessageKind, msg: cc::BuildMessage<'_>, _: &dyn Any) {
        if kind == cc::BuildMessageKind::StderrForwarding {
            self.0.lock().unwrap().push(msg.to_string());
        }
    }
}

fn build(out: &Path) -> cc::Build {
    let mut build = cc::Build::new();
    build
        .host("x86_64-unknown-linux-gnu")
        .target("x86_64-unknown-linux-gnu")
        .out_dir(out)
        .opt_level(0)
        .debug(false)
        .cargo_metadata(false);
    build
}

fn main() {
    assert_eq!(
        blake3::hash(b"").to_hex().as_str(),
        "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"
    );
    std::env::set_var("CC", "cc -DZXS_CC_FLAG=17");
    std::env::set_var("CFLAGS", "-DZXS_CFLAGS_VALUE=19");
    std::env::set_var(
        "ZXS_NATIVE_FLAGS",
        "-DZXS_C_VALUE=7 -DZXS_C_TEXT='\"two words\"'",
    );
    let root = std::env::current_dir().unwrap();
    let out = root.join("objects with spaces");
    fs::create_dir(&out).unwrap();
    let c = out.join("probe source.c");
    let asm = out.join("probe assembly.S");
    fs::write(&c, "#ifndef ZXS_C_VALUE\n#error missing C flag\n#endif\n_Static_assert(sizeof(ZXS_C_TEXT) == 10, \"quoted flag\");\n_Static_assert(ZXS_CC_FLAG == 17 && ZXS_CFLAGS_VALUE == 19, \"compiler environment\");\nint c_value(void) { return ZXS_C_VALUE; }\n").unwrap();
    fs::write(&asm, ".text\n.globl asm_value\n.type asm_value,@function\nasm_value:\nmov $ZXS_ASM_VALUE, %eax\nret\n.section .note.GNU-stack,\"\",@progbits\n").unwrap();
    let marker = "zxs_stderr_".to_owned() + &"x".repeat(256);
    let source = fs::read_to_string(&c).unwrap();
    fs::write(&c, format!("#warning {marker}\n{source}")).unwrap();
    let messages = Arc::new(Messages::default());
    let mut native = build(&out);
    native
        .message_logger(Some(messages.clone()))
        .cargo_warnings(false);
    native
        .shell_escaped_flags(true)
        .try_flags_from_environment("ZXS_NATIVE_FLAGS")
        .unwrap();
    assert!(native.is_flag_supported("-fno-strict-aliasing").unwrap());
    assert!(!native
        .is_flag_supported("-fzxs-nonexistent-compiler-flag")
        .unwrap());
    native
        .file(&c)
        .file(&asm)
        .asm_flag("-DZXS_ASM_VALUE=11")
        .flag_if_supported("-fno-strict-aliasing")
        .try_compile("native_probe")
        .unwrap();
    assert!(
        messages
            .0
            .lock()
            .unwrap()
            .iter()
            .any(|line| line.contains(&marker)),
        "long compiler stderr line was lost"
    );
    let cpp = out.join("probe source.cc");
    fs::write(&cpp, "extern \"C\" int cpp_value() { return 13; }\n").unwrap();
    build(&out)
        .cpp(true)
        .file(cpp)
        .try_compile("cpp_probe")
        .unwrap();
    let main = out.join("main.c");
    fs::write(&main, "int c_value(void); int asm_value(void); int cpp_value(void); int main(void) { return c_value() == 7 && asm_value() == 11 && cpp_value() == 13 ? 0 : 1; }\n").unwrap();
    let exe = out.join("linked-probe");
    assert!(Command::new("cc")
        .arg(&main)
        .arg(out.join("libnative_probe.a"))
        .arg(out.join("libcpp_probe.a"))
        .arg("-o")
        .arg(&exe)
        .status()
        .unwrap()
        .success());
    assert!(Command::new(exe).status().unwrap().success());
    for target in [
        "x86_64-pc-windows-msvc",
        "aarch64-unknown-linux-gnu",
        "x86_64-apple-darwin",
        "x86_64-unknown-linux-gnux32",
        "wasm32-wasip1",
    ] {
        assert!(
            build(&out).target(target).try_get_compiler().is_err(),
            "accepted unsupported target {target}"
        );
    }
    assert_eq!(
        shell_tokenizer::split("-DVALUE='two words' -I\"include dir\"").unwrap(),
        ["-DVALUE=two words", "-Iinclude dir"]
    );
    println!("native C/C++/assembly compilation, archive link/run, flag probes, target rejection and shell tokenization passed");
}
