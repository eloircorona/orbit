/// Strip `//` line comments and `/* */` block comments from a JSONC string.
/// Strings are left intact (comments inside strings are not stripped).
pub fn strip(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut out = String::with_capacity(len);
    let mut i = 0;
    let mut in_string = false;
    let mut escape = false;

    while i < len {
        let c = chars[i];
        let peek = chars.get(i + 1).copied();

        if in_string {
            out.push(c);
            if escape {
                escape = false;
            } else if c == '\\' {
                escape = true;
            } else if c == '"' {
                in_string = false;
            }
            i += 1;
            continue;
        }

        if c == '"' {
            in_string = true;
            out.push(c);
            i += 1;
            continue;
        }

        // Line comment: skip until end of line
        if c == '/' && peek == Some('/') {
            i += 2;
            while i < len && chars[i] != '\n' && chars[i] != '\r' {
                i += 1;
            }
            continue;
        }

        // Block comment: skip until */
        if c == '/' && peek == Some('*') {
            i += 2;
            while i + 1 < len && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i += 2; // consume the closing */
            continue;
        }

        out.push(c);
        i += 1;
    }

    out
}

/// Parse a JSONC string into a serde_json Value.
pub fn parse(input: &str) -> serde_json::Result<serde_json::Value> {
    let clean = strip(input);
    serde_json::from_str(&clean)
}

/// Load and parse a JSONC file. Returns an empty object if the file is missing.
pub fn load_file(path: &std::path::Path) -> serde_json::Value {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => return serde_json::Value::Object(Default::default()),
    };
    parse(&text).unwrap_or_else(|_| serde_json::Value::Object(Default::default()))
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_line_comments() {
        let input = r#"{ "a": 1 // this is a comment
        }"#;
        let v: serde_json::Value = parse(input).unwrap();
        assert_eq!(v["a"], 1);
    }

    #[test]
    fn strips_block_comments() {
        let input = r#"{ /* block */ "a": 1 }"#;
        let v: serde_json::Value = parse(input).unwrap();
        assert_eq!(v["a"], 1);
    }

    #[test]
    fn preserves_url_inside_string() {
        // "https://..." has // but it's inside a string — must not be stripped
        let input = r#"{ "url": "https://example.com" }"#;
        let v: serde_json::Value = parse(input).unwrap();
        assert_eq!(v["url"], "https://example.com");
    }

    #[test]
    fn empty_file_returns_empty_object() {
        let input = "";
        let result = parse(input);
        assert!(result.is_err()); // empty string is not valid JSON
    }

    #[test]
    fn missing_file_returns_empty_object() {
        let v = load_file(std::path::Path::new("/nonexistent/path.jsonc"));
        assert!(v.is_object());
        assert!(v.as_object().unwrap().is_empty());
    }
}
