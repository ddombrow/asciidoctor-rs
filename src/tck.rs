use std::collections::BTreeMap;

use serde::Deserialize;
use serde::Serialize;
use serde::ser::{SerializeStruct, Serializer};

use crate::ast::{Inline, InlineForm, InlineVariant};
use crate::inline::parse_spanned_inlines;
use crate::normalize::{normalize_asciidoc, trim_outer_blank_lines};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsgDocument {
    pub name: &'static str,
    pub node_type: &'static str,
    pub attributes: BTreeMap<String, String>,
    pub header: Option<AsgHeader>,
    pub blocks: Vec<AsgBlock>,
    pub location: [Position; 2],
}

impl Serialize for AsgDocument {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let include_attributes = self.header.is_some() || !self.attributes.is_empty();
        let include_header = self.header.is_some();
        let include_blocks = !self.blocks.is_empty();
        let field_count = 3
            + usize::from(include_attributes)
            + usize::from(include_header)
            + usize::from(include_blocks);
        let mut state = serializer.serialize_struct("AsgDocument", field_count)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("type", &self.node_type)?;
        if include_attributes {
            state.serialize_field("attributes", &self.attributes)?;
        }
        if let Some(header) = &self.header {
            state.serialize_field("header", header)?;
        }
        if include_blocks {
            state.serialize_field("blocks", &self.blocks)?;
        }
        state.serialize_field("location", &self.location)?;
        state.end()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AsgHeader {
    pub title: Vec<InlineText>,
    pub location: [Position; 2],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AsgBlock {
    pub name: &'static str,
    #[serde(rename = "type")]
    pub node_type: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<Vec<InlineText>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<AsgBlockMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delimiter: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inlines: Option<Vec<AsgInline>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocks: Option<Vec<AsgBlock>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub marker: Option<&'static str>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<AsgListItem>,
    pub location: [Position; 2],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AsgBlockMetadata {
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub attributes: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<String>,
    pub location: [Position; 2],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AsgListItem {
    pub name: &'static str,
    #[serde(rename = "type")]
    pub node_type: &'static str,
    pub marker: &'static str,
    pub principal: Vec<AsgInline>,
    pub location: [Position; 2],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InlineText {
    pub name: &'static str,
    #[serde(rename = "type")]
    pub node_type: &'static str,
    pub value: String,
    pub location: [Position; 2],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum AsgInline {
    Text(InlineText),
    Span(InlineSpanNode),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InlineSpanNode {
    pub name: &'static str,
    #[serde(rename = "type")]
    pub node_type: &'static str,
    pub variant: &'static str,
    pub form: &'static str,
    pub inlines: Vec<AsgInline>,
    pub location: [Position; 2],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Position {
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TckListKind {
    Ordered,
    Unordered,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingBlockAnchor {
    id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedTckTableCell {
    content: String,
    colspan: usize,
    rowspan: usize,
    style: Option<String>,
    start_line: usize,
    end_line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TckTableFormat {
    Psv,
    Csv,
    Dsv,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TckListMarker<'a> {
    kind: TckListKind,
    level: usize,
    marker: &'static str,
    content: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct TckRequest {
    pub contents: String,
    pub path: Option<String>,
    #[serde(rename = "type")]
    pub request_type: String,
}

pub fn render_tck_json(input: &str) -> serde_json::Result<String> {
    let normalized = normalize_asciidoc(input);
    let document = parse_tck_document(normalized.as_ref());
    serde_json::to_string_pretty(&document)
}

pub fn render_tck_inline_json(input: &str) -> serde_json::Result<String> {
    let normalized = normalize_asciidoc(input);
    serde_json::to_string_pretty(&parse_tck_inlines(trim_tck_inline_terminal_newline(
        normalized.as_ref(),
    )))
}

pub fn render_tck_json_from_request(request_json: &str) -> Result<String, String> {
    let request: TckRequest = serde_json::from_str(request_json)
        .map_err(|error| format!("invalid TCK request: {error}"))?;

    match request.request_type.as_str() {
        "block" => render_tck_json(&request.contents)
            .map_err(|error| format!("failed to serialize TCK ASG: {error}")),
        "inline" => render_tck_inline_json(&request.contents)
            .map_err(|error| format!("failed to serialize TCK inline ASG: {error}")),
        other => Err(format!("unsupported TCK request type: {other}")),
    }
}

pub fn parse_tck_document(input: &str) -> AsgDocument {
    let normalized = normalize_asciidoc(input);
    let lines: Vec<&str> = normalized.lines().collect();
    let mut index = 0;
    let mut attributes = BTreeMap::new();
    let mut header = None;
    let mut saw_explicit_author = false;
    let mut saw_explicit_authors = false;
    let mut saw_explicit_authorinitials = false;

    index = skip_header_comments(&lines, index);

    if let Some((title, title_range, consumed)) = parse_heading_line(&lines, index, 1) {
        if title.level == 0 {
            let mut header_end = title_range[1].clone();
            index += consumed;
            index = skip_header_comments(&lines, index);

            if let Some(author_line) = lines
                .get(index)
                .and_then(|line| parse_implicit_author_line(&lines, index, line))
            {
                insert_author_attributes(&mut attributes, &author_line.authors);
                header_end = Position {
                    line: index + 1,
                    col: lines[index].len(),
                };
                index += 1;
                index = skip_header_comments(&lines, index);

                if let Some(revision_line) = lines
                    .get(index)
                    .and_then(|line| parse_implicit_revision_line(line))
                {
                    attributes.insert("revnumber".to_owned(), revision_line.number);
                    if let Some(date) = revision_line.date {
                        attributes.insert("revdate".to_owned(), date);
                    }
                    if let Some(remark) = revision_line.remark {
                        attributes.insert("revremark".to_owned(), remark);
                    }
                    header_end = Position {
                        line: index + 1,
                        col: lines[index].len(),
                    };
                    index += 1;
                }
            }

            while index < lines.len() {
                let line = lines[index];
                if line.trim().is_empty() {
                    index += 1;
                    break;
                }

                if is_comment_line(line) {
                    index += 1;
                    continue;
                }

                if let Some((name, value, consumed_lines, end_col)) =
                    parse_attribute_entry_at(&lines, index)
                {
                    match name.as_str() {
                        "author" => saw_explicit_author = true,
                        "authors" => saw_explicit_authors = true,
                        "authorinitials" => saw_explicit_authorinitials = true,
                        _ => {}
                    }
                    attributes.insert(name, value);
                    header_end = Position {
                        line: index + consumed_lines,
                        col: end_col,
                    };
                    index += consumed_lines;
                    continue;
                }

                break;
            }

            if saw_explicit_authors {
                normalize_explicit_author_attributes(
                    &mut attributes,
                    "authors",
                    saw_explicit_authorinitials,
                );
            } else if saw_explicit_author {
                normalize_explicit_author_attributes(
                    &mut attributes,
                    "author",
                    saw_explicit_authorinitials,
                );
            }

            header = Some(AsgHeader {
                title: vec![InlineText {
                    name: "text",
                    node_type: "string",
                    value: title.title,
                    location: [
                        Position {
                            line: title_range[0].line,
                            col: title_range[0].col + 2,
                        },
                        title_range[1].clone(),
                    ],
                }],
                location: [title_range[0].clone(), header_end],
            });
        }
    }

    while header.is_none() && index < lines.len() {
        let line = lines[index];
        if line.trim().is_empty() {
            index += 1;
            break;
        }

        if is_comment_line(line) {
            index += 1;
            continue;
        }

        if let Some((name, value, consumed_lines, _end_col)) =
            parse_attribute_entry_at(&lines, index)
        {
            match name.as_str() {
                "author" => saw_explicit_author = true,
                "authors" => saw_explicit_authors = true,
                "authorinitials" => saw_explicit_authorinitials = true,
                _ => {}
            }
            attributes.insert(name, value);
            index += consumed_lines;
            continue;
        }

        break;
    }

    if header.is_none() {
        if saw_explicit_authors {
            normalize_explicit_author_attributes(
                &mut attributes,
                "authors",
                saw_explicit_authorinitials,
            );
        } else if saw_explicit_author {
            normalize_explicit_author_attributes(
                &mut attributes,
                "author",
                saw_explicit_authorinitials,
            );
        }
    }

    let (blocks, end) = parse_blocks(&lines[index..], index + 1, None, Some(&mut attributes));
    let start = header
        .as_ref()
        .map(|header| header.location[0].clone())
        .or_else(|| blocks.first().map(block_start_position))
        .unwrap_or(Position { line: 1, col: 1 });
    let end = header
        .as_ref()
        .map(|header| header.location[1].clone())
        .into_iter()
        .chain(end)
        .last()
        .unwrap_or_else(|| start.clone());

    AsgDocument {
        name: "document",
        node_type: "block",
        attributes,
        header,
        blocks,
        location: [start, end],
    }
}

fn block_start_position(block: &AsgBlock) -> Position {
    block
        .metadata
        .as_ref()
        .map(|metadata| metadata.location[0].clone())
        .unwrap_or_else(|| block.location[0].clone())
}

pub fn parse_tck_inlines(input: &str) -> Vec<AsgInline> {
    let normalized = normalize_asciidoc(input);
    parse_tck_inlines_at(normalized.as_ref(), 1, 1)
}

fn trim_tck_inline_terminal_newline(input: &str) -> &str {
    input.strip_suffix('\n').unwrap_or(input)
}

fn parse_blocks(
    lines: &[&str],
    line_offset: usize,
    stop_at_level: Option<u8>,
    document_attributes: Option<&mut BTreeMap<String, String>>,
) -> (Vec<AsgBlock>, Option<Position>) {
    let mut blocks = Vec::new();
    let mut index = 0;
    let mut paragraph_start = None::<usize>;
    let mut paragraph_lines = Vec::new();
    let mut last_end = None;
    let mut pending_anchor = None::<PendingBlockAnchor>;
    let mut document_attributes = document_attributes;

    while index < lines.len() {
        let absolute_index = line_offset + index - 1;
        let line = lines[index];

        // Block comment delimiter: consume everything until the matching closing delimiter.
        if let Some((delimiter, _, _)) =
            parse_delimited_block_marker(line).filter(|(_, kind, _)| *kind == "comment")
        {
            index += 1;
            while index < lines.len() && lines[index].trim() != delimiter {
                index += 1;
            }
            if index < lines.len() {
                index += 1;
            }
            continue;
        }

        // Line comment (// ...): skip the line without affecting paragraph state
        if is_comment_line(line) {
            index += 1;
            continue;
        }

        if let Some(anchor) = parse_block_anchor(line) {
            flush_paragraph(
                &mut blocks,
                &mut paragraph_start,
                &mut paragraph_lines,
                line_offset,
                &mut last_end,
            );
            pending_anchor = Some(anchor);
            index += 1;
            continue;
        }

        if let Some((mut block, consumed_lines)) = parse_table(lines, index, line_offset) {
            flush_paragraph(
                &mut blocks,
                &mut paragraph_start,
                &mut paragraph_lines,
                line_offset,
                &mut last_end,
            );
            apply_anchor_to_block(&mut block, pending_anchor.take());
            last_end = Some(block.location[1].clone());
            blocks.push(block);
            index += consumed_lines;
            continue;
        }

        if let Some((mut block, consumed_lines)) = parse_block_image(lines, index, line_offset) {
            flush_paragraph(
                &mut blocks,
                &mut paragraph_start,
                &mut paragraph_lines,
                line_offset,
                &mut last_end,
            );
            apply_anchor_to_block(&mut block, pending_anchor.take());
            last_end = Some(block.location[1].clone());
            blocks.push(block);
            index += consumed_lines;
            continue;
        }

        if let Some((mut block, consumed_lines)) = parse_delimited_block(lines, index, line_offset)
        {
            flush_paragraph(
                &mut blocks,
                &mut paragraph_start,
                &mut paragraph_lines,
                line_offset,
                &mut last_end,
            );
            apply_anchor_to_block(&mut block, pending_anchor.take());
            last_end = Some(block.location[1].clone());
            blocks.push(block);
            index += consumed_lines;
            continue;
        }

        let heading = if paragraph_lines.is_empty() {
            parse_heading_line(lines, index, line_offset)
        } else {
            parse_atx_heading_line(lines[index], line_offset + index)
                .map(|(heading, range)| (heading, range, 1))
        };
        if let Some((heading, heading_range, consumed_lines)) = heading {
            if let Some(level) = stop_at_level {
                if heading.level <= level {
                    break;
                }
            }

            flush_paragraph(
                &mut blocks,
                &mut paragraph_start,
                &mut paragraph_lines,
                line_offset,
                &mut last_end,
            );

            let child_start = index + consumed_lines;
            let (child_blocks, child_end) = parse_blocks(
                &lines[child_start..],
                line_offset + child_start,
                Some(heading.level),
                None,
            );

            let end = child_end.unwrap_or_else(|| heading_range[1].clone());
            let mut block = AsgBlock {
                name: "section",
                node_type: "block",
                id: None,
                title: Some(vec![InlineText {
                    name: "text",
                    node_type: "string",
                    value: heading.title,
                    location: [
                        Position {
                            line: heading_range[0].line,
                            col: heading_range[0].col + heading.marker_len + 1,
                        },
                        heading_range[1].clone(),
                    ],
                }]),
                metadata: None,
                level: Some(heading.level),
                form: None,
                delimiter: None,
                inlines: None,
                blocks: Some(child_blocks),
                variant: None,
                marker: None,
                items: vec![],
                location: [heading_range[0].clone(), end.clone()],
            };
            apply_anchor_to_block(&mut block, pending_anchor.take());
            blocks.push(block);
            last_end = Some(end);

            index = child_start
                + count_consumed_lines(&lines[child_start..], stop_at_level, heading.level);
            continue;
        }

        if let Some((mut block, consumed_lines)) =
            parse_styled_paragraph_block(lines, index, line_offset)
        {
            flush_paragraph(
                &mut blocks,
                &mut paragraph_start,
                &mut paragraph_lines,
                line_offset,
                &mut last_end,
            );
            apply_anchor_to_block(&mut block, pending_anchor.take());
            last_end = Some(block.location[1].clone());
            blocks.push(block);
            index += consumed_lines;
            continue;
        }

        if let Some((mut block, consumed_lines)) =
            parse_admonition_paragraph(lines, index, line_offset)
        {
            flush_paragraph(
                &mut blocks,
                &mut paragraph_start,
                &mut paragraph_lines,
                line_offset,
                &mut last_end,
            );
            apply_anchor_to_block(&mut block, pending_anchor.take());
            last_end = Some(block.location[1].clone());
            blocks.push(block);
            index += consumed_lines;
            continue;
        }

        if paragraph_start.is_none() && pending_anchor.is_none() {
            if let Some((name, value, consumed_lines, _end_col)) =
                parse_attribute_entry_at(&lines, index)
            {
                if let Some(attributes) = document_attributes.as_deref_mut() {
                    attributes.insert(name, value);
                    index += consumed_lines;
                    continue;
                }
            }
        }

        if let Some(list_marker) = parse_list_item_line(line).filter(|marker| marker.level == 1) {
            flush_paragraph(
                &mut blocks,
                &mut paragraph_start,
                &mut paragraph_lines,
                line_offset,
                &mut last_end,
            );

            let mut items = Vec::new();
            let mut list_end = None;
            let mut list_index = index;

            while list_index < lines.len() {
                let list_line = lines[list_index];
                let Some(marker) = parse_list_item_line(list_line) else {
                    break;
                };
                if marker.kind != list_marker.kind || marker.level != list_marker.level {
                    break;
                }
                let item_line_no = line_offset + list_index;
                let content_col = list_line.len() - marker.content.len() + 1;
                let item_end_col = list_line.trim_end().len();
                let item_start = Position {
                    line: item_line_no,
                    col: 1,
                };
                let item_end = Position {
                    line: item_line_no,
                    col: item_end_col,
                };
                let principal = parse_tck_inlines_at(marker.content, item_line_no, content_col);
                items.push(AsgListItem {
                    name: "listItem",
                    node_type: "block",
                    marker: marker.marker,
                    principal,
                    location: [item_start, item_end.clone()],
                });
                list_end = Some(item_end);
                list_index += 1;
            }

            let list_start = Position {
                line: line_offset + index,
                col: 1,
            };
            let list_end = list_end.unwrap_or_else(|| list_start.clone());
            let mut block = AsgBlock {
                name: "list",
                node_type: "block",
                id: None,
                title: None,
                metadata: None,
                level: None,
                form: None,
                delimiter: None,
                inlines: None,
                blocks: None,
                variant: Some(match list_marker.kind {
                    TckListKind::Ordered => "ordered",
                    TckListKind::Unordered => "unordered",
                }),
                marker: Some(list_marker.marker),
                items,
                location: [list_start, list_end.clone()],
            };
            apply_anchor_to_block(&mut block, pending_anchor.take());
            blocks.push(block);
            last_end = Some(list_end);
            index = list_index;
            continue;
        }

        // Callout list items (<N> description) — skip them in TCK output
        {
            let trimmed = line.trim_start();
            let is_callout = trimmed.starts_with('<')
                && trimmed.find('>').is_some_and(|i| {
                    let inner = &trimmed[1..i];
                    inner == "." || (inner.chars().all(|c| c.is_ascii_digit()) && i > 1)
                });
            if is_callout {
                flush_paragraph(
                    &mut blocks,
                    &mut paragraph_start,
                    &mut paragraph_lines,
                    line_offset,
                    &mut last_end,
                );
                index += 1;
                continue;
            }
        }

        if line.trim().is_empty() {
            flush_paragraph(
                &mut blocks,
                &mut paragraph_start,
                &mut paragraph_lines,
                line_offset,
                &mut last_end,
            );
            index += 1;
            continue;
        }

        if paragraph_start.is_none() {
            paragraph_start = Some(absolute_index);
        }
        paragraph_lines.push(line.to_owned());
        index += 1;
    }

    flush_paragraph(
        &mut blocks,
        &mut paragraph_start,
        &mut paragraph_lines,
        line_offset,
        &mut last_end,
    );

    (blocks, last_end)
}

fn apply_anchor_to_block(block: &mut AsgBlock, anchor: Option<PendingBlockAnchor>) {
    if let Some(anchor) = anchor
        && block.id.is_none()
    {
        block.id = Some(anchor.id);
    }
}

fn count_consumed_lines(lines: &[&str], stop_at_level: Option<u8>, current_level: u8) -> usize {
    let mut index = 0;
    while index < lines.len() {
        if let Some((heading, _, _)) = parse_heading_line(lines, index, 1) {
            if heading.level <= stop_at_level.unwrap_or(current_level) {
                break;
            }
        }
        index += 1;
    }
    index
}

fn parse_delimited_block(
    lines: &[&str],
    index: usize,
    line_offset: usize,
) -> Option<(AsgBlock, usize)> {
    let prelude = parse_block_prelude(lines, index, line_offset);
    let delimiter_index = index + prelude.consumed_lines;
    let delimiter_line = lines.get(delimiter_index)?;
    let fenced_entries = parse_fenced_code_opening(delimiter_line);
    let (delimiter, name, canonical_delimiter) = parse_delimited_block_marker(delimiter_line)?;
    if name == "comment" {
        return None;
    }

    let closing_index = lines[delimiter_index + 1..]
        .iter()
        .position(|line| line.trim() == delimiter)
        .map(|offset| delimiter_index + 1 + offset)?;
    let start_line = line_offset + delimiter_index;
    let end_line = line_offset + closing_index;
    let inner_lines = &lines[delimiter_index + 1..closing_index];
    let consumed = closing_index - index + 1;
    let content = inner_lines.join("\n");

    if name == "example"
        && let Some(variant) = prelude
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.attributes.get("style"))
            .and_then(|style| admonition_variant_from_style(style))
    {
        let (children, _) = parse_blocks(inner_lines, start_line + 1, None, None);
        return Some((
            AsgBlock {
                name: "admonition",
                node_type: "block",
                id: prelude.id.clone(),
                title: prelude.title,
                metadata: prelude.metadata,
                level: None,
                form: Some("delimited"),
                delimiter: Some(canonical_delimiter),
                inlines: None,
                blocks: Some(children),
                variant: Some(variant),
                marker: None,
                items: vec![],
                location: [
                    Position {
                        line: start_line,
                        col: 1,
                    },
                    Position {
                        line: end_line,
                        col: lines[closing_index].trim_end().len(),
                    },
                ],
            },
            consumed,
        ));
    }

    if name == "open" {
        return Some((
            parse_tck_open_block(
                prelude,
                canonical_delimiter,
                start_line,
                end_line,
                lines[closing_index],
                inner_lines,
                &content,
            ),
            consumed,
        ));
    }

    let mut metadata = prelude.metadata.clone();
    if let Some(entries) = fenced_entries.as_ref() {
        apply_fenced_code_metadata(
            &mut metadata,
            start_line,
            lines[delimiter_index].trim_end().len(),
            entries,
        );
    }

    let mut block = AsgBlock {
        name,
        node_type: "block",
        id: prelude.id.clone(),
        title: prelude.title,
        metadata,
        level: None,
        form: Some("delimited"),
        delimiter: Some(canonical_delimiter),
        inlines: None,
        blocks: None,
        variant: None,
        marker: None,
        items: vec![],
        location: [
            Position {
                line: start_line,
                col: 1,
            },
            Position {
                line: end_line,
                col: lines[closing_index].trim_end().len(),
            },
        ],
    };

    match name {
        "listing" | "literal" => {
            block.inlines =
                text_inlines_for_delimited_content(&content, inner_lines, start_line, end_line);
        }
        "passthrough" => {
            block.inlines =
                text_inlines_for_delimited_content(&content, inner_lines, start_line, end_line);
        }
        "example" | "sidebar" | "open" => {
            let (children, _) = parse_blocks(inner_lines, start_line + 1, None, None);
            block.blocks = Some(children);
        }
        "quote" => {
            let is_verse = block
                .metadata
                .as_ref()
                .and_then(|m| m.attributes.get("style"))
                .is_some_and(|s| s.eq_ignore_ascii_case("verse"));
            if is_verse {
                block.name = "verse";
                block.inlines =
                    text_inlines_for_delimited_content(&content, inner_lines, start_line, end_line);
            } else {
                let (children, _) = parse_blocks(inner_lines, start_line + 1, None, None);
                block.blocks = Some(children);
            }
        }
        _ => {}
    }

    Some((block, consumed))
}

fn parse_tck_open_block(
    prelude: ParsedBlockPrelude,
    canonical_delimiter: &'static str,
    start_line: usize,
    end_line: usize,
    closing_line: &str,
    inner_lines: &[&str],
    content: &str,
) -> AsgBlock {
    let style = prelude
        .metadata
        .as_ref()
        .and_then(|metadata| metadata.attributes.get("style"))
        .cloned()
        .unwrap_or_default();
    let end = Position {
        line: end_line,
        col: closing_line.trim_end().len(),
    };

    if let Some(variant) = admonition_variant_from_style(&style) {
        let (children, _) = parse_blocks(inner_lines, start_line + 1, None, None);
        return AsgBlock {
            name: "admonition",
            node_type: "block",
            id: prelude.id,
            title: prelude.title,
            metadata: prelude.metadata,
            level: None,
            form: Some("delimited"),
            delimiter: Some(canonical_delimiter),
            inlines: None,
            blocks: Some(children),
            variant: Some(variant),
            marker: None,
            items: vec![],
            location: [
                Position {
                    line: start_line,
                    col: 1,
                },
                end,
            ],
        };
    }

    let mut block = AsgBlock {
        name: "open",
        node_type: "block",
        id: prelude.id,
        title: prelude.title,
        metadata: prelude.metadata,
        level: None,
        form: Some("delimited"),
        delimiter: Some(canonical_delimiter),
        inlines: None,
        blocks: None,
        variant: None,
        marker: None,
        items: vec![],
        location: [
            Position {
                line: start_line,
                col: 1,
            },
            end,
        ],
    };

    let (children, _) = parse_blocks(inner_lines, start_line + 1, None, None);
    match style.as_str() {
        "sidebar" => {
            block.name = "sidebar";
            block.blocks = Some(children);
        }
        "example" => {
            block.name = "example";
            block.blocks = Some(children);
        }
        "quote" => {
            block.name = "quote";
            block.blocks = Some(children);
        }
        "verse" => {
            block.name = "verse";
            block.inlines =
                text_inlines_for_delimited_content(content, inner_lines, start_line, end_line);
        }
        "pass" | "stem" | "latexmath" | "asciimath" => {
            block.name = "passthrough";
            block.inlines =
                text_inlines_for_delimited_content(content, inner_lines, start_line, end_line);
        }
        _ => {
            block.blocks = Some(children);
        }
    }
    block
}

fn text_inlines_for_delimited_content(
    content: &str,
    inner_lines: &[&str],
    start_line: usize,
    _end_line: usize,
) -> Option<Vec<AsgInline>> {
    let (start_offset, end_offset) = trimmed_delimited_content_offsets(inner_lines);
    if start_offset == end_offset {
        return None;
    }

    let trimmed = trim_outer_blank_lines(content);
    let start = Position {
        line: start_line + 1 + start_offset,
        col: 1,
    };
    let end = Position {
        line: start_line + end_offset,
        col: inner_lines[end_offset - 1].len(),
    };
    Some(vec![AsgInline::Text(InlineText {
        name: "text",
        node_type: "string",
        value: trimmed,
        location: [start, end],
    })])
}

fn trimmed_delimited_content_offsets(lines: &[&str]) -> (usize, usize) {
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

fn parse_table(lines: &[&str], index: usize, line_offset: usize) -> Option<(AsgBlock, usize)> {
    let prelude = parse_block_prelude(lines, index, line_offset);
    let delimiter_index = index + prelude.consumed_lines;
    let (delimiter, delimiter_char, canonical_delimiter) =
        parse_table_delimiter(lines.get(delimiter_index)?)?;
    let metadata = prelude.metadata.as_ref();
    let header_enabled = metadata.is_some_and(table_has_header_option);
    let expected_columns = metadata.and_then(table_column_count);
    let format = table_format(metadata, delimiter_char);
    let separator = table_separator(metadata, format, delimiter_char)?;
    let start_line = line_offset + delimiter_index;

    let (row_groups, consumed) = match format {
        TckTableFormat::Psv => parse_psv_table_rows(
            lines,
            delimiter_index,
            start_line,
            delimiter,
            separator,
            expected_columns,
        )?,
        TckTableFormat::Csv | TckTableFormat::Dsv => parse_separated_value_table_rows(
            lines,
            delimiter_index,
            start_line,
            delimiter,
            separator,
        )?,
    };

    let closing_index = delimiter_index + consumed - 1;
    let end_line = line_offset + closing_index;
    let mut rows = if let Some(column_count) = expected_columns {
        assemble_tck_table_rows_with_known_columns(&row_groups, column_count)?
    } else {
        assemble_tck_table_rows_without_known_columns(&row_groups)?
    };

    if header_enabled && !rows.is_empty() {
        rows[0].variant = Some("header");
    }

    Some((
        AsgBlock {
            name: "table",
            node_type: "block",
            id: prelude.id,
            title: prelude.title,
            metadata: prelude.metadata,
            level: None,
            form: Some("delimited"),
            delimiter: Some(canonical_delimiter),
            inlines: None,
            blocks: Some(rows),
            variant: None,
            marker: None,
            items: vec![],
            location: [
                Position {
                    line: start_line,
                    col: 1,
                },
                Position {
                    line: end_line,
                    col: lines[closing_index].trim_end().len(),
                },
            ],
        },
        prelude.consumed_lines + consumed,
    ))
}

fn parse_block_image(
    lines: &[&str],
    index: usize,
    line_offset: usize,
) -> Option<(AsgBlock, usize)> {
    let prelude = parse_block_prelude(lines, index, line_offset);
    let image_index = index + prelude.consumed_lines;
    let line = *lines.get(image_index)?;
    let image = parse_block_image_line(line)?;
    let line_no = line_offset + image_index;
    let start = prelude.start.clone().unwrap_or(Position {
        line: line_no,
        col: 1,
    });
    let end = Position {
        line: line_no,
        col: line.trim_end().len(),
    };

    let mut attributes = prelude
        .metadata
        .as_ref()
        .map(|metadata| metadata.attributes.clone())
        .unwrap_or_default();
    attributes.insert("target".into(), image.target.clone());
    attributes.insert("alt".into(), image.alt.clone());
    if let Some(width) = image.width {
        attributes.insert("width".into(), width);
    }
    if let Some(height) = image.height {
        attributes.insert("height".into(), height);
    }
    attributes.extend(image.named_attributes);

    let metadata = Some(AsgBlockMetadata {
        attributes,
        options: prelude
            .metadata
            .as_ref()
            .map(|metadata| metadata.options.clone())
            .unwrap_or_default(),
        roles: prelude
            .metadata
            .as_ref()
            .map(|metadata| metadata.roles.clone())
            .unwrap_or_default(),
        location: [start.clone(), end.clone()],
    });

    Some((
        AsgBlock {
            name: "image",
            node_type: "block",
            id: prelude.id,
            title: prelude.title,
            metadata,
            level: None,
            form: None,
            delimiter: None,
            inlines: None,
            blocks: None,
            variant: None,
            marker: None,
            items: vec![],
            location: [
                Position {
                    line: line_no,
                    col: 1,
                },
                end,
            ],
        },
        prelude.consumed_lines + 1,
    ))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedTckBlockImage {
    target: String,
    alt: String,
    width: Option<String>,
    height: Option<String>,
    named_attributes: BTreeMap<String, String>,
}

fn parse_block_image_line(line: &str) -> Option<ParsedTckBlockImage> {
    let rest = line.strip_prefix("image::")?;
    let bracket_start = rest.find('[')?;
    let bracket_end = rest.rfind(']')?;
    if bracket_end <= bracket_start {
        return None;
    }
    let target = rest[..bracket_start].trim().to_owned();
    if target.is_empty() {
        return None;
    }
    let attr_text = &rest[bracket_start + 1..bracket_end];
    let (alt, width, height, named_attributes) = parse_image_attributes(attr_text, &target);

    Some(ParsedTckBlockImage {
        target,
        alt,
        width,
        height,
        named_attributes,
    })
}

fn parse_image_attributes(
    attr_text: &str,
    target: &str,
) -> (
    String,
    Option<String>,
    Option<String>,
    BTreeMap<String, String>,
) {
    let mut named_attributes = BTreeMap::new();
    let mut positional = Vec::new();

    if !attr_text.is_empty() {
        for part in split_image_attrs(attr_text) {
            let part = part.trim();
            if let Some((key, value)) = part.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches('"').trim_matches('\'');
                named_attributes.insert(key.to_owned(), value.to_owned());
            } else {
                positional.push(part.to_owned());
            }
        }
    }

    let alt = positional
        .first()
        .filter(|value| !value.is_empty())
        .cloned()
        .unwrap_or_else(|| auto_generate_image_alt(target));
    let width = positional.get(1).filter(|value| !value.is_empty()).cloned();
    let height = positional.get(2).filter(|value| !value.is_empty()).cloned();

    (alt, width, height, named_attributes)
}

fn split_image_attrs(text: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;
    let mut quote_char = '"';
    for ch in text.chars() {
        if !in_quote && (ch == '"' || ch == '\'') {
            in_quote = true;
            quote_char = ch;
        } else if in_quote && ch == quote_char {
            in_quote = false;
        } else if !in_quote && ch == ',' {
            parts.push(std::mem::take(&mut current));
            continue;
        }
        current.push(ch);
    }
    parts.push(current);
    parts
}

fn auto_generate_image_alt(target: &str) -> String {
    let filename = target.rsplit('/').next().unwrap_or(target);
    let filename = filename.rsplit('\\').next().unwrap_or(filename);
    let stem = filename
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(filename);
    stem.replace('-', " ").replace('_', " ")
}

fn parse_psv_table_rows(
    lines: &[&str],
    delimiter_index: usize,
    start_line: usize,
    delimiter: &str,
    separator: char,
    expected_columns: Option<usize>,
) -> Option<(Vec<Vec<ParsedTckTableCell>>, usize)> {
    let mut row_groups: Vec<Vec<ParsedTckTableCell>> = Vec::new();
    let mut current_group: Vec<ParsedTckTableCell> = Vec::new();
    let mut current_cell: Option<ParsedTckTableCell> = None;
    let mut consumed = 1;
    let mut closed = false;

    while delimiter_index + consumed < lines.len() {
        let line = lines[delimiter_index + consumed];
        let trimmed = line.trim();
        if trimmed == delimiter {
            if let Some(cell) = current_cell.take() {
                current_group.push(cell);
            }
            if !current_group.is_empty() {
                row_groups.push(std::mem::take(&mut current_group));
            }
            consumed += 1;
            closed = true;
            break;
        }
        if trimmed.is_empty() {
            let next_nonempty = next_nonempty_table_line(lines, delimiter_index + consumed + 1);
            if next_nonempty.is_some_and(|line| starts_table_cell_line(line, separator)) {
                if let Some(cell) = current_cell.take() {
                    current_group.push(cell);
                }
                if !current_group.is_empty() {
                    row_groups.push(std::mem::take(&mut current_group));
                }
            } else if let Some(cell) = &mut current_cell {
                if !cell.content.is_empty() {
                    cell.content.push('\n');
                }
                cell.content.push('\n');
                cell.end_line = start_line + consumed - 1;
            }
            consumed += 1;
            continue;
        }

        if starts_table_cell_line(trimmed, separator) {
            let had_cells_in_group = !current_group.is_empty();
            if let Some(cell) = current_cell.take() {
                current_group.push(cell);
                maybe_finish_tck_table_row_group(
                    &mut row_groups,
                    &mut current_group,
                    expected_columns,
                );
                if expected_columns.is_none() && had_cells_in_group && !current_group.is_empty() {
                    row_groups.push(std::mem::take(&mut current_group));
                }
            }
            let cells = parse_tck_table_cells_from_line(line, separator, start_line + consumed)?;
            if cells.is_empty() {
                return None;
            }
            let mut iter = cells.into_iter();
            let last = iter.next_back()?;
            for cell in iter {
                current_group.push(cell);
                maybe_finish_tck_table_row_group(
                    &mut row_groups,
                    &mut current_group,
                    expected_columns,
                );
            }
            current_cell = Some(last);
        } else {
            let cell = current_cell.as_mut()?;
            if !cell.content.is_empty() {
                cell.content.push('\n');
            }
            cell.content.push_str(line);
            cell.end_line = start_line + consumed - 1;
        }
        consumed += 1;
    }

    closed.then_some((row_groups, consumed))
}

fn parse_separated_value_table_rows(
    lines: &[&str],
    delimiter_index: usize,
    start_line: usize,
    delimiter: &str,
    separator: char,
) -> Option<(Vec<Vec<ParsedTckTableCell>>, usize)> {
    let closing_index = lines[delimiter_index + 1..]
        .iter()
        .position(|line| line.trim() == delimiter)
        .map(|offset| delimiter_index + 1 + offset)?;
    let consumed = closing_index - delimiter_index + 1;
    let content = lines[delimiter_index + 1..closing_index].join("\n");
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(separator as u8)
        .flexible(true)
        .from_reader(content.as_bytes());
    let mut row_groups = Vec::new();
    let mut line_no = start_line + 1;
    for record in reader.records() {
        let record = record.ok()?;
        row_groups.push(
            record
                .iter()
                .map(|value| ParsedTckTableCell {
                    content: value.to_owned(),
                    colspan: 1,
                    rowspan: 1,
                    style: None,
                    start_line: line_no,
                    end_line: line_no,
                })
                .collect(),
        );
        line_no += 1;
    }
    Some((row_groups, consumed))
}

fn maybe_finish_tck_table_row_group(
    row_groups: &mut Vec<Vec<ParsedTckTableCell>>,
    current_group: &mut Vec<ParsedTckTableCell>,
    expected_columns: Option<usize>,
) {
    let Some(column_count) = expected_columns else {
        return;
    };
    if column_count == 0 || current_group.is_empty() {
        return;
    }

    let current_width = current_group
        .iter()
        .map(|cell| cell.colspan.max(1))
        .sum::<usize>();
    if current_width == column_count {
        row_groups.push(std::mem::take(current_group));
    }
}

fn assemble_tck_table_rows_with_known_columns(
    row_groups: &[Vec<ParsedTckTableCell>],
    column_count: usize,
) -> Option<Vec<AsgBlock>> {
    if column_count == 0 {
        return None;
    }

    if row_groups.len() > 1 {
        return assemble_explicit_tck_table_rows_with_known_columns(row_groups, column_count);
    }

    let mut rows = Vec::new();
    let mut current_row = Vec::new();
    let mut current_width = 0;
    for cell in row_groups.iter().flatten() {
        current_width += cell.colspan.max(1);
        current_row.push(build_tck_table_cell(cell));
        if current_width == column_count {
            rows.push(build_tck_table_row(std::mem::take(&mut current_row)));
            current_width = 0;
        }
    }

    if !current_row.is_empty() {
        rows.push(build_tck_table_row(current_row));
    }

    Some(rows)
}

fn assemble_explicit_tck_table_rows_with_known_columns(
    row_groups: &[Vec<ParsedTckTableCell>],
    column_count: usize,
) -> Option<Vec<AsgBlock>> {
    let mut rows = Vec::new();
    let mut active_rowspans = vec![0usize; column_count];

    for group in row_groups {
        let mut next_rowspans = active_rowspans
            .iter()
            .map(|span| span.saturating_sub(1))
            .collect::<Vec<_>>();
        let mut col = 0usize;

        for cell in group {
            while col < column_count && active_rowspans[col] > 0 {
                col += 1;
            }
            if col >= column_count {
                return None;
            }

            let colspan = cell.colspan.max(1);
            let rowspan = cell.rowspan.max(1);
            if col + colspan > column_count {
                return None;
            }
            if (col..col + colspan).any(|index| active_rowspans[index] > 0) {
                return None;
            }

            if rowspan > 1 {
                for index in col..col + colspan {
                    next_rowspans[index] = next_rowspans[index].max(rowspan - 1);
                }
            }
            col += colspan;
        }

        rows.push(build_tck_table_row(
            group.iter().map(build_tck_table_cell).collect(),
        ));
        active_rowspans = next_rowspans;
    }

    Some(rows)
}

fn assemble_tck_table_rows_without_known_columns(
    row_groups: &[Vec<ParsedTckTableCell>],
) -> Option<Vec<AsgBlock>> {
    if row_groups.is_empty() {
        return None;
    }

    if row_groups
        .iter()
        .flatten()
        .any(|cell| cell.colspan > 1 || cell.rowspan > 1)
    {
        let inferred_columns = row_groups
            .iter()
            .map(|group| group.iter().map(|cell| cell.colspan.max(1)).sum::<usize>())
            .max()?;
        return assemble_explicit_tck_table_rows_with_known_columns(row_groups, inferred_columns);
    }

    Some(
        row_groups
            .iter()
            .map(|group| build_tck_table_row(group.iter().map(build_tck_table_cell).collect()))
            .collect(),
    )
}

fn build_tck_table_row(cells: Vec<AsgBlock>) -> AsgBlock {
    let start = cells
        .first()
        .map(|cell| cell.location[0].clone())
        .unwrap_or(Position { line: 1, col: 1 });
    let end = cells
        .last()
        .map(|cell| cell.location[1].clone())
        .unwrap_or_else(|| start.clone());
    AsgBlock {
        name: "tableRow",
        node_type: "block",
        id: None,
        title: None,
        metadata: None,
        level: None,
        form: None,
        delimiter: None,
        inlines: None,
        blocks: Some(cells),
        variant: None,
        marker: None,
        items: vec![],
        location: [start, end],
    }
}

fn build_tck_table_cell(cell: &ParsedTckTableCell) -> AsgBlock {
    let normalized = normalize_table_cell_content(&cell.content);
    let lines: Vec<&str> = normalized.lines().collect();
    let (blocks, end) = if lines.is_empty() {
        (Vec::new(), None)
    } else {
        parse_blocks(&lines, cell.start_line, None, None)
    };

    let mut attributes = BTreeMap::new();
    if cell.colspan > 1 {
        attributes.insert("colspan".into(), cell.colspan.to_string());
    }
    if cell.rowspan > 1 {
        attributes.insert("rowspan".into(), cell.rowspan.to_string());
    }
    if let Some(style) = &cell.style {
        attributes.insert("style".into(), style.clone());
    }

    AsgBlock {
        name: "tableCell",
        node_type: "block",
        id: None,
        title: None,
        metadata: (!attributes.is_empty()).then_some(AsgBlockMetadata {
            attributes,
            options: vec![],
            roles: vec![],
            location: [
                Position {
                    line: cell.start_line,
                    col: 1,
                },
                end.clone().unwrap_or(Position {
                    line: cell.end_line,
                    col: cell.content.lines().last().map(str::len).unwrap_or(1),
                }),
            ],
        }),
        level: None,
        form: None,
        delimiter: None,
        inlines: None,
        blocks: (!blocks.is_empty()).then_some(blocks),
        variant: None,
        marker: None,
        items: vec![],
        location: [
            Position {
                line: cell.start_line,
                col: 1,
            },
            end.unwrap_or(Position {
                line: cell.end_line,
                col: cell.content.lines().last().map(str::len).unwrap_or(1),
            }),
        ],
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct ParsedBlockPrelude {
    consumed_lines: usize,
    id: Option<String>,
    title: Option<Vec<InlineText>>,
    metadata: Option<AsgBlockMetadata>,
    start: Option<Position>,
}

fn parse_block_prelude(lines: &[&str], index: usize, line_offset: usize) -> ParsedBlockPrelude {
    let mut prelude = ParsedBlockPrelude::default();
    let mut cursor = index;
    let mut title_raw = None::<String>;
    let mut metadata_attributes = BTreeMap::new();
    let mut metadata_options = Vec::new();
    let mut metadata_roles = Vec::new();
    let mut metadata_start = None::<Position>;
    let mut metadata_end = None::<Position>;

    if let Some(line) = lines.get(cursor)
        && let Some(title) = parse_block_title(line)
    {
        let next = cursor + 1;
        if lines.get(next).is_some_and(|line| {
            parse_attribute_list_line(line).is_some()
                || is_block_delimiter(line)
                || is_block_image_line(line)
        }) {
            let title_line = line_offset + cursor;
            prelude.title = Some(vec![InlineText {
                name: "text",
                node_type: "string",
                value: title.clone(),
                location: [
                    Position {
                        line: title_line,
                        col: 2,
                    },
                    Position {
                        line: title_line,
                        col: lines[cursor].len(),
                    },
                ],
            }]);
            title_raw = Some(title);
            metadata_start = Some(Position {
                line: title_line,
                col: 1,
            });
            metadata_end = Some(Position {
                line: title_line,
                col: lines[cursor].len(),
            });
            cursor += 1;
        }
    }

    if let Some(line) = lines.get(cursor)
        && let Some(entries) = parse_attribute_list_line(line)
    {
        let next = cursor + 1;
        if lines.get(next).is_some_and(|line| !line.trim().is_empty()) {
            let attr_line = line_offset + cursor;
            apply_attribute_list(
                &mut metadata_attributes,
                &mut prelude.id,
                &mut metadata_options,
                &mut metadata_roles,
                &entries,
            );
            metadata_start.get_or_insert(Position {
                line: attr_line,
                col: 1,
            });
            metadata_end = Some(Position {
                line: attr_line,
                col: lines[cursor].len(),
            });
            cursor += 1;
        }
    }

    if let Some(title_raw) = title_raw {
        metadata_attributes.insert("title".into(), title_raw);
    }
    if let Some(id) = &prelude.id {
        metadata_attributes
            .entry("id".into())
            .or_insert_with(|| id.clone());
    }

    prelude.consumed_lines = cursor - index;
    prelude.start = metadata_start.clone();
    if metadata_start.is_some()
        || !metadata_attributes.is_empty()
        || !metadata_options.is_empty()
        || !metadata_roles.is_empty()
    {
        prelude.metadata = Some(AsgBlockMetadata {
            attributes: metadata_attributes,
            options: metadata_options,
            roles: metadata_roles,
            location: [
                metadata_start.unwrap_or(Position {
                    line: line_offset + index,
                    col: 1,
                }),
                metadata_end.unwrap_or(Position {
                    line: line_offset + index,
                    col: 1,
                }),
            ],
        });
    }

    prelude
}

fn parse_block_title(line: &str) -> Option<String> {
    let title = line.strip_prefix('.')?.trim_end();
    (!title.is_empty()).then(|| title.to_owned())
}

fn parse_attribute_list_line(line: &str) -> Option<Vec<String>> {
    let trimmed = line.trim();
    let inner = trimmed.strip_prefix('[')?.strip_suffix(']')?;
    Some(split_attribute_list(inner))
}

fn split_attribute_list(input: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut current = String::new();
    let mut quote = None;

    for ch in input.chars() {
        match ch {
            '\'' | '"' if quote == Some(ch) => {
                quote = None;
                current.push(ch);
            }
            '\'' | '"' if quote.is_none() => {
                quote = Some(ch);
                current.push(ch);
            }
            ',' if quote.is_none() => {
                values.push(current.trim().to_owned());
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    values.push(current.trim().to_owned());
    values
}

fn is_delimited_block_delimiter(line: &str) -> bool {
    parse_delimited_block_marker(line).is_some()
}

fn is_block_delimiter(line: &str) -> bool {
    is_delimited_block_delimiter(line) || parse_table_delimiter(line).is_some()
}

fn is_block_image_line(line: &str) -> bool {
    line.trim_start().starts_with("image::")
}

fn parse_delimited_block_marker(line: &str) -> Option<(&str, &'static str, &'static str)> {
    let trimmed = line.trim();
    if trimmed == "--" {
        return Some((trimmed, "open", "--"));
    }

    if parse_fenced_code_opening(line).is_some() {
        return Some(("```", "listing", "```"));
    }

    let bytes = trimmed.as_bytes();
    if bytes.len() < 4 {
        return None;
    }

    let first = *bytes.first()?;
    if !bytes.iter().all(|byte| *byte == first) {
        return None;
    }

    let (name, canonical_delimiter) = match first {
        b'-' => ("listing", "----"),
        b'=' => ("example", "===="),
        b'*' => ("sidebar", "****"),
        b'+' => ("passthrough", "++++"),
        b'_' => ("quote", "____"),
        b'.' => ("literal", "...."),
        b'/' => ("comment", "////"),
        _ => return None,
    };

    Some((trimmed, name, canonical_delimiter))
}

fn parse_fenced_code_opening(line: &str) -> Option<Vec<String>> {
    let trimmed = line.trim();
    let rest = trimmed.strip_prefix("```")?;
    if rest.starts_with('`') {
        return None;
    }

    let attrs = rest.trim();
    if attrs.is_empty() {
        Some(Vec::new())
    } else {
        Some(split_attribute_list(attrs))
    }
}

fn apply_fenced_code_metadata(
    metadata: &mut Option<AsgBlockMetadata>,
    line: usize,
    end_col: usize,
    entries: &[String],
) {
    let metadata = metadata.get_or_insert_with(|| AsgBlockMetadata {
        attributes: BTreeMap::new(),
        options: Vec::new(),
        roles: Vec::new(),
        location: [Position { line, col: 1 }, Position { line, col: end_col }],
    });
    metadata.attributes.insert("style".into(), "source".into());
    metadata
        .attributes
        .insert("cloaked-context".into(), "fenced_code".into());
    if let Some(language) = entries.first().map(String::as_str).map(str::trim)
        && !language.is_empty()
    {
        metadata.attributes.insert("$1".into(), language.to_owned());
        metadata
            .attributes
            .insert("language".into(), language.to_owned());
    }

    for (index, entry) in entries.iter().enumerate().skip(1) {
        let slot = index + 1;
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }

        metadata
            .attributes
            .insert(format!("${slot}"), entry.to_owned());

        if let Some((name, value)) = parse_named_attribute(entry) {
            metadata.attributes.insert(name.clone(), value.clone());
            if name == "opts" {
                for option in value
                    .split(',')
                    .map(str::trim)
                    .filter(|option| !option.is_empty())
                {
                    if !metadata.options.iter().any(|existing| existing == option) {
                        metadata.options.push(option.to_owned());
                    }
                    metadata
                        .attributes
                        .entry(format!("{option}-option"))
                        .or_default();
                }
            }
            continue;
        }

        if let Some(option_entry) = entry.strip_prefix('%') {
            for option in option_entry
                .split('%')
                .map(str::trim)
                .filter(|option| !option.is_empty())
            {
                if !metadata.options.iter().any(|existing| existing == option) {
                    metadata.options.push(option.to_owned());
                }
                metadata
                    .attributes
                    .entry(format!("{option}-option"))
                    .or_default();
            }
            continue;
        }

        if !metadata.options.iter().any(|existing| existing == entry) {
            metadata.options.push(entry.to_owned());
        }
        metadata
            .attributes
            .entry(format!("{entry}-option"))
            .or_default();
    }
}

fn parse_table_delimiter(line: &str) -> Option<(&str, char, &'static str)> {
    let trimmed = line.trim();
    let mut chars = trimmed.chars();
    let marker = chars.next()?;
    let canonical = match marker {
        '|' => "|===",
        ',' => ",===",
        ':' => ":===",
        '!' => "!===",
        _ => return None,
    };
    let rest = chars.as_str();
    (rest.len() >= 3 && rest.chars().all(|ch| ch == '=')).then_some((trimmed, marker, canonical))
}

fn table_format(metadata: Option<&AsgBlockMetadata>, delimiter_char: char) -> TckTableFormat {
    match metadata
        .and_then(|metadata| metadata.attributes.get("format"))
        .map(|format| format.trim().to_ascii_lowercase())
        .as_deref()
    {
        Some("csv") => TckTableFormat::Csv,
        Some("dsv") => TckTableFormat::Dsv,
        _ => match delimiter_char {
            ',' => TckTableFormat::Csv,
            ':' => TckTableFormat::Dsv,
            _ => TckTableFormat::Psv,
        },
    }
}

fn table_separator(
    metadata: Option<&AsgBlockMetadata>,
    format: TckTableFormat,
    delimiter_char: char,
) -> Option<char> {
    metadata
        .and_then(|metadata| metadata.attributes.get("separator"))
        .and_then(|separator| parse_table_separator_attribute(separator))
        .or(match format {
            TckTableFormat::Csv => Some(','),
            TckTableFormat::Dsv => Some(':'),
            TckTableFormat::Psv => Some(if delimiter_char == '!' { '!' } else { '|' }),
        })
}

fn parse_table_separator_attribute(value: &str) -> Option<char> {
    if value == r"\t" {
        return Some('\t');
    }

    let mut chars = value.chars();
    let separator = chars.next()?;
    chars.next().is_none().then_some(separator)
}

fn starts_table_cell_line(line: &str, separator: char) -> bool {
    parse_leading_table_cell_spec(line.trim_start(), separator).is_some()
}

fn next_nonempty_table_line<'a>(lines: &'a [&str], mut index: usize) -> Option<&'a str> {
    while let Some(line) = lines.get(index) {
        if !line.trim().is_empty() {
            return Some(*line);
        }
        index += 1;
    }
    None
}

fn parse_tck_table_cells_from_line(
    line: &str,
    separator: char,
    line_no: usize,
) -> Option<Vec<ParsedTckTableCell>> {
    let trimmed = line.trim_start();
    let segments = split_table_row_cells_after_marker(trimmed, separator);
    if segments.len() < 2 {
        return None;
    }

    let (colspan, rowspan, style) = parse_table_cell_spec(segments[0].trim())?;
    let mut cells = vec![ParsedTckTableCell {
        content: segments[1].trim().to_owned(),
        colspan,
        rowspan,
        style,
        start_line: line_no,
        end_line: line_no,
    }];

    parse_tck_table_cells_from_segments(&segments, 2, line_no, &mut cells).then_some(cells)
}

fn split_table_row_cells_after_marker(line: &str, separator: char) -> Vec<String> {
    let mut cells = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\\' && chars.peek() == Some(&separator) {
            current.push(separator);
            chars.next();
            continue;
        }

        if ch == separator {
            cells.push(current.trim().to_owned());
            current.clear();
            continue;
        }

        current.push(ch);
    }

    cells.push(current.trim().to_owned());
    cells
}

fn parse_tck_table_cells_from_segments(
    segments: &[String],
    index: usize,
    line_no: usize,
    cells: &mut Vec<ParsedTckTableCell>,
) -> bool {
    if index >= segments.len() {
        return true;
    }

    if index + 1 < segments.len() {
        let spec = segments[index].trim();
        if !spec.is_empty()
            && let Some((colspan, rowspan, style)) = parse_table_cell_spec(spec)
        {
            cells.push(ParsedTckTableCell {
                content: segments[index + 1].trim().to_owned(),
                colspan,
                rowspan,
                style,
                start_line: line_no,
                end_line: line_no,
            });
            if parse_tck_table_cells_from_segments(segments, index + 2, line_no, cells) {
                return true;
            }
            cells.pop();
        }
    }

    cells.push(ParsedTckTableCell {
        content: segments[index].trim().to_owned(),
        colspan: 1,
        rowspan: 1,
        style: None,
        start_line: line_no,
        end_line: line_no,
    });
    if parse_tck_table_cells_from_segments(segments, index + 1, line_no, cells) {
        return true;
    }
    cells.pop();
    false
}

fn parse_leading_table_cell_spec(
    line: &str,
    separator: char,
) -> Option<(usize, usize, Option<String>, &str)> {
    let separator_index = line.find(separator)?;
    let spec = &line[..separator_index];
    let rest = &line[separator_index + separator.len_utf8()..];
    let (colspan, rowspan, style) = parse_table_cell_spec(spec.trim())?;
    Some((colspan, rowspan, style, rest))
}

fn parse_table_cell_spec(spec: &str) -> Option<(usize, usize, Option<String>)> {
    if spec.is_empty() {
        return Some((1, 1, None));
    }

    let style = match spec {
        "a" => return Some((1, 1, Some("asciidoc".into()))),
        "h" => return Some((1, 1, Some("header".into()))),
        _ => None,
    };

    if let Some(rowspan) = spec
        .strip_prefix('.')
        .and_then(|rest| rest.strip_suffix('+'))
    {
        let rowspan = rowspan.parse().ok()?;
        return Some((1, rowspan, style));
    }

    if let Some(span_spec) = spec.strip_suffix('+') {
        if let Some((colspan, rowspan)) = span_spec.split_once('.') {
            let colspan = colspan.parse().ok()?;
            let rowspan = rowspan.parse().ok()?;
            return Some((colspan, rowspan, style));
        }

        let colspan = span_spec.parse().ok()?;
        return Some((colspan, 1, style));
    }

    None
}

fn table_has_header_option(metadata: &AsgBlockMetadata) -> bool {
    metadata.options.iter().any(|option| option == "header")
        || metadata
            .attributes
            .get("options")
            .is_some_and(|options| options.split(',').any(|option| option.trim() == "header"))
        || metadata.attributes.contains_key("header-option")
}

fn table_column_count(metadata: &AsgBlockMetadata) -> Option<usize> {
    let cols = metadata.attributes.get("cols")?;
    let parts = cols.split(',').map(str::trim).collect::<Vec<_>>();
    (!parts.is_empty() && parts.iter().any(|part| !part.is_empty())).then_some(parts.len())
}

fn normalize_table_cell_content(content: &str) -> String {
    content
        .lines()
        .map(|line| if line.trim() == "+" { "" } else { line })
        .collect::<Vec<_>>()
        .join("\n")
}

fn apply_attribute_list(
    attributes: &mut BTreeMap<String, String>,
    id: &mut Option<String>,
    options: &mut Vec<String>,
    roles: &mut Vec<String>,
    entries: &[String],
) {
    for (index, entry) in entries.iter().enumerate() {
        let slot = index + 1;
        if entry.is_empty() {
            continue;
        }

        if let Some((name, value)) = parse_named_attribute(entry) {
            attributes.insert(name.clone(), value.clone());
            if name == "opts" {
                for option in value
                    .split(',')
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    if !options.iter().any(|existing| existing == option) {
                        options.push(option.to_owned());
                    }
                }
            } else if name == "role" {
                for role in value.split_whitespace().filter(|value| !value.is_empty()) {
                    if !roles.iter().any(|existing| existing == role) {
                        roles.push(role.to_owned());
                    }
                }
            }
            continue;
        }

        if let Some(value) = entry.strip_prefix('#') {
            if !value.is_empty() {
                *id = Some(value.to_owned());
                attributes.insert(format!("${slot}"), entry.clone());
            }
            continue;
        }

        if let Some(value) = entry.strip_prefix('.') {
            attributes.insert(format!("${slot}"), entry.clone());
            for role in value
                .split('.')
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                if !roles.iter().any(|existing| existing == role) {
                    roles.push(role.to_owned());
                }
            }
            if !roles.is_empty() {
                attributes.insert("role".into(), roles.join(" "));
            }
            continue;
        }

        if let Some(value) = entry.strip_prefix('%') {
            attributes.insert(format!("${slot}"), entry.clone());
            for option in value
                .split('%')
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                if !options.iter().any(|existing| existing == option) {
                    options.push(option.to_owned());
                }
                attributes.entry(format!("{option}-option")).or_default();
            }
            continue;
        }

        attributes.insert(format!("${slot}"), entry.clone());
        if !attributes.contains_key("style") {
            attributes.insert("style".into(), entry.clone());
        } else if attributes
            .get("style")
            .is_some_and(|style| style == "source")
            && !attributes.contains_key("language")
        {
            attributes.insert("language".into(), entry.clone());
        }
    }

    normalize_source_listing_metadata(attributes, options);
}

fn normalize_source_listing_metadata(
    attributes: &mut BTreeMap<String, String>,
    options: &mut Vec<String>,
) {
    if attributes.get("style").map(String::as_str) != Some("source") {
        return;
    }

    if attributes.contains_key("$3") && !options.iter().any(|option| option == "linenums") {
        options.push("linenums".into());
    }

    let mut normalized_options = Vec::new();
    for option in options.iter() {
        let option = if option == "numbered" {
            "linenums"
        } else {
            option.as_str()
        };
        if !normalized_options.iter().any(|existing| existing == option) {
            normalized_options.push(option.to_owned());
        }
    }
    *options = normalized_options;

    if options.iter().any(|option| option == "linenums") {
        attributes.remove("numbered-option");
        attributes.entry("linenums-option".into()).or_default();
    }
}

fn parse_named_attribute(entry: &str) -> Option<(String, String)> {
    let separator = entry.find('=')?;
    let name = entry[..separator].trim();
    if name.is_empty() {
        return None;
    }
    Some((
        name.to_owned(),
        unquote_attribute_value(entry[separator + 1..].trim()),
    ))
}

fn unquote_attribute_value(value: &str) -> String {
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        let first = bytes[0] as char;
        let last = bytes[value.len() - 1] as char;
        if (first == '"' && last == '"') || (first == '\'' && last == '\'') {
            return value[1..value.len() - 1].to_owned();
        }
    }
    value.to_owned()
}

fn parse_admonition_paragraph(
    lines: &[&str],
    index: usize,
    line_offset: usize,
) -> Option<(AsgBlock, usize)> {
    let (variant, content_col, first_line_content) = parse_admonition_prefix(lines.get(index)?)?;
    let mut paragraph_lines = vec![first_line_content.to_owned()];
    let mut consumed = 1;

    while index + consumed < lines.len() {
        let line = lines[index + consumed];
        if line.trim().is_empty()
            || parse_heading_line(lines, index + consumed, 1).is_some()
            || parse_list_item_line(line).is_some()
            || is_block_delimiter(line)
        {
            break;
        }
        paragraph_lines.push(line.to_owned());
        consumed += 1;
    }

    let start_line = line_offset + index;
    let end_line = start_line + paragraph_lines.len() - 1;
    let end_col = lines[index + consumed - 1].trim_end().len();
    let value = paragraph_lines.join("\n");
    let paragraph = AsgBlock {
        name: "paragraph",
        node_type: "block",
        id: None,
        title: None,
        metadata: None,
        level: None,
        form: None,
        delimiter: None,
        inlines: Some(parse_tck_inlines_at(&value, start_line, content_col)),
        blocks: None,
        variant: None,
        marker: None,
        items: vec![],
        location: [
            Position {
                line: start_line,
                col: 1,
            },
            Position {
                line: end_line,
                col: end_col,
            },
        ],
    };

    Some((
        AsgBlock {
            name: "admonition",
            node_type: "block",
            id: None,
            title: None,
            metadata: None,
            level: None,
            form: Some("paragraph"),
            delimiter: None,
            inlines: None,
            blocks: Some(vec![paragraph]),
            variant: Some(variant),
            marker: None,
            items: vec![],
            location: [
                Position {
                    line: start_line,
                    col: 1,
                },
                Position {
                    line: end_line,
                    col: end_col,
                },
            ],
        },
        consumed,
    ))
}

fn parse_styled_paragraph_block(
    lines: &[&str],
    index: usize,
    line_offset: usize,
) -> Option<(AsgBlock, usize)> {
    let prelude = parse_block_prelude(lines, index, line_offset);
    if prelude.consumed_lines == 0 {
        return None;
    }
    let variant = prelude
        .metadata
        .as_ref()
        .and_then(|metadata| metadata.attributes.get("style"))
        .cloned()?;
    let paragraph_index = index + prelude.consumed_lines;
    let line = *lines.get(paragraph_index)?;
    if line.trim().is_empty() || is_block_delimiter(line) {
        return None;
    }

    let mut paragraph_lines = vec![line.to_owned()];
    let mut consumed = prelude.consumed_lines + 1;
    let mut cursor = paragraph_index + 1;

    while let Some(line) = lines.get(cursor) {
        if line.trim().is_empty()
            || parse_heading_line(lines, cursor, 1).is_some()
            || parse_list_item_line(line).is_some()
            || is_block_delimiter(line)
        {
            break;
        }
        paragraph_lines.push((*line).to_owned());
        cursor += 1;
        consumed += 1;
    }

    let start_line = line_offset + paragraph_index;
    let end_line = start_line + paragraph_lines.len() - 1;
    let end_col = lines[index + consumed - 1].trim_end().len();
    let value = paragraph_lines.join("\n");
    let paragraph = AsgBlock {
        name: "paragraph",
        node_type: "block",
        id: None,
        title: None,
        metadata: None,
        level: None,
        form: None,
        delimiter: None,
        inlines: Some(parse_tck_inlines_at(&value, start_line, 1)),
        blocks: None,
        variant: None,
        marker: None,
        items: vec![],
        location: [
            Position {
                line: start_line,
                col: 1,
            },
            Position {
                line: end_line,
                col: end_col,
            },
        ],
    };

    let style = variant;
    let block = match style.as_str() {
        "listing" | "source" => AsgBlock {
            name: "listing",
            node_type: "block",
            id: prelude.id.clone(),
            title: prelude.title,
            metadata: prelude.metadata,
            level: None,
            form: Some("paragraph"),
            delimiter: None,
            inlines: Some(vec![AsgInline::Text(InlineText {
                name: "text",
                node_type: "string",
                value,
                location: [
                    Position {
                        line: start_line,
                        col: 1,
                    },
                    Position {
                        line: end_line,
                        col: end_col,
                    },
                ],
            })]),
            blocks: None,
            variant: None,
            marker: None,
            items: vec![],
            location: [
                Position {
                    line: start_line,
                    col: 1,
                },
                Position {
                    line: end_line,
                    col: end_col,
                },
            ],
        },
        "quote" => AsgBlock {
            name: "quote",
            node_type: "block",
            id: prelude.id.clone(),
            title: prelude.title,
            metadata: prelude.metadata,
            level: None,
            form: Some("paragraph"),
            delimiter: None,
            inlines: None,
            blocks: Some(vec![paragraph]),
            variant: None,
            marker: None,
            items: vec![],
            location: [
                Position {
                    line: start_line,
                    col: 1,
                },
                Position {
                    line: end_line,
                    col: end_col,
                },
            ],
        },
        "pass" => AsgBlock {
            name: "passthrough",
            node_type: "block",
            id: prelude.id.clone(),
            title: prelude.title,
            metadata: prelude.metadata,
            level: None,
            form: Some("paragraph"),
            delimiter: None,
            inlines: Some(vec![AsgInline::Text(InlineText {
                name: "text",
                node_type: "string",
                value,
                location: [
                    Position {
                        line: start_line,
                        col: 1,
                    },
                    Position {
                        line: end_line,
                        col: end_col,
                    },
                ],
            })]),
            blocks: None,
            variant: None,
            marker: None,
            items: vec![],
            location: [
                Position {
                    line: start_line,
                    col: 1,
                },
                Position {
                    line: end_line,
                    col: end_col,
                },
            ],
        },
        _ => {
            let admonition_variant = admonition_variant_from_style(&style)?;
            AsgBlock {
                name: "admonition",
                node_type: "block",
                id: prelude.id.clone(),
                title: prelude.title,
                metadata: prelude.metadata,
                level: None,
                form: Some("paragraph"),
                delimiter: None,
                inlines: None,
                blocks: Some(vec![paragraph]),
                variant: Some(admonition_variant),
                marker: None,
                items: vec![],
                location: [
                    Position {
                        line: start_line,
                        col: 1,
                    },
                    Position {
                        line: end_line,
                        col: end_col,
                    },
                ],
            }
        }
    };

    Some((block, consumed))
}

fn parse_admonition_prefix(line: &str) -> Option<(&'static str, usize, &str)> {
    let trimmed = line.trim_start();
    let leading_ws = line.len() - trimmed.len();
    for (prefix, variant) in [
        ("NOTE:", "note"),
        ("TIP:", "tip"),
        ("IMPORTANT:", "important"),
        ("CAUTION:", "caution"),
        ("WARNING:", "warning"),
    ] {
        let Some(remainder) = trimmed.strip_prefix(prefix) else {
            continue;
        };
        if !remainder.starts_with(char::is_whitespace) {
            continue;
        }
        let content = remainder.trim();
        if content.is_empty() {
            continue;
        }
        return Some((variant, leading_ws + prefix.len() + 2, content));
    }
    None
}

fn admonition_variant_from_style(style: &str) -> Option<&'static str> {
    if style.eq_ignore_ascii_case("NOTE") {
        Some("note")
    } else if style.eq_ignore_ascii_case("TIP") {
        Some("tip")
    } else if style.eq_ignore_ascii_case("IMPORTANT") {
        Some("important")
    } else if style.eq_ignore_ascii_case("CAUTION") {
        Some("caution")
    } else if style.eq_ignore_ascii_case("WARNING") {
        Some("warning")
    } else {
        None
    }
}

fn parse_block_anchor(line: &str) -> Option<PendingBlockAnchor> {
    let trimmed = line.trim();

    if let Some(inner) = trimmed
        .strip_prefix("[[")
        .and_then(|rest| rest.strip_suffix("]]"))
    {
        let id = inner
            .split_once(',')
            .map(|(id, _)| id)
            .unwrap_or(inner)
            .trim();
        if !id.is_empty() {
            return Some(PendingBlockAnchor { id: id.to_owned() });
        }
    }

    if let Some(inner) = trimmed
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
    {
        for part in split_attribute_list(inner) {
            if let Some(id) = part.strip_prefix('#') {
                let id = id.trim();
                if !id.is_empty() {
                    return Some(PendingBlockAnchor { id: id.to_owned() });
                }
            }
        }
    }

    None
}

fn flush_paragraph(
    blocks: &mut Vec<AsgBlock>,
    paragraph_start: &mut Option<usize>,
    paragraph_lines: &mut Vec<String>,
    _line_offset: usize,
    last_end: &mut Option<Position>,
) {
    let Some(start_index) = paragraph_start.take() else {
        return;
    };

    let value = paragraph_lines.join("\n");
    let start = Position {
        line: start_index + 1,
        col: 1,
    };
    let end = Position {
        line: start_index + paragraph_lines.len(),
        col: paragraph_lines.last().map(|line| line.len()).unwrap_or(1),
    };
    blocks.push(AsgBlock {
        name: "paragraph",
        node_type: "block",
        id: None,
        title: None,
        metadata: None,
        level: None,
        form: None,
        delimiter: None,
        inlines: Some(parse_tck_inlines_at(&value, start.line, start.col)),
        blocks: None,
        variant: None,
        marker: None,
        items: vec![],
        location: [start, end.clone()],
    });
    *last_end = Some(end);
    paragraph_lines.clear();
}

fn parse_list_item_line(line: &str) -> Option<TckListMarker<'_>> {
    let trimmed = line.trim_start();
    let first = trimmed.chars().next()?;

    match first {
        '*' | '-' => {
            let level = trimmed.chars().take_while(|&ch| ch == first).count();
            let remainder = &trimmed[level..];
            parse_list_content(remainder).map(|content| TckListMarker {
                kind: TckListKind::Unordered,
                level,
                marker: if first == '*' { "*" } else { "-" },
                content,
            })
        }
        '.' => {
            let level = trimmed.chars().take_while(|&ch| ch == '.').count();
            let remainder = &trimmed[level..];
            parse_list_content(remainder).map(|content| TckListMarker {
                kind: TckListKind::Ordered,
                level,
                marker: ".",
                content,
            })
        }
        ch if ch.is_ascii_digit() => {
            let digits = trimmed.chars().take_while(|ch| ch.is_ascii_digit()).count();
            let remainder = trimmed.get(digits..)?;
            let remainder = remainder.strip_prefix('.')?;
            parse_list_content(remainder).map(|content| TckListMarker {
                kind: TckListKind::Ordered,
                level: 1,
                marker: ".",
                content,
            })
        }
        _ => None,
    }
}

fn parse_list_content(remainder: &str) -> Option<&str> {
    if !remainder.starts_with(char::is_whitespace) {
        return None;
    }

    let content = remainder.trim();
    if content.is_empty() {
        return None;
    }

    Some(content)
}

fn is_comment_line(line: &str) -> bool {
    line.trim_start().starts_with("//") && parse_delimited_block_marker(line).is_none()
}

fn skip_header_comments(lines: &[&str], mut index: usize) -> usize {
    while index < lines.len() && is_comment_line(lines[index]) {
        index += 1;
    }
    index
}

fn parse_attribute_entry(line: &str) -> Option<(String, String, usize)> {
    let stripped = line.strip_prefix(':')?;
    let separator = stripped.find(':')?;
    let name = stripped[..separator].trim();
    let value = stripped[separator + 1..].trim_start().to_owned();
    if name.is_empty() {
        return None;
    }
    Some((name.to_owned(), value, line.len()))
}

fn parse_attribute_entry_at(
    lines: &[&str],
    index: usize,
) -> Option<(String, String, usize, usize)> {
    let line = *lines.get(index)?;
    let stripped = line.strip_prefix(':')?;
    let separator = stripped.find(':')?;
    let name = stripped[..separator].trim();
    if name.is_empty() {
        return None;
    }

    let mut value = String::new();
    let mut consumed_lines = 0;
    let mut segment = stripped[separator + 1..].trim_start();

    loop {
        consumed_lines += 1;
        let has_next = index + consumed_lines < lines.len();

        if let Some((continued, hard_wrap)) =
            parse_attribute_continuation_segment(segment, has_next)
        {
            value.push_str(continued);
            value.push(if hard_wrap { '\n' } else { ' ' });
            segment = lines[index + consumed_lines].trim_start();
            continue;
        }

        value.push_str(segment);
        return Some((
            name.to_owned(),
            value,
            consumed_lines,
            lines[index + consumed_lines - 1].len(),
        ));
    }
}

fn parse_attribute_continuation_segment<'a>(
    segment: &'a str,
    has_next: bool,
) -> Option<(&'a str, bool)> {
    if !has_next {
        return None;
    }

    let trimmed = segment.trim_end();
    let continued = trimmed.strip_suffix(" \\")?;
    Some((continued, continued.ends_with(" +")))
}

fn parse_implicit_author_line(
    lines: &[&str],
    index: usize,
    line: &str,
) -> Option<ImplicitAuthorLine> {
    let trimmed = line.trim();
    if trimmed.is_empty()
        || is_comment_line(line)
        || parse_attribute_entry(line).is_some()
        || parse_heading_line(lines, index, 1).is_some()
        || parse_implicit_revision_line(line).is_some()
    {
        return None;
    }

    let authors = trimmed
        .split(';')
        .filter_map(parse_implicit_author_entry)
        .collect::<Vec<_>>();

    if authors.is_empty() {
        return None;
    }

    Some(ImplicitAuthorLine { authors })
}

fn parse_implicit_author_entry(entry: &str) -> Option<ImplicitAuthor> {
    let trimmed = entry.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some(without_close) = trimmed.strip_suffix('>') {
        if let Some(open_index) = without_close.rfind('<') {
            let name = without_close[..open_index].trim();
            let email = without_close[open_index + 1..].trim();
            if !name.is_empty() && !email.is_empty() {
                return build_author(name, Some(email.to_owned()));
            }
        }
    }

    build_author(trimmed, None)
}

fn insert_author_attributes(attributes: &mut BTreeMap<String, String>, authors: &[ImplicitAuthor]) {
    if authors.is_empty() {
        return;
    }

    attributes.insert(
        "authors".to_owned(),
        authors
            .iter()
            .map(|author| author.name.as_str())
            .collect::<Vec<_>>()
            .join(", "),
    );
    attributes.insert("authorcount".to_owned(), authors.len().to_string());
    insert_primary_author_attributes(attributes, &authors[0], authors.len() == 1, false);

    if authors.len() > 1 {
        insert_indexed_author_attributes(attributes, &authors[0], 1);
        for (index, author) in authors.iter().enumerate() {
            if index == 0 {
                continue;
            }
            insert_indexed_author_attributes(attributes, author, index + 1);
        }
    }
}

fn insert_primary_author_attributes(
    attributes: &mut BTreeMap<String, String>,
    author: &ImplicitAuthor,
    preserve_existing_initials: bool,
    preserve_existing_email: bool,
) {
    attributes.insert("author".to_owned(), author.name.clone());
    attributes.insert("firstname".to_owned(), author.firstname.clone());
    if let Some(middlename) = &author.middlename {
        attributes.insert("middlename".to_owned(), middlename.clone());
    }
    if let Some(lastname) = &author.lastname {
        attributes.insert("lastname".to_owned(), lastname.clone());
    }
    if !(preserve_existing_initials && attributes.contains_key("authorinitials")) {
        attributes.insert("authorinitials".to_owned(), author.authorinitials.clone());
    }
    if let Some(email) = &author.email {
        if !preserve_existing_email || !attributes.contains_key("email") {
            attributes.insert("email".to_owned(), email.clone());
        }
    }
}

fn insert_indexed_author_attributes(
    attributes: &mut BTreeMap<String, String>,
    author: &ImplicitAuthor,
    index: usize,
) {
    attributes.insert(format!("author_{index}"), author.name.clone());
    attributes.insert(format!("firstname_{index}"), author.firstname.clone());
    if let Some(middlename) = &author.middlename {
        attributes.insert(format!("middlename_{index}"), middlename.clone());
    }
    if let Some(lastname) = &author.lastname {
        attributes.insert(format!("lastname_{index}"), lastname.clone());
    }
    attributes.insert(
        format!("authorinitials_{index}"),
        author.authorinitials.clone(),
    );
    if let Some(email) = &author.email {
        attributes.insert(format!("email_{index}"), email.clone());
    }
}

fn normalize_explicit_author_attributes(
    attributes: &mut BTreeMap<String, String>,
    source_key: &str,
    preserve_primary_initials: bool,
) {
    let explicit_primary_initials = preserve_primary_initials
        .then(|| attributes.get("authorinitials").cloned())
        .flatten();
    let source_value = attributes.get(source_key).cloned().unwrap_or_default();
    let mut authors = if source_key == "authors" {
        source_value
            .split(';')
            .filter_map(|entry| build_author(entry, None))
            .collect::<Vec<_>>()
    } else {
        match build_author(&source_value, attributes.get("email").cloned()) {
            Some(author) => vec![author],
            None => Vec::new(),
        }
    };

    if authors.is_empty() {
        return;
    }

    if source_key == "authors" {
        for (index, author) in authors.iter_mut().enumerate() {
            if let Some(email) = attributes.get(&format!("email_{}", index + 1)).cloned() {
                author.email = Some(email);
            }
        }
    }

    clear_derived_author_attributes(attributes, preserve_primary_initials && authors.len() == 1);
    insert_author_attributes(attributes, &authors);
    if let Some(authorinitials) = explicit_primary_initials.filter(|_| authors.len() == 1) {
        attributes.insert("authorinitials".to_owned(), authorinitials);
    }
}

fn clear_derived_author_attributes(
    attributes: &mut BTreeMap<String, String>,
    preserve_primary_initials: bool,
) {
    let mut keys_to_remove = Vec::new();
    for key in attributes.keys() {
        let remove = key == "authorcount"
            || key == "firstname"
            || key == "middlename"
            || key == "lastname"
            || key == "email"
            || key.starts_with("author_")
            || key.starts_with("firstname_")
            || key.starts_with("middlename_")
            || key.starts_with("lastname_")
            || key.starts_with("authorinitials_")
            || key.starts_with("email_")
            || (!preserve_primary_initials && key == "authorinitials");
        if remove {
            keys_to_remove.push(key.clone());
        }
    }
    for key in keys_to_remove {
        attributes.remove(&key);
    }
}

fn build_author(name: &str, email: Option<String>) -> Option<ImplicitAuthor> {
    let normalized_name = name.replace('_', " ");
    let segments = normalized_name
        .split_whitespace()
        .map(str::to_owned)
        .collect::<Vec<_>>();

    if segments.is_empty() {
        return None;
    }

    let firstname = segments[0].clone();
    let middlename = if segments.len() > 2 {
        Some(segments[1].clone())
    } else {
        None
    };
    let lastname = if segments.len() == 2 {
        Some(segments[1].clone())
    } else if segments.len() > 2 {
        Some(segments[2..].join(" "))
    } else {
        None
    };
    let authorinitials = [
        Some(firstname.as_str()),
        middlename.as_deref(),
        lastname.as_deref(),
    ]
    .into_iter()
    .flatten()
    .filter_map(|part| part.chars().next())
    .collect::<String>();
    let display_name = match (&middlename, &lastname) {
        (Some(middlename), Some(lastname)) => format!("{firstname} {middlename} {lastname}"),
        (None, Some(lastname)) => format!("{firstname} {lastname}"),
        _ => firstname.clone(),
    };

    Some(ImplicitAuthor {
        name: display_name,
        firstname,
        middlename,
        lastname,
        authorinitials,
        email,
    })
}

fn parse_implicit_revision_line(line: &str) -> Option<ImplicitRevisionLine> {
    let trimmed = line.trim();
    let remainder = trimmed
        .strip_prefix('v')
        .or_else(|| trimmed.strip_prefix('V'))?;

    let (number_and_date, remark) = match remainder.split_once(':') {
        Some((value, remark)) => (value.trim_end(), Some(remark.trim())),
        None => (remainder, None),
    };

    let (number, date) = match number_and_date.split_once(',') {
        Some((number, date)) => (number.trim(), Some(date.trim())),
        None => (number_and_date.trim(), None),
    };

    if number.is_empty() || number.chars().any(char::is_whitespace) {
        return None;
    }

    Some(ImplicitRevisionLine {
        number: number.to_owned(),
        date: date.filter(|value| !value.is_empty()).map(str::to_owned),
        remark: remark.filter(|value| !value.is_empty()).map(str::to_owned),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ImplicitAuthor {
    name: String,
    firstname: String,
    middlename: Option<String>,
    lastname: Option<String>,
    authorinitials: String,
    email: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ImplicitAuthorLine {
    authors: Vec<ImplicitAuthor>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ImplicitRevisionLine {
    number: String,
    date: Option<String>,
    remark: Option<String>,
}

fn parse_heading_line(
    lines: &[&str],
    index: usize,
    line_offset: usize,
) -> Option<(HeadingParse, [Position; 2], usize)> {
    parse_atx_heading_line(lines[index], line_offset + index)
        .map(|(heading, range)| (heading, range, 1))
        .or_else(|| parse_setext_heading_line(lines, index, line_offset))
}

fn parse_atx_heading_line(line: &str, line_no: usize) -> Option<(HeadingParse, [Position; 2])> {
    let marker = line.chars().next()?;
    if marker != '=' && marker != '#' {
        return None;
    }

    let marker_len = line.chars().take_while(|&ch| ch == marker).count();
    let remainder = &line[marker_len..];
    if !remainder.starts_with(' ') {
        return None;
    }

    let title = remainder
        .trim()
        .trim_end_matches(marker)
        .trim_end()
        .to_owned();
    if title.is_empty() {
        return None;
    }

    Some((
        HeadingParse {
            level: (marker_len - 1) as u8,
            title,
            marker_len,
        },
        [
            Position {
                line: line_no,
                col: 1,
            },
            Position {
                line: line_no,
                col: line.len(),
            },
        ],
    ))
}

fn parse_setext_heading_line(
    lines: &[&str],
    index: usize,
    line_offset: usize,
) -> Option<(HeadingParse, [Position; 2], usize)> {
    let title_line = *lines.get(index)?;
    let title = title_line.trim();
    if title.is_empty()
        || !title.chars().any(char::is_alphanumeric)
        || parse_attribute_list_line(title).is_some()
    {
        return None;
    }
    let underline = lines.get(index + 1)?.trim();
    let marker = underline.chars().next()?;
    if (marker != '=' && marker != '-') || !underline.chars().all(|ch| ch == marker) {
        return None;
    }

    Some((
        HeadingParse {
            level: if marker == '=' { 0 } else { 1 },
            title: title_line.trim().to_owned(),
            marker_len: 1,
        },
        [
            Position {
                line: line_offset + index,
                col: 1,
            },
            Position {
                line: line_offset + index,
                col: title.len(),
            },
        ],
        2,
    ))
}

#[derive(Debug)]
struct HeadingParse {
    level: u8,
    title: String,
    marker_len: usize,
}

fn parse_tck_inlines_at(input: &str, start_line: usize, start_col: usize) -> Vec<AsgInline> {
    let line_starts = compute_line_starts(input);
    parse_spanned_inlines(input)
        .into_iter()
        .map(|inline| {
            map_inline(
                &inline.inline,
                inline.start,
                inline.end,
                input,
                &line_starts,
                start_line,
                start_col,
            )
        })
        .collect()
}

fn map_inline(
    inline: &Inline,
    start: usize,
    end: usize,
    source: &str,
    line_starts: &[usize],
    base_line: usize,
    base_col: usize,
) -> AsgInline {
    match inline {
        Inline::Text(value) => AsgInline::Text(InlineText {
            name: "text",
            node_type: "string",
            value: value.clone(),
            location: [
                offset_to_position(start, line_starts, base_line, base_col),
                offset_to_end_position(end, line_starts, base_line, base_col),
            ],
        }),
        Inline::Span(span) => {
            let child_source = &source[start..end];
            let child_line_starts = compute_line_starts(child_source);
            let child_base = offset_to_position(
                start + span_delimiter_len(span.form),
                line_starts,
                base_line,
                base_col,
            );

            AsgInline::Span(InlineSpanNode {
                name: "span",
                node_type: "inline",
                variant: match span.variant {
                    InlineVariant::Strong => "strong",
                    InlineVariant::Emphasis => "emphasis",
                    InlineVariant::Monospace => "monospace",
                    InlineVariant::Subscript => "subscript",
                    InlineVariant::Superscript => "superscript",
                },
                form: match span.form {
                    InlineForm::Constrained => "constrained",
                    InlineForm::Unconstrained => "unconstrained",
                },
                inlines: parse_spanned_inlines(
                    &child_source[span_delimiter_len(span.form)
                        ..child_source.len() - span_delimiter_len(span.form)],
                )
                .into_iter()
                .map(|child| {
                    map_inline(
                        &child.inline,
                        child.start,
                        child.end,
                        &child_source[span_delimiter_len(span.form)
                            ..child_source.len() - span_delimiter_len(span.form)],
                        &child_line_starts,
                        child_base.line,
                        child_base.col,
                    )
                })
                .collect(),
                location: [
                    offset_to_position(start, line_starts, base_line, base_col),
                    offset_to_end_position(end, line_starts, base_line, base_col),
                ],
            })
        }
        Inline::Link(link) => AsgInline::Span(InlineSpanNode {
            name: "link",
            node_type: "inline",
            variant: "link",
            form: if link.bare { "bare" } else { "macro" },
            inlines: link
                .text
                .iter()
                .enumerate()
                .map(|(idx, child)| {
                    let child_text = child.plain_text();
                    let child_start = link
                        .text
                        .iter()
                        .take(idx)
                        .map(Inline::plain_text)
                        .collect::<String>()
                        .len();
                    map_inline(
                        child,
                        child_start,
                        child_start + child_text.len(),
                        &link.text.iter().map(Inline::plain_text).collect::<String>(),
                        &compute_line_starts(
                            &link.text.iter().map(Inline::plain_text).collect::<String>(),
                        ),
                        base_line,
                        base_col,
                    )
                })
                .collect(),
            location: [
                offset_to_position(start, line_starts, base_line, base_col),
                offset_to_end_position(end, line_starts, base_line, base_col),
            ],
        }),
        Inline::Xref(xref) => AsgInline::Span(InlineSpanNode {
            name: "xref",
            node_type: "inline",
            variant: "xref",
            form: if xref.shorthand { "shorthand" } else { "macro" },
            inlines: xref
                .text
                .iter()
                .enumerate()
                .map(|(idx, child)| {
                    let child_text = child.plain_text();
                    let child_start = xref
                        .text
                        .iter()
                        .take(idx)
                        .map(Inline::plain_text)
                        .collect::<String>()
                        .len();
                    map_inline(
                        child,
                        child_start,
                        child_start + child_text.len(),
                        &xref.text.iter().map(Inline::plain_text).collect::<String>(),
                        &compute_line_starts(
                            &xref.text.iter().map(Inline::plain_text).collect::<String>(),
                        ),
                        base_line,
                        base_col,
                    )
                })
                .collect(),
            location: [
                offset_to_position(start, line_starts, base_line, base_col),
                offset_to_end_position(end, line_starts, base_line, base_col),
            ],
        }),
        Inline::Anchor(anchor) => AsgInline::Span(InlineSpanNode {
            name: "anchor",
            node_type: "inline",
            variant: "anchor",
            form: "point",
            inlines: anchor
                .reftext
                .as_ref()
                .map(|reftext| {
                    vec![AsgInline::Text(InlineText {
                        name: "text",
                        node_type: "string",
                        value: reftext.clone(),
                        location: [
                            offset_to_position(start, line_starts, base_line, base_col),
                            offset_to_end_position(end, line_starts, base_line, base_col),
                        ],
                    })]
                })
                .unwrap_or_default(),
            location: [
                offset_to_position(start, line_starts, base_line, base_col),
                offset_to_end_position(end, line_starts, base_line, base_col),
            ],
        }),
        Inline::Passthrough(value) => AsgInline::Text(InlineText {
            name: "text",
            node_type: "string",
            value: value.clone(),
            location: [
                offset_to_position(start, line_starts, base_line, base_col),
                offset_to_end_position(end, line_starts, base_line, base_col),
            ],
        }),
        Inline::Image(image) => AsgInline::Text(InlineText {
            name: "text",
            node_type: "string",
            value: image.alt.clone(),
            location: [
                offset_to_position(start, line_starts, base_line, base_col),
                offset_to_end_position(end, line_starts, base_line, base_col),
            ],
        }),
        Inline::Icon(icon) => AsgInline::Text(InlineText {
            name: "text",
            node_type: "string",
            value: icon.name.clone(),
            location: [
                offset_to_position(start, line_starts, base_line, base_col),
                offset_to_end_position(end, line_starts, base_line, base_col),
            ],
        }),
        Inline::Footnote(footnote) => AsgInline::Span(InlineSpanNode {
            name: "footnote",
            node_type: "inline",
            variant: "footnote",
            form: "macro",
            inlines: footnote
                .inlines
                .iter()
                .enumerate()
                .map(|(idx, child)| {
                    let child_text = child.plain_text();
                    let child_start = footnote
                        .inlines
                        .iter()
                        .take(idx)
                        .map(Inline::plain_text)
                        .collect::<String>()
                        .len();
                    map_inline(
                        child,
                        child_start,
                        child_start + child_text.len(),
                        &footnote
                            .inlines
                            .iter()
                            .map(Inline::plain_text)
                            .collect::<String>(),
                        &compute_line_starts(
                            &footnote
                                .inlines
                                .iter()
                                .map(Inline::plain_text)
                                .collect::<String>(),
                        ),
                        base_line,
                        base_col,
                    )
                })
                .collect(),
            location: [
                offset_to_position(start, line_starts, base_line, base_col),
                offset_to_end_position(end, line_starts, base_line, base_col),
            ],
        }),
    }
}

fn span_delimiter_len(form: InlineForm) -> usize {
    match form {
        InlineForm::Constrained => 1,
        InlineForm::Unconstrained => 2,
    }
}

fn compute_line_starts(source: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (index, ch) in source.chars().enumerate() {
        if ch == '\n' {
            starts.push(index + 1);
        }
    }
    starts
}

fn offset_to_position(
    offset: usize,
    line_starts: &[usize],
    base_line: usize,
    base_col: usize,
) -> Position {
    let line_index = line_starts
        .iter()
        .rposition(|&start| start <= offset)
        .unwrap_or(0);
    Position {
        line: base_line + line_index,
        col: if line_index == 0 {
            base_col + (offset - line_starts[line_index])
        } else {
            offset - line_starts[line_index] + 1
        },
    }
}

fn offset_to_end_position(
    offset: usize,
    line_starts: &[usize],
    base_line: usize,
    base_col: usize,
) -> Position {
    offset_to_position(offset.saturating_sub(1), line_starts, base_line, base_col)
}

#[cfg(test)]
mod tests {
    use crate::tck::{
        AsgInline, parse_tck_document, parse_tck_inlines, render_tck_json_from_request,
    };

    #[test]
    fn renders_ordered_list_block() {
        let document = parse_tck_document(". item one");
        let json = serde_json::to_string_pretty(&document).expect("json");

        assert!(json.contains("\"name\": \"list\""));
        assert!(json.contains("\"variant\": \"ordered\""));
        assert!(json.contains("\"marker\": \".\""));
        assert!(json.contains("\"name\": \"listItem\""));
        assert!(json.contains("\"value\": \"item one\""));
    }

    #[test]
    fn renders_numeric_ordered_list_block() {
        let document = parse_tck_document("1. item one");
        let json = serde_json::to_string_pretty(&document).expect("json");

        assert!(json.contains("\"variant\": \"ordered\""));
        assert!(json.contains("\"marker\": \".\""));
        assert!(json.contains("\"value\": \"item one\""));
    }

    #[test]
    fn renders_unordered_list_block() {
        let document = parse_tck_document("* item one");
        let json = serde_json::to_string_pretty(&document).expect("json");

        assert!(json.contains("\"name\": \"list\""));
        assert!(json.contains("\"variant\": \"unordered\""));
        assert!(json.contains("\"marker\": \"*\""));
        assert!(json.contains("\"value\": \"item one\""));
    }

    #[test]
    fn renders_tck_document_with_header_and_paragraph() {
        let document = parse_tck_document("= Document Title\n\nbody");
        let json = serde_json::to_string_pretty(&document).expect("json");

        assert!(json.contains("\"attributes\": {}"));
        assert!(json.contains("\"header\""));
        assert!(json.contains("\"value\": \"Document Title\""));
        assert!(json.contains("\"name\": \"paragraph\""));
        assert!(json.contains("\"value\": \"body\""));
    }

    #[test]
    fn normalizes_trailing_spaces_in_tck_document_parsing() {
        let document = parse_tck_document("body  \r\nmore\t\r\n");
        let block = document.blocks.first().expect("paragraph");
        let text = block
            .inlines
            .as_ref()
            .and_then(|inlines| inlines.first())
            .expect("text");

        let AsgInline::Text(text) = text else {
            panic!("expected text");
        };
        assert_eq!(text.value, "body\nmore");
    }

    #[test]
    fn omits_empty_document_attributes_when_no_header_is_present() {
        let document = parse_tck_document("body");
        let json = serde_json::to_string_pretty(&document).expect("json");

        assert!(!json.contains("\"attributes\": {}"));
        assert!(json.contains("\"name\": \"paragraph\""));
    }

    #[test]
    fn ignores_header_comments_in_tck_document_parsing() {
        let document =
            parse_tck_document("// comment\n= Document Title\n// note\n:toc: left\n\nbody");

        assert_eq!(
            document
                .header
                .as_ref()
                .and_then(|header| header.title.first())
                .map(|title| title.value.as_str()),
            Some("Document Title")
        );
        assert_eq!(
            document.attributes.get("toc").map(String::as_str),
            Some("left")
        );
        assert_eq!(document.blocks.len(), 1);
    }

    #[test]
    fn parses_multiline_header_attribute_with_soft_wraps_in_tck_document() {
        let document = parse_tck_document(
            "= Document Title\n:description: If you have a very long line of text \\\nthat you need to substitute regularly in a document, \\\n  you may find it easier to split the value neatly.\n\nbody",
        );

        assert_eq!(
            document.attributes.get("description").map(String::as_str),
            Some(
                "If you have a very long line of text that you need to substitute regularly in a document, you may find it easier to split the value neatly."
            )
        );
    }

    #[test]
    fn parses_multiline_header_attribute_with_hard_wraps_in_tck_document() {
        let document = parse_tck_document(
            "= Document Title\n:haiku: Write your docs in text, + \\\n  AsciiDoc makes it easy, + \\\n  Now get back to work!\n\nbody",
        );

        assert_eq!(
            document.attributes.get("haiku").map(String::as_str),
            Some("Write your docs in text, +\nAsciiDoc makes it easy, +\nNow get back to work!")
        );
    }

    #[test]
    fn strips_line_comments_in_body() {
        // "= Title" becomes the document header; body blocks are at document.blocks directly
        let document = parse_tck_document("= Title\n\n// invisible comment\n\nVisible paragraph.");

        // Should have exactly one body block (the paragraph), not two
        assert_eq!(document.blocks.len(), 1);
        assert_eq!(document.blocks[0].name, "paragraph");
    }

    #[test]
    fn strips_block_comments_in_body() {
        let document = parse_tck_document(
            "= Title\n\nBefore.\n\n////\nThis is invisible.\nSo is this.\n////\n\nAfter.",
        );

        // Should have exactly two body paragraphs: "Before." and "After."
        assert_eq!(document.blocks.len(), 2);
        assert_eq!(document.blocks[0].name, "paragraph");
        assert_eq!(document.blocks[1].name, "paragraph");
    }

    #[test]
    fn preserves_comments_inside_listing_block() {
        let document = parse_tck_document("= Title\n\n----\n// keep this line\ncode here\n----");

        // One body block: the listing
        assert_eq!(document.blocks.len(), 1);
        assert_eq!(document.blocks[0].name, "listing");
        // The // line must be preserved as content inside the listing
        let inline_value = document.blocks[0]
            .inlines
            .as_ref()
            .and_then(|i| i.first())
            .map(|i| match i {
                AsgInline::Text(t) => t.value.as_str(),
                _ => "",
            })
            .unwrap_or("");
        assert!(
            inline_value.contains("// keep this line"),
            "listing content should preserve // lines, got: {inline_value:?}"
        );
    }

    #[test]
    fn parses_top_level_attributes_without_header_in_tck_document() {
        let document =
            parse_tck_document(":icons:\n:iconsdir: /site/icons\n\nTIP: Ship it carefully.");

        assert!(document.header.is_none());
        assert_eq!(
            document.attributes.get("icons").map(String::as_str),
            Some("")
        );
        assert_eq!(
            document.attributes.get("iconsdir").map(String::as_str),
            Some("/site/icons")
        );
        assert_eq!(document.blocks.len(), 1);
    }

    #[test]
    fn parses_body_attributes_before_later_blocks_in_tck_document() {
        let document = parse_tck_document(
            "= Demo\n\nIntro paragraph.\n\n:icons:\n:iconsdir: /site/icons\n\nTIP: Ship it carefully.",
        );

        assert_eq!(
            document.attributes.get("icons").map(String::as_str),
            Some("")
        );
        assert_eq!(
            document.attributes.get("iconsdir").map(String::as_str),
            Some("/site/icons")
        );
        assert_eq!(document.blocks.len(), 2);
    }

    #[test]
    fn parses_multiline_body_attribute_before_later_blocks_in_tck_document() {
        let document = parse_tck_document(
            "= Demo\n\nIntro paragraph.\n\n:description: first segment \\\n  second segment\n\nTIP: Ship it carefully.",
        );

        assert_eq!(
            document.attributes.get("description").map(String::as_str),
            Some("first segment second segment")
        );
        assert_eq!(document.blocks.len(), 2);
    }

    #[test]
    fn parses_implicit_metadata_in_tck_document_parsing() {
        let document = parse_tck_document(
            "= Document Title\nStuart Rackham <founder@asciidoc.org>\nv1.0, 2001-01-01\n:toc: left\n\nbody",
        );

        assert_eq!(
            document.attributes.get("author").map(String::as_str),
            Some("Stuart Rackham")
        );
        assert_eq!(
            document.attributes.get("email").map(String::as_str),
            Some("founder@asciidoc.org")
        );
        assert_eq!(
            document.attributes.get("revnumber").map(String::as_str),
            Some("1.0")
        );
        assert_eq!(
            document.attributes.get("revdate").map(String::as_str),
            Some("2001-01-01")
        );
        assert_eq!(
            document.attributes.get("toc").map(String::as_str),
            Some("left")
        );
        assert_eq!(
            document.attributes.get("firstname").map(String::as_str),
            Some("Stuart")
        );
        assert_eq!(
            document.attributes.get("lastname").map(String::as_str),
            Some("Rackham")
        );
        assert_eq!(
            document
                .attributes
                .get("authorinitials")
                .map(String::as_str),
            Some("SR")
        );
    }

    #[test]
    fn parses_multiple_implicit_authors_in_tck_document_parsing() {
        let document = parse_tck_document(
            "= Document Title\nDoc Writer <thedoctor@asciidoc.org>; Junior Writer <junior@asciidoctor.org>\n\nbody",
        );

        assert_eq!(
            document.attributes.get("author").map(String::as_str),
            Some("Doc Writer")
        );
        assert_eq!(
            document.attributes.get("author_1").map(String::as_str),
            Some("Doc Writer")
        );
        assert_eq!(
            document.attributes.get("author_2").map(String::as_str),
            Some("Junior Writer")
        );
        assert_eq!(
            document.attributes.get("email_1").map(String::as_str),
            Some("thedoctor@asciidoc.org")
        );
        assert_eq!(
            document.attributes.get("email_2").map(String::as_str),
            Some("junior@asciidoctor.org")
        );
        assert_eq!(
            document
                .attributes
                .get("authorinitials_2")
                .map(String::as_str),
            Some("JW")
        );
    }

    #[test]
    fn parses_explicit_author_metadata_in_tck_document_parsing() {
        let document = parse_tck_document(
            "= Document Title\n:author: Doc Writer\n:email: thedoctor@asciidoc.org\n\nbody",
        );

        assert_eq!(
            document.attributes.get("firstname").map(String::as_str),
            Some("Doc")
        );
        assert_eq!(
            document.attributes.get("lastname").map(String::as_str),
            Some("Writer")
        );
        assert_eq!(
            document
                .attributes
                .get("authorinitials")
                .map(String::as_str),
            Some("DW")
        );
        assert_eq!(
            document.attributes.get("email").map(String::as_str),
            Some("thedoctor@asciidoc.org")
        );
    }

    #[test]
    fn parses_explicit_authors_metadata_in_tck_document_parsing() {
        let document =
            parse_tck_document("= Document Title\n:authors: Doc Writer; Other Author\n\nbody");

        assert_eq!(
            document.attributes.get("author_1").map(String::as_str),
            Some("Doc Writer")
        );
        assert_eq!(
            document.attributes.get("firstname_2").map(String::as_str),
            Some("Other")
        );
        assert_eq!(
            document.attributes.get("lastname_2").map(String::as_str),
            Some("Author")
        );
        assert_eq!(
            document
                .attributes
                .get("authorinitials_2")
                .map(String::as_str),
            Some("OA")
        );
    }

    #[test]
    fn renders_tck_section_structure() {
        let document = parse_tck_document("== Section Title\n\nparagraph");
        let json = serde_json::to_string_pretty(&document).expect("json");

        assert!(json.contains("\"name\": \"section\""));
        assert!(json.contains("\"level\": 1"));
        assert!(json.contains("\"value\": \"Section Title\""));
        assert!(json.contains("\"value\": \"paragraph\""));
    }

    #[test]
    fn preserves_document_line_numbers_for_nested_sections() {
        let document = parse_tck_document("= Title\n\n== First\n\n=== Nested\n\nbody");
        let section = document.blocks.first().expect("top-level section");
        let nested = section
            .blocks
            .as_ref()
            .and_then(|blocks| blocks.get(0))
            .expect("nested section");

        assert_eq!(nested.location[0].line, 5);
        assert_eq!(
            nested
                .title
                .as_ref()
                .and_then(|title| title.first())
                .expect("nested title")
                .location[0]
                .line,
            5
        );
    }

    #[test]
    fn accepts_tck_request_envelope() {
        let request = r#"{"contents":"A paragraph that consists of a single line.","path":"/tmp/in.adoc","type":"block"}"#;
        let json = render_tck_json_from_request(request).expect("request should work");

        assert!(json.contains("\"name\": \"document\""));
        assert!(json.contains("\"name\": \"paragraph\""));
    }

    #[test]
    fn parses_simple_inline_text() {
        let inlines = parse_tck_inlines("hello");

        assert_eq!(inlines.len(), 1);
        let super::AsgInline::Text(text) = &inlines[0] else {
            panic!("expected text inline");
        };
        assert_eq!(text.value, "hello");
        assert_eq!(text.location[0].line, 1);
        assert_eq!(text.location[1].col, 5);
    }

    #[test]
    fn accepts_inline_tck_request_envelope() {
        let request = r#"{"contents":"hello","path":"/tmp/in.adoc","type":"inline"}"#;
        let json = render_tck_json_from_request(request).expect("request should work");

        assert!(json.contains("\"name\": \"text\""));
        assert!(json.contains("\"value\": \"hello\""));
    }

    #[test]
    fn renders_strong_span_for_inline_tck_requests() {
        let request = r#"{"contents":"*s*","path":"/tmp/in.adoc","type":"inline"}"#;
        let json = render_tck_json_from_request(request).expect("request should work");

        assert!(json.contains("\"variant\": \"strong\""));
        assert!(json.contains("\"form\": \"constrained\""));
    }

    #[test]
    fn keeps_escaped_markup_as_text_in_tck_inline_requests() {
        let request = r#"{"contents":"\\*not strong*","path":"/tmp/in.adoc","type":"inline"}"#;
        let json = render_tck_json_from_request(request).expect("request should work");

        assert!(json.contains("\"name\": \"text\""));
        assert!(json.contains("\"value\": \"*not strong*\""));
        assert!(!json.contains("\"variant\": \"strong\""));
    }

    #[test]
    fn renders_tck_delimited_listing_block() {
        let document = parse_tck_document("----\ndef main\n  puts 'hello'\nend\n----");
        let block = document.blocks.first().expect("listing block");

        assert_eq!(block.name, "listing");
        assert_eq!(block.form, Some("delimited"));
        assert_eq!(block.delimiter, Some("----"));
        let Some(inlines) = &block.inlines else {
            panic!("expected listing text");
        };
        let AsgInline::Text(text) = &inlines[0] else {
            panic!("expected text");
        };
        assert_eq!(text.value, "def main\n  puts 'hello'\nend");
    }

    #[test]
    fn renders_tck_delimited_listing_block_with_longer_delimiters() {
        let document = parse_tck_document("------\ndef main\n  puts 'hello'\nend\n------");
        let block = document.blocks.first().expect("listing block");

        assert_eq!(block.name, "listing");
        assert_eq!(block.form, Some("delimited"));
        assert_eq!(block.delimiter, Some("----"));
        let Some(inlines) = &block.inlines else {
            panic!("expected listing text");
        };
        let AsgInline::Text(text) = &inlines[0] else {
            panic!("expected text");
        };
        assert_eq!(text.value, "def main\n  puts 'hello'\nend");
    }

    #[test]
    fn renders_tck_fenced_code_block() {
        let document = parse_tck_document("```rust,linenums\nfn main() {}\n```");
        let block = document.blocks.first().expect("listing block");

        assert_eq!(block.name, "listing");
        assert_eq!(block.form, Some("delimited"));
        assert_eq!(block.delimiter, Some("```"));
        let metadata = block.metadata.as_ref().expect("metadata");
        assert_eq!(
            metadata.attributes.get("style").map(String::as_str),
            Some("source")
        );
        assert_eq!(
            metadata.attributes.get("language").map(String::as_str),
            Some("rust")
        );
        assert!(metadata.options.iter().any(|option| option == "linenums"));
        let AsgInline::Text(text) = &block.inlines.as_ref().expect("text")[0] else {
            panic!("expected text");
        };
        assert_eq!(text.value, "fn main() {}");
    }

    #[test]
    fn does_not_recognize_tck_fenced_code_with_more_than_three_backticks() {
        let document = parse_tck_document("````rust\nfn main() {}\n````");
        let block = document.blocks.first().expect("paragraph");

        assert_eq!(block.name, "paragraph");
    }

    #[test]
    fn trims_outer_blank_lines_in_tck_delimited_content() {
        let document = parse_tck_document(
            "----\n\ncode\n\n----\n\n++++\n\n<span>ok</span>\n\n++++\n\n[verse]\n____\n\nline\n\n____",
        );

        let listing = document.blocks.first().expect("listing block");
        let AsgInline::Text(text) = &listing.inlines.as_ref().expect("text")[0] else {
            panic!("expected text");
        };
        assert_eq!(text.value, "code");
        assert_eq!(text.location[0].line, 3);
        assert_eq!(text.location[1].line, 3);

        let passthrough = document.blocks.get(1).expect("passthrough block");
        let AsgInline::Text(text) = &passthrough.inlines.as_ref().expect("text")[0] else {
            panic!("expected text");
        };
        assert_eq!(text.value, "<span>ok</span>");
        assert_eq!(text.location[0].line, 9);
        assert_eq!(text.location[1].line, 9);

        let verse = document.blocks.get(2).expect("verse block");
        let AsgInline::Text(text) = &verse.inlines.as_ref().expect("text")[0] else {
            panic!("expected text");
        };
        assert_eq!(text.value, "line");
        assert_eq!(text.location[0].line, 16);
        assert_eq!(text.location[1].line, 16);
    }

    #[test]
    fn renders_tck_delimited_passthrough_block() {
        let document = parse_tck_document("++++\n<span>ok</span>\n++++");
        let block = document.blocks.first().expect("passthrough block");

        assert_eq!(block.name, "passthrough");
        assert_eq!(block.form, Some("delimited"));
        assert_eq!(block.delimiter, Some("++++"));
        let Some(inlines) = &block.inlines else {
            panic!("expected passthrough text");
        };
        let AsgInline::Text(text) = &inlines[0] else {
            panic!("expected text");
        };
        assert_eq!(text.value, "<span>ok</span>");
    }

    #[test]
    fn renders_tck_delimited_sidebar_block() {
        let document = parse_tck_document("****\n* one\n* two\n****");
        let block = document.blocks.first().expect("sidebar block");

        assert_eq!(block.name, "sidebar");
        assert_eq!(block.form, Some("delimited"));
        assert_eq!(block.delimiter, Some("****"));
        assert!(
            block
                .blocks
                .as_ref()
                .is_some_and(|blocks| !blocks.is_empty())
        );
    }

    #[test]
    fn renders_tck_delimited_block_metadata() {
        let document = parse_tck_document(".Exhibit A\n[source,rust]\n----\nputs 'hello'\n----");
        let block = document.blocks.first().expect("listing block");

        assert_eq!(
            block
                .title
                .as_ref()
                .and_then(|title| title.first())
                .map(|text| text.value.as_str()),
            Some("Exhibit A")
        );
        let metadata = block.metadata.as_ref().expect("metadata");
        assert_eq!(
            metadata.attributes.get("$1").map(String::as_str),
            Some("source")
        );
        assert_eq!(
            metadata.attributes.get("style").map(String::as_str),
            Some("source")
        );
        assert_eq!(
            metadata.attributes.get("language").map(String::as_str),
            Some("rust")
        );
        assert_eq!(
            metadata.attributes.get("title").map(String::as_str),
            Some("Exhibit A")
        );
    }

    #[test]
    fn renders_tck_admonition_paragraph() {
        let document = parse_tck_document("NOTE: This is just a note.");
        let block = document.blocks.first().expect("admonition block");

        assert_eq!(block.name, "admonition");
        assert_eq!(block.form, Some("paragraph"));
        assert_eq!(block.variant, Some("note"));
        let paragraph = block
            .blocks
            .as_ref()
            .and_then(|blocks| blocks.first())
            .expect("paragraph block");
        assert_eq!(paragraph.name, "paragraph");
        let text = paragraph
            .inlines
            .as_ref()
            .and_then(|inlines| inlines.first())
            .expect("paragraph text");
        let AsgInline::Text(text) = text else {
            panic!("expected text");
        };
        assert_eq!(text.value, "This is just a note.");
    }

    #[test]
    fn renders_tck_anchored_admonition_paragraph() {
        let document = parse_tck_document("[[install-note]]\nNOTE: This is just a note.");
        let block = document.blocks.first().expect("admonition block");

        assert_eq!(block.name, "admonition");
        assert_eq!(block.id.as_deref(), Some("install-note"));
    }

    #[test]
    fn renders_tck_styled_admonition_paragraph() {
        let document = parse_tck_document("[NOTE]\nRemember the milk.");
        let block = document.blocks.first().expect("admonition block");

        assert_eq!(block.name, "admonition");
        assert_eq!(block.form, Some("paragraph"));
        assert_eq!(block.variant, Some("note"));
        let metadata = block.metadata.as_ref().expect("metadata");
        assert_eq!(
            metadata.attributes.get("$1").map(String::as_str),
            Some("NOTE")
        );
        assert_eq!(
            metadata.attributes.get("style").map(String::as_str),
            Some("NOTE")
        );
    }

    #[test]
    fn renders_tck_listing_styled_paragraph() {
        let document = parse_tck_document("[listing]\nputs 'hello'");
        let block = document.blocks.first().expect("listing block");

        assert_eq!(block.name, "listing");
        assert_eq!(block.form, Some("paragraph"));
        let text = block
            .inlines
            .as_ref()
            .and_then(|inlines| inlines.first())
            .expect("listing text");
        let AsgInline::Text(text) = text else {
            panic!("expected text");
        };
        assert_eq!(text.value, "puts 'hello'");
    }

    #[test]
    fn renders_tck_source_styled_paragraph() {
        let document = parse_tck_document("[source,rust]\nfn main() {}");
        let block = document.blocks.first().expect("listing block");

        assert_eq!(block.name, "listing");
        assert_eq!(block.form, Some("paragraph"));
        let metadata = block.metadata.as_ref().expect("metadata");
        assert_eq!(
            metadata.attributes.get("style").map(String::as_str),
            Some("source")
        );
        assert_eq!(
            metadata.attributes.get("language").map(String::as_str),
            Some("rust")
        );
    }

    #[test]
    fn renders_tck_source_blocks_with_positional_linenums_option() {
        let document = parse_tck_document("[source,rust,linenums]\n----\nfn main() {}\n----");
        let block = document.blocks.first().expect("listing block");

        assert_eq!(block.name, "listing");
        let metadata = block.metadata.as_ref().expect("metadata");
        assert_eq!(
            metadata.attributes.get("style").map(String::as_str),
            Some("source")
        );
        assert_eq!(
            metadata.attributes.get("language").map(String::as_str),
            Some("rust")
        );
        assert!(metadata.options.iter().any(|option| option == "linenums"));
        assert_eq!(
            metadata
                .attributes
                .get("linenums-option")
                .map(String::as_str),
            Some("")
        );
    }

    #[test]
    fn renders_tck_quote_styled_paragraph() {
        let document = parse_tck_document("[quote, Abraham Lincoln]\nFour score.");
        let block = document.blocks.first().expect("quote block");

        assert_eq!(block.name, "quote");
        assert_eq!(block.form, Some("paragraph"));
        let metadata = block.metadata.as_ref().expect("metadata");
        assert_eq!(
            metadata.attributes.get("$2").map(String::as_str),
            Some("Abraham Lincoln")
        );
        let paragraph = block
            .blocks
            .as_ref()
            .and_then(|blocks| blocks.first())
            .expect("paragraph block");
        assert_eq!(paragraph.name, "paragraph");
    }

    #[test]
    fn renders_tck_pass_styled_paragraph() {
        let document = parse_tck_document("[pass]\n<span>ok</span>");
        let block = document.blocks.first().expect("passthrough block");

        assert_eq!(block.name, "passthrough");
        assert_eq!(block.form, Some("paragraph"));
        let text = block
            .inlines
            .as_ref()
            .and_then(|inlines| inlines.first())
            .expect("passthrough text");
        let AsgInline::Text(text) = text else {
            panic!("expected text");
        };
        assert_eq!(text.value, "<span>ok</span>");
    }

    #[test]
    fn renders_tck_stem_delimited_passthrough_block() {
        let document = parse_tck_document("[stem]\n++++\nsqrt(4) = 2\n++++");
        let block = document.blocks.first().expect("passthrough block");

        assert_eq!(block.name, "passthrough");
        assert_eq!(block.form, Some("delimited"));
        let metadata = block.metadata.as_ref().expect("metadata");
        assert_eq!(
            metadata.attributes.get("style").map(String::as_str),
            Some("stem")
        );
        let text = block
            .inlines
            .as_ref()
            .and_then(|inlines| inlines.first())
            .expect("passthrough text");
        let AsgInline::Text(text) = text else {
            panic!("expected text");
        };
        assert_eq!(text.value, "sqrt(4) = 2");
    }

    #[test]
    fn renders_tck_pass_styled_open_block_as_passthrough() {
        let document = parse_tck_document("[pass]\n--\n<span>ok</span>\n--");
        let block = document.blocks.first().expect("passthrough block");

        assert_eq!(block.name, "passthrough");
        assert_eq!(block.form, Some("delimited"));
        assert_eq!(block.delimiter, Some("--"));
        let text = block
            .inlines
            .as_ref()
            .and_then(|inlines| inlines.first())
            .expect("passthrough text");
        let AsgInline::Text(text) = text else {
            panic!("expected text");
        };
        assert_eq!(text.value, "<span>ok</span>");
    }

    #[test]
    fn renders_tck_stem_styled_open_block_as_passthrough() {
        let document = parse_tck_document("[stem]\n--\nsqrt(4) = 2\n--");
        let block = document.blocks.first().expect("passthrough block");

        assert_eq!(block.name, "passthrough");
        assert_eq!(block.form, Some("delimited"));
        assert_eq!(block.delimiter, Some("--"));
        let metadata = block.metadata.as_ref().expect("metadata");
        assert_eq!(
            metadata.attributes.get("style").map(String::as_str),
            Some("stem")
        );
        let text = block
            .inlines
            .as_ref()
            .and_then(|inlines| inlines.first())
            .expect("passthrough text");
        let AsgInline::Text(text) = text else {
            panic!("expected text");
        };
        assert_eq!(text.value, "sqrt(4) = 2");
    }

    #[test]
    fn renders_tck_styled_delimited_admonition() {
        let document = parse_tck_document("[TIP]\n====\nRemember the milk.\n====");
        let block = document.blocks.first().expect("admonition block");

        assert_eq!(block.name, "admonition");
        assert_eq!(block.form, Some("delimited"));
        assert_eq!(block.variant, Some("tip"));
        assert_eq!(block.delimiter, Some("===="));
        let metadata = block.metadata.as_ref().expect("metadata");
        assert_eq!(
            metadata.attributes.get("$1").map(String::as_str),
            Some("TIP")
        );
        assert_eq!(
            metadata.attributes.get("style").map(String::as_str),
            Some("TIP")
        );
    }

    #[test]
    fn renders_tck_nested_example_blocks_with_longer_child_delimiters() {
        let document = parse_tck_document("====\n======\ninside\n======\n====");
        let block = document.blocks.first().expect("outer example");

        assert_eq!(block.name, "example");
        let inner = block
            .blocks
            .as_ref()
            .and_then(|blocks| blocks.first())
            .expect("inner example");
        assert_eq!(inner.name, "example");
    }

    #[test]
    fn renders_tck_styled_open_block_as_sidebar() {
        let document = parse_tck_document("[sidebar]\n--\ninside\n--");
        let block = document.blocks.first().expect("sidebar block");

        assert_eq!(block.name, "sidebar");
        assert_eq!(block.delimiter, Some("--"));
        assert!(block.blocks.is_some());
    }

    #[test]
    fn renders_tck_styled_open_block_as_admonition() {
        let document = parse_tck_document("[NOTE]\n--\nRemember this.\n--");
        let block = document.blocks.first().expect("admonition block");

        assert_eq!(block.name, "admonition");
        assert_eq!(block.variant, Some("note"));
        assert_eq!(block.delimiter, Some("--"));
    }

    #[test]
    fn renders_tck_styled_open_block_as_quote() {
        let document = parse_tck_document("[quote, Abraham Lincoln]\n--\nFour score.\n--");
        let block = document.blocks.first().expect("quote block");

        assert_eq!(block.name, "quote");
        assert_eq!(block.delimiter, Some("--"));
        let metadata = block.metadata.as_ref().expect("metadata");
        assert_eq!(
            metadata.attributes.get("$2").map(String::as_str),
            Some("Abraham Lincoln")
        );
        assert!(block.blocks.is_some());
    }

    #[test]
    fn renders_tck_styled_open_block_as_verse() {
        let document = parse_tck_document("[verse, Carl Sandburg, Fog]\n--\nThe fog comes\n--");
        let block = document.blocks.first().expect("verse block");

        assert_eq!(block.name, "verse");
        assert_eq!(block.delimiter, Some("--"));
        let inlines = block.inlines.as_ref().expect("text");
        let AsgInline::Text(text) = &inlines[0] else {
            panic!("expected text");
        };
        assert_eq!(text.value, "The fog comes");
    }

    #[test]
    fn ignores_tck_comment_blocks_with_longer_delimiters() {
        let document = parse_tck_document("//////\nignore me\n//////\n\nvisible");

        assert_eq!(document.blocks.len(), 1);
        assert_eq!(document.blocks[0].name, "paragraph");
    }

    #[test]
    fn renders_tck_anchored_list_block() {
        let document = parse_tck_document("[[steps]]\n* one");
        let block = document.blocks.first().expect("list block");

        assert_eq!(block.name, "list");
        assert_eq!(block.id.as_deref(), Some("steps"));
        assert_eq!(block.variant, Some("unordered"));
    }

    #[test]
    fn renders_tck_anchored_delimited_blocks() {
        let document = parse_tck_document(
            "[[code-sample]]\n----\nputs 'hello'\n----\n\n[[aside]]\n****\ninside\n****",
        );

        let listing = document.blocks.first().expect("listing block");
        assert_eq!(listing.name, "listing");
        assert_eq!(listing.id.as_deref(), Some("code-sample"));

        let sidebar = document.blocks.get(1).expect("sidebar block");
        assert_eq!(sidebar.name, "sidebar");
        assert_eq!(sidebar.id.as_deref(), Some("aside"));
    }

    #[test]
    fn renders_tck_pipe_table() {
        let document = parse_tck_document("|===\n|A |B\n|1 |2\n|===");
        let block = document.blocks.first().expect("table block");

        assert_eq!(block.name, "table");
        assert_eq!(block.form, Some("delimited"));
        assert_eq!(block.delimiter, Some("|==="));
        let rows = block.blocks.as_ref().expect("rows");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, "tableRow");
        let first_row_cells = rows[0].blocks.as_ref().expect("cells");
        assert_eq!(first_row_cells.len(), 2);
        assert_eq!(first_row_cells[0].name, "tableCell");
        let cell_blocks = first_row_cells[0].blocks.as_ref().expect("cell blocks");
        assert_eq!(cell_blocks[0].name, "paragraph");
    }

    #[test]
    fn renders_tck_table_with_metadata() {
        let document = parse_tck_document(
            ".Roster\n[%header,cols=\"1,1\"]\n|===\n|Name |Role\n|Ada |Author\n|===",
        );
        let block = document.blocks.first().expect("table block");

        assert_eq!(
            block
                .title
                .as_ref()
                .and_then(|title| title.first())
                .map(|text| text.value.as_str()),
            Some("Roster")
        );
        let metadata = block.metadata.as_ref().expect("metadata");
        assert_eq!(
            metadata.attributes.get("title").map(String::as_str),
            Some("Roster")
        );
        assert_eq!(
            metadata.attributes.get("cols").map(String::as_str),
            Some("1,1")
        );
        let rows = block.blocks.as_ref().expect("rows");
        assert_eq!(rows[0].variant, Some("header"));
    }

    #[test]
    fn renders_tck_bang_table() {
        let document = parse_tck_document("!===\n!outer !value\n!===");
        let block = document.blocks.first().expect("table block");

        assert_eq!(block.name, "table");
        assert_eq!(block.delimiter, Some("!==="));
        let rows = block.blocks.as_ref().expect("rows");
        let first_row_cells = rows[0].blocks.as_ref().expect("cells");
        assert_eq!(first_row_cells.len(), 2);
    }

    #[test]
    fn renders_tck_csv_shorthand_table() {
        let document = parse_tck_document(",===\nAda,Author\nGrace,Reviewer\n,===");
        let block = document.blocks.first().expect("table block");

        assert_eq!(block.name, "table");
        assert_eq!(block.delimiter, Some(",==="));
        let rows = block.blocks.as_ref().expect("rows");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].blocks.as_ref().expect("cells").len(), 2);
    }

    #[test]
    fn renders_tck_dsv_shorthand_table() {
        let document = parse_tck_document(":===\nleft:right\nup:down\n:===");
        let block = document.blocks.first().expect("table block");

        assert_eq!(block.name, "table");
        assert_eq!(block.delimiter, Some(":==="));
        let rows = block.blocks.as_ref().expect("rows");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1].blocks.as_ref().expect("cells").len(), 2);
    }

    #[test]
    fn renders_tck_custom_separator_table() {
        let document =
            parse_tck_document("[separator=!,cols=\"1,1\"]\n|===\n!left!right\n!up!down\n|===");
        let block = document.blocks.first().expect("table block");

        assert_eq!(block.name, "table");
        assert_eq!(block.delimiter, Some("|==="));
        let rows = block.blocks.as_ref().expect("rows");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].blocks.as_ref().expect("cells").len(), 2);
    }

    #[test]
    fn renders_tck_block_image() {
        let document = parse_tck_document("image::images/tiger.png[Tiger, 200, 300]");
        let block = document.blocks.first().expect("image block");

        assert_eq!(block.name, "image");
        let metadata = block.metadata.as_ref().expect("metadata");
        assert_eq!(
            metadata.attributes.get("target").map(String::as_str),
            Some("images/tiger.png")
        );
        assert_eq!(
            metadata.attributes.get("alt").map(String::as_str),
            Some("Tiger")
        );
        assert_eq!(
            metadata.attributes.get("width").map(String::as_str),
            Some("200")
        );
        assert_eq!(
            metadata.attributes.get("height").map(String::as_str),
            Some("300")
        );
    }

    #[test]
    fn renders_tck_block_image_with_prelude_metadata() {
        let document =
            parse_tck_document(".The AsciiDoc Tiger\n[#tiger,.hero]\nimage::tiger.png[]");
        let block = document.blocks.first().expect("image block");

        assert_eq!(block.name, "image");
        assert_eq!(block.id.as_deref(), Some("tiger"));
        assert_eq!(
            block
                .title
                .as_ref()
                .and_then(|title| title.first())
                .map(|text| text.value.as_str()),
            Some("The AsciiDoc Tiger")
        );
        let metadata = block.metadata.as_ref().expect("metadata");
        assert_eq!(
            metadata.attributes.get("title").map(String::as_str),
            Some("The AsciiDoc Tiger")
        );
        assert_eq!(
            metadata.attributes.get("id").map(String::as_str),
            Some("tiger")
        );
        assert_eq!(
            metadata.attributes.get("role").map(String::as_str),
            Some("hero")
        );
        assert_eq!(
            metadata.attributes.get("alt").map(String::as_str),
            Some("tiger")
        );
    }

    #[test]
    fn renders_tck_quote_block() {
        let document = parse_tck_document("[quote, Abraham Lincoln]\n____\nFour score.\n____");
        let block = document.blocks.first().expect("quote block");

        assert_eq!(block.name, "quote");
        assert_eq!(block.form, Some("delimited"));
        assert_eq!(block.delimiter, Some("____"));
        assert!(block.blocks.is_some());
        let metadata = block.metadata.as_ref().expect("metadata");
        assert_eq!(
            metadata.attributes.get("$2").map(String::as_str),
            Some("Abraham Lincoln")
        );
    }

    #[test]
    fn renders_tck_verse_block() {
        let document = parse_tck_document(
            "[verse, Carl Sandburg, Fog]\n____\nThe fog comes\non little cat feet.\n____",
        );
        let block = document.blocks.first().expect("verse block");

        assert_eq!(block.name, "verse");
        assert_eq!(block.form, Some("delimited"));
        assert_eq!(block.delimiter, Some("____"));
        assert!(block.inlines.is_some());
        let inlines = block.inlines.as_ref().unwrap();
        let AsgInline::Text(text) = inlines.first().expect("text inline") else {
            panic!("expected text inline");
        };
        assert_eq!(text.value, "The fog comes\non little cat feet.");
    }
}
