use crate::ast::{Block, Document, Heading, Paragraph};
use crate::inline::parse_inlines;

pub fn parse_document(input: &str) -> Document {
    let lines: Vec<&str> = input.lines().collect();
    let mut blocks = Vec::new();
    let mut current_paragraph = Vec::new();
    let mut title = None;
    let mut index = 0;

    while index < lines.len() {
        let line = lines[index];

        if let Some((heading, consumed_lines)) = parse_heading(&lines, index) {
            flush_paragraph(&mut blocks, &mut current_paragraph);

            if heading.level == 0 && title.is_none() && blocks.is_empty() {
                title = Some(heading);
            } else {
                blocks.push(Block::Heading(heading));
            }

            index += consumed_lines;
            continue;
        }

        if line.trim().is_empty() {
            flush_paragraph(&mut blocks, &mut current_paragraph);
            index += 1;
            continue;
        }

        current_paragraph.push(line.to_owned());
        index += 1;
    }

    flush_paragraph(&mut blocks, &mut current_paragraph);

    Document { title, blocks }
}

fn flush_paragraph(blocks: &mut Vec<Block>, current_paragraph: &mut Vec<String>) {
    if current_paragraph.is_empty() {
        return;
    }

    let lines = std::mem::take(current_paragraph);
    blocks.push(Block::Paragraph(Paragraph {
        inlines: parse_inlines(&lines.join("\n")),
        lines,
    }));
}

fn parse_heading(lines: &[&str], index: usize) -> Option<(Heading, usize)> {
    parse_atx_heading(lines[index])
        .map(|heading| (heading, 1))
        .or_else(|| parse_setext_heading(lines, index))
}

fn parse_atx_heading(line: &str) -> Option<Heading> {
    let trimmed = line.trim();
    let marker = trimmed.chars().next()?;

    if marker != '=' && marker != '#' {
        return None;
    }

    let level = trimmed.chars().take_while(|&ch| ch == marker).count();
    if level == 0 || level > 6 {
        return None;
    }

    let remainder = &trimmed[level..];
    if !remainder.starts_with(char::is_whitespace) {
        return None;
    }

    let title = remainder
        .trim()
        .trim_end_matches(marker)
        .trim_end()
        .to_owned();

    if title.is_empty() || !title.chars().any(char::is_alphanumeric) {
        return None;
    }

    Some(Heading {
        level: (level - 1) as u8,
        title,
    })
}

fn parse_setext_heading(lines: &[&str], index: usize) -> Option<(Heading, usize)> {
    let title = lines.get(index)?.trim();
    let underline = lines.get(index + 1)?.trim();

    if title.is_empty() || !title.chars().any(char::is_alphanumeric) {
        return None;
    }

    let marker = underline.chars().next()?;
    if (marker != '=' && marker != '-') || !underline.chars().all(|ch| ch == marker) {
        return None;
    }

    let level = if marker == '=' { 0 } else { 1 };
    Some((
        Heading {
            level,
            title: title.to_owned(),
        },
        2,
    ))
}

#[cfg(test)]
mod tests {
    use crate::ast::{Block, Heading, Inline, InlineForm, InlineVariant, Paragraph};
    use crate::parser::parse_document;

    #[test]
    fn parses_blank_line_separated_paragraphs() {
        let document = parse_document("first line\nsecond line\n\nthird line");

        assert_eq!(
            document.blocks,
            vec![
                Block::Paragraph(Paragraph {
                    inlines: vec![Inline::Text("first line\nsecond line".into())],
                    lines: vec!["first line".into(), "second line".into()],
                }),
                Block::Paragraph(Paragraph {
                    inlines: vec![Inline::Text("third line".into())],
                    lines: vec!["third line".into()],
                }),
            ]
        );
        assert_eq!(document.title, None);
    }

    #[test]
    fn parses_atx_document_title_and_section_headings() {
        let document = parse_document("= Document Title\n\n== Section One\n\ncontent");

        assert_eq!(
            document.title,
            Some(Heading {
                level: 0,
                title: "Document Title".into(),
            })
        );
        assert_eq!(
            document.blocks,
            vec![
                Block::Heading(Heading {
                    level: 1,
                    title: "Section One".into(),
                }),
                Block::Paragraph(Paragraph {
                    inlines: vec![Inline::Text("content".into())],
                    lines: vec!["content".into()],
                }),
            ]
        );
    }

    #[test]
    fn parses_markdown_style_symmetric_atx_heading_markers() {
        let document = parse_document("## Section One ##");

        assert_eq!(
            document.blocks,
            vec![Block::Heading(Heading {
                level: 1,
                title: "Section One".into(),
            })]
        );
    }

    #[test]
    fn parses_setext_document_title_and_section_heading() {
        let document = parse_document("Document Title\n==============\n\nSection A\n---------");

        assert_eq!(
            document.title,
            Some(Heading {
                level: 0,
                title: "Document Title".into(),
            })
        );
        assert_eq!(
            document.blocks,
            vec![Block::Heading(Heading {
                level: 1,
                title: "Section A".into(),
            })]
        );
    }

    #[test]
    fn does_not_treat_mixed_markers_as_heading() {
        let document = parse_document("=#= My Title");

        assert_eq!(document.title, None);
        assert_eq!(
            document.blocks,
            vec![Block::Paragraph(Paragraph {
                inlines: vec![Inline::Text("=#= My Title".into())],
                lines: vec!["=#= My Title".into()],
            })]
        );
    }

    #[test]
    fn parses_inline_markup_inside_paragraphs() {
        let document = parse_document("before *strong* and _emphasis_ after");
        let Block::Paragraph(paragraph) = &document.blocks[0] else {
            panic!("expected paragraph");
        };

        assert_eq!(paragraph.inlines.len(), 5);
        let Inline::Span(strong) = &paragraph.inlines[1] else {
            panic!("expected strong span");
        };
        assert_eq!(strong.variant, InlineVariant::Strong);
        assert_eq!(strong.form, InlineForm::Constrained);

        let Inline::Span(emphasis) = &paragraph.inlines[3] else {
            panic!("expected emphasis span");
        };
        assert_eq!(emphasis.variant, InlineVariant::Emphasis);
        assert_eq!(emphasis.form, InlineForm::Constrained);
    }

    #[test]
    fn keeps_escaped_markup_literal_inside_paragraphs() {
        let document = parse_document(r"\*not strong* and \_not emphasis_");
        let Block::Paragraph(paragraph) = &document.blocks[0] else {
            panic!("expected paragraph");
        };

        assert_eq!(
            paragraph.inlines,
            vec![Inline::Text("*not strong* and _not emphasis_".into())]
        );
    }
}
