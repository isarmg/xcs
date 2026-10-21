#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escaping_depfile() {
        let spec = DepfileSpec {
            output_module: "Mod Name".to_owned(),
            depfile_path: PathBuf::new(),
        };

        let deps: BTreeSet<_> = vec![
            r"/absolute/path".into(),
            r"C:\win\absolute\path".into(),
            r"../relative/path".into(),
            r"..\win\relative\path".into(),
            r"../path/with spaces/in/it".into(),
            r"..\win\path\with spaces\in\it".into(),
            r"path\with/mixed\separators".into(),
        ]
        .into_iter()
        .collect();
        assert_eq!(
            spec.to_string(&deps),
            "Mod\\ Name: \
            ../path/with\\ spaces/in/it \
            ../relative/path \
            ..\\\\win\\\\path\\\\with\\ spaces\\\\in\\\\it \
            ..\\\\win\\\\relative\\\\path \
            /absolute/path \
            C:\\\\win\\\\absolute\\\\path \
            path\\\\with/mixed\\\\separators"
        );
    }
}
