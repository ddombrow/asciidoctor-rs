use std::borrow::Cow;
use std::path::Path;

const ASCIIDOC_EXTENSIONS: &[&str] = &["adoc", "asciidoc", "asc", "ad", "txt"];

pub fn normalize_asciidoc(input: &str) -> Cow<'_, str> {
    normalize_text(input, true)
}

pub fn normalize_include_text(input: &str, is_asciidoc: bool) -> Cow<'_, str> {
    normalize_text(input, is_asciidoc)
}

pub fn has_asciidoc_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ASCIIDOC_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
}

fn normalize_text(input: &str, strip_trailing_spaces: bool) -> Cow<'_, str> {
    let newline_normalized = normalize_line_endings(input);
    if !strip_trailing_spaces {
        return newline_normalized;
    }

    let text = newline_normalized.as_ref();
    if !text
        .split('\n')
        .any(|line| line.ends_with([' ', '\t']))
    {
        return newline_normalized;
    }

    Cow::Owned(
        text.split('\n')
            .map(|line| line.trim_end_matches([' ', '\t']))
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

fn normalize_line_endings(input: &str) -> Cow<'_, str> {
    if !input.contains('\r') {
        return Cow::Borrowed(input);
    }

    Cow::Owned(input.replace("\r\n", "\n").replace('\r', "\n"))
}

#[cfg(test)]
mod tests {
    use super::{has_asciidoc_extension, normalize_asciidoc, normalize_include_text};
    use std::path::Path;

    #[test]
    fn strips_trailing_spaces_from_asciidoc_lines() {
        let normalized = normalize_asciidoc("alpha  \r\nbeta\t\r\ngamma\n");
        assert_eq!(normalized.as_ref(), "alpha\nbeta\ngamma\n");
    }

    #[test]
    fn keeps_trailing_spaces_for_non_asciidoc_include_text() {
        let normalized = normalize_include_text("left  \r\nright\t\r\n", false);
        assert_eq!(normalized.as_ref(), "left  \nright\t\n");
    }

    #[test]
    fn recognizes_asciidoc_extensions() {
        assert!(has_asciidoc_extension(Path::new("doc.adoc")));
        assert!(has_asciidoc_extension(Path::new("doc.ASC")));
        assert!(has_asciidoc_extension(Path::new("doc.txt")));
        assert!(!has_asciidoc_extension(Path::new("data.csv")));
    }
}
