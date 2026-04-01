use std::collections::BTreeMap;

use crate::ast::{
    Block, BlockMetadata, CompoundBlock, Document, Heading, ListItem, Listing, OrderedList, Paragraph,
    UnorderedList,
};
use crate::inline::parse_inlines;

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingAnchor {
    id: String,
    reftext: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ListKind {
    Unordered,
    Ordered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ParsedListMarker<'a> {
    kind: ListKind,
    level: usize,
    content: &'a str,
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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct BlockPrelude {
    metadata: BlockMetadata,
    consumed_lines: usize,
}

pub fn parse_document(input: &str) -> Document {
    let lines: Vec<&str> = input.lines().collect();
    let (mut title, attributes, index) = parse_document_header(&lines);
    let blocks = parse_blocks_from_lines(&lines[index..], &mut title, true);

    Document {
        title,
        attributes,
        blocks,
    }
}

fn parse_document_header(lines: &[&str]) -> (Option<Heading>, BTreeMap<String, String>, usize) {
    let mut attributes = BTreeMap::new();
    let mut index = 0;
    let mut saw_explicit_author = false;
    let mut saw_explicit_authors = false;
    let mut saw_explicit_authorinitials = false;

    index = skip_header_comments(lines, index);

    let title = match parse_heading(lines, index) {
        Some((heading, consumed_lines)) if heading.level == 0 => {
            index += consumed_lines;
            Some(heading)
        }
        _ => return (None, attributes, index),
    };

    index = skip_header_comments(lines, index);

    if let Some(author_line) =
        lines.get(index).and_then(|line| parse_implicit_author_line(lines, index, line))
    {
        insert_author_attributes(&mut attributes, &author_line.authors);
        index += 1;
        index = skip_header_comments(lines, index);

        if let Some(revision_line) = lines.get(index).and_then(|line| parse_implicit_revision_line(line))
        {
            attributes.insert("revnumber".to_owned(), revision_line.number);
            if let Some(date) = revision_line.date {
                attributes.insert("revdate".to_owned(), date);
            }
            if let Some(remark) = revision_line.remark {
                attributes.insert("revremark".to_owned(), remark);
            }
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

        let Some((name, value)) = parse_attribute_entry(line) else {
            break;
        };
        match name.as_str() {
            "author" => saw_explicit_author = true,
            "authors" => saw_explicit_authors = true,
            "authorinitials" => saw_explicit_authorinitials = true,
            _ => {}
        }
        attributes.insert(name, value);
        index += 1;
    }

    if saw_explicit_authors {
        normalize_explicit_author_attributes(&mut attributes, "authors", saw_explicit_authorinitials);
    } else if saw_explicit_author {
        normalize_explicit_author_attributes(&mut attributes, "author", saw_explicit_authorinitials);
    }

    (title, attributes, index)
}

fn skip_header_comments(lines: &[&str], mut index: usize) -> usize {
    while index < lines.len() && is_comment_line(lines[index]) {
        index += 1;
    }
    index
}

fn parse_blocks_from_lines(
    lines: &[&str],
    title: &mut Option<Heading>,
    allow_document_title: bool,
) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut index = 0;
    let mut current_paragraph = Vec::new();
    let mut current_paragraph_anchor = None;
    let mut pending_anchor = None;

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

        if let Some((block, consumed_lines)) = parse_delimited_block(lines, index) {
            flush_paragraph(
                &mut blocks,
                &mut current_paragraph,
                &mut current_paragraph_anchor,
            );
            push_block(
                &mut blocks,
                title,
                allow_document_title,
                block,
                pending_anchor.take(),
            );
            index += consumed_lines;
            continue;
        }

        if let Some((heading, consumed_lines)) = parse_heading(lines, index) {
            flush_paragraph(
                &mut blocks,
                &mut current_paragraph,
                &mut current_paragraph_anchor,
            );
            push_block(
                &mut blocks,
                title,
                allow_document_title,
                Block::Heading(heading),
                pending_anchor.take(),
            );
            index += consumed_lines;
            continue;
        }

        if let Some((list, consumed_lines)) = parse_unordered_list(lines, index) {
            flush_paragraph(
                &mut blocks,
                &mut current_paragraph,
                &mut current_paragraph_anchor,
            );
            blocks.push(Block::UnorderedList(list));
            pending_anchor = None;
            index += consumed_lines;
            continue;
        }

        if let Some((list, consumed_lines)) = parse_ordered_list(lines, index) {
            flush_paragraph(
                &mut blocks,
                &mut current_paragraph,
                &mut current_paragraph_anchor,
            );
            blocks.push(Block::OrderedList(list));
            pending_anchor = None;
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

    blocks
}

fn push_block(
    blocks: &mut Vec<Block>,
    title: &mut Option<Heading>,
    allow_document_title: bool,
    block: Block,
    anchor: Option<PendingAnchor>,
) {
    match block {
        Block::Heading(heading) => {
            let heading = apply_anchor_to_heading(heading, anchor);
            if allow_document_title && heading.level == 0 && title.is_none() && blocks.is_empty() {
                *title = Some(heading);
            } else {
                blocks.push(Block::Heading(heading));
            }
        }
        other => blocks.push(other),
    }
}

fn parse_delimited_block(lines: &[&str], index: usize) -> Option<(Block, usize)> {
    let prelude = parse_block_prelude(lines, index);
    let delimiter_index = index + prelude.consumed_lines;
    let delimiter = lines.get(delimiter_index)?.trim();
    let block_kind = match delimiter {
        "----" => "listing",
        "====" => "example",
        "****" => "sidebar",
        _ => return None,
    };

    let closing_index = lines[delimiter_index + 1..]
        .iter()
        .position(|line| line.trim() == delimiter)
        .map(|offset| delimiter_index + 1 + offset)?;
    let inner_lines = &lines[delimiter_index + 1..closing_index];
    let consumed = closing_index - index + 1;

    let block = match block_kind {
        "listing" => Block::Listing(Listing {
            lines: inner_lines.iter().map(|line| (*line).to_owned()).collect(),
            metadata: prelude.metadata,
        }),
        "example" => {
            let mut nested_title = None;
            Block::Example(CompoundBlock {
                blocks: parse_blocks_from_lines(inner_lines, &mut nested_title, false),
                metadata: prelude.metadata,
            })
        }
        "sidebar" => {
            let mut nested_title = None;
            Block::Sidebar(CompoundBlock {
                blocks: parse_blocks_from_lines(inner_lines, &mut nested_title, false),
                metadata: prelude.metadata,
            })
        }
        _ => return None,
    };

    Some((block, consumed))
}

fn parse_block_prelude(lines: &[&str], index: usize) -> BlockPrelude {
    let mut prelude = BlockPrelude::default();
    let mut cursor = index;

    if let Some(title) = lines.get(cursor).and_then(|line| parse_block_title(line)) {
        let next = cursor + 1;
        if lines
            .get(next)
            .is_some_and(|line| parse_attribute_list_line(line).is_some() || is_delimited_block_delimiter(line))
        {
            prelude.metadata.title = Some(title.clone());
            prelude.metadata.attributes.insert("title".into(), title);
            cursor += 1;
        }
    }

    if let Some(attr_line) = lines.get(cursor).and_then(|line| parse_attribute_list_line(line)) {
        let next = cursor + 1;
        if lines.get(next).is_some_and(|line| is_delimited_block_delimiter(line)) {
            apply_attribute_list_to_metadata(&mut prelude.metadata, &attr_line);
            cursor += 1;
        }
    }

    prelude.consumed_lines = cursor - index;
    prelude
}

fn is_delimited_block_delimiter(line: &str) -> bool {
    matches!(line.trim(), "----" | "====" | "****")
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

fn apply_attribute_list_to_metadata(metadata: &mut BlockMetadata, entries: &[String]) {
    for (index, entry) in entries.iter().enumerate() {
        let slot = index + 1;
        if entry.is_empty() {
            continue;
        }

        if let Some((name, value)) = parse_named_attribute(entry) {
            metadata.attributes.insert(name.clone(), value.clone());
            if name == "opts" {
                metadata.options = value
                    .split(',')
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_owned)
                    .collect();
            } else if name == "role" {
                metadata.roles = value
                    .split_whitespace()
                    .map(str::to_owned)
                    .collect();
                if !metadata.roles.is_empty() {
                    metadata.role = Some(metadata.roles.join(" "));
                }
            }
            continue;
        }

        if let Some(id) = entry.strip_prefix('#') {
            if !id.is_empty() {
                metadata.id = Some(id.to_owned());
                metadata.attributes.insert(format!("${slot}"), entry.clone());
                metadata.attributes.insert("id".into(), id.to_owned());
            }
            continue;
        }

        if let Some(role_entry) = entry.strip_prefix('.') {
            let roles = role_entry
                .split('.')
                .map(str::trim)
                .filter(|role| !role.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>();
            if !roles.is_empty() {
                metadata.attributes.insert(format!("${slot}"), entry.clone());
                metadata.roles.extend(roles);
                let mut deduped_roles = Vec::new();
                for role in std::mem::take(&mut metadata.roles) {
                    if !deduped_roles.contains(&role) {
                        deduped_roles.push(role);
                    }
                }
                metadata.roles = deduped_roles;
                metadata.role = Some(metadata.roles.join(" "));
                metadata
                    .attributes
                    .insert("role".into(), metadata.roles.join(" "));
            }
            continue;
        }

        if let Some(option_entry) = entry.strip_prefix('%') {
            let options = option_entry
                .split('%')
                .map(str::trim)
                .filter(|option| !option.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>();
            if !options.is_empty() {
                metadata.attributes.insert(format!("${slot}"), entry.clone());
                for option in options {
                    if !metadata.options.contains(&option) {
                        metadata.options.push(option.clone());
                    }
                    metadata
                        .attributes
                        .entry(format!("{option}-option"))
                        .or_default();
                }
            }
            continue;
        }

        metadata.attributes.insert(format!("${slot}"), entry.clone());
        if metadata.style.is_none() {
            metadata.style = Some(entry.clone());
            metadata
                .attributes
                .entry("style".into())
                .or_insert_with(|| entry.clone());
        } else if metadata.style.as_deref() == Some("source")
            && !metadata.attributes.contains_key("language")
        {
            metadata
                .attributes
                .insert("language".into(), entry.clone());
        }
    }
}

fn parse_named_attribute(entry: &str) -> Option<(String, String)> {
    let separator = entry.find('=')?;
    let name = entry[..separator].trim();
    if name.is_empty() {
        return None;
    }
    let value = unquote_attribute_value(entry[separator + 1..].trim());
    Some((name.to_owned(), value))
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

fn parse_unordered_list(lines: &[&str], index: usize) -> Option<(UnorderedList, usize)> {
    parse_list(lines, index, ListKind::Unordered, 1).map(|(items, consumed)| {
        (UnorderedList { items }, consumed)
    })
}

fn parse_ordered_list(lines: &[&str], index: usize) -> Option<(OrderedList, usize)> {
    parse_list(lines, index, ListKind::Ordered, 1).map(|(items, consumed)| {
        (OrderedList { items }, consumed)
    })
}

fn parse_list(lines: &[&str], index: usize, kind: ListKind, level: usize) -> Option<(Vec<ListItem>, usize)> {
    let marker = parse_list_marker(*lines.get(index)?)?;
    if marker.kind != kind || marker.level != level {
        return None;
    }

    let mut items = Vec::new();
    let mut consumed = 0;

    while index + consumed < lines.len() {
        let Some(next_marker) = parse_list_marker(lines[index + consumed]) else {
            break;
        };
        if next_marker.kind != kind || next_marker.level != level {
            break;
        }

        let (item, item_consumed) = parse_list_item(lines, index + consumed, kind, level)?;
        items.push(item);
        consumed += item_consumed;

        let blank_lines = count_blank_lines(&lines[index + consumed..]);
        if blank_lines == 0 {
            continue;
        }

        let next_index = index + consumed + blank_lines;
        let Some(next_line) = lines.get(next_index) else {
            break;
        };
        let Some(next_marker) = parse_list_marker(next_line) else {
            break;
        };
        if next_marker.kind != kind || next_marker.level != level {
            break;
        }
        consumed += blank_lines;
    }

    Some((items, consumed))
}

fn parse_list_item(
    lines: &[&str],
    index: usize,
    kind: ListKind,
    level: usize,
) -> Option<(ListItem, usize)> {
    let marker = parse_list_marker(*lines.get(index)?)?;
    if marker.kind != kind || marker.level != level {
        return None;
    }

    let mut blocks = vec![Block::Paragraph(make_paragraph(vec![marker.content.to_owned()]))];
    let mut consumed = 1;

    while index + consumed < lines.len() {
        let line = lines[index + consumed];
        let trimmed = line.trim();

        if trimmed.is_empty() {
            break;
        }

        if trimmed == "+" {
            if let Some((block, continuation_consumed)) =
                parse_list_item_continuation_block(lines, index + consumed + 1, level)
            {
                blocks.push(block);
                consumed += 1 + continuation_consumed;
                continue;
            }

            consumed += 1;
            break;
        }

        if let Some(next_marker) = parse_list_marker(line) {
            if next_marker.level > level {
                let (block, nested_consumed) =
                    parse_list_block(lines, index + consumed, next_marker.kind, next_marker.level)?;
                blocks.push(block);
                consumed += nested_consumed;
                continue;
            }

            break;
        }

        append_to_last_paragraph(&mut blocks, line.trim_start().to_owned());
        consumed += 1;
    }

    Some((ListItem { blocks }, consumed))
}

fn parse_list_item_continuation_block(
    lines: &[&str],
    index: usize,
    parent_level: usize,
) -> Option<(Block, usize)> {
    let blank_lines = count_blank_lines(&lines[index..]);
    let start = index + blank_lines;
    let line = *lines.get(start)?;

    if let Some(marker) = parse_list_marker(line) {
        if marker.level > parent_level {
            let (block, consumed) = parse_list_block(lines, start, marker.kind, marker.level)?;
            return Some((block, blank_lines + consumed));
        }

        return None;
    }

    if let Some((block, consumed)) = parse_delimited_block(lines, start) {
        return Some((block, blank_lines + consumed));
    }

    let mut paragraph_lines = Vec::new();
    let mut consumed = blank_lines;
    let mut cursor = start;

    while let Some(line) = lines.get(cursor) {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed == "+" {
            break;
        }
        if parse_list_marker(line).is_some() {
            break;
        }

        paragraph_lines.push(line.trim_start().to_owned());
        cursor += 1;
        consumed += 1;
    }

    if paragraph_lines.is_empty() {
        None
    } else {
        Some((Block::Paragraph(make_paragraph(paragraph_lines)), consumed))
    }
}

fn parse_list_block(lines: &[&str], index: usize, kind: ListKind, level: usize) -> Option<(Block, usize)> {
    let (items, consumed) = parse_list(lines, index, kind, level)?;
    let block = match kind {
        ListKind::Unordered => Block::UnorderedList(UnorderedList { items }),
        ListKind::Ordered => Block::OrderedList(OrderedList { items }),
    };
    Some((block, consumed))
}

fn parse_list_marker(line: &str) -> Option<ParsedListMarker<'_>> {
    let trimmed = line.trim_start();
    let first = trimmed.chars().next()?;

    match first {
        '*' | '-' => {
            let level = trimmed.chars().take_while(|&ch| ch == first).count();
            let remainder = &trimmed[first.len_utf8() * level..];
            parse_list_content(remainder).map(|content| ParsedListMarker {
                kind: ListKind::Unordered,
                level,
                content,
            })
        }
        '.' => {
            let level = trimmed.chars().take_while(|&ch| ch == '.').count();
            let remainder = &trimmed[level..];
            parse_list_content(remainder).map(|content| ParsedListMarker {
                kind: ListKind::Ordered,
                level,
                content,
            })
        }
        ch if ch.is_ascii_digit() => {
            let digits = trimmed.chars().take_while(|ch| ch.is_ascii_digit()).count();
            let remainder = trimmed.get(digits..)?;
            let remainder = remainder.strip_prefix('.')?;
            parse_list_content(remainder).map(|content| ParsedListMarker {
                kind: ListKind::Ordered,
                level: 1,
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

fn count_blank_lines(lines: &[&str]) -> usize {
    lines.iter().take_while(|line| line.trim().is_empty()).count()
}

fn make_paragraph(lines: Vec<String>) -> Paragraph {
    Paragraph {
        inlines: parse_inlines(&lines.join("\n")),
        lines,
        id: None,
        reftext: None,
    }
}

fn append_to_last_paragraph(blocks: &mut Vec<Block>, line: String) {
    if let Some(Block::Paragraph(paragraph)) = blocks.last_mut() {
        paragraph.lines.push(line);
        paragraph.inlines = parse_inlines(&paragraph.lines.join("\n"));
        return;
    }

    blocks.push(Block::Paragraph(make_paragraph(vec![line])));
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

fn parse_attribute_entry(line: &str) -> Option<(String, String)> {
    let stripped = line.strip_prefix(':')?;
    let separator = stripped.find(':')?;
    let name = stripped[..separator].trim();
    if name.is_empty() {
        return None;
    }

    Some((name.to_owned(), stripped[separator + 1..].trim_start().to_owned()))
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
        || parse_heading(lines, index).is_some()
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

fn is_comment_line(line: &str) -> bool {
    line.trim_start().starts_with("//")
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
        Block, BlockMetadata, CompoundBlock, Heading, Inline, InlineForm, InlineVariant, ListItem, Listing,
        OrderedList, Paragraph, UnorderedList,
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
    fn parses_document_header_attributes_after_title() {
        let document =
            parse_document("= Document Title\n:toc: left\n:source-highlighter: rouge\n\ncontent");

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
            document.attributes,
            [
                ("source-highlighter".to_owned(), "rouge".to_owned()),
                ("toc".to_owned(), "left".to_owned()),
            ]
            .into_iter()
            .collect()
        );
        assert_eq!(
            document.blocks,
            vec![Block::Paragraph(Paragraph {
                inlines: vec![Inline::Text("content".into())],
                lines: vec!["content".into()],
                id: None,
                reftext: None,
            })]
        );
    }

    #[test]
    fn parses_author_attribute_in_document_header() {
        let document = parse_document("= Document Title\n:author: Jane Doe\n\ncontent");

        assert_eq!(
            document.attributes,
            [
                ("author".to_owned(), "Jane Doe".to_owned()),
                ("authorcount".to_owned(), "1".to_owned()),
                ("authorinitials".to_owned(), "JD".to_owned()),
                ("authors".to_owned(), "Jane Doe".to_owned()),
                ("firstname".to_owned(), "Jane".to_owned()),
                ("lastname".to_owned(), "Doe".to_owned()),
            ]
            .into_iter()
            .collect()
        );
    }

    #[test]
    fn parses_email_attribute_in_document_header() {
        let document = parse_document("= Document Title\n:email: jane@example.com\n\ncontent");

        assert_eq!(
            document.attributes,
            [("email".to_owned(), "jane@example.com".to_owned())]
                .into_iter()
                .collect()
        );
    }

    #[test]
    fn preserves_explicit_authorinitials_for_single_author_attribute() {
        let document =
            parse_document("= Document Title\n:authorinitials: DOC\n:author: Doc Writer\n\ncontent");

        assert_eq!(
            document.attributes,
            [
                ("author".to_owned(), "Doc Writer".to_owned()),
                ("authorcount".to_owned(), "1".to_owned()),
                ("authorinitials".to_owned(), "DOC".to_owned()),
                ("authors".to_owned(), "Doc Writer".to_owned()),
                ("firstname".to_owned(), "Doc".to_owned()),
                ("lastname".to_owned(), "Writer".to_owned()),
            ]
            .into_iter()
            .collect()
        );
    }

    #[test]
    fn parses_authors_attribute_into_indexed_name_parts() {
        let document =
            parse_document("= Document Title\n:authors: Doc Writer; Other Author\n\ncontent");

        assert_eq!(
            document.attributes,
            [
                ("author".to_owned(), "Doc Writer".to_owned()),
                ("author_1".to_owned(), "Doc Writer".to_owned()),
                ("author_2".to_owned(), "Other Author".to_owned()),
                ("authorcount".to_owned(), "2".to_owned()),
                ("authorinitials".to_owned(), "DW".to_owned()),
                ("authorinitials_1".to_owned(), "DW".to_owned()),
                ("authorinitials_2".to_owned(), "OA".to_owned()),
                ("authors".to_owned(), "Doc Writer, Other Author".to_owned()),
                ("firstname".to_owned(), "Doc".to_owned()),
                ("firstname_1".to_owned(), "Doc".to_owned()),
                ("firstname_2".to_owned(), "Other".to_owned()),
                ("lastname".to_owned(), "Writer".to_owned()),
                ("lastname_1".to_owned(), "Writer".to_owned()),
                ("lastname_2".to_owned(), "Author".to_owned()),
            ]
            .into_iter()
            .collect()
        );
    }

    #[test]
    fn parses_middle_name_parts_for_author_attribute() {
        let document =
            parse_document("= Document Title\n:author: Doc Middle Writer\n\ncontent");

        assert_eq!(
            document.attributes,
            [
                ("author".to_owned(), "Doc Middle Writer".to_owned()),
                ("authorcount".to_owned(), "1".to_owned()),
                ("authorinitials".to_owned(), "DMW".to_owned()),
                ("authors".to_owned(), "Doc Middle Writer".to_owned()),
                ("firstname".to_owned(), "Doc".to_owned()),
                ("lastname".to_owned(), "Writer".to_owned()),
                ("middlename".to_owned(), "Middle".to_owned()),
            ]
            .into_iter()
            .collect()
        );
    }

    #[test]
    fn parses_revision_attributes_in_document_header() {
        let document = parse_document(
            "= Document Title\n:revnumber: 1.2\n:revdate: 2026-03-31\n:revremark: Draft\n\ncontent",
        );

        assert_eq!(
            document.attributes,
            [
                ("revdate".to_owned(), "2026-03-31".to_owned()),
                ("revnumber".to_owned(), "1.2".to_owned()),
                ("revremark".to_owned(), "Draft".to_owned())
            ]
            .into_iter()
            .collect()
        );
    }

    #[test]
    fn parses_implicit_author_line_in_document_header() {
        let document = parse_document("= Document Title\nJane Doe\n\ncontent");

        assert_eq!(
            document.attributes,
            [
                ("author".to_owned(), "Jane Doe".to_owned()),
                ("authorcount".to_owned(), "1".to_owned()),
                ("authorinitials".to_owned(), "JD".to_owned()),
                ("authors".to_owned(), "Jane Doe".to_owned()),
                ("firstname".to_owned(), "Jane".to_owned()),
                ("lastname".to_owned(), "Doe".to_owned()),
            ]
            .into_iter()
            .collect()
        );
    }

    #[test]
    fn parses_implicit_author_email_line_in_document_header() {
        let document = parse_document("= Document Title\nStuart Rackham <founder@asciidoc.org>\n\ncontent");

        assert_eq!(
            document.attributes,
            [
                ("author".to_owned(), "Stuart Rackham".to_owned()),
                ("authorcount".to_owned(), "1".to_owned()),
                ("authorinitials".to_owned(), "SR".to_owned()),
                ("authors".to_owned(), "Stuart Rackham".to_owned()),
                ("email".to_owned(), "founder@asciidoc.org".to_owned()),
                ("firstname".to_owned(), "Stuart".to_owned()),
                ("lastname".to_owned(), "Rackham".to_owned()),
            ]
            .into_iter()
            .collect()
        );
    }

    #[test]
    fn parses_implicit_revision_line_in_document_header() {
        let document = parse_document(
            "= Document Title\nStuart Rackham <founder@asciidoc.org>\nv8.6.8, 2012-07-12: See changelog.\n\ncontent",
        );

        assert_eq!(
            document.attributes,
            [
                ("author".to_owned(), "Stuart Rackham".to_owned()),
                ("authorcount".to_owned(), "1".to_owned()),
                ("authorinitials".to_owned(), "SR".to_owned()),
                ("authors".to_owned(), "Stuart Rackham".to_owned()),
                ("email".to_owned(), "founder@asciidoc.org".to_owned()),
                ("firstname".to_owned(), "Stuart".to_owned()),
                ("lastname".to_owned(), "Rackham".to_owned()),
                ("revdate".to_owned(), "2012-07-12".to_owned()),
                ("revnumber".to_owned(), "8.6.8".to_owned()),
                ("revremark".to_owned(), "See changelog.".to_owned()),
            ]
            .into_iter()
            .collect()
        );
    }

    #[test]
    fn parses_implicit_revision_line_without_date() {
        let document =
            parse_document("= Document Title\nAuthor Name\nv1.0.0,:remark\n\ncontent");

        assert_eq!(
            document.attributes,
            [
                ("author".to_owned(), "Author Name".to_owned()),
                ("authorcount".to_owned(), "1".to_owned()),
                ("authorinitials".to_owned(), "AN".to_owned()),
                ("authors".to_owned(), "Author Name".to_owned()),
                ("firstname".to_owned(), "Author".to_owned()),
                ("lastname".to_owned(), "Name".to_owned()),
                ("revnumber".to_owned(), "1.0.0".to_owned()),
                ("revremark".to_owned(), "remark".to_owned()),
            ]
            .into_iter()
            .collect()
        );
    }

    #[test]
    fn parses_implicit_revision_line_without_date_or_remark() {
        let document = parse_document("= Document Title\nAndrew Stanton\nv1.0.0\n\ncontent");

        assert_eq!(
            document.attributes,
            [
                ("author".to_owned(), "Andrew Stanton".to_owned()),
                ("authorcount".to_owned(), "1".to_owned()),
                ("authorinitials".to_owned(), "AS".to_owned()),
                ("authors".to_owned(), "Andrew Stanton".to_owned()),
                ("firstname".to_owned(), "Andrew".to_owned()),
                ("lastname".to_owned(), "Stanton".to_owned()),
                ("revnumber".to_owned(), "1.0.0".to_owned()),
            ]
            .into_iter()
            .collect()
        );
    }

    #[test]
    fn ignores_implicit_revision_line_without_author_line() {
        let document = parse_document("= Document Title\nv1.0.0\n\ncontent");

        assert!(document.attributes.is_empty());
        assert_eq!(
            document.blocks,
            vec![
                Block::Paragraph(Paragraph {
                    inlines: vec![Inline::Text("v1.0.0".into())],
                    lines: vec!["v1.0.0".into()],
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
    fn parses_implicit_metadata_before_explicit_attributes() {
        let document = parse_document(
            "= Document Title\n// author comment\nStuart Rackham <founder@asciidoc.org>\n// revision comment\nv1.0, 2001-01-01\n:toc: left\n\ncontent",
        );

        assert_eq!(
            document.attributes,
            [
                ("author".to_owned(), "Stuart Rackham".to_owned()),
                ("authorcount".to_owned(), "1".to_owned()),
                ("authorinitials".to_owned(), "SR".to_owned()),
                ("authors".to_owned(), "Stuart Rackham".to_owned()),
                ("email".to_owned(), "founder@asciidoc.org".to_owned()),
                ("firstname".to_owned(), "Stuart".to_owned()),
                ("lastname".to_owned(), "Rackham".to_owned()),
                ("revdate".to_owned(), "2001-01-01".to_owned()),
                ("revnumber".to_owned(), "1.0".to_owned()),
                ("toc".to_owned(), "left".to_owned()),
            ]
            .into_iter()
            .collect()
        );
    }

    #[test]
    fn parses_multiple_implicit_authors_without_trailing_semicolon() {
        let document = parse_document(
            "= Document Title\nDoc Writer <thedoctor@asciidoc.org>; Junior Writer <junior@asciidoctor.org>\n\ncontent",
        );

        assert_eq!(
            document.attributes,
            [
                ("author".to_owned(), "Doc Writer".to_owned()),
                ("author_1".to_owned(), "Doc Writer".to_owned()),
                ("author_2".to_owned(), "Junior Writer".to_owned()),
                ("authorcount".to_owned(), "2".to_owned()),
                ("authorinitials".to_owned(), "DW".to_owned()),
                ("authorinitials_1".to_owned(), "DW".to_owned()),
                ("authorinitials_2".to_owned(), "JW".to_owned()),
                ("authors".to_owned(), "Doc Writer, Junior Writer".to_owned()),
                ("email".to_owned(), "thedoctor@asciidoc.org".to_owned()),
                ("email_1".to_owned(), "thedoctor@asciidoc.org".to_owned()),
                ("email_2".to_owned(), "junior@asciidoctor.org".to_owned()),
                ("firstname".to_owned(), "Doc".to_owned()),
                ("firstname_1".to_owned(), "Doc".to_owned()),
                ("firstname_2".to_owned(), "Junior".to_owned()),
                ("lastname".to_owned(), "Writer".to_owned()),
                ("lastname_1".to_owned(), "Writer".to_owned()),
                ("lastname_2".to_owned(), "Writer".to_owned()),
            ]
            .into_iter()
            .collect()
        );
    }

    #[test]
    fn ignores_leading_header_comments_before_document_title() {
        let document = parse_document("// comment one\n// comment two\n= Document Title\n\ncontent");

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
            vec![Block::Paragraph(Paragraph {
                inlines: vec![Inline::Text("content".into())],
                lines: vec!["content".into()],
                id: None,
                reftext: None,
            })]
        );
    }

    #[test]
    fn ignores_header_comments_between_title_and_attributes() {
        let document =
            parse_document("= Document Title\n// comment\n:toc: left\n// another\n\ncontent");

        assert_eq!(
            document.attributes,
            [("toc".to_owned(), "left".to_owned())].into_iter().collect()
        );
        assert_eq!(
            document.blocks,
            vec![Block::Paragraph(Paragraph {
                inlines: vec![Inline::Text("content".into())],
                lines: vec!["content".into()],
                id: None,
                reftext: None,
            })]
        );
    }

    #[test]
    fn ignores_leading_header_comments_without_title() {
        let document = parse_document("// comment one\n// comment two\nbody");

        assert_eq!(document.title, None);
        assert!(document.attributes.is_empty());
        assert_eq!(
            document.blocks,
            vec![Block::Paragraph(Paragraph {
                inlines: vec![Inline::Text("body".into())],
                lines: vec!["body".into()],
                id: None,
                reftext: None,
            })]
        );
    }

    #[test]
    fn stops_parsing_header_attributes_at_first_non_attribute_line() {
        let document = parse_document("= Document Title\n:toc: left\nintro text\n:ignored: value");

        assert_eq!(
            document.attributes,
            [("toc".to_owned(), "left".to_owned())].into_iter().collect()
        );
        assert_eq!(
            document.blocks,
            vec![Block::Paragraph(Paragraph {
                inlines: vec![Inline::Text("intro text\n:ignored: value".into())],
                lines: vec!["intro text".into(), ":ignored: value".into()],
                id: None,
                reftext: None,
            })]
        );
    }

    #[test]
    fn stops_parsing_header_attributes_after_blank_line() {
        let document = parse_document("= Document Title\n:toc: left\n\n:body-attr: value");

        assert_eq!(
            document.attributes,
            [("toc".to_owned(), "left".to_owned())].into_iter().collect()
        );
        assert_eq!(
            document.blocks,
            vec![Block::Paragraph(Paragraph {
                inlines: vec![Inline::Text(":body-attr: value".into())],
                lines: vec![":body-attr: value".into()],
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
    fn parses_numeric_ordered_lists() {
        let document = parse_document("1. first item\n2. second item");

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

    #[test]
    fn parses_nested_lists() {
        let document = parse_document("* parent\n** child\n* sibling");

        assert_eq!(
            document.blocks,
            vec![Block::UnorderedList(UnorderedList {
                items: vec![
                    ListItem {
                        blocks: vec![
                            Block::Paragraph(Paragraph {
                                inlines: vec![Inline::Text("parent".into())],
                                lines: vec!["parent".into()],
                                id: None,
                                reftext: None,
                            }),
                            Block::UnorderedList(UnorderedList {
                                items: vec![ListItem {
                                    blocks: vec![Block::Paragraph(Paragraph {
                                        inlines: vec![Inline::Text("child".into())],
                                        lines: vec!["child".into()],
                                        id: None,
                                        reftext: None,
                                    })],
                                }],
                            }),
                        ],
                    },
                    ListItem {
                        blocks: vec![Block::Paragraph(Paragraph {
                            inlines: vec![Inline::Text("sibling".into())],
                            lines: vec!["sibling".into()],
                            id: None,
                            reftext: None,
                        })],
                    },
                ],
            })]
        );
    }

    #[test]
    fn parses_list_continuation_paragraphs() {
        let document = parse_document("1. first item\n+\ncontinued paragraph\n2. second item");

        assert_eq!(
            document.blocks,
            vec![Block::OrderedList(OrderedList {
                items: vec![
                    ListItem {
                        blocks: vec![
                            Block::Paragraph(Paragraph {
                                inlines: vec![Inline::Text("first item".into())],
                                lines: vec!["first item".into()],
                                id: None,
                                reftext: None,
                            }),
                            Block::Paragraph(Paragraph {
                                inlines: vec![Inline::Text("continued paragraph".into())],
                                lines: vec!["continued paragraph".into()],
                                id: None,
                                reftext: None,
                            }),
                        ],
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
    fn parses_delimited_listing_blocks() {
        let document = parse_document("----\ndef main\n  puts 'hello'\nend\n----");

        assert_eq!(
            document.blocks,
            vec![Block::Listing(Listing {
                lines: vec!["def main".into(), "  puts 'hello'".into(), "end".into()],
                metadata: BlockMetadata::default(),
            })]
        );
    }

    #[test]
    fn parses_delimited_sidebar_blocks() {
        let document = parse_document("****\n* one\n* two\n****");

        assert_eq!(
            document.blocks,
            vec![Block::Sidebar(CompoundBlock {
                blocks: vec![Block::UnorderedList(UnorderedList {
                    items: vec![
                        ListItem {
                            blocks: vec![Block::Paragraph(Paragraph {
                                inlines: vec![Inline::Text("one".into())],
                                lines: vec!["one".into()],
                                id: None,
                                reftext: None,
                            })],
                        },
                        ListItem {
                            blocks: vec![Block::Paragraph(Paragraph {
                                inlines: vec![Inline::Text("two".into())],
                                lines: vec!["two".into()],
                                id: None,
                                reftext: None,
                            })],
                        },
                    ],
                })],
                metadata: BlockMetadata::default(),
            })]
        );
    }

    #[test]
    fn parses_delimited_example_blocks() {
        let document = parse_document("====\nA paragraph.\n====");

        assert_eq!(
            document.blocks,
            vec![Block::Example(CompoundBlock {
                blocks: vec![Block::Paragraph(Paragraph {
                    inlines: vec![Inline::Text("A paragraph.".into())],
                    lines: vec!["A paragraph.".into()],
                    id: None,
                    reftext: None,
                })],
                metadata: BlockMetadata::default(),
            })]
        );
    }

    #[test]
    fn parses_delimited_listing_block_title_and_attributes() {
        let document = parse_document(".Exhibit A\n[source,rust]\n----\nfn main() {}\n----");

        let [Block::Listing(listing)] = document.blocks.as_slice() else {
            panic!("expected listing");
        };
        assert_eq!(listing.metadata.title.as_deref(), Some("Exhibit A"));
        assert_eq!(listing.metadata.style.as_deref(), Some("source"));
        assert_eq!(listing.metadata.attributes.get("$1").map(String::as_str), Some("source"));
        assert_eq!(listing.metadata.attributes.get("$2").map(String::as_str), Some("rust"));
        assert_eq!(listing.metadata.attributes.get("language").map(String::as_str), Some("rust"));
    }

    #[test]
    fn parses_delimited_sidebar_block_attributes() {
        let document = parse_document("[foo=bar,%open,.callout]\n****\ninside\n****");

        let [Block::Sidebar(sidebar)] = document.blocks.as_slice() else {
            panic!("expected sidebar");
        };
        assert_eq!(sidebar.metadata.attributes.get("foo").map(String::as_str), Some("bar"));
        assert_eq!(
            sidebar.metadata.attributes.get("open-option").map(String::as_str),
            Some("")
        );
        assert_eq!(sidebar.metadata.role.as_deref(), Some("callout"));
        assert_eq!(sidebar.metadata.options, vec!["open"]);
        assert_eq!(sidebar.metadata.roles, vec!["callout"]);
    }
}
