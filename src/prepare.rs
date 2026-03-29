use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ast::{Block, Document, Paragraph};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentModel {
    Simple,
    Compound,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentBlock {
    #[serde(rename = "type")]
    pub node_type: String,
    pub title: String,
    pub has_header: bool,
    pub no_header: bool,
    pub attributes: BTreeMap<String, String>,
    pub blocks: Vec<PreparedBlock>,
    pub content_model: Option<ContentModel>,
    pub footnotes: Vec<Footnote>,
    pub sections: Vec<DocumentSection>,
    pub authors: Vec<Author>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSection {
    pub id: String,
    pub title: String,
    pub level: u8,
    pub num: String,
    pub sections: Vec<DocumentSection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PreparedBlock {
    Preamble(CompoundBlock),
    Paragraph(ParagraphBlock),
    Section(SectionBlock),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParagraphBlock {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub blocks: Vec<PreparedBlock>,
    pub content: String,
    pub attributes: BTreeMap<String, String>,
    pub content_model: Option<ContentModel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_number: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    pub level: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionBlock {
    pub id: String,
    pub blocks: Vec<PreparedBlock>,
    pub attributes: BTreeMap<String, String>,
    pub content_model: Option<ContentModel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_number: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    pub level: u8,
    pub title: String,
    pub numbered: bool,
    pub num: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompoundBlock {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub blocks: Vec<PreparedBlock>,
    pub attributes: BTreeMap<String, String>,
    pub content_model: Option<ContentModel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_number: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    pub level: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Footnote {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Author {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}

pub fn prepare_document(document: &Document) -> DocumentBlock {
    let mut next_section_ids = Vec::new();
    let blocks = prepare_blocks(&document.blocks, true, &mut next_section_ids);
    let sections = collect_sections(&blocks);

    DocumentBlock {
        node_type: "document".into(),
        title: document
            .title
            .as_ref()
            .map(|heading| heading.title.clone())
            .unwrap_or_default(),
        has_header: document.title.is_some(),
        no_header: document.title.is_none(),
        attributes: BTreeMap::new(),
        blocks,
        content_model: Some(ContentModel::Compound),
        footnotes: Vec::new(),
        sections,
        authors: Vec::new(),
    }
}

fn prepare_blocks(
    blocks: &[Block],
    wrap_document_preamble: bool,
    section_ids: &mut Vec<u32>,
) -> Vec<PreparedBlock> {
    let mut prepared = Vec::new();
    let mut index = 0;
    let mut preamble_blocks = Vec::new();
    let mut seen_section = false;

    while index < blocks.len() {
        match &blocks[index] {
            Block::Paragraph(paragraph) => {
                let paragraph = PreparedBlock::Paragraph(prepare_paragraph(paragraph));
                if wrap_document_preamble && !seen_section {
                    preamble_blocks.push(paragraph);
                } else {
                    prepared.push(paragraph);
                }
                index += 1;
            }
            Block::Heading(heading) => {
                if wrap_document_preamble && !seen_section && !preamble_blocks.is_empty() {
                    prepared.push(PreparedBlock::Preamble(prepare_preamble(std::mem::take(
                        &mut preamble_blocks,
                    ))));
                }
                seen_section = true;
                let section_level = heading.level;
                let id = next_section_id(section_ids, section_level, &heading.title);
                let next_heading_index = find_section_end(blocks, index + 1, section_level);
                let section_blocks =
                    prepare_blocks(&blocks[index + 1..next_heading_index], false, section_ids);

                prepared.push(PreparedBlock::Section(SectionBlock {
                    id,
                    blocks: section_blocks,
                    attributes: BTreeMap::new(),
                    content_model: Some(ContentModel::Compound),
                    line_number: None,
                    style: None,
                    role: None,
                    level: section_level,
                    title: heading.title.clone(),
                    numbered: false,
                    num: String::new(),
                    name: "section".into(),
                }));

                index = next_heading_index;
            }
        }
    }

    if wrap_document_preamble && !preamble_blocks.is_empty() {
        prepared.push(PreparedBlock::Preamble(prepare_preamble(preamble_blocks)));
    } else if !preamble_blocks.is_empty() {
        prepared.extend(preamble_blocks);
    }

    prepared
}

fn find_section_end(blocks: &[Block], mut index: usize, level: u8) -> usize {
    while index < blocks.len() {
        match &blocks[index] {
            Block::Heading(next_heading) if next_heading.level <= level => return index,
            _ => index += 1,
        }
    }

    index
}

fn prepare_paragraph(paragraph: &Paragraph) -> ParagraphBlock {
    ParagraphBlock {
        id: None,
        blocks: Vec::new(),
        content: paragraph.lines.join("\n"),
        attributes: BTreeMap::new(),
        content_model: Some(ContentModel::Simple),
        line_number: None,
        style: None,
        role: None,
        level: 0,
        title: None,
    }
}

fn prepare_preamble(blocks: Vec<PreparedBlock>) -> CompoundBlock {
    CompoundBlock {
        id: None,
        blocks,
        attributes: BTreeMap::new(),
        content_model: Some(ContentModel::Compound),
        line_number: None,
        style: None,
        role: None,
        level: 0,
        title: None,
    }
}

pub fn prepared_document_to_json(document: &DocumentBlock) -> serde_json::Result<String> {
    serde_json::to_string_pretty(document)
}

fn collect_sections(blocks: &[PreparedBlock]) -> Vec<DocumentSection> {
    blocks
        .iter()
        .filter_map(|block| match block {
            PreparedBlock::Section(section) => Some(DocumentSection {
                id: section.id.clone(),
                title: section.title.clone(),
                level: section.level,
                num: section.num.clone(),
                sections: collect_sections(&section.blocks),
            }),
            PreparedBlock::Preamble(_) | PreparedBlock::Paragraph(_) => None,
        })
        .collect()
}

fn next_section_id(section_ids: &mut Vec<u32>, level: u8, title: &str) -> String {
    let depth = usize::from(level);
    if section_ids.len() <= depth {
        section_ids.resize(depth + 1, 0);
    }

    section_ids[depth] += 1;
    for count in &mut section_ids[depth + 1..] {
        *count = 0;
    }

    let base = slugify(title);
    let sequence = section_ids[depth];
    if sequence == 1 {
        base
    } else {
        format!("{base}-{sequence}")
    }
}

fn slugify(title: &str) -> String {
    let mut slug = String::from("_");
    let mut previous_was_separator = false;

    for ch in title.chars() {
        if ch.is_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            previous_was_separator = false;
        } else if !previous_was_separator {
            slug.push('_');
            previous_was_separator = true;
        }
    }

    while slug.ends_with('_') {
        slug.pop();
    }

    if slug == "_" { "_section".into() } else { slug }
}

#[cfg(test)]
mod tests {
    use crate::ast::{Block, Document, Heading, Paragraph};
    use crate::prepare::{
        ContentModel, PreparedBlock, prepare_document, prepared_document_to_json,
    };

    #[test]
    fn prepares_nested_sections_for_react_facing_output() {
        let document = Document {
            title: Some(Heading {
                level: 0,
                title: "Document Title".into(),
            }),
            blocks: vec![
                Block::Paragraph(Paragraph {
                    lines: vec!["Preamble paragraph.".into()],
                }),
                Block::Heading(Heading {
                    level: 1,
                    title: "Section A".into(),
                }),
                Block::Paragraph(Paragraph {
                    lines: vec!["Section body.".into()],
                }),
                Block::Heading(Heading {
                    level: 2,
                    title: "Section A Child".into(),
                }),
                Block::Paragraph(Paragraph {
                    lines: vec!["Nested body.".into()],
                }),
                Block::Heading(Heading {
                    level: 1,
                    title: "Section B".into(),
                }),
            ],
        };

        let prepared = prepare_document(&document);

        assert_eq!(prepared.node_type, "document");
        assert_eq!(prepared.title, "Document Title");
        assert_eq!(prepared.content_model, Some(ContentModel::Compound));
        assert!(prepared.footnotes.is_empty());
        assert!(prepared.authors.is_empty());
        assert_eq!(prepared.sections.len(), 2);

        let PreparedBlock::Preamble(preamble) = &prepared.blocks[0] else {
            panic!("expected preamble block");
        };

        assert_eq!(preamble.blocks.len(), 1);

        let PreparedBlock::Section(section_a) = &prepared.blocks[1] else {
            panic!("expected section block");
        };

        assert_eq!(section_a.title, "Section A");
        assert_eq!(section_a.level, 1);
        assert_eq!(section_a.num, "");
        assert!(!section_a.numbered);
        assert_eq!(section_a.id, "_section_a");

        let PreparedBlock::Section(section_a_child) = &section_a.blocks[1] else {
            panic!("expected nested section block");
        };

        assert_eq!(section_a_child.title, "Section A Child");
        assert_eq!(section_a_child.level, 2);
        assert_eq!(section_a_child.num, "");
        assert!(!section_a_child.numbered);

        let PreparedBlock::Section(section_b) = &prepared.blocks[2] else {
            panic!("expected second top-level section block");
        };

        assert_eq!(section_b.num, "");
        assert!(!section_b.numbered);
    }

    #[test]
    fn prepares_paragraph_content_as_simple_blocks() {
        let document = Document {
            title: None,
            blocks: vec![Block::Paragraph(Paragraph {
                lines: vec!["first line".into(), "second line".into()],
            })],
        };

        let prepared = prepare_document(&document);

        let PreparedBlock::Preamble(preamble) = &prepared.blocks[0] else {
            panic!("expected preamble");
        };

        let PreparedBlock::Paragraph(paragraph) = &preamble.blocks[0] else {
            panic!("expected paragraph in preamble");
        };

        assert_eq!(paragraph.content, "first line\nsecond line");
        assert_eq!(paragraph.content_model, Some(ContentModel::Simple));
    }

    #[test]
    fn serializes_with_react_asciidoc_style_field_names() {
        let document = Document {
            title: Some(Heading {
                level: 0,
                title: "Document Title".into(),
            }),
            blocks: vec![Block::Paragraph(Paragraph {
                lines: vec!["hello".into()],
            })],
        };

        let prepared = prepare_document(&document);
        let json = prepared_document_to_json(&prepared).expect("json serialization");

        assert!(json.contains("\"hasHeader\""));
        assert!(json.contains("\"noHeader\""));
        assert!(json.contains("\"contentModel\""));
        assert!(json.contains("\"footnotes\": []"));
        assert!(json.contains("\"authors\": []"));
    }
}
