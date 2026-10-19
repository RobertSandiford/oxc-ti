//! The JavaScript string and path operations the Babel plugins use, where Rust's differ.

/// tungsten's `isBareImport`: anything not starting `./` or `../`, including `/abs` and `.x`.
pub fn is_bare_import(specifier: &str) -> bool {
    !specifier.starts_with("./") && !specifier.starts_with("../")
}

fn is_separator(c: u8) -> bool {
    c == b'/' || (cfg!(windows) && c == b'\\')
}

/// Node's `path.dirname`, for paths without a drive or UNC root.
pub fn dirname(path: &str) -> &str {
    let bytes = path.as_bytes();
    if bytes.is_empty() {
        return ".";
    }
    let has_root = is_separator(bytes[0]);
    let mut end = None;
    let mut matched_separator = true;
    for i in (1..bytes.len()).rev() {
        if is_separator(bytes[i]) {
            if !matched_separator {
                end = Some(i);
                break;
            }
        } else {
            matched_separator = false;
        }
    }
    match end {
        None if has_root => &path[..1],
        None => ".",
        Some(end) => &path[..end],
    }
}

/// tungsten's `slash`: backslashes to forward slashes, extended-length paths left alone.
pub fn slash(path: &str) -> String {
    if path.starts_with(r"\\?\") {
        path.to_string()
    } else {
        path.chars().map(|c| if c == '\\' { '/' } else { c }).collect()
    }
}

/// JavaScript's `text.slice(start)`, where `start` counts UTF-16 code units.
pub fn slice_utf16(text: &str, start: usize) -> &str {
    let mut units = 0;
    for (index, c) in text.char_indices() {
        if units >= start {
            return &text[index..];
        }
        units += c.len_utf16();
    }
    ""
}

/// The 1-based line and 0-based UTF-16 column of a byte offset, counting line breaks as Babel does.
pub fn line_column(text: &str, offset: usize) -> (u32, u32) {
    let mut line = 1;
    let mut column: usize = 0;
    let mut chars = text[..offset].chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\r' if chars.peek() == Some(&'\n') => {}
            '\n' | '\r' | '\u{2028}' | '\u{2029}' => {
                line += 1;
                column = 0;
            }
            _ => column += c.len_utf16(),
        }
    }
    (line, u32::try_from(column).unwrap_or(u32::MAX))
}

#[cfg(test)]
mod test {
    use super::{dirname, is_bare_import, line_column, slice_utf16};

    #[test]
    fn dirname_matches_node() {
        assert_eq!(dirname("/pages/Home.jsx"), "/pages");
        assert_eq!(dirname("/Home.jsx"), "/");
        assert_eq!(dirname("Home.jsx"), ".");
        assert_eq!(dirname("/a//b"), "/a/");
        assert_eq!(dirname("/a/b/"), "/a");
    }

    #[test]
    fn bare_imports() {
        assert!(is_bare_import("pkg") && is_bare_import("/abs") && is_bare_import(".x"));
        assert!(!is_bare_import("./x") && !is_bare_import("../x"));
    }

    #[test]
    fn utf16_positions() {
        assert_eq!(slice_utf16("😀ab", 2), "ab");
        assert_eq!(line_column("a\r\nb😀c", "a\r\nb😀".len()), (2, 3));
        assert_eq!(line_column("a\rb\u{2028}c", "a\rb\u{2028}".len()), (3, 0));
    }
}
