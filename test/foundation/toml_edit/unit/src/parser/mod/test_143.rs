#[cfg(test)]
#[cfg(feature = "parse")]
#[cfg(feature = "display")]
mod test {
    use super::*;
    use snapbox::assert_data_eq;
    use snapbox::prelude::*;

    #[test]
    fn documents() {
        let documents = [
            "",
            r#"
# This is a TOML document.

title = "TOML Example"

    [owner]
    name = "Tom Preston-Werner"
    dob = 1979-05-27T07:32:00-08:00 # First class dates

    [database]
    server = "192.168.1.1"
    ports = [ 8001, 8001, 8002 ]
    connection_max = 5000
    enabled = true

    [servers]

    # Indentation (tabs and/or spaces) is allowed but not required
[servers.alpha]
    ip = "10.0.0.1"
    dc = "eqdc10"

    [servers.beta]
    ip = "10.0.0.2"
    dc = "eqdc10"

    [clients]
    data = [ ["gamma", "delta"], [1, 2] ]

    # Line breaks are OK when inside arrays
hosts = [
    "alpha",
    "omega"
]

   'some.weird .stuff'   =  """
                         like
                         that
                      #   """ # this broke my syntax highlighting
   " also. like " = '''
that
'''
   double = 2e39 # this number looks familiar
# trailing comment"#,
            r#""#,
            r#"  "#,
            r#" hello = 'darkness' # my old friend
"#,
            r#"[parent . child]
key = "value"
"#,
            r#"hello.world = "a"
"#,
            r#"foo = 1979-05-27 # Comment
"#,
        ];
        for input in documents {
            dbg!(input);
            let parsed = parse_document(input).map(|d| d.into_mut());
            let doc = match parsed {
                Ok(doc) => doc,
                Err(err) => {
                    panic!("Parse error: {err:?}\nFailed to parse:\n```\n{input}\n```")
                }
            };

            assert_data_eq!(doc.to_string(), input.raw());
        }
    }

    #[test]
    fn documents_parse_only() {
        let parse_only = ["\u{FEFF}
[package]
name = \"foo\"
version = \"0.0.1\"
authors = []
"];
        for input in parse_only {
            dbg!(input);
            let parsed = parse_document(input).map(|d| d.into_mut());
            match parsed {
                Ok(_) => (),
                Err(err) => {
                    panic!("Parse error: {err:?}\nFailed to parse:\n```\n{input}\n```")
                }
            }
        }
    }

    #[test]
    fn invalid_documents() {
        let invalid_inputs = [r#" hello = 'darkness' # my old friend
$"#];
        for input in invalid_inputs {
            dbg!(input);
            let parsed = parse_document(input).map(|d| d.into_mut());
            assert!(parsed.is_err(), "Input: {input:?}");
        }
    }
}
