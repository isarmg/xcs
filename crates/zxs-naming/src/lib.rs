//! PortableNameV2 / PortablePathV1 with fixed Unicode 16.0 data.
#![forbid(unsafe_code)]
use unicode_normalization::UnicodeNormalization;
use zxs_contract::{Error, Result};
mod unicode16;
pub const UNICODE_VERSION: &str = "16.0.0";
pub const NAME_POLICY: &str = "PortableNameV2";
fn in_ranges(cp: u32, ranges: &[(u32, u32)]) -> bool {
    let i = ranges.partition_point(|(_, end)| *end < cp);
    ranges
        .get(i)
        .is_some_and(|(start, end)| (*start..=*end).contains(&cp))
}
pub fn lookup_key(name: &str) -> String {
    let nfc = name.nfc().collect::<String>();
    let mut out = String::new();
    for c in nfc.chars() {
        match unicode16::FOLD.binary_search_by_key(&(c as u32), |(v, _)| *v) {
            Ok(i) => out.push_str(unicode16::FOLD[i].1),
            Err(_) => out.push(c),
        }
    }
    out.nfc().collect()
}
pub fn validate_name(name: &str) -> Result<()> {
    let err = |code: &str, message: &str| Error::new(code, message);
    if name.is_empty() || [".", ".."].contains(&name) {
        return Err(err("NAME_INVALID_CHARACTER", "名称不能为空或点路径"));
    }
    if name.len() > 240 || name.encode_utf16().count() > 240 {
        return Err(err("NAME_TOO_LONG", "名称超过240字节或UTF-16代码单元"));
    }
    if name.starts_with(' ') || name.ends_with([' ', '.']) {
        return Err(err(
            "NAME_INVALID_CHARACTER",
            "名称不能以空格开头或以空格、句点结尾",
        ));
    }
    let ascii = " !#$%&'()+,-.;=@[]^_`{}~";
    let cjk = "、。，；：？！“”‘’（）【】《》〈〉「」『』〔〕·—…";
    for c in name.chars() {
        let cp = c as u32;
        if in_ranges(cp, unicode16::IGNORED)
            || (!in_ranges(cp, unicode16::ALLOWED) && !ascii.contains(c) && !cjk.contains(c))
        {
            return Err(Error::new(
                "NAME_INVALID_CHARACTER",
                format!("名称包含不支持的字符 U+{cp:04X}"),
            ));
        }
    }
    // Use the repository's fixed Unicode table even for defensive comparisons;
    // Rust's host-version Unicode uppercase tables must not change this policy.
    let stem = lookup_key(
        name.split('.')
            .next()
            .unwrap_or("")
            .trim_end_matches([' ', '.']),
    );
    let reserved = ["con", "prn", "aux", "nul", "conin$", "conout$", "clock$"];
    let numeric = stem
        .strip_prefix("com")
        .or_else(|| stem.strip_prefix("lpt"))
        .is_some_and(|s| s.chars().count() == 1 && "0123456789¹²³".contains(s));
    if reserved.contains(&stem.as_str()) || numeric {
        return Err(err("NAME_RESERVED", "名称属于设备保留名"));
    }
    if name.nfc().collect::<String>() != name {
        return Err(Error::new(
            "NAME_NOT_NORMALIZED",
            format!(
                "名称须明确确认为NFC形式：{}",
                name.nfc().collect::<String>()
            ),
        ));
    }
    Ok(())
}
pub fn validate_path(path: &str) -> Result<()> {
    if path.starts_with('/')
        || path.ends_with('/')
        || path.contains('\\')
        || path.len() > 768
        || path.encode_utf16().count() > 200
    {
        return Err(Error::new(
            "PATH_INVALID",
            "路径不是允许的仓库相对路径或超过长度预算",
        ));
    }
    let components = path.split('/').collect::<Vec<_>>();
    if components.len() > 32 {
        return Err(Error::new("PATH_TOO_DEEP", "目录层级超过32"));
    }
    for c in components {
        validate_name(c)?;
    }
    Ok(())
}
pub fn path_key(path: &str) -> String {
    path.split('/')
        .map(lookup_key)
        .collect::<Vec<_>>()
        .join("/")
}
pub fn validate_slot(slot: &str) -> Result<()> {
    if slot.is_empty()
        || slot.len() > 64
        || !slot.as_bytes()[0].is_ascii_alphabetic()
        || !slot
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err(Error::new(
            "INVALID_SLOT_ID",
            "盘位标识须为字母开头的ASCII字母数字、下划线或连字符，最多64字节",
        ));
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn portable_names() {
        for s in [
            "报告-2026.docx",
            ".gitignore",
            "~$报告.docx",
            "报告（终稿）.docx",
            "项目、预算.xlsx",
        ] {
            validate_name(s).unwrap();
        }
        for s in [
            "a:b.txt",
            "CON",
            "nul.txt",
            "COM¹.txt",
            "LPT².txt",
            "CON .txt",
            " bad",
            "bad.",
            ".",
            "..",
            "a\u{200d}b",
            "🚀",
        ] {
            assert!(validate_name(s).is_err(), "{s}");
        }
    }
    #[test]
    fn normalized_and_folded() {
        assert_eq!(lookup_key("Straße.txt"), "strasse.txt");
        assert_eq!(lookup_key("Σ"), lookup_key("ς"));
        assert_eq!(lookup_key("e\u{301}"), lookup_key("é"));
        assert_eq!(
            validate_name("e\u{301}").unwrap_err().code(),
            "NAME_NOT_NORMALIZED"
        );
        assert_eq!(unicode_normalization::UNICODE_VERSION, (16, 0, 0));
    }
    #[test]
    fn path_budgets() {
        assert!(validate_path("../escape").is_err());
        assert!(validate_path("a//b").is_err());
        assert!(validate_path("/root").is_err());
        assert!(validate_path(&vec!["a"; 33].join("/")).is_err());
    }
}
