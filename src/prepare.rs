use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ast::{
    AdmonitionBlock as AstAdmonitionBlock, Block, CompoundBlock as AstCompoundBlock,
    DescriptionList as AstDescriptionList, Document, ImageBlock as AstImageBlock, Inline,
    InlineForm, InlineVariant, Listing as AstListing, OrderedList, Paragraph,
    QuoteBlock as AstQuoteBlock, TableBlock as AstTableBlock, TableCell as AstTableCell,
    TableRow as AstTableRow, UnorderedList,
};
use crate::normalize::trim_outer_blank_lines;

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision: Option<Revision>,
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
    Admonition(AdmonitionBlock),
    Section(SectionBlock),
    UnorderedList(ListBlock),
    OrderedList(ListBlock),
    DescriptionList(DescriptionListBlock),
    Table(TableBlock),
    Listing(ListingBlock),
    Literal(ListingBlock),
    CalloutList(CalloutListBlock),
    Example(CompoundBlock),
    Sidebar(CompoundBlock),
    Open(CompoundBlock),
    Quote(QuoteBlock),
    Passthrough(PassthroughBlock),
    Image(ImageBlock),
    Toc(TocBlock),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParagraphBlock {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reftext: Option<String>,
    pub blocks: Vec<PreparedBlock>,
    pub content: String,
    pub inlines: Vec<PreparedInline>,
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
pub struct AdmonitionBlock {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reftext: Option<String>,
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
    pub variant: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListBlock {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reftext: Option<String>,
    pub items: Vec<ListItemBlock>,
    pub attributes: BTreeMap<String, String>,
    pub content_model: Option<ContentModel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    pub level: u8,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListItemBlock {
    pub blocks: Vec<PreparedBlock>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DescriptionListBlock {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reftext: Option<String>,
    pub items: Vec<DescriptionListItemBlock>,
    pub attributes: BTreeMap<String, String>,
    pub content_model: Option<ContentModel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    pub level: u8,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DescriptionListItemBlock {
    pub terms: Vec<DescriptionListTermBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<ListItemBlock>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DescriptionListTermBlock {
    pub text: String,
    pub inlines: Vec<PreparedInline>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListingBlock {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reftext: Option<String>,
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
    /// (0-based line index, callout number) — empty when block has no callouts
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub callout_lines: Vec<(usize, u32)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalloutListBlock {
    pub items: Vec<CalloutItemBlock>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalloutItemBlock {
    pub number: u32,
    pub inlines: Vec<PreparedInline>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableBlock {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reftext: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<TableRow>,
    pub rows: Vec<TableRow>,
    pub attributes: BTreeMap<String, String>,
    pub content_model: Option<ContentModel>,
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
pub struct TableRow {
    pub cells: Vec<TableCell>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableCell {
    pub content: String,
    pub inlines: Vec<PreparedInline>,
    pub blocks: Vec<PreparedBlock>,
    pub colspan: usize,
    pub rowspan: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PreparedInline {
    Text(TextInline),
    Span(SpanInline),
    Link(LinkInline),
    Xref(XrefInline),
    Anchor(AnchorInline),
    Passthrough(PassthroughInline),
    Image(ImageInline),
    Icon(IconInline),
    Footnote(FootnoteInline),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextInline {
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpanInline {
    pub variant: String,
    pub form: String,
    pub inlines: Vec<PreparedInline>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkInline {
    pub target: String,
    pub inlines: Vec<PreparedInline>,
    pub bare: bool,
    pub window: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XrefInline {
    pub target: String,
    pub href: String,
    pub inlines: Vec<PreparedInline>,
    pub shorthand: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnchorInline {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reftext: Option<String>,
    pub inlines: Vec<PreparedInline>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PassthroughInline {
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PassthroughBlock {
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TocBlock {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageBlock {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reftext: Option<String>,
    pub target: String,
    pub alt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<String>,
    pub attributes: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub float: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub align: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageInline {
    pub target: String,
    pub alt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IconInline {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FootnoteInline {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<u32>,
    pub inlines: Vec<PreparedInline>,
}

#[derive(Debug, Clone)]
struct SectionRef {
    id: String,
    title: String,
}

#[derive(Debug, Clone)]
struct BlockRef {
    id: String,
    title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionBlock {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reftext: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reftext: Option<String>,
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
pub struct QuoteBlock {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reftext: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attribution: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citetitle: Option<String>,
    pub is_verse: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Footnote {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inlines: Vec<PreparedInline>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Author {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Revision {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remark: Option<String>,
}

pub fn prepare_document(document: &Document) -> DocumentBlock {
    let mut next_section_ids = Vec::new();
    let mut blocks = prepare_blocks(&document.blocks, true, &mut next_section_ids);
    let mut footnotes = collect_footnotes(&mut blocks);
    let section_refs = collect_section_refs(&blocks);
    let block_refs = collect_block_refs(&blocks);
    resolve_xrefs_in_blocks(&mut blocks, &section_refs, &block_refs);
    resolve_xrefs_in_footnotes(&mut footnotes, &section_refs, &block_refs);
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
        attributes: document.attributes.clone(),
        revision: prepare_revision(document),
        blocks,
        content_model: Some(ContentModel::Compound),
        footnotes,
        sections,
        authors: prepare_authors(document),
    }
}

fn prepare_authors(document: &Document) -> Vec<Author> {
    let indexed_authors = collect_indexed_authors(&document.attributes);
    if !indexed_authors.is_empty() {
        return indexed_authors;
    }

    let name = document
        .attributes
        .get("author")
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    let email = document
        .attributes
        .get("email")
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);

    if name.is_none() && email.is_none() {
        return Vec::new();
    }

    vec![Author { name, email }]
}

fn collect_indexed_authors(attributes: &BTreeMap<String, String>) -> Vec<Author> {
    let mut authors = Vec::new();
    let mut index = 1;

    loop {
        let name = attributes
            .get(&format!("author_{index}"))
            .map(String::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned);
        let email = attributes
            .get(&format!("email_{index}"))
            .map(String::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned);

        if name.is_none() && email.is_none() {
            break;
        }

        authors.push(Author { name, email });
        index += 1;
    }

    authors
}

fn prepare_revision(document: &Document) -> Option<Revision> {
    let number = document
        .attributes
        .get("revnumber")
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    let date = document
        .attributes
        .get("revdate")
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    let remark = document
        .attributes
        .get("revremark")
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);

    if number.is_none() && date.is_none() && remark.is_none() {
        None
    } else {
        Some(Revision {
            number,
            date,
            remark,
        })
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
            Block::Admonition(admonition) => {
                let admonition = PreparedBlock::Admonition(prepare_admonition_block(admonition));
                if wrap_document_preamble && !seen_section {
                    preamble_blocks.push(admonition);
                } else {
                    prepared.push(admonition);
                }
                index += 1;
            }
            Block::UnorderedList(list) => {
                let list = PreparedBlock::UnorderedList(prepare_unordered_list(list));
                if wrap_document_preamble && !seen_section {
                    preamble_blocks.push(list);
                } else {
                    prepared.push(list);
                }
                index += 1;
            }
            Block::OrderedList(list) => {
                let list = PreparedBlock::OrderedList(prepare_ordered_list(list));
                if wrap_document_preamble && !seen_section {
                    preamble_blocks.push(list);
                } else {
                    prepared.push(list);
                }
                index += 1;
            }
            Block::DescriptionList(list) => {
                let list = PreparedBlock::DescriptionList(prepare_description_list(list));
                if wrap_document_preamble && !seen_section {
                    preamble_blocks.push(list);
                } else {
                    prepared.push(list);
                }
                index += 1;
            }
            Block::Table(table) => {
                let table = PreparedBlock::Table(prepare_table(table));
                if wrap_document_preamble && !seen_section {
                    preamble_blocks.push(table);
                } else {
                    prepared.push(table);
                }
                index += 1;
            }
            Block::Listing(listing) => {
                let listing = PreparedBlock::Listing(prepare_listing(listing));
                if wrap_document_preamble && !seen_section {
                    preamble_blocks.push(listing);
                } else {
                    prepared.push(listing);
                }
                index += 1;
            }
            Block::Literal(literal) => {
                let literal = PreparedBlock::Literal(prepare_listing(literal));
                if wrap_document_preamble && !seen_section {
                    preamble_blocks.push(literal);
                } else {
                    prepared.push(literal);
                }
                index += 1;
            }
            Block::CalloutList(colist) => {
                let colist = PreparedBlock::CalloutList(prepare_callout_list(colist));
                if wrap_document_preamble && !seen_section {
                    preamble_blocks.push(colist);
                } else {
                    prepared.push(colist);
                }
                index += 1;
            }
            Block::Example(example) => {
                let example = PreparedBlock::Example(prepare_compound_block(example));
                if wrap_document_preamble && !seen_section {
                    preamble_blocks.push(example);
                } else {
                    prepared.push(example);
                }
                index += 1;
            }
            Block::Sidebar(sidebar) => {
                let sidebar = PreparedBlock::Sidebar(prepare_compound_block(sidebar));
                if wrap_document_preamble && !seen_section {
                    preamble_blocks.push(sidebar);
                } else {
                    prepared.push(sidebar);
                }
                index += 1;
            }
            Block::Open(open) => {
                let open = PreparedBlock::Open(prepare_compound_block(open));
                if wrap_document_preamble && !seen_section {
                    preamble_blocks.push(open);
                } else {
                    prepared.push(open);
                }
                index += 1;
            }
            Block::Quote(quote) => {
                let quote = PreparedBlock::Quote(prepare_quote_block(quote));
                if wrap_document_preamble && !seen_section {
                    preamble_blocks.push(quote);
                } else {
                    prepared.push(quote);
                }
                index += 1;
            }
            Block::Passthrough(content) => {
                let passthrough = PreparedBlock::Passthrough(PassthroughBlock {
                    content: trim_outer_blank_lines(content),
                });
                if wrap_document_preamble && !seen_section {
                    preamble_blocks.push(passthrough);
                } else {
                    prepared.push(passthrough);
                }
                index += 1;
            }
            Block::Image(image) => {
                let image = PreparedBlock::Image(prepare_image_block(image));
                if wrap_document_preamble && !seen_section {
                    preamble_blocks.push(image);
                } else {
                    prepared.push(image);
                }
                index += 1;
            }
            Block::Toc => {
                let toc = PreparedBlock::Toc(TocBlock {});
                if wrap_document_preamble && !seen_section {
                    preamble_blocks.push(toc);
                } else {
                    prepared.push(toc);
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
                let id = heading
                    .id
                    .clone()
                    .unwrap_or_else(|| next_section_id(section_ids, section_level, &heading.title));
                let next_heading_index = find_section_end(blocks, index + 1, section_level);
                let section_blocks =
                    prepare_blocks(&blocks[index + 1..next_heading_index], false, section_ids);

                prepared.push(PreparedBlock::Section(SectionBlock {
                    id,
                    reftext: heading.reftext.clone(),
                    blocks: section_blocks,
                    attributes: heading.metadata.attributes.clone(),
                    content_model: Some(ContentModel::Compound),
                    line_number: None,
                    style: heading.metadata.style.clone(),
                    role: heading.metadata.role.clone(),
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
        id: paragraph.id.clone(),
        reftext: paragraph.reftext.clone(),
        blocks: Vec::new(),
        content: paragraph.plain_text(),
        inlines: prepare_inlines(&paragraph.inlines),
        attributes: paragraph.metadata.attributes.clone(),
        content_model: Some(ContentModel::Simple),
        line_number: None,
        style: paragraph.metadata.style.clone(),
        role: paragraph.metadata.role.clone(),
        level: 0,
        title: paragraph.metadata.title.clone(),
    }
}

fn prepare_admonition_block(block: &AstAdmonitionBlock) -> AdmonitionBlock {
    AdmonitionBlock {
        id: block.id.clone().or_else(|| block.metadata.id.clone()),
        reftext: block.reftext.clone(),
        blocks: prepare_blocks(&block.blocks, false, &mut Vec::new()),
        attributes: block.metadata.attributes.clone(),
        content_model: Some(ContentModel::Compound),
        line_number: None,
        style: block.metadata.style.clone(),
        role: block.metadata.role.clone(),
        level: 0,
        title: block.metadata.title.clone(),
        variant: block.variant.as_str().into(),
    }
}

fn prepare_unordered_list(list: &UnorderedList) -> ListBlock {
    ListBlock {
        items: list
            .items
            .iter()
            .map(|item| ListItemBlock {
                blocks: prepare_blocks(&item.blocks, false, &mut Vec::new()),
            })
            .collect(),
        attributes: list.metadata.attributes.clone(),
        content_model: Some(ContentModel::Compound),
        level: 0,
        name: "ulist".into(),
        id: list.metadata.id.clone(),
        reftext: list.reftext.clone(),
        style: list.metadata.style.clone(),
        role: list.metadata.role.clone(),
        title: list.metadata.title.clone(),
    }
}

fn prepare_ordered_list(list: &OrderedList) -> ListBlock {
    ListBlock {
        items: list
            .items
            .iter()
            .map(|item| ListItemBlock {
                blocks: prepare_blocks(&item.blocks, false, &mut Vec::new()),
            })
            .collect(),
        attributes: list.metadata.attributes.clone(),
        content_model: Some(ContentModel::Compound),
        level: 0,
        name: "olist".into(),
        id: list.metadata.id.clone(),
        reftext: list.reftext.clone(),
        style: list.metadata.style.clone(),
        role: list.metadata.role.clone(),
        title: list.metadata.title.clone(),
    }
}

fn prepare_description_list(list: &AstDescriptionList) -> DescriptionListBlock {
    DescriptionListBlock {
        items: list
            .items
            .iter()
            .map(|item| DescriptionListItemBlock {
                terms: item
                    .terms
                    .iter()
                    .map(|term| DescriptionListTermBlock {
                        text: term.text.clone(),
                        inlines: prepare_inlines(&term.inlines),
                    })
                    .collect(),
                description: item.description.as_ref().map(|desc| ListItemBlock {
                    blocks: prepare_blocks(&desc.blocks, false, &mut Vec::new()),
                }),
            })
            .collect(),
        attributes: list.metadata.attributes.clone(),
        content_model: Some(ContentModel::Compound),
        level: 0,
        name: "dlist".into(),
        id: list.metadata.id.clone(),
        reftext: list.reftext.clone(),
        style: list.metadata.style.clone(),
        role: list.metadata.role.clone(),
        title: list.metadata.title.clone(),
    }
}

fn prepare_listing(listing: &AstListing) -> ListingBlock {
    let (trimmed_start, trimmed_end) =
        trimmed_content_bounds(&listing.lines.iter().map(String::as_str).collect::<Vec<_>>());
    ListingBlock {
        id: listing.metadata.id.clone(),
        reftext: listing.reftext.clone(),
        content: listing.lines[trimmed_start..trimmed_end].join("\n"),
        attributes: listing.metadata.attributes.clone(),
        content_model: Some(ContentModel::Simple),
        line_number: None,
        style: listing.metadata.style.clone(),
        role: listing.metadata.role.clone(),
        level: 0,
        title: listing.metadata.title.clone(),
        callout_lines: listing
            .callouts
            .iter()
            .filter_map(|(line, number)| {
                (*line >= trimmed_start && *line < trimmed_end)
                    .then_some((line - trimmed_start, *number))
            })
            .collect(),
    }
}

fn prepare_callout_list(colist: &crate::ast::CalloutList) -> CalloutListBlock {
    CalloutListBlock {
        items: colist
            .items
            .iter()
            .map(|item| CalloutItemBlock {
                number: item.number,
                inlines: prepare_inlines(&item.inlines),
            })
            .collect(),
    }
}

fn prepare_table(table: &AstTableBlock) -> TableBlock {
    TableBlock {
        id: table.metadata.id.clone(),
        reftext: table.reftext.clone(),
        header: table.header.as_ref().map(prepare_table_row),
        rows: table.rows.iter().map(prepare_table_row).collect(),
        attributes: table.metadata.attributes.clone(),
        content_model: Some(ContentModel::Compound),
        style: table.metadata.style.clone(),
        role: table.metadata.role.clone(),
        level: 0,
        title: table.metadata.title.clone(),
    }
}

fn prepare_table_row(row: &AstTableRow) -> TableRow {
    TableRow {
        cells: row.cells.iter().map(prepare_table_cell).collect(),
    }
}

fn prepare_table_cell(cell: &AstTableCell) -> TableCell {
    TableCell {
        content: cell.content.clone(),
        inlines: prepare_inlines(&cell.inlines),
        blocks: prepare_blocks(&cell.blocks, false, &mut Vec::new()),
        colspan: cell.colspan,
        rowspan: cell.rowspan,
        style: cell.style.clone(),
    }
}

fn prepare_compound_block(block: &AstCompoundBlock) -> CompoundBlock {
    CompoundBlock {
        id: block.metadata.id.clone(),
        reftext: block.reftext.clone(),
        blocks: prepare_blocks(&block.blocks, false, &mut Vec::new()),
        attributes: block.metadata.attributes.clone(),
        content_model: Some(ContentModel::Compound),
        line_number: None,
        style: block.metadata.style.clone(),
        role: block.metadata.role.clone(),
        level: 0,
        title: block.metadata.title.clone(),
    }
}

fn prepare_quote_block(block: &AstQuoteBlock) -> QuoteBlock {
    QuoteBlock {
        id: block.metadata.id.clone(),
        reftext: block.reftext.clone(),
        blocks: prepare_blocks(&block.blocks, false, &mut Vec::new()),
        content: trim_outer_blank_lines(block.content.as_deref().unwrap_or_default()),
        attributes: block.metadata.attributes.clone(),
        content_model: if block.is_verse {
            Some(ContentModel::Simple)
        } else {
            Some(ContentModel::Compound)
        },
        line_number: None,
        style: block.metadata.style.clone(),
        role: block.metadata.role.clone(),
        level: 0,
        title: block.metadata.title.clone(),
        attribution: block.attribution.clone(),
        citetitle: block.citetitle.clone(),
        is_verse: block.is_verse,
    }
}

fn trimmed_content_bounds(lines: &[&str]) -> (usize, usize) {
    let start = lines
        .iter()
        .position(|line| !line.trim().is_empty())
        .unwrap_or(lines.len());
    let end = lines
        .iter()
        .rposition(|line| !line.trim().is_empty())
        .map(|index| index + 1)
        .unwrap_or(start);
    (start, end)
}

fn prepare_inlines(inlines: &[Inline]) -> Vec<PreparedInline> {
    inlines
        .iter()
        .map(|inline| match inline {
            Inline::Text(text) => PreparedInline::Text(TextInline {
                value: text.clone(),
            }),
            Inline::Span(span) => PreparedInline::Span(SpanInline {
                variant: inline_variant_name(span.variant).into(),
                form: inline_form_name(span.form).into(),
                inlines: prepare_inlines(&span.inlines),
            }),
            Inline::Link(link) => PreparedInline::Link(LinkInline {
                target: link.target.clone(),
                inlines: prepare_inlines(&link.text),
                bare: link.bare,
                window: link.window.clone(),
            }),
            Inline::Xref(xref) => PreparedInline::Xref(XrefInline {
                target: xref.target.clone(),
                href: xref.target.clone(),
                inlines: prepare_inlines(&xref.text),
                shorthand: xref.shorthand,
            }),
            Inline::Anchor(anchor) => PreparedInline::Anchor(AnchorInline {
                id: anchor.id.clone(),
                reftext: anchor.reftext.clone(),
                inlines: prepare_inlines(&anchor.inlines),
            }),
            Inline::Passthrough(raw) => {
                PreparedInline::Passthrough(PassthroughInline { value: raw.clone() })
            }
            Inline::Image(image) => PreparedInline::Image(ImageInline {
                target: image.target.clone(),
                alt: image.alt.clone(),
                width: image.width.clone(),
                height: image.height.clone(),
            }),
            Inline::Icon(icon) => PreparedInline::Icon(IconInline {
                name: icon.name.clone(),
                size: icon.size.clone(),
                title: icon.title.clone(),
                role: icon.role.clone(),
            }),
            Inline::Footnote(footnote) => PreparedInline::Footnote(FootnoteInline {
                index: None,
                inlines: prepare_inlines(&footnote.inlines),
            }),
        })
        .collect()
}

fn prepare_image_block(image: &AstImageBlock) -> ImageBlock {
    let named = |name: &str| -> Option<String> {
        image
            .metadata
            .attributes
            .get(name)
            .cloned()
            .filter(|v| !v.is_empty())
    };
    ImageBlock {
        id: image.metadata.id.clone(),
        reftext: None,
        target: image.target.clone(),
        alt: image.alt.clone(),
        width: image.width.clone(),
        height: image.height.clone(),
        attributes: image.metadata.attributes.clone(),
        title: image.metadata.title.clone(),
        style: image.metadata.style.clone(),
        role: image.metadata.role.clone(),
        link: named("link"),
        float: named("float"),
        align: named("align"),
    }
}

fn prepare_preamble(blocks: Vec<PreparedBlock>) -> CompoundBlock {
    CompoundBlock {
        id: None,
        reftext: None,
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
            PreparedBlock::Preamble(_)
            | PreparedBlock::Paragraph(_)
            | PreparedBlock::Admonition(_)
            | PreparedBlock::UnorderedList(_)
            | PreparedBlock::OrderedList(_)
            | PreparedBlock::DescriptionList(_)
            | PreparedBlock::Table(_)
            | PreparedBlock::Listing(_)
            | PreparedBlock::Literal(_)
            | PreparedBlock::CalloutList(_)
            | PreparedBlock::Example(_)
            | PreparedBlock::Sidebar(_)
            | PreparedBlock::Open(_)
            | PreparedBlock::Quote(_)
            | PreparedBlock::Passthrough(_)
            | PreparedBlock::Image(_)
            | PreparedBlock::Toc(_) => None,
        })
        .collect()
}

fn collect_section_refs(blocks: &[PreparedBlock]) -> BTreeMap<String, SectionRef> {
    let mut refs = BTreeMap::new();
    collect_section_refs_into(blocks, &mut refs);
    refs
}

fn collect_section_refs_into(blocks: &[PreparedBlock], refs: &mut BTreeMap<String, SectionRef>) {
    for block in blocks {
        if let PreparedBlock::Section(section) = block {
            let section_ref = SectionRef {
                id: section.id.clone(),
                title: section
                    .reftext
                    .clone()
                    .unwrap_or_else(|| section.title.clone()),
            };
            for key in section_ref_keys(&section.id, &section.title) {
                refs.entry(key).or_insert_with(|| section_ref.clone());
            }
            collect_section_refs_into(&section.blocks, refs);
        }
    }
}

fn collect_block_refs(blocks: &[PreparedBlock]) -> BTreeMap<String, BlockRef> {
    let mut refs = BTreeMap::new();
    collect_block_refs_into(blocks, &mut refs);
    refs
}

fn collect_block_refs_into(blocks: &[PreparedBlock], refs: &mut BTreeMap<String, BlockRef>) {
    for block in blocks {
        match block {
            PreparedBlock::Preamble(preamble) => collect_block_refs_into(&preamble.blocks, refs),
            PreparedBlock::Paragraph(paragraph) => {
                if let Some(id) = &paragraph.id {
                    refs.entry(normalize_section_ref_key(id))
                        .or_insert(BlockRef {
                            id: id.clone(),
                            title: paragraph
                                .reftext
                                .clone()
                                .or_else(|| paragraph.title.clone()),
                        });
                }
                collect_inline_anchor_refs(&paragraph.inlines, refs);
            }
            PreparedBlock::Admonition(admonition) => {
                if let Some(id) = &admonition.id {
                    refs.entry(normalize_section_ref_key(id))
                        .or_insert(BlockRef {
                            id: id.clone(),
                            title: admonition
                                .reftext
                                .clone()
                                .or_else(|| admonition.title.clone()),
                        });
                }
                collect_block_refs_into(&admonition.blocks, refs);
            }
            PreparedBlock::UnorderedList(list) | PreparedBlock::OrderedList(list) => {
                if let Some(id) = &list.id {
                    refs.entry(normalize_section_ref_key(id))
                        .or_insert(BlockRef {
                            id: id.clone(),
                            title: list.reftext.clone().or_else(|| list.title.clone()),
                        });
                }
                for item in &list.items {
                    collect_block_refs_into(&item.blocks, refs);
                }
            }
            PreparedBlock::DescriptionList(list) => {
                if let Some(id) = &list.id {
                    refs.entry(normalize_section_ref_key(id))
                        .or_insert(BlockRef {
                            id: id.clone(),
                            title: list.reftext.clone().or_else(|| list.title.clone()),
                        });
                }
                for item in &list.items {
                    for term in &item.terms {
                        collect_inline_anchor_refs(&term.inlines, refs);
                    }
                    if let Some(desc) = &item.description {
                        collect_block_refs_into(&desc.blocks, refs);
                    }
                }
            }
            PreparedBlock::Listing(listing) | PreparedBlock::Literal(listing) => {
                if let Some(id) = &listing.id {
                    refs.entry(normalize_section_ref_key(id))
                        .or_insert(BlockRef {
                            id: id.clone(),
                            title: listing.reftext.clone().or_else(|| listing.title.clone()),
                        });
                }
            }
            PreparedBlock::Table(table) => {
                if let Some(id) = &table.id {
                    refs.entry(normalize_section_ref_key(id))
                        .or_insert(BlockRef {
                            id: id.clone(),
                            title: table.reftext.clone().or_else(|| table.title.clone()),
                        });
                }
                if let Some(header) = &table.header {
                    collect_table_row_inline_anchor_refs(header, refs);
                }
                for row in &table.rows {
                    collect_table_row_inline_anchor_refs(row, refs);
                }
            }
            PreparedBlock::Example(example)
            | PreparedBlock::Sidebar(example)
            | PreparedBlock::Open(example) => {
                if let Some(id) = &example.id {
                    refs.entry(normalize_section_ref_key(id))
                        .or_insert(BlockRef {
                            id: id.clone(),
                            title: example.reftext.clone().or_else(|| example.title.clone()),
                        });
                }
                collect_block_refs_into(&example.blocks, refs);
            }
            PreparedBlock::Quote(quote) => {
                if let Some(id) = &quote.id {
                    refs.entry(normalize_section_ref_key(id))
                        .or_insert(BlockRef {
                            id: id.clone(),
                            title: quote.reftext.clone().or_else(|| quote.title.clone()),
                        });
                }
                collect_block_refs_into(&quote.blocks, refs);
            }
            PreparedBlock::Section(section) => {
                refs.entry(normalize_section_ref_key(&section.id))
                    .or_insert(BlockRef {
                        id: section.id.clone(),
                        title: section
                            .reftext
                            .clone()
                            .or_else(|| Some(section.title.clone())),
                    });
                collect_block_refs_into(&section.blocks, refs);
            }
            PreparedBlock::Passthrough(_)
            | PreparedBlock::Toc(_)
            | PreparedBlock::CalloutList(_) => {}
            PreparedBlock::Image(image) => {
                if let Some(id) = &image.id {
                    refs.entry(normalize_section_ref_key(id))
                        .or_insert(BlockRef {
                            id: id.clone(),
                            title: image.reftext.clone().or_else(|| image.title.clone()),
                        });
                }
            }
        }
    }
}

fn collect_inline_anchor_refs(inlines: &[PreparedInline], refs: &mut BTreeMap<String, BlockRef>) {
    for inline in inlines {
        match inline {
            PreparedInline::Anchor(anchor) => {
                let anchor_text = anchor
                    .inlines
                    .iter()
                    .map(prepared_inline_plain_text)
                    .collect::<Vec<_>>()
                    .join("");
                refs.entry(normalize_section_ref_key(&anchor.id))
                    .or_insert(BlockRef {
                        id: anchor.id.clone(),
                        title: anchor
                            .reftext
                            .clone()
                            .or_else(|| (!anchor_text.is_empty()).then_some(anchor_text.clone())),
                    });
                collect_inline_anchor_refs(&anchor.inlines, refs);
            }
            PreparedInline::Span(span) => collect_inline_anchor_refs(&span.inlines, refs),
            PreparedInline::Link(link) => collect_inline_anchor_refs(&link.inlines, refs),
            PreparedInline::Xref(xref) => collect_inline_anchor_refs(&xref.inlines, refs),
            PreparedInline::Footnote(footnote) => {
                collect_inline_anchor_refs(&footnote.inlines, refs)
            }
            PreparedInline::Text(_)
            | PreparedInline::Passthrough(_)
            | PreparedInline::Image(_)
            | PreparedInline::Icon(_) => {}
        }
    }
}

fn collect_table_row_inline_anchor_refs(row: &TableRow, refs: &mut BTreeMap<String, BlockRef>) {
    for cell in &row.cells {
        collect_block_refs_into(&cell.blocks, refs);
    }
}

fn resolve_xrefs_in_blocks(
    blocks: &mut [PreparedBlock],
    section_refs: &BTreeMap<String, SectionRef>,
    block_refs: &BTreeMap<String, BlockRef>,
) {
    for block in blocks {
        match block {
            PreparedBlock::Preamble(preamble) => {
                resolve_xrefs_in_blocks(&mut preamble.blocks, section_refs, block_refs)
            }
            PreparedBlock::Paragraph(paragraph) => {
                resolve_xrefs_in_inlines(&mut paragraph.inlines, section_refs, block_refs);
                paragraph.content = paragraph
                    .inlines
                    .iter()
                    .map(prepared_inline_plain_text)
                    .collect::<Vec<_>>()
                    .join("");
            }
            PreparedBlock::Admonition(admonition) => {
                resolve_xrefs_in_blocks(&mut admonition.blocks, section_refs, block_refs)
            }
            PreparedBlock::UnorderedList(list) | PreparedBlock::OrderedList(list) => {
                for item in &mut list.items {
                    resolve_xrefs_in_blocks(&mut item.blocks, section_refs, block_refs);
                }
            }
            PreparedBlock::DescriptionList(list) => {
                for item in &mut list.items {
                    for term in &mut item.terms {
                        resolve_xrefs_in_inlines(&mut term.inlines, section_refs, block_refs);
                        term.text = term
                            .inlines
                            .iter()
                            .map(prepared_inline_plain_text)
                            .collect::<Vec<_>>()
                            .join("");
                    }
                    if let Some(desc) = &mut item.description {
                        resolve_xrefs_in_blocks(&mut desc.blocks, section_refs, block_refs);
                    }
                }
            }
            PreparedBlock::Table(table) => {
                if let Some(header) = &mut table.header {
                    resolve_xrefs_in_table_row(header, section_refs, block_refs);
                }
                for row in &mut table.rows {
                    resolve_xrefs_in_table_row(row, section_refs, block_refs);
                }
            }
            PreparedBlock::Listing(_)
            | PreparedBlock::Literal(_)
            | PreparedBlock::Passthrough(_)
            | PreparedBlock::Image(_)
            | PreparedBlock::Toc(_) => {}
            PreparedBlock::Example(example)
            | PreparedBlock::Sidebar(example)
            | PreparedBlock::Open(example) => {
                resolve_xrefs_in_blocks(&mut example.blocks, section_refs, block_refs)
            }
            PreparedBlock::Quote(quote) => {
                resolve_xrefs_in_blocks(&mut quote.blocks, section_refs, block_refs)
            }
            PreparedBlock::Section(section) => {
                resolve_xrefs_in_blocks(&mut section.blocks, section_refs, block_refs)
            }
            PreparedBlock::CalloutList(_) => {}
        }
    }
}

fn resolve_xrefs_in_table_row(
    row: &mut TableRow,
    section_refs: &BTreeMap<String, SectionRef>,
    block_refs: &BTreeMap<String, BlockRef>,
) {
    for cell in &mut row.cells {
        resolve_xrefs_in_blocks(&mut cell.blocks, section_refs, block_refs);
        if let [PreparedBlock::Paragraph(paragraph)] = cell.blocks.as_slice() {
            cell.inlines = paragraph.inlines.clone();
            cell.content = paragraph.content.clone();
        } else {
            cell.inlines = Vec::new();
            cell.content = prepared_blocks_plain_text(&cell.blocks);
        }
    }
}

fn collect_footnotes(blocks: &mut [PreparedBlock]) -> Vec<Footnote> {
    let mut footnotes = Vec::new();
    let mut next_index = 1;
    collect_footnotes_from_blocks(blocks, &mut footnotes, &mut next_index);
    footnotes
}

fn collect_footnotes_from_blocks(
    blocks: &mut [PreparedBlock],
    footnotes: &mut Vec<Footnote>,
    next_index: &mut u32,
) {
    for block in blocks {
        match block {
            PreparedBlock::Preamble(preamble) => {
                collect_footnotes_from_blocks(&mut preamble.blocks, footnotes, next_index)
            }
            PreparedBlock::Paragraph(paragraph) => {
                collect_footnotes_from_inlines(&mut paragraph.inlines, footnotes, next_index);
                paragraph.content = paragraph
                    .inlines
                    .iter()
                    .map(prepared_inline_plain_text)
                    .collect::<Vec<_>>()
                    .join("");
            }
            PreparedBlock::Admonition(admonition) => {
                collect_footnotes_from_blocks(&mut admonition.blocks, footnotes, next_index)
            }
            PreparedBlock::UnorderedList(list) | PreparedBlock::OrderedList(list) => {
                for item in &mut list.items {
                    collect_footnotes_from_blocks(&mut item.blocks, footnotes, next_index);
                }
            }
            PreparedBlock::DescriptionList(list) => {
                for item in &mut list.items {
                    for term in &mut item.terms {
                        collect_footnotes_from_inlines(&mut term.inlines, footnotes, next_index);
                        term.text = term
                            .inlines
                            .iter()
                            .map(prepared_inline_plain_text)
                            .collect::<Vec<_>>()
                            .join("");
                    }
                    if let Some(desc) = &mut item.description {
                        collect_footnotes_from_blocks(&mut desc.blocks, footnotes, next_index);
                    }
                }
            }
            PreparedBlock::Table(table) => {
                if let Some(header) = &mut table.header {
                    collect_footnotes_from_table_row(header, footnotes, next_index);
                }
                for row in &mut table.rows {
                    collect_footnotes_from_table_row(row, footnotes, next_index);
                }
            }
            PreparedBlock::Listing(_)
            | PreparedBlock::Literal(_)
            | PreparedBlock::Passthrough(_)
            | PreparedBlock::Image(_)
            | PreparedBlock::Toc(_) => {}
            PreparedBlock::Example(example)
            | PreparedBlock::Sidebar(example)
            | PreparedBlock::Open(example) => {
                collect_footnotes_from_blocks(&mut example.blocks, footnotes, next_index)
            }
            PreparedBlock::Quote(quote) => {
                collect_footnotes_from_blocks(&mut quote.blocks, footnotes, next_index)
            }
            PreparedBlock::Section(section) => {
                collect_footnotes_from_blocks(&mut section.blocks, footnotes, next_index)
            }
            PreparedBlock::CalloutList(_) => {}
        }
    }
}

fn collect_footnotes_from_inlines(
    inlines: &mut [PreparedInline],
    footnotes: &mut Vec<Footnote>,
    next_index: &mut u32,
) {
    for inline in inlines {
        match inline {
            PreparedInline::Text(_)
            | PreparedInline::Passthrough(_)
            | PreparedInline::Image(_)
            | PreparedInline::Icon(_) => {}
            PreparedInline::Span(span) => {
                collect_footnotes_from_inlines(&mut span.inlines, footnotes, next_index)
            }
            PreparedInline::Link(link) => {
                collect_footnotes_from_inlines(&mut link.inlines, footnotes, next_index)
            }
            PreparedInline::Xref(xref) => {
                collect_footnotes_from_inlines(&mut xref.inlines, footnotes, next_index)
            }
            PreparedInline::Anchor(anchor) => {
                collect_footnotes_from_inlines(&mut anchor.inlines, footnotes, next_index)
            }
            PreparedInline::Footnote(footnote) => {
                collect_footnotes_from_inlines(&mut footnote.inlines, footnotes, next_index);
                let index = *next_index;
                *next_index += 1;
                footnote.index = Some(index);
                footnotes.push(Footnote {
                    text: Some(
                        footnote
                            .inlines
                            .iter()
                            .map(prepared_inline_plain_text)
                            .collect::<Vec<_>>()
                            .join(""),
                    ),
                    index: Some(index),
                    inlines: footnote.inlines.clone(),
                });
            }
        }
    }
}

fn collect_footnotes_from_table_row(
    row: &mut TableRow,
    footnotes: &mut Vec<Footnote>,
    next_index: &mut u32,
) {
    for cell in &mut row.cells {
        collect_footnotes_from_blocks(&mut cell.blocks, footnotes, next_index);
        if let [PreparedBlock::Paragraph(paragraph)] = cell.blocks.as_slice() {
            cell.inlines = paragraph.inlines.clone();
            cell.content = paragraph.content.clone();
        } else {
            collect_footnotes_from_inlines(&mut cell.inlines, footnotes, next_index);
            cell.content = if cell.inlines.is_empty() {
                prepared_blocks_plain_text(&cell.blocks)
            } else {
                cell.inlines
                    .iter()
                    .map(prepared_inline_plain_text)
                    .collect::<Vec<_>>()
                    .join("")
            };
        }
    }
}

fn resolve_xrefs_in_footnotes(
    footnotes: &mut [Footnote],
    section_refs: &BTreeMap<String, SectionRef>,
    block_refs: &BTreeMap<String, BlockRef>,
) {
    for footnote in footnotes {
        resolve_xrefs_in_inlines(&mut footnote.inlines, section_refs, block_refs);
        footnote.text = Some(
            footnote
                .inlines
                .iter()
                .map(prepared_inline_plain_text)
                .collect::<Vec<_>>()
                .join(""),
        );
    }
}

fn prepared_blocks_plain_text(blocks: &[PreparedBlock]) -> String {
    blocks
        .iter()
        .map(prepared_block_plain_text)
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn prepared_block_plain_text(block: &PreparedBlock) -> String {
    match block {
        PreparedBlock::Preamble(preamble)
        | PreparedBlock::Example(preamble)
        | PreparedBlock::Sidebar(preamble)
        | PreparedBlock::Open(preamble) => prepared_blocks_plain_text(&preamble.blocks),
        PreparedBlock::Paragraph(paragraph) => paragraph.content.clone(),
        PreparedBlock::Admonition(admonition) => prepared_blocks_plain_text(&admonition.blocks),
        PreparedBlock::Section(section) => prepared_blocks_plain_text(&section.blocks),
        PreparedBlock::UnorderedList(list) | PreparedBlock::OrderedList(list) => list
            .items
            .iter()
            .map(|item| prepared_blocks_plain_text(&item.blocks))
            .collect::<Vec<_>>()
            .join("\n"),
        PreparedBlock::DescriptionList(list) => list
            .items
            .iter()
            .flat_map(|item| {
                item.terms.iter().map(|term| term.text.clone()).chain(
                    item.description
                        .as_ref()
                        .map(|desc| prepared_blocks_plain_text(&desc.blocks))
                        .into_iter(),
                )
            })
            .collect::<Vec<_>>()
            .join("\n"),
        PreparedBlock::Table(table) => table
            .rows
            .iter()
            .flat_map(|row| row.cells.iter().map(|cell| cell.content.clone()))
            .collect::<Vec<_>>()
            .join("\n"),
        PreparedBlock::Listing(listing) | PreparedBlock::Literal(listing) => {
            listing.content.clone()
        }
        PreparedBlock::Quote(quote) => {
            if quote.content.is_empty() {
                prepared_blocks_plain_text(&quote.blocks)
            } else {
                quote.content.clone()
            }
        }
        PreparedBlock::Passthrough(passthrough) => passthrough.content.clone(),
        PreparedBlock::Image(image) => image.alt.clone(),
        PreparedBlock::Toc(_) | PreparedBlock::CalloutList(_) => String::new(),
    }
}

fn resolve_xrefs_in_inlines(
    inlines: &mut [PreparedInline],
    section_refs: &BTreeMap<String, SectionRef>,
    block_refs: &BTreeMap<String, BlockRef>,
) {
    for inline in inlines {
        match inline {
            PreparedInline::Text(_)
            | PreparedInline::Link(_)
            | PreparedInline::Passthrough(_)
            | PreparedInline::Image(_)
            | PreparedInline::Icon(_) => {}
            PreparedInline::Anchor(anchor) => {
                resolve_xrefs_in_inlines(&mut anchor.inlines, section_refs, block_refs)
            }
            PreparedInline::Span(span) => {
                resolve_xrefs_in_inlines(&mut span.inlines, section_refs, block_refs)
            }
            PreparedInline::Footnote(footnote) => {
                resolve_xrefs_in_inlines(&mut footnote.inlines, section_refs, block_refs)
            }
            PreparedInline::Xref(xref) => {
                if let Some(section_ref) = resolve_section_ref(&xref.target, section_refs) {
                    xref.href = format!("#{}", section_ref.id);
                    if xref.inlines.len() == 1
                        && matches!(xref.inlines.first(), Some(PreparedInline::Text(text)) if text.value == xref.target)
                    {
                        xref.inlines = vec![PreparedInline::Text(TextInline {
                            value: section_ref.title.clone(),
                        })];
                    }
                } else if let Some(block_ref) = resolve_block_ref(&xref.target, block_refs) {
                    xref.href = format!("#{}", block_ref.id);
                    if xref.inlines.len() == 1
                        && matches!(xref.inlines.first(), Some(PreparedInline::Text(text)) if text.value == xref.target)
                        && let Some(title) = &block_ref.title
                    {
                        xref.inlines = vec![PreparedInline::Text(TextInline {
                            value: title.clone(),
                        })];
                    }
                } else {
                    xref.href = xref_href(&xref.target);
                }
                resolve_xrefs_in_inlines(&mut xref.inlines, section_refs, block_refs);
            }
        }
    }
}

fn resolve_section_ref<'a>(
    target: &str,
    section_refs: &'a BTreeMap<String, SectionRef>,
) -> Option<&'a SectionRef> {
    if target.contains(".adoc") {
        return None;
    }
    section_refs.get(&normalize_section_ref_key(target))
}

fn resolve_block_ref<'a>(
    target: &str,
    block_refs: &'a BTreeMap<String, BlockRef>,
) -> Option<&'a BlockRef> {
    if target.contains(".adoc") {
        return None;
    }
    block_refs.get(&normalize_section_ref_key(target))
}

fn section_ref_keys(id: &str, title: &str) -> Vec<String> {
    let slug = slugify(title);
    let mut keys = vec![
        normalize_section_ref_key(id),
        normalize_section_ref_key(id.trim_start_matches('_')),
        normalize_section_ref_key(title),
        normalize_section_ref_key(&slug),
        normalize_section_ref_key(slug.trim_start_matches('_')),
    ];
    keys.sort();
    keys.dedup();
    keys
}

fn normalize_section_ref_key(value: &str) -> String {
    value.trim().trim_start_matches('#').to_ascii_lowercase()
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

fn xref_href(target: &str) -> String {
    if target.starts_with('#') || target.contains(".adoc") {
        target.to_owned()
    } else {
        format!("#{target}")
    }
}

fn prepared_inline_plain_text(inline: &PreparedInline) -> String {
    match inline {
        PreparedInline::Text(text) => text.value.clone(),
        PreparedInline::Span(span) => span
            .inlines
            .iter()
            .map(prepared_inline_plain_text)
            .collect::<Vec<_>>()
            .join(""),
        PreparedInline::Link(link) => link
            .inlines
            .iter()
            .map(prepared_inline_plain_text)
            .collect::<Vec<_>>()
            .join(""),
        PreparedInline::Xref(xref) => xref
            .inlines
            .iter()
            .map(prepared_inline_plain_text)
            .collect::<Vec<_>>()
            .join(""),
        PreparedInline::Anchor(anchor) => anchor
            .inlines
            .iter()
            .map(prepared_inline_plain_text)
            .collect::<Vec<_>>()
            .join(""),
        PreparedInline::Passthrough(p) => p.value.clone(),
        PreparedInline::Image(image) => image.alt.clone(),
        PreparedInline::Icon(icon) => icon.name.clone(),
        PreparedInline::Footnote(footnote) => format!("[{}]", footnote.index.unwrap_or(0)),
    }
}

fn inline_variant_name(variant: InlineVariant) -> &'static str {
    match variant {
        InlineVariant::Strong => "strong",
        InlineVariant::Emphasis => "emphasis",
        InlineVariant::Monospace => "monospace",
        InlineVariant::Subscript => "subscript",
        InlineVariant::Superscript => "superscript",
    }
}

fn inline_form_name(form: InlineForm) -> &'static str {
    match form {
        InlineForm::Constrained => "constrained",
        InlineForm::Unconstrained => "unconstrained",
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::{
        Block, BlockMetadata, CompoundBlock as AstCompoundBlock, Document, Heading, Inline,
        InlineFootnote, InlineForm, InlineLink, InlineSpan, InlineVariant, InlineXref, ListItem,
        Listing, Paragraph, UnorderedList,
    };
    use crate::parser::parse_document;
    use crate::prepare::{
        ContentModel, PreparedBlock, PreparedInline, TextInline, prepare_document,
        prepared_document_to_json,
    };

    #[test]
    fn prepares_nested_sections_for_react_facing_output() {
        let document = Document {
            attributes: Default::default(),
            title: Some(Heading {
                level: 0,
                title: "Document Title".into(),
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
            }),
            blocks: vec![
                Block::Paragraph(Paragraph {
                    inlines: vec![Inline::Text("Preamble paragraph.".into())],
                    lines: vec!["Preamble paragraph.".into()],
                    id: None,
                    reftext: None,
                    metadata: BlockMetadata::default(),
                }),
                Block::Heading(Heading {
                    level: 1,
                    title: "Section A".into(),
                    id: None,
                    reftext: None,
                    metadata: BlockMetadata::default(),
                }),
                Block::Paragraph(Paragraph {
                    inlines: vec![Inline::Text("Section body.".into())],
                    lines: vec!["Section body.".into()],
                    id: None,
                    reftext: None,
                    metadata: BlockMetadata::default(),
                }),
                Block::Heading(Heading {
                    level: 2,
                    title: "Section A Child".into(),
                    id: None,
                    reftext: None,
                    metadata: BlockMetadata::default(),
                }),
                Block::Paragraph(Paragraph {
                    inlines: vec![Inline::Text("Nested body.".into())],
                    lines: vec!["Nested body.".into()],
                    id: None,
                    reftext: None,
                    metadata: BlockMetadata::default(),
                }),
                Block::Heading(Heading {
                    level: 1,
                    title: "Section B".into(),
                    id: None,
                    reftext: None,
                    metadata: BlockMetadata::default(),
                }),
            ],
        };

        let prepared = prepare_document(&document);

        assert_eq!(prepared.node_type, "document");
        assert_eq!(prepared.title, "Document Title");
        assert_eq!(prepared.content_model, Some(ContentModel::Compound));
        assert!(prepared.attributes.is_empty());
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
    fn carries_document_attributes_into_prepared_output() {
        let document = Document {
            attributes: [("toc".to_owned(), "left".to_owned())]
                .into_iter()
                .collect(),
            title: Some(Heading {
                level: 0,
                title: "Document Title".into(),
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
            }),
            blocks: Vec::new(),
        };

        let prepared = prepare_document(&document);

        assert_eq!(
            prepared.attributes.get("toc").map(String::as_str),
            Some("left")
        );
    }

    #[test]
    fn carries_author_attribute_into_prepared_authors() {
        let document = Document {
            attributes: [("author".to_owned(), "Jane Doe".to_owned())]
                .into_iter()
                .collect(),
            title: Some(Heading {
                level: 0,
                title: "Document Title".into(),
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
            }),
            blocks: Vec::new(),
        };

        let prepared = prepare_document(&document);

        assert_eq!(
            prepared.authors,
            vec![crate::prepare::Author {
                name: Some("Jane Doe".into()),
                email: None,
            }]
        );
    }

    #[test]
    fn carries_revision_attributes_into_prepared_revision() {
        let document = Document {
            attributes: [
                ("revnumber".to_owned(), "1.2".to_owned()),
                ("revdate".to_owned(), "2026-03-31".to_owned()),
                ("revremark".to_owned(), "Draft".to_owned()),
            ]
            .into_iter()
            .collect(),
            title: Some(Heading {
                level: 0,
                title: "Document Title".into(),
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
            }),
            blocks: Vec::new(),
        };

        let prepared = prepare_document(&document);

        assert_eq!(
            prepared.revision,
            Some(crate::prepare::Revision {
                number: Some("1.2".into()),
                date: Some("2026-03-31".into()),
                remark: Some("Draft".into()),
            })
        );
    }

    #[test]
    fn carries_implicit_header_metadata_into_prepared_output() {
        let document = parse_document(
            "= Document Title\nStuart Rackham <founder@asciidoc.org>\nv8.6.8, 2012-07-12: See changelog.\n\ncontent",
        );

        let prepared = prepare_document(&document);

        assert_eq!(
            prepared.authors,
            vec![crate::prepare::Author {
                name: Some("Stuart Rackham".into()),
                email: Some("founder@asciidoc.org".into()),
            }]
        );
        assert_eq!(
            prepared.revision,
            Some(crate::prepare::Revision {
                number: Some("8.6.8".into()),
                date: Some("2012-07-12".into()),
                remark: Some("See changelog.".into()),
            })
        );
        assert_eq!(
            prepared.attributes.get("author").map(String::as_str),
            Some("Stuart Rackham")
        );
        assert_eq!(
            prepared.attributes.get("email").map(String::as_str),
            Some("founder@asciidoc.org")
        );
    }

    #[test]
    fn carries_multiple_implicit_authors_into_prepared_authors() {
        let document = parse_document(
            "= Document Title\nDoc Writer <thedoctor@asciidoc.org>; Junior Writer <junior@asciidoctor.org>\n\ncontent",
        );

        let prepared = prepare_document(&document);

        assert_eq!(
            prepared.authors,
            vec![
                crate::prepare::Author {
                    name: Some("Doc Writer".into()),
                    email: Some("thedoctor@asciidoc.org".into()),
                },
                crate::prepare::Author {
                    name: Some("Junior Writer".into()),
                    email: Some("junior@asciidoctor.org".into()),
                },
            ]
        );
    }

    #[test]
    fn carries_explicit_authors_attribute_into_prepared_authors() {
        let document =
            parse_document("= Document Title\n:authors: Doc Writer; Other Author\n\ncontent");

        let prepared = prepare_document(&document);

        assert_eq!(
            prepared.authors,
            vec![
                crate::prepare::Author {
                    name: Some("Doc Writer".into()),
                    email: None,
                },
                crate::prepare::Author {
                    name: Some("Other Author".into()),
                    email: None,
                },
            ]
        );
        assert_eq!(
            prepared
                .attributes
                .get("authorinitials")
                .map(String::as_str),
            Some("DW")
        );
        assert_eq!(
            prepared
                .attributes
                .get("authorinitials_2")
                .map(String::as_str),
            Some("OA")
        );
    }

    #[test]
    fn carries_email_attribute_into_prepared_authors() {
        let document = Document {
            attributes: [("email".to_owned(), "jane@example.com".to_owned())]
                .into_iter()
                .collect(),
            title: Some(Heading {
                level: 0,
                title: "Document Title".into(),
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
            }),
            blocks: Vec::new(),
        };

        let prepared = prepare_document(&document);

        assert_eq!(
            prepared.authors,
            vec![crate::prepare::Author {
                name: None,
                email: Some("jane@example.com".into()),
            }]
        );
    }

    #[test]
    fn merges_author_and_email_attributes_into_single_author() {
        let document = Document {
            attributes: [
                ("author".to_owned(), "Jane Doe".to_owned()),
                ("email".to_owned(), "jane@example.com".to_owned()),
            ]
            .into_iter()
            .collect(),
            title: Some(Heading {
                level: 0,
                title: "Document Title".into(),
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
            }),
            blocks: Vec::new(),
        };

        let prepared = prepare_document(&document);

        assert_eq!(
            prepared.authors,
            vec![crate::prepare::Author {
                name: Some("Jane Doe".into()),
                email: Some("jane@example.com".into()),
            }]
        );
    }

    #[test]
    fn serializes_document_attributes_at_top_level() {
        let document = Document {
            attributes: [
                ("source-highlighter".to_owned(), "rouge".to_owned()),
                ("toc".to_owned(), "left".to_owned()),
            ]
            .into_iter()
            .collect(),
            title: Some(Heading {
                level: 0,
                title: "Document Title".into(),
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
            }),
            blocks: Vec::new(),
        };

        let prepared = prepare_document(&document);
        let json = prepared_document_to_json(&prepared).expect("json serialization");

        assert!(json.contains("\"attributes\": {"));
        assert!(json.contains("\"toc\": \"left\""));
        assert!(json.contains("\"source-highlighter\": \"rouge\""));
    }

    #[test]
    fn serializes_author_attribute_into_authors_metadata() {
        let document = Document {
            attributes: [("author".to_owned(), "Jane Doe".to_owned())]
                .into_iter()
                .collect(),
            title: Some(Heading {
                level: 0,
                title: "Document Title".into(),
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
            }),
            blocks: Vec::new(),
        };

        let prepared = prepare_document(&document);
        let json = prepared_document_to_json(&prepared).expect("json serialization");

        assert!(json.contains("\"author\": \"Jane Doe\""));
        assert!(json.contains("\"authors\": ["));
        assert!(json.contains("\"name\": \"Jane Doe\""));
    }

    #[test]
    fn serializes_email_attribute_into_authors_metadata() {
        let document = Document {
            attributes: [
                ("author".to_owned(), "Jane Doe".to_owned()),
                ("email".to_owned(), "jane@example.com".to_owned()),
            ]
            .into_iter()
            .collect(),
            title: Some(Heading {
                level: 0,
                title: "Document Title".into(),
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
            }),
            blocks: Vec::new(),
        };

        let prepared = prepare_document(&document);
        let json = prepared_document_to_json(&prepared).expect("json serialization");

        assert!(json.contains("\"email\": \"jane@example.com\""));
        assert!(json.contains("\"name\": \"Jane Doe\""));
    }

    #[test]
    fn serializes_revision_attributes_into_revision_metadata() {
        let document = Document {
            attributes: [
                ("revnumber".to_owned(), "1.2".to_owned()),
                ("revdate".to_owned(), "2026-03-31".to_owned()),
                ("revremark".to_owned(), "Draft".to_owned()),
            ]
            .into_iter()
            .collect(),
            title: Some(Heading {
                level: 0,
                title: "Document Title".into(),
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
            }),
            blocks: Vec::new(),
        };

        let prepared = prepare_document(&document);
        let json = prepared_document_to_json(&prepared).expect("json serialization");

        assert!(json.contains("\"revision\": {"));
        assert!(json.contains("\"number\": \"1.2\""));
        assert!(json.contains("\"date\": \"2026-03-31\""));
        assert!(json.contains("\"remark\": \"Draft\""));
    }

    #[test]
    fn serializes_implicit_header_metadata_into_json() {
        let document = parse_document(
            "= Document Title\nStuart Rackham <founder@asciidoc.org>\nv8.6.8, 2012-07-12: See changelog.\n\ncontent",
        );

        let prepared = prepare_document(&document);
        let json = prepared_document_to_json(&prepared).expect("json serialization");

        assert!(json.contains("\"author\": \"Stuart Rackham\""));
        assert!(json.contains("\"email\": \"founder@asciidoc.org\""));
        assert!(json.contains("\"number\": \"8.6.8\""));
        assert!(json.contains("\"date\": \"2012-07-12\""));
        assert!(json.contains("\"remark\": \"See changelog.\""));
    }

    #[test]
    fn serializes_multiple_implicit_authors_into_json() {
        let document = parse_document(
            "= Document Title\nDoc Writer <thedoctor@asciidoc.org>; Junior Writer <junior@asciidoctor.org>\n\ncontent",
        );

        let prepared = prepare_document(&document);
        let json = prepared_document_to_json(&prepared).expect("json serialization");

        assert!(json.contains("\"authors\": ["));
        assert!(json.contains("\"name\": \"Doc Writer\""));
        assert!(json.contains("\"email\": \"thedoctor@asciidoc.org\""));
        assert!(json.contains("\"name\": \"Junior Writer\""));
        assert!(json.contains("\"email\": \"junior@asciidoctor.org\""));
    }

    #[test]
    fn prepares_paragraph_content_as_simple_blocks() {
        let document = Document {
            attributes: Default::default(),
            title: None,
            blocks: vec![Block::Paragraph(Paragraph {
                inlines: vec![Inline::Text("first line\nsecond line".into())],
                lines: vec!["first line".into(), "second line".into()],
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
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
        assert_eq!(paragraph.inlines.len(), 1);
        assert_eq!(paragraph.content_model, Some(ContentModel::Simple));
    }

    #[test]
    fn serializes_with_react_asciidoc_style_field_names() {
        let document = Document {
            attributes: Default::default(),
            title: Some(Heading {
                level: 0,
                title: "Document Title".into(),
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
            }),
            blocks: vec![Block::Paragraph(Paragraph {
                inlines: vec![Inline::Text("hello".into())],
                lines: vec!["hello".into()],
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
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

    #[test]
    fn prepares_inline_spans_for_wasm_facing_output() {
        let document = Document {
            attributes: Default::default(),
            title: None,
            blocks: vec![Block::Paragraph(Paragraph {
                lines: vec!["before *strong* after".into()],
                inlines: vec![
                    Inline::Text("before ".into()),
                    Inline::Span(InlineSpan {
                        variant: InlineVariant::Strong,
                        form: InlineForm::Constrained,
                        inlines: vec![Inline::Text("strong".into())],
                    }),
                    Inline::Text(" after".into()),
                ],
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
            })],
        };

        let prepared = prepare_document(&document);
        let PreparedBlock::Preamble(preamble) = &prepared.blocks[0] else {
            panic!("expected preamble");
        };
        let PreparedBlock::Paragraph(paragraph) = &preamble.blocks[0] else {
            panic!("expected paragraph");
        };

        assert_eq!(paragraph.inlines.len(), 3);
    }

    #[test]
    fn prepares_links_for_wasm_facing_output() {
        let document = Document {
            attributes: Default::default(),
            title: None,
            blocks: vec![Block::Paragraph(Paragraph {
                lines: vec!["See https://example.org[example]".into()],
                inlines: vec![
                    Inline::Text("See ".into()),
                    Inline::Link(InlineLink {
                        target: "https://example.org".into(),
                        text: vec![Inline::Text("example".into())],
                        bare: false,
                        window: None,
                    }),
                ],
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
            })],
        };

        let prepared = prepare_document(&document);
        let PreparedBlock::Preamble(preamble) = &prepared.blocks[0] else {
            panic!("expected preamble");
        };
        let PreparedBlock::Paragraph(paragraph) = &preamble.blocks[0] else {
            panic!("expected paragraph");
        };

        assert_eq!(paragraph.inlines.len(), 2);
        let PreparedInline::Link(link) = &paragraph.inlines[1] else {
            panic!("expected link inline");
        };
        assert_eq!(link.target, "https://example.org");
        assert_eq!(link.window, None);
    }

    #[test]
    fn prepares_xrefs_for_wasm_facing_output() {
        let document = Document {
            attributes: Default::default(),
            title: None,
            blocks: vec![Block::Paragraph(Paragraph {
                lines: vec!["See <<install,Installation>>".into()],
                inlines: vec![
                    Inline::Text("See ".into()),
                    Inline::Xref(InlineXref {
                        target: "install".into(),
                        text: vec![Inline::Text("Installation".into())],
                        shorthand: true,
                        explicit_text: true,
                    }),
                ],
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
            })],
        };

        let prepared = prepare_document(&document);
        let PreparedBlock::Preamble(preamble) = &prepared.blocks[0] else {
            panic!("expected preamble");
        };
        let PreparedBlock::Paragraph(paragraph) = &preamble.blocks[0] else {
            panic!("expected paragraph");
        };

        let PreparedInline::Xref(xref) = &paragraph.inlines[1] else {
            panic!("expected xref inline");
        };
        assert_eq!(xref.target, "install");
        assert_eq!(xref.href, "#install");
        assert!(xref.shorthand);
    }

    #[test]
    fn resolves_xrefs_to_prepared_section_ids_and_titles() {
        let document = Document {
            attributes: Default::default(),
            title: Some(Heading {
                level: 0,
                title: "Sample Document".into(),
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
            }),
            blocks: vec![
                Block::Paragraph(Paragraph {
                    lines: vec!["See <<First Section>>.".into()],
                    inlines: vec![
                        Inline::Text("See ".into()),
                        Inline::Xref(InlineXref {
                            target: "First Section".into(),
                            text: vec![Inline::Text("First Section".into())],
                            shorthand: true,
                            explicit_text: false,
                        }),
                        Inline::Text(".".into()),
                    ],
                    id: None,
                    reftext: None,
                    metadata: BlockMetadata::default(),
                }),
                Block::Heading(Heading {
                    level: 1,
                    title: "First Section".into(),
                    id: None,
                    reftext: None,
                    metadata: BlockMetadata::default(),
                }),
            ],
        };

        let prepared = prepare_document(&document);
        let PreparedBlock::Preamble(preamble) = &prepared.blocks[0] else {
            panic!("expected preamble");
        };
        let PreparedBlock::Paragraph(paragraph) = &preamble.blocks[0] else {
            panic!("expected paragraph");
        };
        let PreparedInline::Xref(xref) = &paragraph.inlines[1] else {
            panic!("expected xref");
        };

        assert_eq!(xref.href, "#_first_section");
        assert_eq!(
            xref.inlines,
            vec![PreparedInline::Text(TextInline {
                value: "First Section".into(),
            })]
        );
    }

    #[test]
    fn uses_explicit_section_anchor_for_id_and_xref_resolution() {
        let document = Document {
            attributes: Default::default(),
            title: None,
            blocks: vec![
                Block::Paragraph(Paragraph {
                    lines: vec!["See <<install>>.".into()],
                    inlines: vec![
                        Inline::Text("See ".into()),
                        Inline::Xref(InlineXref {
                            target: "install".into(),
                            text: vec![Inline::Text("install".into())],
                            shorthand: true,
                            explicit_text: false,
                        }),
                        Inline::Text(".".into()),
                    ],
                    id: None,
                    reftext: None,
                    metadata: BlockMetadata::default(),
                }),
                Block::Heading(Heading {
                    level: 1,
                    title: "First Section".into(),
                    id: Some("install".into()),
                    reftext: Some("Installation".into()),
                    metadata: BlockMetadata::default(),
                }),
            ],
        };

        let prepared = prepare_document(&document);
        let PreparedBlock::Preamble(preamble) = &prepared.blocks[0] else {
            panic!("expected preamble");
        };
        let PreparedBlock::Paragraph(paragraph) = &preamble.blocks[0] else {
            panic!("expected paragraph");
        };
        let PreparedInline::Xref(xref) = &paragraph.inlines[1] else {
            panic!("expected xref");
        };
        let PreparedBlock::Section(section) = &prepared.blocks[1] else {
            panic!("expected section");
        };

        assert_eq!(section.id, "install");
        assert_eq!(xref.href, "#install");
        assert_eq!(
            xref.inlines,
            vec![PreparedInline::Text(TextInline {
                value: "Installation".into(),
            })]
        );
    }

    #[test]
    fn resolves_xrefs_to_inline_anchor_targets() {
        let document = Document {
            attributes: Default::default(),
            title: None,
            blocks: vec![Block::Paragraph(Paragraph {
                lines: vec!["See <<bookmark-a>> and [[bookmark-a,Marked Spot]]look here".into()],
                inlines: vec![
                    Inline::Text("See ".into()),
                    Inline::Xref(InlineXref {
                        target: "bookmark-a".into(),
                        text: vec![Inline::Text("bookmark-a".into())],
                        shorthand: true,
                        explicit_text: false,
                    }),
                    Inline::Text(" and ".into()),
                    Inline::Anchor(crate::ast::InlineAnchor {
                        id: "bookmark-a".into(),
                        reftext: Some("Marked Spot".into()),
                        inlines: Vec::new(),
                    }),
                    Inline::Text("look here".into()),
                ],
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
            })],
        };

        let prepared = prepare_document(&document);
        let PreparedBlock::Preamble(preamble) = &prepared.blocks[0] else {
            panic!("expected preamble");
        };
        let PreparedBlock::Paragraph(paragraph) = &preamble.blocks[0] else {
            panic!("expected paragraph");
        };
        let PreparedInline::Xref(xref) = &paragraph.inlines[1] else {
            panic!("expected xref");
        };

        assert_eq!(xref.href, "#bookmark-a");
        assert_eq!(
            xref.inlines,
            vec![PreparedInline::Text(TextInline {
                value: "Marked Spot".into(),
            })]
        );
    }

    #[test]
    fn preserves_phrase_anchor_text_and_uses_it_for_xrefs() {
        let document = Document {
            attributes: Default::default(),
            title: None,
            blocks: vec![Block::Paragraph(Paragraph {
                lines: vec!["See <<bookmark-b>> and [#bookmark-b]#visible text#".into()],
                inlines: vec![
                    Inline::Text("See ".into()),
                    Inline::Xref(InlineXref {
                        target: "bookmark-b".into(),
                        text: vec![Inline::Text("bookmark-b".into())],
                        shorthand: true,
                        explicit_text: false,
                    }),
                    Inline::Text(" and ".into()),
                    Inline::Anchor(crate::ast::InlineAnchor {
                        id: "bookmark-b".into(),
                        reftext: None,
                        inlines: vec![Inline::Text("visible text".into())],
                    }),
                ],
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
            })],
        };

        let prepared = prepare_document(&document);
        let PreparedBlock::Preamble(preamble) = &prepared.blocks[0] else {
            panic!("expected preamble");
        };
        let PreparedBlock::Paragraph(paragraph) = &preamble.blocks[0] else {
            panic!("expected paragraph");
        };
        let PreparedInline::Xref(xref) = &paragraph.inlines[1] else {
            panic!("expected xref");
        };
        let PreparedInline::Anchor(anchor) = &paragraph.inlines[3] else {
            panic!("expected anchor");
        };

        assert_eq!(xref.href, "#bookmark-b");
        assert_eq!(
            xref.inlines,
            vec![PreparedInline::Text(TextInline {
                value: "visible text".into(),
            })]
        );
        assert_eq!(
            anchor.inlines,
            vec![PreparedInline::Text(TextInline {
                value: "visible text".into(),
            })]
        );
    }

    #[test]
    fn sets_has_header_true_when_title_present() {
        let document = Document {
            attributes: Default::default(),
            title: Some(Heading {
                level: 0,
                title: "My Title".into(),
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
            }),
            blocks: vec![],
        };

        let prepared = prepare_document(&document);

        assert_eq!(prepared.title, "My Title");
        assert!(prepared.has_header);
        assert!(!prepared.no_header);
    }

    #[test]
    fn sets_no_header_true_when_no_title() {
        let document = Document {
            attributes: Default::default(),
            title: None,
            blocks: vec![],
        };

        let prepared = prepare_document(&document);

        assert_eq!(prepared.title, "");
        assert!(!prepared.has_header);
        assert!(prepared.no_header);
    }

    #[test]
    fn does_not_create_preamble_when_no_content_precedes_first_section() {
        let document = Document {
            attributes: Default::default(),
            title: Some(Heading {
                level: 0,
                title: "Doc".into(),
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
            }),
            blocks: vec![
                Block::Heading(Heading {
                    level: 1,
                    title: "First Section".into(),
                    id: None,
                    reftext: None,
                    metadata: BlockMetadata::default(),
                }),
                Block::Paragraph(Paragraph {
                    inlines: vec![Inline::Text("Section body.".into())],
                    lines: vec!["Section body.".into()],
                    id: None,
                    reftext: None,
                    metadata: BlockMetadata::default(),
                }),
            ],
        };

        let prepared = prepare_document(&document);

        assert_eq!(prepared.blocks.len(), 1);
        let PreparedBlock::Section(section) = &prepared.blocks[0] else {
            panic!("expected section, not preamble");
        };
        assert_eq!(section.title, "First Section");
    }

    #[test]
    fn wraps_multiple_blocks_before_first_section_in_preamble() {
        let document = Document {
            attributes: Default::default(),
            title: Some(Heading {
                level: 0,
                title: "Doc".into(),
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
            }),
            blocks: vec![
                Block::Paragraph(Paragraph {
                    inlines: vec![Inline::Text("First preamble paragraph.".into())],
                    lines: vec!["First preamble paragraph.".into()],
                    id: None,
                    reftext: None,
                    metadata: BlockMetadata::default(),
                }),
                Block::Paragraph(Paragraph {
                    inlines: vec![Inline::Text("Second preamble paragraph.".into())],
                    lines: vec!["Second preamble paragraph.".into()],
                    id: None,
                    reftext: None,
                    metadata: BlockMetadata::default(),
                }),
                Block::Heading(Heading {
                    level: 1,
                    title: "Section One".into(),
                    id: None,
                    reftext: None,
                    metadata: BlockMetadata::default(),
                }),
            ],
        };

        let prepared = prepare_document(&document);

        let PreparedBlock::Preamble(preamble) = &prepared.blocks[0] else {
            panic!("expected preamble as first block");
        };
        assert_eq!(preamble.blocks.len(), 2);
        let PreparedBlock::Paragraph(p1) = &preamble.blocks[0] else {
            panic!("expected paragraph");
        };
        assert_eq!(p1.content, "First preamble paragraph.");
        let PreparedBlock::Paragraph(p2) = &preamble.blocks[1] else {
            panic!("expected paragraph");
        };
        assert_eq!(p2.content, "Second preamble paragraph.");
    }

    #[test]
    fn prepares_unordered_lists() {
        let document = Document {
            attributes: Default::default(),
            title: None,
            blocks: vec![Block::UnorderedList(UnorderedList {
                items: vec![ListItem {
                    blocks: vec![Block::Paragraph(Paragraph {
                        inlines: vec![Inline::Text("first item".into())],
                        lines: vec!["first item".into()],
                        id: None,
                        reftext: None,
                        metadata: BlockMetadata::default(),
                    })],
                }],
                reftext: None,
                metadata: BlockMetadata::default(),
            })],
        };

        let prepared = prepare_document(&document);
        let PreparedBlock::Preamble(preamble) = &prepared.blocks[0] else {
            panic!("expected preamble");
        };
        let PreparedBlock::UnorderedList(list) = &preamble.blocks[0] else {
            panic!("expected unordered list");
        };

        assert_eq!(list.name, "ulist");
        assert_eq!(list.items.len(), 1);
    }

    #[test]
    fn resolves_xrefs_to_anchored_lists() {
        let document = parse_document("[[steps]]\n* one\n\nSee <<steps>>.");
        let prepared = prepare_document(&document);

        let PreparedBlock::Preamble(preamble) = &prepared.blocks[0] else {
            panic!("expected preamble");
        };
        let PreparedBlock::Paragraph(paragraph) = &preamble.blocks[1] else {
            panic!("expected paragraph");
        };
        let PreparedInline::Xref(xref) = &paragraph.inlines[1] else {
            panic!("expected xref");
        };
        assert_eq!(xref.href, "#steps");
    }

    #[test]
    fn resolves_xrefs_to_list_anchor_reftext() {
        let document = parse_document("[[steps,Setup Steps]]\n* one\n\nSee <<steps>>.");
        let prepared = prepare_document(&document);

        let PreparedBlock::Preamble(preamble) = &prepared.blocks[0] else {
            panic!("expected preamble");
        };
        let PreparedBlock::UnorderedList(list) = &preamble.blocks[0] else {
            panic!("expected list");
        };
        assert_eq!(list.reftext.as_deref(), Some("Setup Steps"));

        let PreparedBlock::Paragraph(paragraph) = &preamble.blocks[1] else {
            panic!("expected paragraph");
        };
        let PreparedInline::Xref(xref) = &paragraph.inlines[1] else {
            panic!("expected xref");
        };
        assert_eq!(xref.href, "#steps");
        assert_eq!(
            xref.inlines,
            vec![PreparedInline::Text(TextInline {
                value: "Setup Steps".into(),
            })]
        );
    }

    #[test]
    fn prepares_anchored_delimited_blocks() {
        let document = parse_document(
            "[[code-sample]]\n----\nputs 'hello'\n----\n\n[[aside]]\n****\ninside sidebar\n****\n\n[[walkthrough]]\n====\ninside example\n====",
        );
        let prepared = prepare_document(&document);

        let PreparedBlock::Preamble(preamble) = &prepared.blocks[0] else {
            panic!("expected preamble");
        };
        let PreparedBlock::Listing(listing) = &preamble.blocks[0] else {
            panic!("expected listing");
        };
        assert_eq!(listing.id.as_deref(), Some("code-sample"));
        assert_eq!(listing.reftext, None);

        let PreparedBlock::Sidebar(sidebar) = &preamble.blocks[1] else {
            panic!("expected sidebar");
        };
        assert_eq!(sidebar.id.as_deref(), Some("aside"));
        assert_eq!(sidebar.reftext, None);

        let PreparedBlock::Example(example) = &preamble.blocks[2] else {
            panic!("expected example");
        };
        assert_eq!(example.id.as_deref(), Some("walkthrough"));
        assert_eq!(example.reftext, None);
    }

    #[test]
    fn resolves_xrefs_to_delimited_block_anchor_reftext() {
        let document = parse_document(
            "[[code-sample,Code Sample]]\n----\nputs 'hello'\n----\n\nSee <<code-sample>>.",
        );
        let prepared = prepare_document(&document);

        let PreparedBlock::Preamble(preamble) = &prepared.blocks[0] else {
            panic!("expected preamble");
        };
        let PreparedBlock::Listing(listing) = &preamble.blocks[0] else {
            panic!("expected listing");
        };
        assert_eq!(listing.reftext.as_deref(), Some("Code Sample"));

        let PreparedBlock::Paragraph(paragraph) = &preamble.blocks[1] else {
            panic!("expected paragraph");
        };
        let PreparedInline::Xref(xref) = &paragraph.inlines[1] else {
            panic!("expected xref");
        };
        assert_eq!(xref.href, "#code-sample");
        assert_eq!(
            xref.inlines,
            vec![PreparedInline::Text(TextInline {
                value: "Code Sample".into(),
            })]
        );
    }

    #[test]
    fn prepares_listing_and_compound_delimited_blocks() {
        let document = Document {
            attributes: Default::default(),
            title: None,
            blocks: vec![
                Block::Listing(Listing {
                    lines: vec!["puts 'hello'".into()],
                    callouts: vec![],
                    reftext: None,
                    metadata: Default::default(),
                }),
                Block::Sidebar(AstCompoundBlock {
                    blocks: vec![Block::Paragraph(Paragraph {
                        inlines: vec![Inline::Text("inside sidebar".into())],
                        lines: vec!["inside sidebar".into()],
                        id: None,
                        reftext: None,
                        metadata: BlockMetadata::default(),
                    })],
                    reftext: None,
                    metadata: Default::default(),
                }),
                Block::Example(AstCompoundBlock {
                    blocks: vec![Block::Paragraph(Paragraph {
                        inlines: vec![Inline::Text("inside example".into())],
                        lines: vec!["inside example".into()],
                        id: None,
                        reftext: None,
                        metadata: BlockMetadata::default(),
                    })],
                    reftext: None,
                    metadata: Default::default(),
                }),
            ],
        };

        let prepared = prepare_document(&document);
        let PreparedBlock::Preamble(preamble) = &prepared.blocks[0] else {
            panic!("expected preamble");
        };

        let PreparedBlock::Listing(listing) = &preamble.blocks[0] else {
            panic!("expected listing");
        };
        assert_eq!(listing.content, "puts 'hello'");

        let PreparedBlock::Sidebar(sidebar) = &preamble.blocks[1] else {
            panic!("expected sidebar");
        };
        assert_eq!(sidebar.blocks.len(), 1);

        let PreparedBlock::Example(example) = &preamble.blocks[2] else {
            panic!("expected example");
        };
        assert_eq!(example.blocks.len(), 1);
    }

    #[test]
    fn prepares_tables() {
        let document = parse_document(
            ".Agents\n[%header,cols=\"30%,\"]\n|===\n|Name|Email\n|Peter|peter@example.com\n|Adam|adam@example.com\n|===",
        );
        let prepared = prepare_document(&document);
        let PreparedBlock::Preamble(preamble) = &prepared.blocks[0] else {
            panic!("expected preamble");
        };
        let PreparedBlock::Table(table) = &preamble.blocks[0] else {
            panic!("expected table");
        };

        assert_eq!(table.title.as_deref(), Some("Agents"));
        assert_eq!(table.header.as_ref().map(|row| row.cells.len()), Some(2));
        assert_eq!(
            table
                .header
                .as_ref()
                .map(|row| row.cells[0].content.as_str()),
            Some("Name")
        );
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[1].cells[0].content, "Adam");
    }

    #[test]
    fn prepares_tables_with_stacked_cells() {
        let document = parse_document(
            ".Agents\n[%header,cols=\"30%,70%\"]\n|===\n|Name\n|Email\n|Peter\n|peter@example.com\n|Adam\n|adam@example.com\n|===",
        );
        let prepared = prepare_document(&document);
        let PreparedBlock::Preamble(preamble) = &prepared.blocks[0] else {
            panic!("expected preamble");
        };
        let PreparedBlock::Table(table) = &preamble.blocks[0] else {
            panic!("expected table");
        };

        assert_eq!(table.header.as_ref().map(|row| row.cells.len()), Some(2));
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[0].cells[0].content, "Peter");
        assert_eq!(table.rows[0].cells[1].content, "peter@example.com");
    }

    #[test]
    fn prepares_tables_with_stacked_cells_without_cols() {
        let document = parse_document(
            ".Agents\n[%header]\n|===\n|Name\n|Email\n\n|Peter\n|peter@example.com\n\n|Adam\n|adam@example.com\n|===",
        );
        let prepared = prepare_document(&document);
        let PreparedBlock::Preamble(preamble) = &prepared.blocks[0] else {
            panic!("expected preamble");
        };
        let PreparedBlock::Table(table) = &preamble.blocks[0] else {
            panic!("expected table");
        };

        assert_eq!(table.header.as_ref().map(|row| row.cells.len()), Some(2));
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[1].cells[0].content, "Adam");
        assert_eq!(table.rows[1].cells[1].content, "adam@example.com");
    }

    #[test]
    fn prepares_block_content_inside_table_cells() {
        let document = parse_document(
            ".Services\n[%header,cols=\"1,3\"]\n|===\n|Name\n|Details\n|API\n|First paragraph.\n\n* fast\n* typed\n|===",
        );
        let prepared = prepare_document(&document);
        let PreparedBlock::Preamble(preamble) = &prepared.blocks[0] else {
            panic!("expected preamble");
        };
        let PreparedBlock::Table(table) = &preamble.blocks[0] else {
            panic!("expected table");
        };
        let detail_cell = &table.rows[0].cells[1];

        assert_eq!(detail_cell.blocks.len(), 2);
        let PreparedBlock::Paragraph(paragraph) = &detail_cell.blocks[0] else {
            panic!("expected paragraph block");
        };
        assert_eq!(paragraph.content, "First paragraph.");
        let PreparedBlock::UnorderedList(list) = &detail_cell.blocks[1] else {
            panic!("expected list block");
        };
        assert_eq!(list.items.len(), 2);
    }

    #[test]
    fn prepares_table_cell_specs_for_rowspan_and_asciidoc_style() {
        let document = parse_document(
            "[%header,cols=\"1,2\"]\n|===\nh|Area\n|Description\n\n.2+|Shared\na|First paragraph.\n+\nSecond paragraph.\n\n|Another description\n|===",
        );
        let prepared = prepare_document(&document);
        let PreparedBlock::Preamble(preamble) = &prepared.blocks[0] else {
            panic!("expected preamble");
        };
        let PreparedBlock::Table(table) = &preamble.blocks[0] else {
            panic!("expected table");
        };

        assert_eq!(
            table
                .header
                .as_ref()
                .and_then(|row| row.cells[0].style.as_deref()),
            Some("header")
        );
        assert_eq!(table.rows[0].cells[0].rowspan, 2);
        assert_eq!(table.rows[0].cells[1].style.as_deref(), Some("asciidoc"));
        assert_eq!(table.rows[0].cells[1].blocks.len(), 2);
    }

    #[test]
    fn prepares_delimited_block_metadata() {
        let document = parse_document(".Exhibit A\n[source,rust]\n----\nfn main() {}\n----");
        let prepared = prepare_document(&document);
        let PreparedBlock::Preamble(preamble) = &prepared.blocks[0] else {
            panic!("expected preamble");
        };
        let PreparedBlock::Listing(listing) = &preamble.blocks[0] else {
            panic!("expected listing");
        };

        assert_eq!(listing.title.as_deref(), Some("Exhibit A"));
        assert_eq!(listing.style.as_deref(), Some("source"));
        assert_eq!(
            listing.attributes.get("language").map(String::as_str),
            Some("rust")
        );
        assert_eq!(
            listing.attributes.get("title").map(String::as_str),
            Some("Exhibit A")
        );
    }

    #[test]
    fn trims_outer_blank_lines_in_prepared_delimited_content() {
        let document = parse_document(
            "----\n\ncode\n\n----\n\n....\n\nliteral\n\n....\n\n[verse]\n____\n\nline\n\n____\n\n++++\n\n<span>ok</span>\n\n++++",
        );
        let prepared = prepare_document(&document);
        let PreparedBlock::Preamble(preamble) = &prepared.blocks[0] else {
            panic!("expected preamble");
        };

        let PreparedBlock::Listing(listing) = &preamble.blocks[0] else {
            panic!("expected listing");
        };
        assert_eq!(listing.content, "code");

        let PreparedBlock::Literal(literal) = &preamble.blocks[1] else {
            panic!("expected literal");
        };
        assert_eq!(literal.content, "literal");

        let PreparedBlock::Quote(verse) = &preamble.blocks[2] else {
            panic!("expected verse");
        };
        assert!(verse.is_verse);
        assert_eq!(verse.content, "line");

        let PreparedBlock::Passthrough(passthrough) = &preamble.blocks[3] else {
            panic!("expected passthrough");
        };
        assert_eq!(passthrough.content, "<span>ok</span>");
    }

    #[test]
    fn prepares_admonition_paragraphs() {
        let document = parse_document("NOTE: This is just a note.");
        let prepared = prepare_document(&document);
        let PreparedBlock::Preamble(preamble) = &prepared.blocks[0] else {
            panic!("expected preamble");
        };
        let PreparedBlock::Admonition(admonition) = &preamble.blocks[0] else {
            panic!("expected admonition");
        };

        assert_eq!(admonition.variant, "note");
        let PreparedBlock::Paragraph(paragraph) = &admonition.blocks[0] else {
            panic!("expected paragraph");
        };
        assert_eq!(paragraph.content, "This is just a note.");
    }

    #[test]
    fn prepares_styled_block_admonitions() {
        let document = parse_document("[TIP]\n====\nRemember the milk.\n====");
        let prepared = prepare_document(&document);
        let PreparedBlock::Preamble(preamble) = &prepared.blocks[0] else {
            panic!("expected preamble");
        };
        let PreparedBlock::Admonition(admonition) = &preamble.blocks[0] else {
            panic!("expected admonition");
        };

        assert_eq!(admonition.variant, "tip");
        assert_eq!(admonition.style.as_deref(), Some("TIP"));
        let PreparedBlock::Paragraph(paragraph) = &admonition.blocks[0] else {
            panic!("expected paragraph");
        };
        assert_eq!(paragraph.content, "Remember the milk.");
    }

    #[test]
    fn prepares_anchored_admonitions_with_reftext() {
        let document = parse_document("[[install-note,Install Note]]\nNOTE: Read this first.");
        let prepared = prepare_document(&document);

        let PreparedBlock::Preamble(preamble) = &prepared.blocks[0] else {
            panic!("expected preamble");
        };
        let PreparedBlock::Admonition(admonition) = &preamble.blocks[0] else {
            panic!("expected admonition");
        };
        assert_eq!(admonition.id.as_deref(), Some("install-note"));
        assert_eq!(admonition.reftext.as_deref(), Some("Install Note"));
    }

    #[test]
    fn resolves_xrefs_to_admonition_anchor_reftext() {
        let document = parse_document(
            "[[install-note,Install Note]]\nNOTE: Read this first.\n\nSee <<install-note>>.",
        );
        let prepared = prepare_document(&document);

        let PreparedBlock::Preamble(preamble) = &prepared.blocks[0] else {
            panic!("expected preamble");
        };
        let PreparedBlock::Paragraph(paragraph) = &preamble.blocks[1] else {
            panic!("expected paragraph");
        };
        let PreparedInline::Xref(xref) = &paragraph.inlines[1] else {
            panic!("expected xref");
        };
        assert_eq!(xref.href, "#install-note");
        assert_eq!(
            xref.inlines,
            vec![PreparedInline::Text(TextInline {
                value: "Install Note".into(),
            })]
        );
    }

    #[test]
    fn collects_and_numbers_footnotes() {
        let document = Document {
            attributes: Default::default(),
            title: None,
            blocks: vec![Block::Paragraph(Paragraph {
                lines: vec!["A notefootnote:[Read *this* first.] here.".into()],
                inlines: vec![
                    Inline::Text("A note".into()),
                    Inline::Footnote(InlineFootnote {
                        inlines: vec![
                            Inline::Text("Read ".into()),
                            Inline::Span(InlineSpan {
                                variant: InlineVariant::Strong,
                                form: InlineForm::Constrained,
                                inlines: vec![Inline::Text("this".into())],
                            }),
                            Inline::Text(" first.".into()),
                        ],
                    }),
                    Inline::Text(" here.".into()),
                ],
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
            })],
        };

        let prepared = prepare_document(&document);

        assert_eq!(prepared.footnotes.len(), 1);
        assert_eq!(prepared.footnotes[0].index, Some(1));
        assert_eq!(
            prepared.footnotes[0].text.as_deref(),
            Some("Read this first.")
        );

        let PreparedBlock::Preamble(preamble) = &prepared.blocks[0] else {
            panic!("expected preamble");
        };
        let PreparedBlock::Paragraph(paragraph) = &preamble.blocks[0] else {
            panic!("expected paragraph");
        };
        let PreparedInline::Footnote(footnote) = &paragraph.inlines[1] else {
            panic!("expected footnote");
        };
        assert_eq!(footnote.index, Some(1));
        assert_eq!(paragraph.content, "A note[1] here.");
    }
}
