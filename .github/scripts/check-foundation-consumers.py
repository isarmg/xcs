#!/usr/bin/env python3
"""Exercise maintained bindgen, attribute diagnostics and terminal output through real consumers."""
from pathlib import Path
import os
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
CONSUMER = r'''
#[cfg(test)]
mod tests {
    use attribute_derive::FromAttr;
    use std::io::Write;

    #[derive(Debug, FromAttr)]
    #[from_attr(ident = one)]
    struct One { required: String }

    #[derive(Debug, FromAttr)]
    struct Two { alpha: Option<String>, beta: Option<String> }

    #[derive(Debug, FromAttr)]
    struct Flags {
        #[from_attr(conflicts = [beta])]
        alpha: bool,
        beta: bool,
    }

    #[test]
    fn attribute_templates_preserve_missing_duplicate_unknown_and_conflict_diagnostics() {
        assert_eq!(syn::parse_str::<One>(r#"required = "文件 Ω""#).unwrap().required, "文件 Ω");
        let unknown = syn::parse_str::<One>("unknown = 1").unwrap_err().to_string();
        assert_eq!(unknown, "expected supported field `required`");
        let missing = syn::parse_str::<One>("").unwrap_err().to_string();
        assert_eq!(missing, "required `required` is not specified\n\n= help: try `#[one(required = ...)]`");
        let duplicate = syn::parse_str::<One>(r#"required = "a", required = "b""#).unwrap_err().to_string();
        assert_eq!(duplicate, "`required` is specified multiple times");
        let two = syn::parse_str::<Two>("").unwrap();
        assert!(two.alpha.is_none() && two.beta.is_none());
        assert_eq!(syn::parse_str::<Two>("unknown = 1").unwrap_err().to_string(),
                   "supported fields are `alpha` and `beta`");
        let flags = syn::parse_str::<Flags>("alpha").unwrap();
        assert!(flags.alpha && !flags.beta);
        let conflict = syn::parse_str::<Flags>("alpha, beta").unwrap_err().to_string();
        assert!(conflict.contains("conflicts with mutually exclusive"), "{conflict}");
        assert!(conflict.contains("alpha") && conflict.contains("beta"), "{conflict}");
    }

    #[test]
    fn terminal_output_preserves_unicode_and_explicit_color_policy() {
        let bytes = "\x1b[31m文件 Ω\x1b[0m\n".as_bytes();
        let mut plain = anstream::AutoStream::new(Vec::new(), anstream::ColorChoice::Never);
        plain.write_all(bytes).unwrap();
        assert_eq!(plain.into_inner(), "文件 Ω\n".as_bytes());
        let mut color = anstream::AutoStream::new(Vec::new(), anstream::ColorChoice::Always);
        color.write_all(bytes).unwrap();
        assert_eq!(color.into_inner(), bytes);
        let file = std::fs::File::open("/dev/null").unwrap();
        assert!(!anstream::stream::IsTerminal::is_terminal(&file));
    }

    #[test]
    fn clang_arguments_and_bitfield_runs_preserve_neighboring_fields() {
        // The invoking script supplies a shell-quoted definition containing a space.
        let source = r#"
            #if VALUE != 7
            #error expected external clang arguments
            #endif
            static const char *label = LABEL;
            struct Packet {
                int leading;
                unsigned low : 3;
                unsigned high : 5;
                int middle;
                unsigned last : 2;
                int trailing;
            };
        "#;
        let output = bindgen::Builder::default()
            .header_contents("foundation-consumer.h", source)
            .layout_tests(false)
            .generate().expect("actual C parsing and binding generation").to_string();
        for field in ["pub leading:", "pub middle:", "pub trailing:", "fn low(", "fn high(", "fn last("] {
            assert!(output.contains(field), "missing {field}: {output}");
        }
    }
}
'''


def check(root):
    (root / 'target').mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='foundation-consumers-', dir=root / 'target') as temporary:
        work = Path(temporary)
        (work / 'src').mkdir()
        dependencies = '\n'.join(f'{name} = {{path = "{root / "src/foundation" / directory}"}}'
                                 for name, directory in [('bindgen', 'bindgen'), ('anstream', 'anstream'),
                                                         ('attribute-derive', 'attribute-derive'), ('syn', 'syn-3.0.9')])
        manifest = work / 'Cargo.toml'
        manifest.write_text('[package]\nname = "foundation-consumer-regression"\nversion = "0.0.0"\n'
                            'edition = "2021"\n[workspace]\n[dependencies]\n' + dependencies + '\n')
        (work / 'src/lib.rs').write_text(CONSUMER)
        subprocess.run(['cargo', 'generate-lockfile', '--offline', '--manifest-path', str(manifest)], cwd=root, check=True)
        environment = dict(os.environ, BINDGEN_EXTRA_CLANG_ARGS='-DVALUE=7 -DLABEL=\'"hello world"\'')
        subprocess.run(['cargo', 'test', '--locked', '--offline', '--manifest-path', str(manifest),
                        '--target-dir', str(root / 'target/foundation-consumer-check')],
                       cwd=root, env=environment, check=True)


if __name__ == '__main__':
    check(ROOT)
