use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ast::{Block, Document, Inline, InlineForm, InlineVariant, OrderedList, Paragraph, UnorderedList};

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
    UnorderedList(ListBlock),
    OrderedList(ListBlock),
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
pub struct ListBlock {
    pub items: Vec<ListItemBlock>,
    pub attributes: BTreeMap<String, String>,
    pub content_model: Option<ContentModel>,
    pub level: u8,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListItemBlock {
    pub blocks: Vec<PreparedBlock>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PreparedInline {
    Text(TextInline),
    Span(SpanInline),
    Link(LinkInline),
    Xref(XrefInline),
    Anchor(AnchorInline),
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
    let mut blocks = prepare_blocks(&document.blocks, true, &mut next_section_ids);
    let section_refs = collect_section_refs(&blocks);
    let block_refs = collect_block_refs(&blocks);
    resolve_xrefs_in_blocks(&mut blocks, &section_refs, &block_refs);
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
        blocks,
        content_model: Some(ContentModel::Compound),
        footnotes: Vec::new(),
        sections,
        authors: prepare_authors(document),
    }
}

fn prepare_authors(document: &Document) -> Vec<Author> {
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

    vec![Author {
        name,
        email,
    }]
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
        id: paragraph.id.clone(),
        reftext: paragraph.reftext.clone(),
        blocks: Vec::new(),
        content: paragraph.plain_text(),
        inlines: prepare_inlines(&paragraph.inlines),
        attributes: BTreeMap::new(),
        content_model: Some(ContentModel::Simple),
        line_number: None,
        style: None,
        role: None,
        level: 0,
        title: None,
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
        attributes: BTreeMap::new(),
        content_model: Some(ContentModel::Compound),
        level: 0,
        name: "ulist".into(),
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
        attributes: BTreeMap::new(),
        content_model: Some(ContentModel::Compound),
        level: 0,
        name: "olist".into(),
    }
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
        })
        .collect()
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
            PreparedBlock::Preamble(_)
            | PreparedBlock::Paragraph(_)
            | PreparedBlock::UnorderedList(_)
            | PreparedBlock::OrderedList(_) => None,
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
            PreparedBlock::UnorderedList(list) | PreparedBlock::OrderedList(list) => {
                for item in &list.items {
                    collect_block_refs_into(&item.blocks, refs);
                }
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
            PreparedInline::Text(_) => {}
        }
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
            PreparedBlock::UnorderedList(list) | PreparedBlock::OrderedList(list) => {
                for item in &mut list.items {
                    resolve_xrefs_in_blocks(&mut item.blocks, section_refs, block_refs);
                }
            }
            PreparedBlock::Section(section) => {
                resolve_xrefs_in_blocks(&mut section.blocks, section_refs, block_refs)
            }
        }
    }
}

fn resolve_xrefs_in_inlines(
    inlines: &mut [PreparedInline],
    section_refs: &BTreeMap<String, SectionRef>,
    block_refs: &BTreeMap<String, BlockRef>,
) {
    for inline in inlines {
        match inline {
            PreparedInline::Text(_) | PreparedInline::Link(_) => {}
            PreparedInline::Anchor(anchor) => {
                resolve_xrefs_in_inlines(&mut anchor.inlines, section_refs, block_refs)
            }
            PreparedInline::Span(span) => {
                resolve_xrefs_in_inlines(&mut span.inlines, section_refs, block_refs)
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
    }
}

fn inline_variant_name(variant: InlineVariant) -> &'static str {
    match variant {
        InlineVariant::Strong => "strong",
        InlineVariant::Emphasis => "emphasis",
        InlineVariant::Monospace => "monospace",
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
        Block, Document, Heading, Inline, InlineForm, InlineLink, InlineSpan, InlineVariant,
        InlineXref, ListItem, Paragraph, UnorderedList,
    };
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
            }),
            blocks: vec![
                Block::Paragraph(Paragraph {
                    inlines: vec![Inline::Text("Preamble paragraph.".into())],
                    lines: vec!["Preamble paragraph.".into()],
                    id: None,
                    reftext: None,
                }),
                Block::Heading(Heading {
                    level: 1,
                    title: "Section A".into(),
                    id: None,
                    reftext: None,
                }),
                Block::Paragraph(Paragraph {
                    inlines: vec![Inline::Text("Section body.".into())],
                    lines: vec!["Section body.".into()],
                    id: None,
                    reftext: None,
                }),
                Block::Heading(Heading {
                    level: 2,
                    title: "Section A Child".into(),
                    id: None,
                    reftext: None,
                }),
                Block::Paragraph(Paragraph {
                    inlines: vec![Inline::Text("Nested body.".into())],
                    lines: vec!["Nested body.".into()],
                    id: None,
                    reftext: None,
                }),
                Block::Heading(Heading {
                    level: 1,
                    title: "Section B".into(),
                    id: None,
                    reftext: None,
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
            attributes: [("toc".to_owned(), "left".to_owned())].into_iter().collect(),
            title: Some(Heading {
                level: 0,
                title: "Document Title".into(),
                id: None,
                reftext: None,
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
            }),
            blocks: Vec::new(),
        };

        let prepared = prepare_document(&document);
        let json = prepared_document_to_json(&prepared).expect("json serialization");

        assert!(json.contains("\"email\": \"jane@example.com\""));
        assert!(json.contains("\"name\": \"Jane Doe\""));
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
            }),
            blocks: vec![Block::Paragraph(Paragraph {
                inlines: vec![Inline::Text("hello".into())],
                lines: vec!["hello".into()],
                id: None,
                reftext: None,
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
                }),
                Block::Heading(Heading {
                    level: 1,
                    title: "First Section".into(),
                    id: None,
                    reftext: None,
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
                }),
                Block::Heading(Heading {
                    level: 1,
                    title: "First Section".into(),
                    id: Some("install".into()),
                    reftext: Some("Installation".into()),
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
            }),
            blocks: vec![
                Block::Heading(Heading {
                    level: 1,
                    title: "First Section".into(),
                    id: None,
                    reftext: None,
                }),
                Block::Paragraph(Paragraph {
                    inlines: vec![Inline::Text("Section body.".into())],
                    lines: vec!["Section body.".into()],
                    id: None,
                    reftext: None,
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
            }),
            blocks: vec![
                Block::Paragraph(Paragraph {
                    inlines: vec![Inline::Text("First preamble paragraph.".into())],
                    lines: vec!["First preamble paragraph.".into()],
                    id: None,
                    reftext: None,
                }),
                Block::Paragraph(Paragraph {
                    inlines: vec![Inline::Text("Second preamble paragraph.".into())],
                    lines: vec!["Second preamble paragraph.".into()],
                    id: None,
                    reftext: None,
                }),
                Block::Heading(Heading {
                    level: 1,
                    title: "Section One".into(),
                    id: None,
                    reftext: None,
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
                    })],
                }],
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
}
