use crate::ast::{Block, Document, Heading, ListItem, OrderedList, Paragraph, UnorderedList};
use crate::inline::parse_inlines;

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingAnchor {
    id: String,
    reftext: Option<String>,
}

pub fn parse_document(input: &str) -> Document {
    let lines: Vec<&str> = input.lines().collect();
    let mut blocks = Vec::new();
    let mut current_paragraph = Vec::new();
    let mut current_paragraph_anchor = None;
    let mut pending_anchor = None;
    let mut title = None;
    let mut index = 0;

    while index < lines.len() {
        let line = lines[index];

        if let Some(anchor) = parse_block_anchor(line) {
            flush_paragraph(
                &mut blocks,
                &mut current_paragraph,
                &mut current_paragraph_anchor,
            );
            pending_anchor = Some(anchor);
            index += 1;
            continue;
        }

        if let Some((heading, consumed_lines)) = parse_heading(&lines, index) {
            flush_paragraph(
                &mut blocks,
                &mut current_paragraph,
                &mut current_paragraph_anchor,
            );
            let heading = apply_anchor_to_heading(heading, pending_anchor.take());

            if heading.level == 0 && title.is_none() && blocks.is_empty() {
                title = Some(heading);
            } else {
                blocks.push(Block::Heading(heading));
            }

            index += consumed_lines;
            continue;
        }

        if let Some((list, consumed_lines)) = parse_unordered_list(&lines, index) {
            flush_paragraph(
                &mut blocks,
                &mut current_paragraph,
                &mut current_paragraph_anchor,
            );
            pending_anchor = None;
            blocks.push(Block::UnorderedList(list));
            index += consumed_lines;
            continue;
        }

        if let Some((list, consumed_lines)) = parse_ordered_list(&lines, index) {
            flush_paragraph(
                &mut blocks,
                &mut current_paragraph,
                &mut current_paragraph_anchor,
            );
            pending_anchor = None;
            blocks.push(Block::OrderedList(list));
            index += consumed_lines;
            continue;
        }

        if line.trim().is_empty() {
            flush_paragraph(
                &mut blocks,
                &mut current_paragraph,
                &mut current_paragraph_anchor,
            );
            index += 1;
            continue;
        }

        if current_paragraph.is_empty() {
            current_paragraph_anchor = pending_anchor.take();
        }
        current_paragraph.push(line.to_owned());
        index += 1;
    }

    flush_paragraph(
        &mut blocks,
        &mut current_paragraph,
        &mut current_paragraph_anchor,
    );

    Document { title, blocks }
}

fn parse_unordered_list(lines: &[&str], index: usize) -> Option<(UnorderedList, usize)> {
    let mut items = Vec::new();
    let mut consumed = 0;

    while index + consumed < lines.len() {
        let line = lines[index + consumed];
        let Some(content) = parse_unordered_list_item(line) else {
            break;
        };

        items.push(ListItem {
            blocks: vec![Block::Paragraph(Paragraph {
                inlines: parse_inlines(content),
                lines: vec![content.to_owned()],
                id: None,
                reftext: None,
            })],
        });
        consumed += 1;
    }

    if items.is_empty() {
        None
    } else {
        Some((UnorderedList { items }, consumed))
    }
}

fn parse_unordered_list_item(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let marker = trimmed.chars().next()?;
    if marker != '*' && marker != '-' {
        return None;
    }

    let remainder = &trimmed[marker.len_utf8()..];
    if !remainder.starts_with(char::is_whitespace) {
        return None;
    }

    let content = remainder.trim();
    if content.is_empty() {
        return None;
    }

    Some(content)
}

fn parse_ordered_list(lines: &[&str], index: usize) -> Option<(OrderedList, usize)> {
    let mut items = Vec::new();
    let mut consumed = 0;

    while index + consumed < lines.len() {
        let line = lines[index + consumed];
        let Some(content) = parse_ordered_list_item(line) else {
            break;
        };

        items.push(ListItem {
            blocks: vec![Block::Paragraph(Paragraph {
                inlines: parse_inlines(content),
                lines: vec![content.to_owned()],
                id: None,
                reftext: None,
            })],
        });
        consumed += 1;
    }

    if items.is_empty() {
        None
    } else {
        Some((OrderedList { items }, consumed))
    }
}

fn parse_ordered_list_item(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('.') {
        return None;
    }

    let remainder = &trimmed[1..];
    if !remainder.starts_with(char::is_whitespace) {
        return None;
    }

    let content = remainder.trim();
    if content.is_empty() {
        return None;
    }

    Some(content)
}

fn flush_paragraph(
    blocks: &mut Vec<Block>,
    current_paragraph: &mut Vec<String>,
    current_paragraph_anchor: &mut Option<PendingAnchor>,
) {
    if current_paragraph.is_empty() {
        return;
    }

    let lines = std::mem::take(current_paragraph);
    let anchor = current_paragraph_anchor.take();
    blocks.push(Block::Paragraph(Paragraph {
        inlines: parse_inlines(&lines.join("\n")),
        lines,
        id: anchor.as_ref().map(|anchor| anchor.id.clone()),
        reftext: anchor.and_then(|anchor| anchor.reftext),
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
        id: None,
        reftext: None,
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
            id: None,
            reftext: None,
        },
        2,
    ))
}

fn apply_anchor_to_heading(mut heading: Heading, anchor: Option<PendingAnchor>) -> Heading {
    if let Some(anchor) = anchor {
        heading.id = Some(anchor.id);
        heading.reftext = anchor.reftext;
    }
    heading
}

fn parse_block_anchor(line: &str) -> Option<PendingAnchor> {
    let trimmed = line.trim();

    if let Some(inner) = trimmed
        .strip_prefix("[[")
        .and_then(|rest| rest.strip_suffix("]]"))
    {
        return parse_anchor_parts(inner);
    }

    if let Some(inner) = trimmed
        .strip_prefix("[#")
        .and_then(|rest| rest.strip_suffix(']'))
    {
        return parse_hash_anchor_parts(inner);
    }

    None
}

fn parse_anchor_parts(inner: &str) -> Option<PendingAnchor> {
    let mut parts = inner.splitn(2, ',');
    let id = parts.next()?.trim();
    if id.is_empty() || !is_valid_anchor_id(id) {
        return None;
    }

    let reftext = parts
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);

    Some(PendingAnchor {
        id: id.to_owned(),
        reftext,
    })
}

fn parse_hash_anchor_parts(inner: &str) -> Option<PendingAnchor> {
    let mut parts = inner.split(',').map(str::trim);
    let id = parts.next()?;
    if id.is_empty() || !is_valid_anchor_id(id) {
        return None;
    }

    let mut reftext = None;
    for part in parts {
        if let Some(value) = part.strip_prefix("reftext=") {
            let value = value.trim().trim_matches('"');
            if !value.is_empty() {
                reftext = Some(value.to_owned());
            }
        }
    }

    Some(PendingAnchor {
        id: id.to_owned(),
        reftext,
    })
}

fn is_valid_anchor_id(id: &str) -> bool {
    id.chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | ':' | '.'))
}

#[cfg(test)]
mod tests {
    use crate::ast::{
        Block, Heading, Inline, InlineForm, InlineVariant, ListItem, OrderedList, Paragraph,
        UnorderedList,
    };
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
                    id: None,
                    reftext: None,
                }),
                Block::Paragraph(Paragraph {
                    inlines: vec![Inline::Text("third line".into())],
                    lines: vec!["third line".into()],
                    id: None,
                    reftext: None,
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
                id: None,
                reftext: None,
            })
        );
        assert_eq!(
            document.blocks,
            vec![
                Block::Heading(Heading {
                    level: 1,
                    title: "Section One".into(),
                    id: None,
                    reftext: None,
                }),
                Block::Paragraph(Paragraph {
                    inlines: vec![Inline::Text("content".into())],
                    lines: vec!["content".into()],
                    id: None,
                    reftext: None,
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
                id: None,
                reftext: None,
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
                id: None,
                reftext: None,
            })
        );
        assert_eq!(
            document.blocks,
            vec![Block::Heading(Heading {
                level: 1,
                title: "Section A".into(),
                id: None,
                reftext: None,
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
                id: None,
                reftext: None,
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

    #[test]
    fn parses_block_anchor_before_section_heading() {
        let document = parse_document("[[install,Installation]]\n== First Section");

        assert_eq!(
            document.blocks,
            vec![Block::Heading(Heading {
                level: 1,
                title: "First Section".into(),
                id: Some("install".into()),
                reftext: Some("Installation".into()),
            })]
        );
    }

    #[test]
    fn parses_hash_anchor_before_paragraph() {
        let document = parse_document("[#intro,reftext=Introduction]\nHello");

        assert_eq!(
            document.blocks,
            vec![Block::Paragraph(Paragraph {
                inlines: vec![Inline::Text("Hello".into())],
                lines: vec!["Hello".into()],
                id: Some("intro".into()),
                reftext: Some("Introduction".into()),
            })]
        );
    }

    #[test]
    fn parses_ordered_lists() {
        let document = parse_document(". first item\n. second item");

        assert_eq!(
            document.blocks,
            vec![Block::OrderedList(OrderedList {
                items: vec![
                    ListItem {
                        blocks: vec![Block::Paragraph(Paragraph {
                            inlines: vec![Inline::Text("first item".into())],
                            lines: vec!["first item".into()],
                            id: None,
                            reftext: None,
                        })],
                    },
                    ListItem {
                        blocks: vec![Block::Paragraph(Paragraph {
                            inlines: vec![Inline::Text("second item".into())],
                            lines: vec!["second item".into()],
                            id: None,
                            reftext: None,
                        })],
                    },
                ],
            })]
        );
    }

    #[test]
    fn parses_document_title_without_sections() {
        let document = parse_document("= My Title\n\nA paragraph.");

        assert_eq!(
            document.title,
            Some(Heading {
                level: 0,
                title: "My Title".into(),
                id: None,
                reftext: None,
            })
        );
        assert_eq!(
            document.blocks,
            vec![Block::Paragraph(Paragraph {
                inlines: vec![Inline::Text("A paragraph.".into())],
                lines: vec!["A paragraph.".into()],
                id: None,
                reftext: None,
            })]
        );
    }

    #[test]
    fn does_not_treat_second_level_zero_heading_as_title() {
        let document = parse_document("= First Title\n\n= Second Title");

        assert_eq!(
            document.title,
            Some(Heading {
                level: 0,
                title: "First Title".into(),
                id: None,
                reftext: None,
            })
        );
        assert_eq!(
            document.blocks,
            vec![Block::Heading(Heading {
                level: 0,
                title: "Second Title".into(),
                id: None,
                reftext: None,
            })]
        );
    }

    #[test]
    fn parses_unordered_lists() {
        let document = parse_document("* first item\n- second item");

        assert_eq!(
            document.blocks,
            vec![Block::UnorderedList(UnorderedList {
                items: vec![
                    ListItem {
                        blocks: vec![Block::Paragraph(Paragraph {
                            inlines: vec![Inline::Text("first item".into())],
                            lines: vec!["first item".into()],
                            id: None,
                            reftext: None,
                        })],
                    },
                    ListItem {
                        blocks: vec![Block::Paragraph(Paragraph {
                            inlines: vec![Inline::Text("second item".into())],
                            lines: vec!["second item".into()],
                            id: None,
                            reftext: None,
                        })],
                    },
                ],
            })]
        );
    }
}
