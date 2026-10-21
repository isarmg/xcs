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
    fn parser_keeps_unicode_partial_input_and_binary_contracts_without_tracing() {
        use winnow::{Parser, ModalResult, error::{ContextError, ErrMode, Needed},
                     stream::Partial, token::{literal, take_while}};
        fn assignment<'i>(input: &mut &'i str) -> ModalResult<&'i str> {
            ("name=", take_while(1.., |c: char| c != ';'), ';')
                .map(|(_, name, _)| name).parse_next(input)
        }
        assert_eq!(assignment.parse("name=文件 Ω;").unwrap(), "文件 Ω");
        assert!(assignment.parse("name=;").is_err());
        let mut partial = Partial::new("ab");
        let result: Result<_, ErrMode<ContextError>> = literal("abc").parse_next(&mut partial);
        assert_eq!(result, Err(ErrMode::Incomplete(Needed::Unknown)));
        assert_eq!(*partial, "ab");
        let mut partial_bytes = Partial::new(&b"ab"[..]);
        let result: Result<_, ErrMode<ContextError>> = literal(&b"abc"[..]).parse_next(&mut partial_bytes);
        assert_eq!(result, Err(ErrMode::Incomplete(Needed::Unknown)));
        assert_eq!(*partial_bytes, &b"ab"[..]);
        let mut empty = Partial::new(&b""[..]);
        let result: Result<u8, ErrMode<ContextError>> = winnow::token::any.parse_next(&mut empty);
        assert_eq!(result, Err(ErrMode::Incomplete(Needed::new(1))));
        let result: ModalResult<u16> = winnow::binary::be_u16.parse_next(&mut &[0x12, 0x34][..]);
        assert_eq!(result.unwrap(), 0x1234);
        let source = "title = \"文件 Ω\"\nvalue = 1_024\n";
        let document: toml_edit::DocumentMut = source.parse().unwrap();
        assert_eq!(document["title"].as_str(), Some("文件 Ω"));
        assert_eq!(document["value"].as_integer(), Some(1024));
        assert_eq!(document.to_string(), source);
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
                                                         ('attribute-derive', 'attribute-derive'), ('syn', 'syn-3.0.9'),
                                                         ('winnow', 'winnow'), ('toml_edit', 'toml_edit')])
        manifest = work / 'Cargo.toml'
        manifest.write_text('[package]\nname = "foundation-consumer-regression"\nversion = "0.0.0"\n'
                            'edition = "2021"\n[workspace]\n[dependencies]\n' + dependencies + '\n')
        (work / 'src/lib.rs').write_text(CONSUMER)
        subprocess.run(['cargo', 'generate-lockfile', '--offline', '--manifest-path', str(manifest)], cwd=root, check=True)
        environment = dict(os.environ, BINDGEN_EXTRA_CLANG_ARGS='-DVALUE=7 -DLABEL=\'"hello world"\'')
        subprocess.run(['cargo', 'test', '--locked', '--offline', '--manifest-path', str(manifest),
                        '--target-dir', str(root / 'target/foundation-consumer-check')],
                       cwd=root, env=environment, check=True)
        command = ['cargo', 'check', '--locked', '--offline', '--manifest-path', str(manifest),
                   '--target-dir', str(root / 'target/foundation-consumer-check')]
        (work / 'src/lib.rs').write_text('use winnow::combinator::trace;\n')
        result = subprocess.run(command, cwd=root, capture_output=True, text=True)
        if not result.returncode or 'error[E0432]' not in result.stderr:
            raise AssertionError('retired parser tracing API was accepted or failed for another reason: ' + result.stderr)
        (work / 'src/lib.rs').write_text('')
        for feature in ['winnow/debug', 'toml_edit/unstable-debug']:
            result = subprocess.run(command + ['--features', feature], cwd=root, capture_output=True, text=True)
            if not result.returncode or 'does not have that feature' not in result.stderr:
                raise AssertionError('retired parser debug feature was accepted or failed for another reason: ' + result.stderr)



if __name__ == '__main__':
    check(ROOT)
