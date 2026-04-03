use std::collections::BTreeMap;
use std::borrow::Cow;

use serde::Deserialize;
use serde::Serialize;
use serde::ser::{SerializeStruct, Serializer};

use crate::ast::{Inline, InlineForm, InlineVariant};
use crate::inline::parse_spanned_inlines;

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
        let field_count = 3 + usize::from(include_attributes) + usize::from(include_header) + usize::from(include_blocks);
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
    let normalized = normalize_tck_newlines(input);
    let document = parse_tck_document(normalized.as_ref());
    serde_json::to_string_pretty(&document)
}

pub fn render_tck_inline_json(input: &str) -> serde_json::Result<String> {
    let normalized = normalize_tck_newlines(input);
    serde_json::to_string_pretty(&parse_tck_inlines(trim_tck_inline_terminal_newline(normalized.as_ref())))
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
    let lines: Vec<&str> = input.lines().collect();
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

            if let Some(author_line) =
                lines.get(index).and_then(|line| parse_implicit_author_line(&lines, index, line))
            {
                insert_author_attributes(&mut attributes, &author_line.authors);
                header_end = Position {
                    line: index + 1,
                    col: lines[index].len(),
                };
                index += 1;
                index = skip_header_comments(&lines, index);

                if let Some(revision_line) =
                    lines.get(index).and_then(|line| parse_implicit_revision_line(line))
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

                if let Some((name, value, end_col)) = parse_attribute_entry(line) {
                    match name.as_str() {
                        "author" => saw_explicit_author = true,
                        "authors" => saw_explicit_authors = true,
                        "authorinitials" => saw_explicit_authorinitials = true,
                        _ => {}
                    }
                    attributes.insert(name, value);
                    header_end = Position {
                        line: index + 1,
                        col: end_col,
                    };
                    index += 1;
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

    let (blocks, end) = parse_blocks(&lines[index..], index + 1, None);
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
    block.metadata
        .as_ref()
        .map(|metadata| metadata.location[0].clone())
        .unwrap_or_else(|| block.location[0].clone())
}

pub fn parse_tck_inlines(input: &str) -> Vec<AsgInline> {
    parse_tck_inlines_at(input, 1, 1)
}

fn normalize_tck_newlines(input: &str) -> Cow<'_, str> {
    if !input.contains('\r') {
        return Cow::Borrowed(input);
    }

    Cow::Owned(input.replace("\r\n", "\n").replace('\r', "\n"))
}

fn trim_tck_inline_terminal_newline(input: &str) -> &str {
    input.strip_suffix('\n').unwrap_or(input)
}

fn parse_blocks(
    lines: &[&str],
    line_offset: usize,
    stop_at_level: Option<u8>,
) -> (Vec<AsgBlock>, Option<Position>) {
    let mut blocks = Vec::new();
    let mut index = 0;
    let mut paragraph_start = None::<usize>;
    let mut paragraph_lines = Vec::new();
    let mut last_end = None;

    while index < lines.len() {
        let absolute_index = line_offset + index - 1;
        let line = lines[index];

        if let Some((heading, heading_range, consumed_lines)) =
            parse_heading_line(lines, index, line_offset)
        {
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
            );

            let end = child_end.unwrap_or_else(|| heading_range[1].clone());
            blocks.push(AsgBlock {
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
            });
            last_end = Some(end);

            index = child_start
                + count_consumed_lines(&lines[child_start..], stop_at_level, heading.level);
            continue;
        }

        if let Some((block, consumed_lines)) = parse_delimited_block(lines, index, line_offset) {
            flush_paragraph(
                &mut blocks,
                &mut paragraph_start,
                &mut paragraph_lines,
                line_offset,
                &mut last_end,
            );
            last_end = Some(block.location[1].clone());
            blocks.push(block);
            index += consumed_lines;
            continue;
        }

        if let Some((block, consumed_lines)) = parse_admonition_paragraph(lines, index, line_offset) {
            flush_paragraph(
                &mut blocks,
                &mut paragraph_start,
                &mut paragraph_lines,
                line_offset,
                &mut last_end,
            );
            last_end = Some(block.location[1].clone());
            blocks.push(block);
            index += consumed_lines;
            continue;
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
                let item_start = Position { line: item_line_no, col: 1 };
                let item_end = Position { line: item_line_no, col: item_end_col };
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

            let list_start = Position { line: line_offset + index, col: 1 };
            let list_end = list_end.unwrap_or_else(|| list_start.clone());
            blocks.push(AsgBlock {
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
            });
            last_end = Some(list_end);
            index = list_index;
            continue;
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
    let delimiter = lines.get(delimiter_index)?.trim();
    let name = match delimiter {
        "----" => "listing",
        "====" => "example",
        "****" => "sidebar",
        _ => return None,
    };

    let closing_index = lines[delimiter_index + 1..]
        .iter()
        .position(|line| line.trim() == delimiter)
        .map(|offset| delimiter_index + 1 + offset)?;
    let start_line = line_offset + delimiter_index;
    let end_line = line_offset + closing_index;
    let inner_lines = &lines[delimiter_index + 1..closing_index];
    let consumed = closing_index - index + 1;

    let mut block = AsgBlock {
        name,
        node_type: "block",
        id: prelude.id.clone(),
        title: prelude.title,
        metadata: prelude.metadata,
        level: None,
        form: Some("delimited"),
        delimiter: Some(match delimiter {
            "----" => "----",
            "====" => "====",
            "****" => "****",
            _ => unreachable!(),
        }),
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
        "listing" => {
            let content = inner_lines.join("\n");
            if !content.is_empty() {
                let start = Position {
                    line: start_line + 1,
                    col: 1,
                };
                let end = Position {
                    line: end_line - 1,
                    col: inner_lines.last().map(|line| line.len()).unwrap_or(1),
                };
                block.inlines = Some(vec![AsgInline::Text(InlineText {
                    name: "text",
                    node_type: "string",
                    value: content,
                    location: [start, end],
                })]);
            }
        }
        "example" | "sidebar" => {
            let (children, _) = parse_blocks(inner_lines, start_line + 1, None);
            block.blocks = Some(children);
        }
        _ => {}
    }

    Some((block, consumed))
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
        if lines
            .get(next)
            .is_some_and(|line| parse_attribute_list_line(line).is_some() || is_delimited_block_delimiter(line))
        {
            let title_line = line_offset + cursor;
            prelude.title = Some(vec![InlineText {
                name: "text",
                node_type: "string",
                value: title.clone(),
                location: [
                    Position { line: title_line, col: 2 },
                    Position {
                        line: title_line,
                        col: lines[cursor].len(),
                    },
                ],
            }]);
            title_raw = Some(title);
            metadata_start = Some(Position { line: title_line, col: 1 });
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
        if lines.get(next).is_some_and(|line| is_delimited_block_delimiter(line)) {
            let attr_line = line_offset + cursor;
            apply_attribute_list(
                &mut metadata_attributes,
                &mut prelude.id,
                &mut metadata_options,
                &mut metadata_roles,
                &entries,
            );
            metadata_start.get_or_insert(Position { line: attr_line, col: 1 });
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
        metadata_attributes.entry("id".into()).or_insert_with(|| id.clone());
    }

    prelude.consumed_lines = cursor - index;
    prelude.start = metadata_start.clone();
    if metadata_start.is_some() || !metadata_attributes.is_empty() || !metadata_options.is_empty() || !metadata_roles.is_empty() {
        prelude.metadata = Some(AsgBlockMetadata {
            attributes: metadata_attributes,
            options: metadata_options,
            roles: metadata_roles,
            location: [
                metadata_start.unwrap_or(Position { line: line_offset + index, col: 1 }),
                metadata_end.unwrap_or(Position { line: line_offset + index, col: 1 }),
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
    matches!(line.trim(), "----" | "====" | "****")
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
                for option in value.split(',').map(str::trim).filter(|value| !value.is_empty()) {
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
            for role in value.split('.').map(str::trim).filter(|value| !value.is_empty()) {
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
            for option in value.split('%').map(str::trim).filter(|value| !value.is_empty()) {
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
        } else if attributes.get("style").is_some_and(|style| style == "source")
            && !attributes.contains_key("language")
        {
            attributes.insert("language".into(), entry.clone());
        }
    }
}

fn parse_named_attribute(entry: &str) -> Option<(String, String)> {
    let separator = entry.find('=')?;
    let name = entry[..separator].trim();
    if name.is_empty() {
        return None;
    }
    Some((name.to_owned(), unquote_attribute_value(entry[separator + 1..].trim())))
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
            || is_delimited_block_delimiter(line)
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
    line.trim_start().starts_with("//")
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

fn insert_author_attributes(
    attributes: &mut BTreeMap<String, String>,
    authors: &[ImplicitAuthor],
) {
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
    let explicit_primary_initials =
        preserve_primary_initials.then(|| attributes.get("authorinitials").cloned()).flatten();
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
    let authorinitials = [Some(firstname.as_str()), middlename.as_deref(), lastname.as_deref()]
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
                col: title_line.len(),
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
    use crate::tck::{AsgInline, parse_tck_document, parse_tck_inlines, render_tck_json_from_request};

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
            document.header.as_ref().and_then(|header| header.title.first()).map(|title| title.value.as_str()),
            Some("Document Title")
        );
        assert_eq!(document.attributes.get("toc").map(String::as_str), Some("left"));
        assert_eq!(document.blocks.len(), 1);
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
        assert_eq!(document.attributes.get("toc").map(String::as_str), Some("left"));
        assert_eq!(
            document.attributes.get("firstname").map(String::as_str),
            Some("Stuart")
        );
        assert_eq!(
            document.attributes.get("lastname").map(String::as_str),
            Some("Rackham")
        );
        assert_eq!(
            document.attributes.get("authorinitials").map(String::as_str),
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
            document.attributes.get("authorinitials_2").map(String::as_str),
            Some("JW")
        );
    }

    #[test]
    fn parses_explicit_author_metadata_in_tck_document_parsing() {
        let document =
            parse_tck_document("= Document Title\n:author: Doc Writer\n:email: thedoctor@asciidoc.org\n\nbody");

        assert_eq!(
            document.attributes.get("firstname").map(String::as_str),
            Some("Doc")
        );
        assert_eq!(
            document.attributes.get("lastname").map(String::as_str),
            Some("Writer")
        );
        assert_eq!(
            document.attributes.get("authorinitials").map(String::as_str),
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
            document.attributes.get("authorinitials_2").map(String::as_str),
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
    fn renders_tck_delimited_sidebar_block() {
        let document = parse_tck_document("****\n* one\n* two\n****");
        let block = document.blocks.first().expect("sidebar block");

        assert_eq!(block.name, "sidebar");
        assert_eq!(block.form, Some("delimited"));
        assert_eq!(block.delimiter, Some("****"));
        assert!(block.blocks.as_ref().is_some_and(|blocks| !blocks.is_empty()));
    }

    #[test]
    fn renders_tck_delimited_block_metadata() {
        let document = parse_tck_document(".Exhibit A\n[source,rust]\n----\nputs 'hello'\n----");
        let block = document.blocks.first().expect("listing block");

        assert_eq!(block.title.as_ref().and_then(|title| title.first()).map(|text| text.value.as_str()), Some("Exhibit A"));
        let metadata = block.metadata.as_ref().expect("metadata");
        assert_eq!(metadata.attributes.get("$1").map(String::as_str), Some("source"));
        assert_eq!(metadata.attributes.get("style").map(String::as_str), Some("source"));
        assert_eq!(metadata.attributes.get("language").map(String::as_str), Some("rust"));
        assert_eq!(metadata.attributes.get("title").map(String::as_str), Some("Exhibit A"));
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
}
