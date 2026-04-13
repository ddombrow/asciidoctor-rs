use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use encoding_rs::{Encoding, UTF_8};

use crate::normalize::{has_asciidoc_extension, normalize_asciidoc, normalize_include_text};

/// Expand all `include::path[attrs]` directives in `input`.
/// `base_dir` is the directory of the document being processed (for relative path resolution).
/// Returns the fully expanded text ready for `parse_document`.
pub fn preprocess(input: &str, base_dir: &Path) -> String {
    let mut seen = HashSet::new();
    let normalized = normalize_asciidoc(input);
    expand_lines(normalized.as_ref(), base_dir, &mut seen, 0)
}

fn expand_lines(input: &str, base_dir: &Path, seen: &mut HashSet<PathBuf>, depth: u32) -> String {
    const MAX_DEPTH: u32 = 64;
    if depth > MAX_DEPTH {
        return input.to_owned();
    }

    let mut out = String::with_capacity(input.len());
    let mut delimited_block_delimiter: Option<String> = None;

    for line in input.split_terminator('\n') {
        // Track delimited block state to skip includes inside verbatim blocks.
        if let Some(open_delim) = delimited_block_delimiter.as_deref() {
            out.push_str(line);
            out.push('\n');
            if line.trim() == open_delim {
                delimited_block_delimiter = None;
            }
            continue;
        }

        if let Some(delim) = opening_block_delimiter(line) {
            delimited_block_delimiter = Some(delim);
            out.push_str(line);
            out.push('\n');
            continue;
        }

        if let Some((path, attrs)) = parse_include_line(line) {
            let resolved = base_dir.join(&path);
            let canonical = resolved.canonicalize().unwrap_or_else(|_| resolved.clone());
            let include_is_asciidoc = has_asciidoc_extension(&resolved);

            if seen.contains(&canonical) {
                // Circular include — skip silently
                continue;
            }

            match fs::read(&resolved) {
                Ok(bytes) => {
                    seen.insert(canonical.clone());
                    let child_dir = resolved.parent().unwrap_or(base_dir);
                    let decoded = decode_include_bytes(&bytes, attrs.encoding.as_deref());
                    let normalized = normalize_include_text(decoded.as_ref(), include_is_asciidoc);
                    let mut expanded = if include_is_asciidoc {
                        expand_lines(normalized.as_ref(), child_dir, seen, depth + 1)
                    } else {
                        normalized.into_owned()
                    };
                    if include_is_asciidoc
                        && let Some(offset) = attrs.leveloffset
                    {
                        expanded = apply_leveloffset(&expanded, offset);
                    }
                    out.push_str(&expanded);
                    if !out.ends_with('\n') {
                        out.push('\n');
                    }
                    seen.remove(&canonical);
                }
                Err(_) => {
                    // Missing file — skip silently (opts=optional behavior by default)
                }
            }
            continue;
        }

        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Returns the exact delimiter string if `line` opens a tracked delimited block.
fn opening_block_delimiter(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let bytes = trimmed.as_bytes();
    if bytes.len() < 4 {
        return None;
    }

    let first = *bytes.first()?;
    if !bytes.iter().all(|byte| *byte == first) {
        return None;
    }

    match first {
        b'-' | b'=' | b'*' | b'+' | b'_' | b'.' | b'/' => Some(trimmed.to_owned()),
        _ => None,
    }
}

struct IncludeAttrs {
    leveloffset: Option<i32>,
    encoding: Option<String>,
}

fn parse_include_line(line: &str) -> Option<(String, IncludeAttrs)> {
    let rest = line.strip_prefix("include::")?;
    let bracket = rest.find('[')?;
    let path = rest[..bracket].trim().to_owned();
    if path.is_empty() {
        return None;
    }
    let attr_str = rest[bracket + 1..].strip_suffix(']')?;
    let attrs = parse_include_attrs(attr_str);
    Some((path, attrs))
}

fn parse_include_attrs(s: &str) -> IncludeAttrs {
    let mut leveloffset = None;
    let mut encoding = None;
    for part in s.split(',') {
        let part = part.trim();
        if let Some(val) = part.strip_prefix("leveloffset=") {
            let val = val.trim();
            if let Some(n) = val.strip_prefix('+') {
                leveloffset = n.parse::<i32>().ok();
            } else if let Some(n) = val.strip_prefix('-') {
                leveloffset = n.parse::<i32>().ok().map(|v| -v);
            } else {
                leveloffset = val.parse::<i32>().ok();
            }
        } else if let Some(value) = part.strip_prefix("encoding=") {
            let value = value.trim().trim_matches('"').trim_matches('\'');
            if !value.is_empty() {
                encoding = Some(value.to_owned());
            }
        }
    }
    IncludeAttrs {
        leveloffset,
        encoding,
    }
}

fn decode_include_bytes(bytes: &[u8], encoding: Option<&str>) -> String {
    let decoder = encoding
        .and_then(|label| Encoding::for_label(label.as_bytes()))
        .unwrap_or(UTF_8);
    let (decoded, _, _) = decoder.decode(bytes);
    decoded.into_owned()
}

/// Adjust heading levels in `content` by `offset` (positive = deeper, negative = shallower).
/// Heading levels are clamped to a minimum of 1 (`=`).
fn apply_leveloffset(content: &str, offset: i32) -> String {
    let mut result = String::with_capacity(content.len());
    for line in content.lines() {
        let adjusted = adjust_heading_level(line, offset);
        result.push_str(&adjusted);
        result.push('\n');
    }
    result
}

fn adjust_heading_level(line: &str, offset: i32) -> String {
    let level = line.chars().take_while(|&c| c == '=').count();
    if level == 0 {
        return line.to_owned();
    }
    let after = &line[level..];
    // Must be followed by a space or end-of-line to be a heading
    if !after.is_empty() && !after.starts_with(' ') {
        return line.to_owned();
    }
    let new_level = ((level as i32) + offset).max(1) as usize;
    format!("{}{}", "=".repeat(new_level), after)
}

#[cfg(test)]
mod tests {
    use super::*;
    use encoding_rs::WINDOWS_1252;
    use std::fs;

    /// Create a unique temp directory for a test, cleaned up on drop via a guard.
    fn make_test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "asciidoctor_rs_test_{}_{}",
            name,
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(dir: &Path, name: &str, content: &str) {
        fs::write(dir.join(name), content).unwrap();
    }

    fn cleanup(dir: &Path) {
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn expands_simple_include() {
        let dir = make_test_dir("simple");
        write(&dir, "child.adoc", "included content\n");
        let input = "before\ninclude::child.adoc[]\nafter\n";
        let out = preprocess(input, &dir);
        cleanup(&dir);
        assert_eq!(out, "before\nincluded content\nafter\n");
    }

    #[test]
    fn skips_missing_file_silently() {
        let dir = make_test_dir("missing");
        let input = "before\ninclude::missing.adoc[]\nafter\n";
        let out = preprocess(input, &dir);
        cleanup(&dir);
        assert_eq!(out, "before\nafter\n");
    }

    #[test]
    fn does_not_expand_inside_listing_block() {
        let dir = make_test_dir("listing");
        write(&dir, "child.adoc", "should not appear\n");
        let input = "----\ninclude::child.adoc[]\n----\n";
        let out = preprocess(input, &dir);
        cleanup(&dir);
        assert_eq!(out, "----\ninclude::child.adoc[]\n----\n");
    }

    #[test]
    fn does_not_expand_inside_listing_block_with_longer_delimiter() {
        let dir = make_test_dir("listing_long");
        write(&dir, "child.adoc", "should not appear\n");
        let input = "------\ninclude::child.adoc[]\n------\n";
        let out = preprocess(input, &dir);
        cleanup(&dir);
        assert_eq!(out, "------\ninclude::child.adoc[]\n------\n");
    }

    #[test]
    fn applies_leveloffset_positive() {
        let dir = make_test_dir("lo_pos");
        write(&dir, "child.adoc", "== Section\n\ntext\n");
        let input = "include::child.adoc[leveloffset=+1]\n";
        let out = preprocess(input, &dir);
        cleanup(&dir);
        assert!(out.contains("=== Section"), "got: {out}");
    }

    #[test]
    fn applies_leveloffset_negative() {
        let dir = make_test_dir("lo_neg");
        write(&dir, "child.adoc", "=== Section\n\ntext\n");
        let input = "include::child.adoc[leveloffset=-1]\n";
        let out = preprocess(input, &dir);
        cleanup(&dir);
        assert!(out.contains("== Section"), "got: {out}");
    }

    #[test]
    fn leveloffset_clamps_to_minimum_one() {
        let dir = make_test_dir("lo_clamp");
        write(&dir, "child.adoc", "== Section\n");
        let input = "include::child.adoc[leveloffset=-5]\n";
        let out = preprocess(input, &dir);
        cleanup(&dir);
        assert!(out.contains("= Section"), "got: {out}");
    }

    #[test]
    fn prevents_circular_include() {
        let dir = make_test_dir("circular");
        write(&dir, "a.adoc", "A\ninclude::b.adoc[]\n");
        write(&dir, "b.adoc", "B\ninclude::a.adoc[]\n");
        let input = "include::a.adoc[]\n";
        let out = preprocess(input, &dir);
        cleanup(&dir);
        assert!(out.contains('A'));
        assert!(out.contains('B'));
    }

    #[test]
    fn resolves_path_relative_to_included_file() {
        let dir = make_test_dir("relpath");
        fs::create_dir_all(dir.join("sub")).unwrap();
        write(&dir.join("sub"), "leaf.adoc", "leaf content\n");
        write(&dir, "mid.adoc", "include::sub/leaf.adoc[]\n");
        let input = "include::mid.adoc[]\n";
        let out = preprocess(input, &dir);
        cleanup(&dir);
        assert!(out.contains("leaf content"), "got: {out}");
    }

    #[test]
    fn normalizes_trailing_spaces_in_top_level_source_before_expanding_includes() {
        let dir = make_test_dir("top_level_normalize");
        write(&dir, "child.adoc", "included\n");
        let input = "before  \r\ninclude::child.adoc[]  \r\nafter\t\r\n";
        let out = preprocess(input, &dir);
        cleanup(&dir);
        assert_eq!(out, "before\nincluded\nafter\n");
    }

    #[test]
    fn normalizes_trailing_spaces_in_asciidoc_include_files() {
        let dir = make_test_dir("adoc_include_normalize");
        write(&dir, "child.adoc", "alpha  \r\nbeta\t\r\n");
        let out = preprocess("include::child.adoc[]\n", &dir);
        cleanup(&dir);
        assert_eq!(out, "alpha\nbeta\n");
    }

    #[test]
    fn preserves_trailing_spaces_in_non_asciidoc_include_files() {
        let dir = make_test_dir("csv_include_spaces");
        write(&dir, "data.csv", "left  \r\nright\t\r\n");
        let out = preprocess("include::data.csv[]\n", &dir);
        cleanup(&dir);
        assert_eq!(out, "left  \nright\t\n");
    }

    #[test]
    fn decodes_include_files_using_requested_encoding() {
        let dir = make_test_dir("include_encoding");
        let (bytes, _, _) = WINDOWS_1252.encode("café\r\n");
        fs::write(dir.join("child.adoc"), bytes.as_ref()).unwrap();
        let out = preprocess("include::child.adoc[encoding=windows-1252]\n", &dir);
        cleanup(&dir);
        assert_eq!(out, "café\n");
    }
}
