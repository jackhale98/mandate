//! JSONC support (DEC-053): JSON plus `//` and `/* */` comments and trailing
//! commas. Comments and trailing commas are replaced by spaces so line and
//! column numbers in parse errors still point at the original file.

/// Convert JSONC text to strict JSON with identical line/column layout.
pub fn to_strict_json(src: &str) -> String {
    let bytes = src.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    let mut in_string = false;
    while i < bytes.len() {
        let c = bytes[i];
        if in_string {
            out.push(c);
            if c == b'\\' && i + 1 < bytes.len() {
                out.push(bytes[i + 1]);
                i += 2;
                continue;
            }
            if c == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        match c {
            b'"' => {
                in_string = true;
                out.push(c);
                i += 1;
            }
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'/' => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    out.push(b' ');
                    i += 1;
                }
            }
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'*' => {
                out.extend_from_slice(b"  ");
                i += 2;
                while i < bytes.len()
                    && !(bytes[i] == b'*' && i + 1 < bytes.len() && bytes[i + 1] == b'/')
                {
                    out.push(if bytes[i] == b'\n' { b'\n' } else { b' ' });
                    i += 1;
                }
                if i < bytes.len() {
                    out.extend_from_slice(b"  ");
                    i += 2;
                }
            }
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }
    remove_trailing_commas(&mut out);
    // Only ASCII bytes were replaced, so UTF-8 validity is preserved.
    String::from_utf8(out).unwrap_or_default()
}

/// Replace commas that are followed (after whitespace) by `]` or `}`.
fn remove_trailing_commas(buf: &mut [u8]) {
    let mut in_string = false;
    let mut i = 0;
    while i < buf.len() {
        let c = buf[i];
        if in_string {
            if c == b'\\' {
                i += 2;
                continue;
            }
            if c == b'"' {
                in_string = false;
            }
        } else if c == b'"' {
            in_string = true;
        } else if c == b',' {
            let mut j = i + 1;
            while j < buf.len() && buf[j].is_ascii_whitespace() {
                j += 1;
            }
            if j < buf.len() && (buf[j] == b']' || buf[j] == b'}') {
                buf[i] = b' ';
            }
        }
        i += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_comments_and_trailing_commas() {
        let src = "{\n  // note\n  \"a\": \"x // not a comment\", /* block\n */ \"b\": [1, 2,],\n}";
        let json = to_strict_json(src);
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["a"], "x // not a comment");
        assert_eq!(v["b"][1], 2);
        assert_eq!(json.lines().count(), src.lines().count());
    }

    #[test]
    fn preserves_escaped_quotes() {
        let v: serde_json::Value =
            serde_json::from_str(&to_strict_json(r#"{"a": "say \"hi\", // ok"}"#)).unwrap();
        assert_eq!(v["a"], "say \"hi\", // ok");
    }
}
