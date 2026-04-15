use std::collections::BTreeMap;

use crate::ast::{
    AdmonitionBlock, AdmonitionVariant, Block, BlockMetadata, CalloutItem, CalloutList,
    CompoundBlock, Document, Heading, ImageBlock, ListItem, Listing, OpenBlockContext,
    OrderedList, Paragraph, QuoteBlock, TableBlock, TableCell, TableRow, UnorderedList,
};
use crate::inline::parse_inlines;
use crate::normalize::normalize_asciidoc;

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

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedTableCell {
    content: String,
    colspan: usize,
    rowspan: usize,
    style: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TableFormat {
    Psv,
    Csv,
    Dsv,
}

pub fn parse_document(input: &str) -> Document {
    if input.is_empty() {
        return Document::default();
    }

    let normalized = normalize_asciidoc(input);
    let lines: Vec<&str> = normalized.lines().collect();
    let (mut title, mut attributes, index) = parse_document_header(&lines);
    let blocks = parse_blocks_from_lines(&lines[index..], &mut title, true, Some(&mut attributes));

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
        _ => None,
    };

    if title.is_some() {
        index = skip_header_comments(lines, index);

        if let Some(author_line) = lines
            .get(index)
            .and_then(|line| parse_implicit_author_line(lines, index, line))
        {
            insert_author_attributes(&mut attributes, &author_line.authors);
            index += 1;
            index = skip_header_comments(lines, index);

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
                index += 1;
            }
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

        let Some((name, value, consumed_lines)) = parse_attribute_entry_at(lines, index) else {
            break;
        };
        match name.as_str() {
            "author" => saw_explicit_author = true,
            "authors" => saw_explicit_authors = true,
            "authorinitials" => saw_explicit_authorinitials = true,
            _ => {}
        }
        attributes.insert(name, value);
        index += consumed_lines;
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
    document_attributes: Option<&mut BTreeMap<String, String>>,
) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut index = 0;
    let mut current_paragraph = Vec::new();
    let mut current_paragraph_anchor = None;
    let mut current_paragraph_prelude = None::<BlockPrelude>;
    let mut pending_anchor = None;
    let mut pending_block_prelude = None::<BlockPrelude>;
    let mut document_attributes = document_attributes;

    while index < lines.len() {
        let line = lines[index];

        // Block comment delimiter: consume everything until the matching closing delimiter.
        if let Some((delimiter, "comment")) = parse_delimited_block_marker(line) {
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
                &mut current_paragraph,
                &mut current_paragraph_anchor,
                &mut current_paragraph_prelude,
            );
            pending_anchor = Some(anchor);
            index += 1;
            continue;
        }

        if current_paragraph.is_empty() && pending_block_prelude.is_none() {
            if let Some(prelude) = try_parse_block_prelude(lines, index) {
                pending_block_prelude = Some(prelude.clone());
                index += prelude.consumed_lines;
                continue;
            }
        }

        if let Some((block, consumed_lines)) =
            parse_delimited_block(lines, index, pending_block_prelude.as_ref())
        {
            flush_paragraph(
                &mut blocks,
                &mut current_paragraph,
                &mut current_paragraph_anchor,
                &mut current_paragraph_prelude,
            );
            push_block(
                &mut blocks,
                title,
                allow_document_title,
                block,
                pending_anchor.take(),
            );
            pending_block_prelude = None;
            index += consumed_lines;
            continue;
        }

        let heading = if current_paragraph.is_empty() {
            parse_heading(lines, index)
        } else {
            parse_atx_heading(lines[index]).map(|heading| (heading, 1))
        };
        if let Some((heading, consumed_lines)) = heading {
            flush_paragraph(
                &mut blocks,
                &mut current_paragraph,
                &mut current_paragraph_anchor,
                &mut current_paragraph_prelude,
            );
            push_block(
                &mut blocks,
                title,
                allow_document_title,
                Block::Heading(apply_prelude_to_heading(
                    heading,
                    pending_block_prelude.take(),
                )),
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
                &mut current_paragraph_prelude,
            );
            push_block(
                &mut blocks,
                title,
                allow_document_title,
                Block::UnorderedList(apply_prelude_to_unordered_list(
                    list,
                    pending_block_prelude.take(),
                )),
                pending_anchor.take(),
            );
            index += consumed_lines;
            continue;
        }

        if let Some((list, consumed_lines)) = parse_description_list(lines, index) {
            flush_paragraph(
                &mut blocks,
                &mut current_paragraph,
                &mut current_paragraph_anchor,
                &mut current_paragraph_prelude,
            );
            push_block(
                &mut blocks,
                title,
                allow_document_title,
                Block::DescriptionList(apply_prelude_to_description_list(
                    list,
                    pending_block_prelude.take(),
                )),
                pending_anchor.take(),
            );
            index += consumed_lines;
            continue;
        }

        if let Some((list, consumed_lines)) = parse_ordered_list(lines, index) {
            flush_paragraph(
                &mut blocks,
                &mut current_paragraph,
                &mut current_paragraph_anchor,
                &mut current_paragraph_prelude,
            );
            push_block(
                &mut blocks,
                title,
                allow_document_title,
                Block::OrderedList(apply_prelude_to_ordered_list(
                    list,
                    pending_block_prelude.take(),
                )),
                pending_anchor.take(),
            );
            index += consumed_lines;
            continue;
        }

        if let Some((colist, consumed_lines)) = parse_callout_list(lines, index) {
            flush_paragraph(
                &mut blocks,
                &mut current_paragraph,
                &mut current_paragraph_anchor,
                &mut current_paragraph_prelude,
            );
            push_block(
                &mut blocks,
                title,
                allow_document_title,
                colist,
                pending_anchor.take(),
            );
            pending_block_prelude = None;
            index += consumed_lines;
            continue;
        }

        if let Some((table, consumed_lines)) =
            parse_table(lines, index, pending_block_prelude.as_ref())
        {
            flush_paragraph(
                &mut blocks,
                &mut current_paragraph,
                &mut current_paragraph_anchor,
                &mut current_paragraph_prelude,
            );
            push_block(
                &mut blocks,
                title,
                allow_document_title,
                Block::Table(apply_prelude_to_table(table, pending_block_prelude.take())),
                pending_anchor.take(),
            );
            index += consumed_lines;
            continue;
        }

        if current_paragraph.is_empty()
            && pending_block_prelude.is_none()
            && pending_anchor.is_none()
        {
            if let Some((name, value, consumed_lines)) = parse_attribute_entry_at(lines, index) {
                if let Some(attributes) = document_attributes.as_deref_mut() {
                    attributes.insert(name, value);
                    index += consumed_lines;
                    continue;
                }
            }
        }

        if current_paragraph.is_empty() && line.trim() == "toc::[]" {
            flush_paragraph(
                &mut blocks,
                &mut current_paragraph,
                &mut current_paragraph_anchor,
                &mut current_paragraph_prelude,
            );
            push_block(
                &mut blocks,
                title,
                allow_document_title,
                Block::Toc,
                pending_anchor.take(),
            );
            pending_block_prelude = None;
            index += 1;
            continue;
        }

        if current_paragraph.is_empty() {
            if let Some(image) = parse_block_image(line) {
                flush_paragraph(
                    &mut blocks,
                    &mut current_paragraph,
                    &mut current_paragraph_anchor,
                    &mut current_paragraph_prelude,
                );
                push_block(
                    &mut blocks,
                    title,
                    allow_document_title,
                    Block::Image(apply_prelude_to_image(image, pending_block_prelude.take())),
                    pending_anchor.take(),
                );
                index += 1;
                continue;
            }
        }

        if current_paragraph.is_empty() {
            if let Some((admonition, consumed_lines)) =
                parse_admonition_paragraph(lines, index, pending_block_prelude.as_ref())
            {
                push_block(
                    &mut blocks,
                    title,
                    allow_document_title,
                    admonition,
                    pending_anchor.take(),
                );
                pending_block_prelude = None;
                index += consumed_lines;
                continue;
            }
        }

        if line.trim().is_empty() {
            flush_paragraph(
                &mut blocks,
                &mut current_paragraph,
                &mut current_paragraph_anchor,
                &mut current_paragraph_prelude,
            );
            pending_block_prelude = None;
            index += 1;
            continue;
        }

        if current_paragraph.is_empty() {
            current_paragraph_anchor = pending_anchor.take();
            current_paragraph_prelude = pending_block_prelude.take();
        }
        current_paragraph.push(line.to_owned());
        index += 1;
    }

    flush_paragraph(
        &mut blocks,
        &mut current_paragraph,
        &mut current_paragraph_anchor,
        &mut current_paragraph_prelude,
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
        Block::Admonition(admonition) => {
            blocks.push(Block::Admonition(apply_anchor_to_admonition(
                admonition, anchor,
            )));
        }
        Block::UnorderedList(list) => {
            blocks.push(Block::UnorderedList(apply_anchor_to_unordered_list(
                list, anchor,
            )));
        }
        Block::DescriptionList(list) => {
            blocks.push(Block::DescriptionList(apply_anchor_to_description_list(
                list, anchor,
            )));
        }
        Block::OrderedList(list) => {
            blocks.push(Block::OrderedList(apply_anchor_to_ordered_list(
                list, anchor,
            )));
        }
        Block::Listing(listing) => {
            blocks.push(Block::Listing(apply_anchor_to_listing(listing, anchor)));
        }
        Block::Literal(literal) => {
            blocks.push(Block::Literal(apply_anchor_to_listing(literal, anchor)));
        }
        Block::Table(table) => {
            blocks.push(Block::Table(apply_anchor_to_table(table, anchor)));
        }
        Block::Example(example) => {
            blocks.push(Block::Example(apply_anchor_to_compound_block(
                example, anchor,
            )));
        }
        Block::Sidebar(sidebar) => {
            blocks.push(Block::Sidebar(apply_anchor_to_compound_block(
                sidebar, anchor,
            )));
        }
        Block::Open(open) => {
            blocks.push(Block::Open(apply_anchor_to_compound_block(open, anchor)));
        }
        other => blocks.push(other),
    }
}

fn apply_prelude_to_heading(mut heading: Heading, prelude: Option<BlockPrelude>) -> Heading {
    if let Some(prelude) = prelude {
        if heading.id.is_none() {
            heading.id = prelude.metadata.id.clone();
        }
        heading.metadata = prelude.metadata;
    }
    heading
}

fn apply_prelude_to_unordered_list(
    mut list: UnorderedList,
    prelude: Option<BlockPrelude>,
) -> UnorderedList {
    if let Some(prelude) = prelude {
        list.metadata = prelude.metadata;
    }
    list
}

fn apply_anchor_to_unordered_list(
    mut list: UnorderedList,
    anchor: Option<PendingAnchor>,
) -> UnorderedList {
    if let Some(anchor) = anchor
        && list.metadata.id.is_none()
    {
        list.metadata.id = Some(anchor.id);
        list.reftext = anchor.reftext;
    }
    list
}

fn apply_prelude_to_ordered_list(
    mut list: OrderedList,
    prelude: Option<BlockPrelude>,
) -> OrderedList {
    if let Some(prelude) = prelude {
        list.metadata = prelude.metadata;
    }
    list
}

fn apply_prelude_to_table(mut table: TableBlock, prelude: Option<BlockPrelude>) -> TableBlock {
    if let Some(prelude) = prelude {
        table.metadata = prelude.metadata;
    }
    table
}

fn apply_anchor_to_ordered_list(
    mut list: OrderedList,
    anchor: Option<PendingAnchor>,
) -> OrderedList {
    if let Some(anchor) = anchor
        && list.metadata.id.is_none()
    {
        list.metadata.id = Some(anchor.id);
        list.reftext = anchor.reftext;
    }
    list
}

fn apply_prelude_to_description_list(
    mut list: crate::ast::DescriptionList,
    prelude: Option<BlockPrelude>,
) -> crate::ast::DescriptionList {
    if let Some(prelude) = prelude {
        list.metadata = prelude.metadata;
    }
    list
}

fn apply_anchor_to_description_list(
    mut list: crate::ast::DescriptionList,
    anchor: Option<PendingAnchor>,
) -> crate::ast::DescriptionList {
    if let Some(anchor) = anchor
        && list.metadata.id.is_none()
    {
        list.metadata.id = Some(anchor.id);
        list.reftext = anchor.reftext;
    }
    list
}

fn apply_anchor_to_table(mut table: TableBlock, anchor: Option<PendingAnchor>) -> TableBlock {
    if let Some(anchor) = anchor
        && table.metadata.id.is_none()
    {
        table.metadata.id = Some(anchor.id);
        table.reftext = anchor.reftext;
    }
    table
}

fn apply_prelude_to_admonition(
    mut admonition: AdmonitionBlock,
    prelude: Option<BlockPrelude>,
) -> AdmonitionBlock {
    if let Some(prelude) = prelude {
        if admonition.id.is_none() {
            admonition.id = prelude.metadata.id.clone();
        }
        admonition.metadata = prelude.metadata;
    }
    admonition
}

fn apply_prelude_to_image(mut image: ImageBlock, prelude: Option<BlockPrelude>) -> ImageBlock {
    if let Some(prelude) = prelude {
        image.metadata = prelude.metadata;
    }
    image
}

fn parse_block_image(line: &str) -> Option<ImageBlock> {
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
    let (alt, width, height, named_attrs) = parse_image_attributes(attr_text, &target);

    Some(ImageBlock {
        target,
        alt,
        width,
        height,
        metadata: BlockMetadata {
            attributes: named_attrs,
            ..Default::default()
        },
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
    let mut named_attrs = BTreeMap::new();
    let mut positional = Vec::new();

    if !attr_text.is_empty() {
        for part in split_image_attrs(attr_text) {
            let part = part.trim();
            if let Some((key, value)) = part.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches('"').trim_matches('\'');
                named_attrs.insert(key.to_owned(), value.to_owned());
            } else {
                positional.push(part.to_owned());
            }
        }
    }

    let alt = positional
        .first()
        .filter(|s| !s.is_empty())
        .cloned()
        .unwrap_or_else(|| auto_generate_alt(target));
    let width = positional.get(1).filter(|s| !s.is_empty()).cloned();
    let height = positional.get(2).filter(|s| !s.is_empty()).cloned();

    (alt, width, height, named_attrs)
}

/// Split on commas but respect quoted values.
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

fn auto_generate_alt(target: &str) -> String {
    let filename = target.rsplit('/').next().unwrap_or(target);
    let filename = filename.rsplit('\\').next().unwrap_or(filename);
    let stem = filename
        .rsplit_once('.')
        .map(|(s, _)| s)
        .unwrap_or(filename);
    stem.replace('-', " ").replace('_', " ")
}

fn parse_table(
    lines: &[&str],
    index: usize,
    pending_block_prelude: Option<&BlockPrelude>,
) -> Option<(TableBlock, usize)> {
    let delimiter = lines.get(index)?.trim();
    let (delimiter, delimiter_char) = parse_table_delimiter(delimiter)?;
    let metadata = pending_block_prelude.map(|prelude| &prelude.metadata);

    let header_enabled = pending_block_prelude
        .map(|prelude| table_has_header_option(&prelude.metadata))
        .unwrap_or(false);
    let expected_columns =
        pending_block_prelude.and_then(|prelude| table_column_count(&prelude.metadata));
    let format = table_format(metadata, delimiter_char);
    let separator = table_separator(metadata, format, delimiter_char)?;
    let (row_groups, consumed) = match format {
        TableFormat::Psv => {
            parse_psv_table_rows(lines, index, delimiter, separator, expected_columns)?
        }
        TableFormat::Csv | TableFormat::Dsv => {
            parse_separated_value_table_rows(lines, index, delimiter, separator)?
        }
    };

    if consumed < 2 || row_groups.is_empty() || index + consumed > lines.len() {
        return None;
    }

    let mut rows = if let Some(column_count) = expected_columns {
        assemble_table_rows_with_known_columns(&row_groups, column_count)?
    } else {
        assemble_table_rows_without_known_columns(&row_groups)?
    };

    let header = header_enabled.then(|| rows.remove(0));
    Some((
        TableBlock {
            header,
            rows,
            reftext: None,
            metadata: BlockMetadata::default(),
        },
        consumed,
    ))
}

fn parse_psv_table_rows(
    lines: &[&str],
    index: usize,
    delimiter: &str,
    separator: char,
    expected_columns: Option<usize>,
) -> Option<(Vec<Vec<ParsedTableCell>>, usize)> {
    let mut row_groups: Vec<Vec<ParsedTableCell>> = Vec::new();
    let mut current_group: Vec<ParsedTableCell> = Vec::new();
    let mut current_cell: Option<ParsedTableCell> = None;
    let mut consumed = 1;
    let mut closed = false;

    while index + consumed < lines.len() {
        let line = lines[index + consumed];
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
            let next_nonempty = next_nonempty_table_line(lines, index + consumed + 1);
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
            }
            consumed += 1;
            continue;
        }

        if starts_table_cell_line(trimmed, separator) {
            let had_cells_in_group = !current_group.is_empty();
            if let Some(cell) = current_cell.take() {
                current_group.push(cell);
                maybe_finish_table_row_group(&mut row_groups, &mut current_group, expected_columns);
                if expected_columns.is_none() && had_cells_in_group && !current_group.is_empty() {
                    row_groups.push(std::mem::take(&mut current_group));
                }
            }
            let cells = parse_table_cells_from_line(line, separator)?;
            if cells.is_empty() {
                return None;
            }
            let mut iter = cells.into_iter();
            let last = iter.next_back()?;
            for cell in iter {
                current_group.push(cell);
                maybe_finish_table_row_group(&mut row_groups, &mut current_group, expected_columns);
            }
            current_cell = Some(last);
        } else {
            let cell = current_cell.as_mut()?;
            if !cell.content.is_empty() {
                cell.content.push('\n');
            }
            cell.content.push_str(line);
        }
        consumed += 1;
    }

    closed.then_some((row_groups, consumed))
}

fn parse_separated_value_table_rows(
    lines: &[&str],
    index: usize,
    delimiter: &str,
    separator: char,
) -> Option<(Vec<Vec<ParsedTableCell>>, usize)> {
    let closing_index = lines[index + 1..]
        .iter()
        .position(|line| line.trim() == delimiter)
        .map(|offset| index + 1 + offset)?;
    let consumed = closing_index - index + 1;
    let content = lines[index + 1..closing_index].join("\n");

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(separator as u8)
        .flexible(true)
        .from_reader(content.as_bytes());
    let mut row_groups = Vec::new();
    for record in reader.records() {
        let record = record.ok()?;
        row_groups.push(
            record
                .iter()
                .map(|value| ParsedTableCell {
                    content: value.to_owned(),
                    colspan: 1,
                    rowspan: 1,
                    style: None,
                })
                .collect(),
        );
    }

    Some((row_groups, consumed))
}

fn maybe_finish_table_row_group(
    row_groups: &mut Vec<Vec<ParsedTableCell>>,
    current_group: &mut Vec<ParsedTableCell>,
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

fn parse_table_delimiter(line: &str) -> Option<(&str, char)> {
    let trimmed = line.trim();
    let mut chars = trimmed.chars();
    let marker = chars.next()?;
    if !matches!(marker, '|' | ',' | ':' | '!') {
        return None;
    }
    let rest = chars.as_str();
    (rest.len() >= 3 && rest.chars().all(|ch| ch == '=')).then_some((trimmed, marker))
}

fn parse_table_cells_from_line(line: &str, separator: char) -> Option<Vec<ParsedTableCell>> {
    let trimmed = line.trim_start();
    let segments = split_table_row_cells_after_marker(trimmed, separator);
    if segments.len() < 2 {
        return None;
    }

    let (colspan, rowspan, style) = parse_table_cell_spec(segments[0].trim())?;
    let mut cells = vec![ParsedTableCell {
        content: segments[1].trim().to_owned(),
        colspan,
        rowspan,
        style,
    }];

    parse_table_cells_from_segments(&segments, 2, &mut cells).then_some(cells)
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

fn parse_table_cells_from_segments(
    segments: &[String],
    index: usize,
    cells: &mut Vec<ParsedTableCell>,
) -> bool {
    if index >= segments.len() {
        return true;
    }

    if index + 1 < segments.len() {
        let spec = segments[index].trim();
        if !spec.is_empty()
            && let Some((colspan, rowspan, style)) = parse_table_cell_spec(spec)
        {
            cells.push(ParsedTableCell {
                content: segments[index + 1].trim().to_owned(),
                colspan,
                rowspan,
                style,
            });
            if parse_table_cells_from_segments(segments, index + 2, cells) {
                return true;
            }
            cells.pop();
        }
    }

    cells.push(ParsedTableCell {
        content: segments[index].trim().to_owned(),
        colspan: 1,
        rowspan: 1,
        style: None,
    });
    if parse_table_cells_from_segments(segments, index + 1, cells) {
        return true;
    }
    cells.pop();
    false
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

fn table_format(metadata: Option<&BlockMetadata>, delimiter_char: char) -> TableFormat {
    match metadata
        .and_then(|metadata| metadata.attributes.get("format"))
        .map(|format| format.trim().to_ascii_lowercase())
        .as_deref()
    {
        Some("csv") => TableFormat::Csv,
        Some("dsv") => TableFormat::Dsv,
        _ => match delimiter_char {
            ',' => TableFormat::Csv,
            ':' => TableFormat::Dsv,
            _ => TableFormat::Psv,
        },
    }
}

fn table_separator(
    metadata: Option<&BlockMetadata>,
    format: TableFormat,
    delimiter_char: char,
) -> Option<char> {
    metadata
        .and_then(|metadata| metadata.attributes.get("separator"))
        .and_then(|separator| parse_table_separator_attribute(separator))
        .or(match format {
            TableFormat::Csv => Some(','),
            TableFormat::Dsv => Some(':'),
            TableFormat::Psv => Some(if delimiter_char == '!' { '!' } else { '|' }),
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

fn table_has_header_option(metadata: &BlockMetadata) -> bool {
    metadata.options.iter().any(|option| option == "header")
        || metadata
            .attributes
            .get("options")
            .is_some_and(|options| options.split(',').any(|option| option.trim() == "header"))
        || metadata.attributes.contains_key("header-option")
}

fn assemble_table_rows_with_known_columns(
    row_groups: &[Vec<ParsedTableCell>],
    column_count: usize,
) -> Option<Vec<TableRow>> {
    if column_count == 0 {
        return None;
    }

    if row_groups.len() > 1 {
        return assemble_explicit_table_rows_with_known_columns(row_groups, column_count);
    }

    let mut rows = Vec::new();
    let mut current_row = Vec::new();
    let mut current_width = 0;
    for cell in row_groups.iter().flatten() {
        current_width += cell.colspan.max(1);
        current_row.push(build_table_cell(cell));
        if current_width == column_count {
            rows.push(TableRow {
                cells: std::mem::take(&mut current_row),
            });
            current_width = 0;
        }
    }

    if !current_row.is_empty() {
        rows.push(TableRow { cells: current_row });
    }

    Some(rows)
}

fn assemble_explicit_table_rows_with_known_columns(
    row_groups: &[Vec<ParsedTableCell>],
    column_count: usize,
) -> Option<Vec<TableRow>> {
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

        rows.push(TableRow {
            cells: group.iter().map(build_table_cell).collect(),
        });
        active_rowspans = next_rowspans;
    }

    Some(rows)
}

fn build_table_cell(cell: &ParsedTableCell) -> TableCell {
    let normalized = normalize_table_cell_content(&cell.content);
    let lines: Vec<&str> = normalized.lines().collect();
    let mut title = None;
    let blocks = if lines.is_empty() {
        Vec::new()
    } else {
        parse_blocks_from_lines(&lines, &mut title, false, None)
    };
    let paragraph_inlines = blocks
        .first()
        .and_then(|block| match block {
            Block::Paragraph(paragraph) if blocks.len() == 1 => Some(paragraph.inlines.clone()),
            _ => None,
        })
        .unwrap_or_default();

    TableCell {
        content: blocks_plain_text(&blocks),
        inlines: paragraph_inlines,
        blocks,
        colspan: cell.colspan.max(1),
        rowspan: cell.rowspan.max(1),
        style: cell.style.clone(),
    }
}

fn normalize_table_cell_content(content: &str) -> String {
    content
        .lines()
        .map(|line| if line.trim() == "+" { "" } else { line })
        .collect::<Vec<_>>()
        .join("\n")
}

fn assemble_table_rows_without_known_columns(
    row_groups: &[Vec<ParsedTableCell>],
) -> Option<Vec<TableRow>> {
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
        return assemble_explicit_table_rows_with_known_columns(row_groups, inferred_columns);
    }

    Some(
        row_groups
            .iter()
            .map(|group| TableRow {
                cells: group.iter().map(build_table_cell).collect(),
            })
            .collect(),
    )
}

fn blocks_plain_text(blocks: &[Block]) -> String {
    blocks
        .iter()
        .map(block_plain_text)
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn block_plain_text(block: &Block) -> String {
    match block {
        Block::Paragraph(paragraph) => paragraph.plain_text(),
        Block::Admonition(admonition) => blocks_plain_text(&admonition.blocks),
        Block::UnorderedList(list) => list
            .items
            .iter()
            .map(|item| blocks_plain_text(&item.blocks))
            .collect::<Vec<_>>()
            .join("\n"),
        Block::OrderedList(list) => list
            .items
            .iter()
            .map(|item| blocks_plain_text(&item.blocks))
            .collect::<Vec<_>>()
            .join("\n"),
        Block::Table(table) => table
            .rows
            .iter()
            .flat_map(|row| row.cells.iter().map(|cell| cell.content.clone()))
            .collect::<Vec<_>>()
            .join("\n"),
        Block::Listing(listing) | Block::Literal(listing) => listing.lines.join("\n"),
        Block::Example(example) | Block::Sidebar(example) | Block::Open(example) => {
            blocks_plain_text(&example.blocks)
        }
        Block::Quote(quote) => {
            if let Some(content) = &quote.content {
                content.clone()
            } else {
                blocks_plain_text(&quote.blocks)
            }
        }
        Block::Passthrough(passthrough) => passthrough.content.clone(),
        Block::Image(image) => image.alt.clone(),
        Block::Heading(heading) => heading.title.clone(),
        Block::Toc => String::new(),
        Block::CalloutList(_) => String::new(),
        Block::DescriptionList(list) => list
            .items
            .iter()
            .map(|item| {
                let mut text = String::new();
                for term in &item.terms {
                    text.push_str(&term.text);
                    text.push('\n');
                }
                if let Some(desc) = &item.description {
                    text.push_str(&blocks_plain_text(&desc.blocks));
                }
                text
            })
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

fn table_column_count(metadata: &BlockMetadata) -> Option<usize> {
    let cols = metadata.attributes.get("cols")?;
    let parts = cols.split(',').map(str::trim).collect::<Vec<_>>();
    (!parts.is_empty() && parts.iter().any(|part| !part.is_empty())).then_some(parts.len())
}

fn apply_anchor_to_listing(mut listing: Listing, anchor: Option<PendingAnchor>) -> Listing {
    if let Some(anchor) = anchor
        && listing.metadata.id.is_none()
    {
        listing.metadata.id = Some(anchor.id);
        listing.reftext = anchor.reftext;
    }
    listing
}

fn apply_anchor_to_compound_block(
    mut block: CompoundBlock,
    anchor: Option<PendingAnchor>,
) -> CompoundBlock {
    if let Some(anchor) = anchor
        && block.metadata.id.is_none()
    {
        block.metadata.id = Some(anchor.id);
        block.reftext = anchor.reftext;
    }
    block
}

fn apply_anchor_to_admonition(
    mut admonition: AdmonitionBlock,
    anchor: Option<PendingAnchor>,
) -> AdmonitionBlock {
    if let Some(anchor) = anchor {
        if admonition.id.is_none() {
            admonition.id = Some(anchor.id);
        }
        if admonition.reftext.is_none() {
            admonition.reftext = anchor.reftext;
        }
    }
    admonition
}

fn clear_resolved_style(metadata: &mut BlockMetadata) {
    metadata.style = None;
    metadata.attributes.remove("style");
}

fn parse_delimited_block(
    lines: &[&str],
    index: usize,
    pending_prelude: Option<&BlockPrelude>,
) -> Option<(Block, usize)> {
    let prelude = pending_prelude.cloned().unwrap_or_default();
    let delimiter_index = index;
    let delimiter_line = lines.get(delimiter_index)?;
    let fenced_entries = parse_fenced_code_opening(delimiter_line);
    let (delimiter, block_kind) = parse_delimited_block_marker(delimiter_line)?;
    if block_kind == "comment" {
        return None;
    }

    let closing_index = lines[delimiter_index + 1..]
        .iter()
        .position(|line| line.trim() == delimiter)
        .map(|offset| delimiter_index + 1 + offset)?;
    let inner_lines = &lines[delimiter_index + 1..closing_index];
    let consumed = closing_index - index + 1;

    if block_kind == "example"
        && let Some(variant) = prelude
            .metadata
            .style
            .as_deref()
            .and_then(admonition_variant_from_style)
    {
        let mut nested_title = None;
        return Some((
            Block::Admonition(AdmonitionBlock {
                variant,
                blocks: parse_blocks_from_lines(inner_lines, &mut nested_title, false, None),
                id: prelude.metadata.id.clone(),
                reftext: None,
                metadata: prelude.metadata,
            }),
            consumed,
        ));
    }

    let effective_block_kind = match (block_kind, prelude.metadata.style.as_deref()) {
        ("listing", Some("literal")) => "literal",
        ("literal", Some("listing" | "source")) => "listing",
        _ => block_kind,
    };

    let block = match effective_block_kind {
        "passthrough" => Block::Passthrough(crate::ast::PassthroughBlock {
            content: inner_lines.join("\n"),
            reftext: None,
            metadata: prelude.metadata,
        }),
        "listing" => {
            let mut metadata = prelude.metadata;
            if metadata.style.as_deref() == Some("listing") {
                clear_resolved_style(&mut metadata);
            }
            if let Some(entries) = fenced_entries.as_ref() {
                apply_fenced_code_metadata(&mut metadata, entries);
            }
            let lines = inner_lines
                .iter()
                .map(|line| (*line).to_owned())
                .collect::<Vec<_>>();
            Block::Listing(make_listing_from_lines(lines, None, metadata))
        }
        "literal" => {
            let mut metadata = prelude.metadata;
            if metadata.style.as_deref() == Some("literal") {
                clear_resolved_style(&mut metadata);
            }
            Block::Literal(Listing {
                lines: inner_lines.iter().map(|line| (*line).to_owned()).collect(),
                callouts: vec![],
                reftext: None,
                metadata,
            })
        }
        "example" => {
            let mut metadata = prelude.metadata;
            clear_resolved_style(&mut metadata);
            let mut nested_title = None;
            Block::Example(CompoundBlock {
                blocks: parse_blocks_from_lines(inner_lines, &mut nested_title, false, None),
                reftext: None,
                context: None,
                metadata,
            })
        }
        "sidebar" => {
            let mut metadata = prelude.metadata;
            clear_resolved_style(&mut metadata);
            let mut nested_title = None;
            Block::Sidebar(CompoundBlock {
                blocks: parse_blocks_from_lines(inner_lines, &mut nested_title, false, None),
                reftext: None,
                context: None,
                metadata,
            })
        }
        "quote" => {
            let is_verse = prelude.metadata.style.as_deref() == Some("verse");
            let mut metadata = prelude.metadata;
            clear_resolved_style(&mut metadata);
            let attribution = metadata.attributes.get("$2").cloned();
            let citetitle = metadata.attributes.get("$3").cloned();
            if is_verse {
                Block::Quote(QuoteBlock {
                    blocks: vec![],
                    content: Some(inner_lines.join("\n")),
                    attribution,
                    citetitle,
                    is_verse: true,
                    reftext: None,
                    metadata,
                })
            } else {
                let mut nested_title = None;
                Block::Quote(QuoteBlock {
                    blocks: parse_blocks_from_lines(inner_lines, &mut nested_title, false, None),
                    content: None,
                    attribution,
                    citetitle,
                    is_verse: false,
                    reftext: None,
                    metadata,
                })
            }
        }
        "open" => {
            // Styled open block: redirect to the appropriate block type.
            masquerade_open_block(prelude.metadata, inner_lines)
        }
        _ => return None,
    };

    Some((block, consumed))
}

fn masquerade_open_block(mut metadata: BlockMetadata, inner_lines: &[&str]) -> Block {
    let style = metadata.style.clone().unwrap_or_default();
    if let Some(variant) = admonition_variant_from_style(&style) {
        let mut nested_title = None;
        return Block::Admonition(AdmonitionBlock {
            variant,
            blocks: parse_blocks_from_lines(inner_lines, &mut nested_title, false, None),
            id: metadata.id.clone(),
            reftext: None,
            metadata,
        });
    }

    match style.as_str() {
        "literal" => Block::Literal(Listing {
            lines: {
                clear_resolved_style(&mut metadata);
                inner_lines.iter().map(|line| (*line).to_owned()).collect()
            },
            callouts: vec![],
            reftext: None,
            metadata,
        }),
        "listing" => {
            clear_resolved_style(&mut metadata);
            Block::Listing(make_listing_from_lines(
                inner_lines.iter().map(|line| (*line).to_owned()).collect(),
                None,
                metadata,
            ))
        }
        "source" => Block::Listing(make_listing_from_lines(
            inner_lines.iter().map(|line| (*line).to_owned()).collect(),
            None,
            metadata,
        )),
        "sidebar" => {
            clear_resolved_style(&mut metadata);
            let mut nested_title = None;
            Block::Sidebar(CompoundBlock {
                blocks: parse_blocks_from_lines(inner_lines, &mut nested_title, false, None),
                reftext: None,
                context: None,
                metadata,
            })
        }
        "example" => {
            clear_resolved_style(&mut metadata);
            let mut nested_title = None;
            Block::Example(CompoundBlock {
                blocks: parse_blocks_from_lines(inner_lines, &mut nested_title, false, None),
                reftext: None,
                context: None,
                metadata,
            })
        }
        "quote" => {
            clear_resolved_style(&mut metadata);
            let mut nested_title = None;
            Block::Quote(QuoteBlock {
                blocks: parse_blocks_from_lines(inner_lines, &mut nested_title, false, None),
                content: None,
                attribution: metadata.attributes.get("$2").cloned(),
                citetitle: metadata.attributes.get("$3").cloned(),
                is_verse: false,
                reftext: None,
                metadata,
            })
        }
        "verse" => {
            clear_resolved_style(&mut metadata);
            Block::Quote(QuoteBlock {
                blocks: vec![],
                content: Some(inner_lines.join("\n")),
                attribution: metadata.attributes.get("$2").cloned(),
                citetitle: metadata.attributes.get("$3").cloned(),
                is_verse: true,
                reftext: None,
                metadata,
            })
        }
        "pass" => {
            clear_resolved_style(&mut metadata);
            Block::Passthrough(crate::ast::PassthroughBlock {
                content: inner_lines.join("\n"),
                reftext: None,
                metadata,
            })
        }
        "stem" | "latexmath" | "asciimath" => Block::Passthrough(crate::ast::PassthroughBlock {
            content: inner_lines.join("\n"),
            reftext: None,
            metadata,
        }),
        "abstract" | "comment" | "partintro" => {
            let context = match style.as_str() {
                "abstract" => OpenBlockContext::Abstract,
                "comment" => OpenBlockContext::Comment,
                "partintro" => OpenBlockContext::PartIntro,
                _ => unreachable!(),
            };
            clear_resolved_style(&mut metadata);
            let mut nested_title = None;
            Block::Open(CompoundBlock {
                blocks: parse_blocks_from_lines(inner_lines, &mut nested_title, false, None),
                reftext: None,
                context: Some(context),
                metadata,
            })
        }
        "open" => {
            clear_resolved_style(&mut metadata);
            let mut nested_title = None;
            Block::Open(CompoundBlock {
                blocks: parse_blocks_from_lines(inner_lines, &mut nested_title, false, None),
                reftext: None,
                context: None,
                metadata,
            })
        }
        _ => {
            let mut nested_title = None;
            Block::Open(CompoundBlock {
                blocks: parse_blocks_from_lines(inner_lines, &mut nested_title, false, None),
                reftext: None,
                context: None,
                metadata,
            })
        }
    }
}

fn parse_admonition_paragraph(
    lines: &[&str],
    index: usize,
    pending_prelude: Option<&BlockPrelude>,
) -> Option<(Block, usize)> {
    let (variant, first_line) = parse_admonition_prefix(lines.get(index)?)?;
    let mut paragraph_lines = vec![first_line.to_owned()];
    let mut consumed = 1;

    while index + consumed < lines.len() {
        let line = lines[index + consumed];
        if line.trim().is_empty()
            || parse_block_anchor(line).is_some()
            || parse_heading(lines, index + consumed).is_some()
            || parse_list_marker(line).is_some()
            || is_delimited_block_delimiter(line)
        {
            break;
        }

        paragraph_lines.push(line.to_owned());
        consumed += 1;
    }

    Some((
        Block::Admonition(apply_prelude_to_admonition(
            AdmonitionBlock {
                variant,
                blocks: vec![make_paragraph_like_block(paragraph_lines)],
                id: pending_prelude.and_then(|prelude| prelude.metadata.id.clone()),
                reftext: None,
                metadata: BlockMetadata::default(),
            },
            pending_prelude.cloned(),
        )),
        consumed,
    ))
}

fn try_parse_block_prelude(lines: &[&str], index: usize) -> Option<BlockPrelude> {
    let mut prelude = BlockPrelude::default();
    let mut cursor = index;

    if let Some(title) = lines.get(cursor).and_then(|line| parse_block_title(line)) {
        let next = cursor + 1;
        if lines.get(next).is_some_and(|line| !line.trim().is_empty()) {
            prelude.metadata.title = Some(title.clone());
            prelude.metadata.attributes.insert("title".into(), title);
            cursor += 1;
        }
    }

    while let Some(attr_line) = lines
        .get(cursor)
        .and_then(|line| parse_attribute_list_line(line))
    {
        let next = cursor + 1;
        if lines.get(next).is_some_and(|line| !line.trim().is_empty()) {
            apply_attribute_list_to_metadata(&mut prelude.metadata, &attr_line);
            cursor += 1;
            continue;
        }
        break;
    }

    prelude.consumed_lines = cursor - index;
    (prelude.consumed_lines > 0).then_some(prelude)
}

fn strip_callout_marker(line: &str, auto_counter: &mut u32) -> (String, Option<u32>) {
    let trimmed = line.trim_end();

    // XML/HTML form: <!--N--> or <!--.--> at end of line
    if let Some(rest) = trimmed.strip_suffix("-->") {
        if let Some(start) = rest.rfind("<!--") {
            let num_str = &rest[start + 4..];
            if num_str == "." {
                *auto_counter += 1;
                let content = rest[..start].trim_end().to_owned();
                return (content, Some(*auto_counter));
            }
            if let Ok(n) = num_str.parse::<u32>() {
                let content = rest[..start].trim_end().to_owned();
                return (content, Some(n));
            }
        }
    }

    // Standard form: <N> or <.> at end of line
    if let Some(rest) = trimmed.strip_suffix('>') {
        if let Some(start) = rest.rfind('<') {
            let num_str = &rest[start + 1..];
            if num_str == "." {
                *auto_counter += 1;
                let content = rest[..start].trim_end().to_owned();
                return (content, Some(*auto_counter));
            }
            if let Ok(n) = num_str.parse::<u32>() {
                let content = rest[..start].trim_end().to_owned();
                return (content, Some(n));
            }
        }
    }

    (line.to_owned(), None)
}

fn parse_callout_list(lines: &[&str], index: usize) -> Option<(Block, usize)> {
    // A callout list is one or more consecutive lines matching `<N>` or `<.>` + description
    let mut items = Vec::new();
    let mut i = index;
    let mut auto_counter: u32 = 0;
    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix('<') {
            if let Some(gt) = rest.find('>') {
                let num_str = &rest[..gt];
                let n = if num_str == "." {
                    auto_counter += 1;
                    Some(auto_counter)
                } else if let Ok(n) = num_str.parse::<u32>() {
                    Some(n)
                } else {
                    None
                };
                if let Some(n) = n {
                    let content = rest[gt + 1..].trim();
                    items.push(CalloutItem {
                        number: n,
                        inlines: parse_inlines(content),
                    });
                    i += 1;
                    continue;
                }
            }
        }
        break;
    }
    if items.is_empty() {
        return None;
    }
    let consumed = i - index;
    Some((
        Block::CalloutList(CalloutList {
            items,
            metadata: BlockMetadata::default(),
        }),
        consumed,
    ))
}

fn is_delimited_block_delimiter(line: &str) -> bool {
    parse_delimited_block_marker(line).is_some()
}

fn parse_delimited_block_marker(line: &str) -> Option<(&str, &'static str)> {
    let trimmed = line.trim();
    if trimmed == "--" {
        return Some((trimmed, "open"));
    }

    if parse_fenced_code_opening(line).is_some() {
        return Some(("```", "listing"));
    }

    let bytes = trimmed.as_bytes();
    if bytes.len() < 4 {
        return None;
    }

    let first = *bytes.first()?;
    if !bytes.iter().all(|byte| *byte == first) {
        return None;
    }

    let kind = match first {
        b'-' => "listing",
        b'=' => "example",
        b'*' => "sidebar",
        b'+' => "passthrough",
        b'_' => "quote",
        b'.' => "literal",
        b'/' => "comment",
        _ => return None,
    };

    Some((trimmed, kind))
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

fn apply_fenced_code_metadata(metadata: &mut BlockMetadata, entries: &[String]) {
    metadata.style = Some("source".into());
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

fn parse_block_title(line: &str) -> Option<String> {
    if parse_list_marker(line).is_some() {
        return None;
    }
    if is_delimited_block_delimiter(line) {
        return None;
    }
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
    let base_slot = next_attribute_slot(metadata);
    for (index, entry) in entries.iter().enumerate() {
        let slot = base_slot + index;
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
                metadata.roles = value.split_whitespace().map(str::to_owned).collect();
                if !metadata.roles.is_empty() {
                    metadata.role = Some(metadata.roles.join(" "));
                }
            }
            continue;
        }

        if let Some(id) = entry.strip_prefix('#') {
            if !id.is_empty() {
                metadata.id = Some(id.to_owned());
                metadata
                    .attributes
                    .insert(format!("${slot}"), entry.clone());
                metadata.attributes.insert("id".into(), id.to_owned());
            }
            continue;
        }

        if metadata.style.is_none()
            && let Some((style, id)) = parse_style_id_shorthand(entry)
        {
            metadata
                .attributes
                .insert(format!("${slot}"), entry.clone());
            metadata.style = Some(style.to_owned());
            metadata
                .attributes
                .entry("style".into())
                .or_insert_with(|| style.to_owned());
            metadata.id = Some(id.to_owned());
            metadata.attributes.insert("id".into(), id.to_owned());
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
                metadata
                    .attributes
                    .insert(format!("${slot}"), entry.clone());
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
                metadata
                    .attributes
                    .insert(format!("${slot}"), entry.clone());
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

        metadata
            .attributes
            .insert(format!("${slot}"), entry.clone());
        if metadata.style.is_none() {
            metadata.style = Some(entry.clone());
            metadata
                .attributes
                .entry("style".into())
                .or_insert_with(|| entry.clone());
        } else if metadata.style.as_deref() == Some("source")
            && !metadata.attributes.contains_key("language")
        {
            metadata.attributes.insert("language".into(), entry.clone());
        }
    }

    normalize_source_listing_metadata(metadata);
}

fn next_attribute_slot(metadata: &BlockMetadata) -> usize {
    metadata
        .attributes
        .keys()
        .filter_map(|key| key.strip_prefix('$')?.parse::<usize>().ok())
        .max()
        .unwrap_or(0)
        + 1
}

fn parse_style_id_shorthand(entry: &str) -> Option<(&str, &str)> {
    let (style, id) = entry.split_once('#')?;
    if style.is_empty() || id.is_empty() || !is_valid_anchor_id(id) {
        return None;
    }

    Some((style, id))
}

fn normalize_source_listing_metadata(metadata: &mut BlockMetadata) {
    if metadata.style.as_deref() != Some("source") {
        return;
    }

    if metadata.attributes.contains_key("$3")
        && !metadata.options.iter().any(|option| option == "linenums")
    {
        metadata.options.push("linenums".into());
    }

    let mut normalized_options = Vec::new();
    for option in &metadata.options {
        let option = if option == "numbered" {
            "linenums"
        } else {
            option.as_str()
        };
        if !normalized_options.iter().any(|existing| existing == option) {
            normalized_options.push(option.to_owned());
        }
    }
    metadata.options = normalized_options;

    if metadata.options.iter().any(|option| option == "linenums") {
        metadata.attributes.remove("numbered-option");
        metadata
            .attributes
            .entry("linenums-option".into())
            .or_default();
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

fn parse_description_list(
    lines: &[&str],
    mut index: usize,
) -> Option<(crate::ast::DescriptionList, usize)> {
    let mut items = Vec::new();
    let start_index = index;

    while index < lines.len() {
        let line = lines[index];
        if line.trim().is_empty() {
            break;
        }

        let mut terms = Vec::new();
        let mut current_desc = String::new();
        let mut term_consumed = 0;

        while index + term_consumed < lines.len() {
            let t_line = lines[index + term_consumed];
            if t_line.trim().is_empty() {
                break;
            }
            if let Some(pos) = t_line.find("::") {
                let remainder = t_line[pos + 2..].trim();
                let is_valid_marker = t_line[pos + 2..].starts_with(' ')
                    || t_line[pos + 2..].starts_with('\t')
                    || t_line[pos + 2..].is_empty();

                if is_valid_marker {
                    let term_text = t_line[..pos].trim().to_string();
                    terms.push(crate::ast::DescriptionListTerm {
                        text: term_text.clone(),
                        inlines: crate::inline::parse_inlines(&term_text),
                    });

                    if !remainder.is_empty() {
                        current_desc.push_str(remainder);
                        term_consumed += 1;
                        break;
                    }
                    term_consumed += 1;
                    continue;
                }
            }
            break;
        }

        if terms.is_empty() {
            break;
        }
        index += term_consumed;

        while index < lines.len() && !lines[index].trim().is_empty() && !lines[index].contains("::")
        {
            if !current_desc.is_empty() {
                current_desc.push('\n');
            }
            current_desc.push_str(lines[index].trim());
            index += 1;
        }

        let description = if current_desc.is_empty() {
            None
        } else {
            Some(crate::ast::ListItem {
                blocks: vec![crate::ast::Block::Paragraph(crate::ast::Paragraph {
                    lines: current_desc.lines().map(|s| s.to_string()).collect(),
                    inlines: crate::inline::parse_inlines(&current_desc),
                    id: None,
                    reftext: None,
                    metadata: crate::ast::BlockMetadata::default(),
                })],
            })
        };

        items.push(crate::ast::DescriptionListItem { terms, description });
    }

    if items.is_empty() {
        None
    } else {
        Some((
            crate::ast::DescriptionList {
                items,
                reftext: None,
                metadata: crate::ast::BlockMetadata::default(),
            },
            index - start_index,
        ))
    }
}

fn parse_unordered_list(lines: &[&str], index: usize) -> Option<(UnorderedList, usize)> {
    parse_list(lines, index, ListKind::Unordered, 1).map(|(items, consumed)| {
        (
            UnorderedList {
                items,
                reftext: None,
                metadata: BlockMetadata::default(),
            },
            consumed,
        )
    })
}

fn parse_ordered_list(lines: &[&str], index: usize) -> Option<(OrderedList, usize)> {
    parse_list(lines, index, ListKind::Ordered, 1).map(|(items, consumed)| {
        (
            OrderedList {
                items,
                reftext: None,
                metadata: BlockMetadata::default(),
            },
            consumed,
        )
    })
}

fn parse_list(
    lines: &[&str],
    index: usize,
    kind: ListKind,
    level: usize,
) -> Option<(Vec<ListItem>, usize)> {
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

    let mut blocks = vec![make_paragraph_like_block(vec![marker.content.to_owned()])];
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

    let continuation_prelude = try_parse_block_prelude(lines, start);
    let continuation_start = start
        + continuation_prelude
            .as_ref()
            .map_or(0, |prelude| prelude.consumed_lines);

    if let Some(prelude) = continuation_prelude.as_ref() {
        if let Some((block, consumed)) =
            parse_delimited_block(lines, continuation_start, Some(prelude))
        {
            return Some((block, blank_lines + prelude.consumed_lines + consumed));
        }
    } else if let Some((block, consumed)) = parse_delimited_block(lines, start, None) {
        return Some((block, blank_lines + consumed));
    }

    let mut paragraph_lines = Vec::new();
    let mut consumed = blank_lines
        + continuation_prelude
            .as_ref()
            .map_or(0, |prelude| prelude.consumed_lines);
    let mut cursor = continuation_start;

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
        Some((
            make_block_from_paragraph(
                paragraph_lines,
                None,
                None,
                continuation_prelude
                    .map(|prelude| prelude.metadata)
                    .unwrap_or_default(),
            ),
            consumed,
        ))
    }
}

fn parse_list_block(
    lines: &[&str],
    index: usize,
    kind: ListKind,
    level: usize,
) -> Option<(Block, usize)> {
    let (items, consumed) = parse_list(lines, index, kind, level)?;
    let block = match kind {
        ListKind::Unordered => Block::UnorderedList(UnorderedList {
            items,
            reftext: None,
            metadata: BlockMetadata::default(),
        }),
        ListKind::Ordered => Block::OrderedList(OrderedList {
            items,
            reftext: None,
            metadata: BlockMetadata::default(),
        }),
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
    lines
        .iter()
        .take_while(|line| line.trim().is_empty())
        .count()
}

fn make_paragraph_like_block(lines: Vec<String>) -> Block {
    if let Some((variant, first_line)) = lines
        .first()
        .and_then(|line| parse_admonition_prefix(line.as_str()))
        .map(|(variant, first_line)| (variant, first_line.to_owned()))
    {
        let mut paragraph_lines = lines;
        paragraph_lines[0] = first_line;
        return Block::Admonition(AdmonitionBlock {
            variant,
            blocks: vec![Block::Paragraph(make_paragraph(paragraph_lines))],
            id: None,
            reftext: None,
            metadata: BlockMetadata::default(),
        });
    }

    make_block_from_paragraph(lines, None, None, BlockMetadata::default())
}

fn make_block_from_paragraph(
    lines: Vec<String>,
    id: Option<String>,
    reftext: Option<String>,
    mut metadata: BlockMetadata,
) -> Block {
    let explicit_normal = metadata.style.as_deref() == Some("normal");
    if explicit_normal {
        clear_resolved_style(&mut metadata);
    }

    if let Some(variant) = metadata
        .style
        .as_deref()
        .and_then(admonition_variant_from_style)
    {
        if metadata.id.is_none() {
            metadata.id = id;
        }
        return Block::Admonition(AdmonitionBlock {
            variant,
            blocks: vec![Block::Paragraph(Paragraph {
                inlines: parse_inlines(&lines.join("\n")),
                lines,
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
            })],
            id: metadata.id.clone(),
            reftext,
            metadata,
        });
    }

    if metadata.style.as_deref() == Some("literal") {
        if metadata.id.is_none() {
            metadata.id = id;
        }
        clear_resolved_style(&mut metadata);
        return Block::Literal(Listing {
            lines,
            callouts: vec![],
            reftext,
            metadata,
        });
    }

    if matches!(metadata.style.as_deref(), Some("sidebar" | "example" | "open" | "abstract" | "partintro" | "comment"))
    {
        if metadata.id.is_none() {
            metadata.id = id;
        }
        let style = metadata.style.clone();
        let block = if style.as_deref() == Some("example") {
            clear_resolved_style(&mut metadata);
            Block::Example(CompoundBlock {
                blocks: vec![Block::Paragraph(Paragraph {
                    inlines: parse_inlines(&lines.join("\n")),
                    lines,
                    id: None,
                    reftext: None,
                    metadata: BlockMetadata::default(),
                })],
                reftext,
                context: None,
                metadata,
            })
        } else if style.as_deref() == Some("sidebar") {
            clear_resolved_style(&mut metadata);
            Block::Sidebar(CompoundBlock {
                blocks: vec![Block::Paragraph(Paragraph {
                    inlines: parse_inlines(&lines.join("\n")),
                    lines,
                    id: None,
                    reftext: None,
                    metadata: BlockMetadata::default(),
                })],
                reftext,
                context: None,
                metadata,
            })
        } else {
            let context = match style.as_deref() {
                Some("abstract") => Some(OpenBlockContext::Abstract),
                Some("comment") => Some(OpenBlockContext::Comment),
                Some("partintro") => Some(OpenBlockContext::PartIntro),
                _ => None,
            };
            clear_resolved_style(&mut metadata);
            Block::Open(CompoundBlock {
                blocks: vec![Block::Paragraph(Paragraph {
                    inlines: parse_inlines(&lines.join("\n")),
                    lines,
                    id: None,
                    reftext: None,
                    metadata: BlockMetadata::default(),
                })],
                reftext,
                context,
                metadata,
            })
        };
        return block;
    }

    if matches!(metadata.style.as_deref(), Some("listing" | "source")) {
        if metadata.id.is_none() {
            metadata.id = id;
        }
        if metadata.style.as_deref() == Some("listing") {
            clear_resolved_style(&mut metadata);
        }
        return Block::Listing(make_listing_from_lines(lines, reftext, metadata));
    }

    if metadata.style.as_deref() == Some("quote") {
        if metadata.id.is_none() {
            metadata.id = id;
        }
        clear_resolved_style(&mut metadata);
        return Block::Quote(QuoteBlock {
            blocks: vec![Block::Paragraph(Paragraph {
                inlines: parse_inlines(&lines.join("\n")),
                lines,
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
            })],
            content: None,
            attribution: metadata.attributes.get("$2").cloned(),
            citetitle: metadata.attributes.get("$3").cloned(),
            is_verse: false,
            reftext,
            metadata,
        });
    }

    if metadata.style.as_deref() == Some("verse") {
        if metadata.id.is_none() {
            metadata.id = id;
        }
        clear_resolved_style(&mut metadata);
        return Block::Quote(QuoteBlock {
            blocks: vec![],
            content: Some(lines.join("\n")),
            attribution: metadata.attributes.get("$2").cloned(),
            citetitle: metadata.attributes.get("$3").cloned(),
            is_verse: true,
            reftext,
            metadata,
        });
    }

    if metadata.style.as_deref() == Some("pass") {
        clear_resolved_style(&mut metadata);
        return Block::Passthrough(crate::ast::PassthroughBlock {
            content: lines.join("\n"),
            reftext,
            metadata,
        });
    }

    // Indented paragraph (leading space/tab) → literal block
    if !explicit_normal
        && lines
        .first()
        .is_some_and(|l| l.starts_with(' ') || l.starts_with('\t'))
    {
        return Block::Literal(Listing {
            lines,
            callouts: vec![],
            reftext,
            metadata,
        });
    }

    Block::Paragraph(Paragraph {
        inlines: parse_inlines(&lines.join("\n")),
        lines,
        id,
        reftext,
        metadata,
    })
}

fn make_listing_from_lines(
    lines: Vec<String>,
    reftext: Option<String>,
    metadata: BlockMetadata,
) -> Listing {
    let mut stripped_lines = Vec::with_capacity(lines.len());
    let mut callouts = Vec::new();
    let mut auto_counter: u32 = 0;

    for (idx, line) in lines.into_iter().enumerate() {
        let (content, marker) = strip_callout_marker(&line, &mut auto_counter);
        if let Some(number) = marker {
            callouts.push((idx, number));
        }
        stripped_lines.push(content);
    }

    Listing {
        lines: stripped_lines,
        callouts,
        reftext,
        metadata,
    }
}

fn make_paragraph(lines: Vec<String>) -> Paragraph {
    Paragraph {
        inlines: parse_inlines(&lines.join("\n")),
        lines,
        id: None,
        reftext: None,
        metadata: BlockMetadata::default(),
    }
}

fn append_to_last_paragraph(blocks: &mut Vec<Block>, line: String) {
    if let Some(Block::Paragraph(paragraph)) = blocks.last_mut() {
        paragraph.lines.push(line);
        paragraph.inlines = parse_inlines(&paragraph.lines.join("\n"));
        return;
    }

    if let Some(Block::Admonition(admonition)) = blocks.last_mut()
        && let Some(Block::Paragraph(paragraph)) = admonition.blocks.last_mut()
    {
        paragraph.lines.push(line);
        paragraph.inlines = parse_inlines(&paragraph.lines.join("\n"));
        return;
    }

    blocks.push(make_paragraph_like_block(vec![line]));
}

fn flush_paragraph(
    blocks: &mut Vec<Block>,
    current_paragraph: &mut Vec<String>,
    current_paragraph_anchor: &mut Option<PendingAnchor>,
    current_paragraph_prelude: &mut Option<BlockPrelude>,
) {
    if current_paragraph.is_empty() {
        return;
    }

    let lines = std::mem::take(current_paragraph);
    let anchor = current_paragraph_anchor.take();
    let prelude = current_paragraph_prelude.take();
    let metadata = prelude.map(|prelude| prelude.metadata).unwrap_or_default();
    let id = anchor
        .as_ref()
        .map(|anchor| anchor.id.clone())
        .or_else(|| metadata.id.clone());
    blocks.push(make_block_from_paragraph(
        lines,
        id,
        anchor.and_then(|anchor| anchor.reftext),
        metadata,
    ));
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
        metadata: BlockMetadata::default(),
    })
}

fn parse_setext_heading(lines: &[&str], index: usize) -> Option<(Heading, usize)> {
    let title = lines.get(index)?.trim();
    let underline = lines.get(index + 1)?.trim();

    if title.is_empty()
        || !title.chars().any(char::is_alphanumeric)
        || parse_attribute_list_line(title).is_some()
    {
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
            metadata: BlockMetadata::default(),
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

    Some((
        name.to_owned(),
        stripped[separator + 1..].trim_start().to_owned(),
    ))
}

fn parse_attribute_entry_at(lines: &[&str], index: usize) -> Option<(String, String, usize)> {
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
        return Some((name.to_owned(), value, consumed_lines));
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

fn is_comment_line(line: &str) -> bool {
    line.trim_start().starts_with("//") && parse_delimited_block_marker(line).is_none()
}

fn apply_anchor_to_heading(mut heading: Heading, anchor: Option<PendingAnchor>) -> Heading {
    if let Some(anchor) = anchor {
        heading.id = Some(anchor.id);
        heading.reftext = anchor.reftext;
    }
    heading
}

fn parse_admonition_prefix(line: &str) -> Option<(AdmonitionVariant, &str)> {
    let trimmed = line.trim_start();
    for (prefix, variant) in [
        ("NOTE:", AdmonitionVariant::Note),
        ("TIP:", AdmonitionVariant::Tip),
        ("IMPORTANT:", AdmonitionVariant::Important),
        ("CAUTION:", AdmonitionVariant::Caution),
        ("WARNING:", AdmonitionVariant::Warning),
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
        return Some((variant, content));
    }
    None
}

fn admonition_variant_from_style(style: &str) -> Option<AdmonitionVariant> {
    if style.eq_ignore_ascii_case("NOTE") {
        Some(AdmonitionVariant::Note)
    } else if style.eq_ignore_ascii_case("TIP") {
        Some(AdmonitionVariant::Tip)
    } else if style.eq_ignore_ascii_case("IMPORTANT") {
        Some(AdmonitionVariant::Important)
    } else if style.eq_ignore_ascii_case("CAUTION") {
        Some(AdmonitionVariant::Caution)
    } else if style.eq_ignore_ascii_case("WARNING") {
        Some(AdmonitionVariant::Warning)
    } else {
        None
    }
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
        AdmonitionBlock, AdmonitionVariant, Block, BlockMetadata, CompoundBlock, Heading, Inline,
        InlineForm, InlineSpan, InlineVariant, ListItem, Listing, OpenBlockContext,
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
                    metadata: BlockMetadata::default(),
                }),
                Block::Paragraph(Paragraph {
                    inlines: vec![Inline::Text("third line".into())],
                    lines: vec!["third line".into()],
                    id: None,
                    reftext: None,
                    metadata: BlockMetadata::default(),
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
                metadata: BlockMetadata::default(),
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
                    metadata: BlockMetadata::default(),
                }),
                Block::Paragraph(Paragraph {
                    inlines: vec![Inline::Text("content".into())],
                    lines: vec!["content".into()],
                    id: None,
                    reftext: None,
                    metadata: BlockMetadata::default(),
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
                metadata: BlockMetadata::default(),
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
                metadata: BlockMetadata::default(),
            })
        );
        assert_eq!(
            document.blocks,
            vec![Block::Heading(Heading {
                level: 1,
                title: "Section A".into(),
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
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
                metadata: BlockMetadata::default(),
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
                metadata: BlockMetadata::default(),
            })]
        );
    }

    #[test]
    fn parses_multiline_document_header_attribute_with_soft_wraps() {
        let document = parse_document(
            "= Document Title\n:description: If you have a very long line of text \\\nthat you need to substitute regularly in a document, \\\n  you may find it easier to split the value neatly.\n\ncontent",
        );

        assert_eq!(
            document.attributes.get("description").map(String::as_str),
            Some(
                "If you have a very long line of text that you need to substitute regularly in a document, you may find it easier to split the value neatly."
            )
        );
    }

    #[test]
    fn parses_multiline_document_header_attribute_with_hard_wraps() {
        let document = parse_document(
            "= Document Title\n:haiku: Write your docs in text, + \\\n  AsciiDoc makes it easy, + \\\n  Now get back to work!\n\ncontent",
        );

        assert_eq!(
            document.attributes.get("haiku").map(String::as_str),
            Some("Write your docs in text, +\nAsciiDoc makes it easy, +\nNow get back to work!")
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
        let document = parse_document(
            "= Document Title\n:authorinitials: DOC\n:author: Doc Writer\n\ncontent",
        );

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
        let document = parse_document("= Document Title\n:author: Doc Middle Writer\n\ncontent");

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
        let document =
            parse_document("= Document Title\nStuart Rackham <founder@asciidoc.org>\n\ncontent");

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
        let document = parse_document("= Document Title\nAuthor Name\nv1.0.0,:remark\n\ncontent");

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
                    metadata: BlockMetadata::default(),
                }),
                Block::Paragraph(Paragraph {
                    inlines: vec![Inline::Text("content".into())],
                    lines: vec!["content".into()],
                    id: None,
                    reftext: None,
                    metadata: BlockMetadata::default(),
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
        let document =
            parse_document("// comment one\n// comment two\n= Document Title\n\ncontent");

        assert_eq!(
            document.title,
            Some(Heading {
                level: 0,
                title: "Document Title".into(),
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
            })
        );
        assert_eq!(
            document.blocks,
            vec![Block::Paragraph(Paragraph {
                inlines: vec![Inline::Text("content".into())],
                lines: vec!["content".into()],
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
            })]
        );
    }

    #[test]
    fn ignores_header_comments_between_title_and_attributes() {
        let document =
            parse_document("= Document Title\n// comment\n:toc: left\n// another\n\ncontent");

        assert_eq!(
            document.attributes,
            [("toc".to_owned(), "left".to_owned())]
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
                metadata: BlockMetadata::default(),
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
                metadata: BlockMetadata::default(),
            })]
        );
    }

    #[test]
    fn stops_parsing_header_attributes_at_first_non_attribute_line() {
        let document = parse_document("= Document Title\n:toc: left\nintro text\n:ignored: value");

        assert_eq!(
            document.attributes,
            [("toc".to_owned(), "left".to_owned())]
                .into_iter()
                .collect()
        );
        assert_eq!(
            document.blocks,
            vec![Block::Paragraph(Paragraph {
                inlines: vec![Inline::Text("intro text\n:ignored: value".into())],
                lines: vec!["intro text".into(), ":ignored: value".into()],
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
            })]
        );
    }

    #[test]
    fn stops_parsing_header_attributes_after_blank_line() {
        let document = parse_document("= Document Title\n:toc: left\n\n:body-attr: value");

        assert_eq!(
            document.attributes,
            [
                ("body-attr".to_owned(), "value".to_owned()),
                ("toc".to_owned(), "left".to_owned())
            ]
            .into_iter()
            .collect()
        );
        assert!(document.blocks.is_empty());
    }

    #[test]
    fn parses_top_level_attributes_without_document_title() {
        let document = parse_document(":icons:\n:iconsdir: /site/icons\n\nTIP: Ship it carefully.");

        assert_eq!(
            document.attributes,
            [
                ("icons".to_owned(), String::new()),
                ("iconsdir".to_owned(), "/site/icons".to_owned()),
            ]
            .into_iter()
            .collect()
        );
        assert_eq!(document.title, None);
        assert_eq!(document.blocks.len(), 1);
    }

    #[test]
    fn parses_body_attributes_before_later_blocks_without_rendering_them() {
        let document = parse_document(
            "= Demo\n\nIntro paragraph.\n\n:icons:\n:iconsdir: /site/icons\n\n[TIP,icon=hint,icontype=svg,caption=\"Custom Tip\"]\nShip it carefully.",
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
    fn parses_multiline_body_attribute_before_later_blocks() {
        let document = parse_document(
            "= Demo\n\nIntro paragraph.\n\n:description: first segment \\\n  second segment\n\nTail paragraph.",
        );

        assert_eq!(
            document.attributes.get("description").map(String::as_str),
            Some("first segment second segment")
        );
        assert_eq!(document.blocks.len(), 2);
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
                metadata: BlockMetadata::default(),
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
                metadata: BlockMetadata::default()
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
                metadata: BlockMetadata::default()
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
                            metadata: BlockMetadata::default(),
                        })],
                    },
                    ListItem {
                        blocks: vec![Block::Paragraph(Paragraph {
                            inlines: vec![Inline::Text("second item".into())],
                            lines: vec!["second item".into()],
                            id: None,
                            reftext: None,
                            metadata: BlockMetadata::default(),
                        })],
                    },
                ],
                reftext: None,
                metadata: BlockMetadata::default(),
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
                            metadata: BlockMetadata::default(),
                        })],
                    },
                    ListItem {
                        blocks: vec![Block::Paragraph(Paragraph {
                            inlines: vec![Inline::Text("second item".into())],
                            lines: vec!["second item".into()],
                            id: None,
                            reftext: None,
                            metadata: BlockMetadata::default(),
                        })],
                    },
                ],
                reftext: None,
                metadata: BlockMetadata::default(),
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
                metadata: BlockMetadata::default(),
            })
        );
        assert_eq!(
            document.blocks,
            vec![Block::Paragraph(Paragraph {
                inlines: vec![Inline::Text("A paragraph.".into())],
                lines: vec!["A paragraph.".into()],
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
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
                metadata: BlockMetadata::default(),
            })
        );
        assert_eq!(
            document.blocks,
            vec![Block::Heading(Heading {
                level: 0,
                title: "Second Title".into(),
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
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
                            metadata: BlockMetadata::default(),
                        })],
                    },
                    ListItem {
                        blocks: vec![Block::Paragraph(Paragraph {
                            inlines: vec![Inline::Text("second item".into())],
                            lines: vec!["second item".into()],
                            id: None,
                            reftext: None,
                            metadata: BlockMetadata::default(),
                        })],
                    },
                ],
                reftext: None,
                metadata: BlockMetadata::default(),
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
                                metadata: BlockMetadata::default(),
                            }),
                            Block::UnorderedList(UnorderedList {
                                items: vec![ListItem {
                                    blocks: vec![Block::Paragraph(Paragraph {
                                        inlines: vec![Inline::Text("child".into())],
                                        lines: vec!["child".into()],
                                        id: None,
                                        reftext: None,
                                        metadata: BlockMetadata::default(),
                                    })],
                                }],
                                reftext: None,
                                metadata: BlockMetadata::default(),
                            }),
                        ],
                    },
                    ListItem {
                        blocks: vec![Block::Paragraph(Paragraph {
                            inlines: vec![Inline::Text("sibling".into())],
                            lines: vec!["sibling".into()],
                            id: None,
                            reftext: None,
                            metadata: BlockMetadata::default(),
                        })],
                    },
                ],
                reftext: None,
                metadata: BlockMetadata::default(),
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
                                metadata: BlockMetadata::default(),
                            }),
                            Block::Paragraph(Paragraph {
                                inlines: vec![Inline::Text("continued paragraph".into())],
                                lines: vec!["continued paragraph".into()],
                                id: None,
                                reftext: None,
                                metadata: BlockMetadata::default(),
                            }),
                        ],
                    },
                    ListItem {
                        blocks: vec![Block::Paragraph(Paragraph {
                            inlines: vec![Inline::Text("second item".into())],
                            lines: vec!["second item".into()],
                            id: None,
                            reftext: None,
                            metadata: BlockMetadata::default(),
                        })],
                    },
                ],
                reftext: None,
                metadata: BlockMetadata::default(),
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
                callouts: vec![],
                reftext: None,
                metadata: BlockMetadata::default(),
            })]
        );
    }

    #[test]
    fn parses_delimited_listing_blocks_with_longer_delimiters() {
        let document = parse_document("------\ndef main\n  puts 'hello'\nend\n------");

        let [Block::Listing(listing)] = document.blocks.as_slice() else {
            panic!("expected listing");
        };
        assert_eq!(listing.lines, vec!["def main", "  puts 'hello'", "end"]);
    }

    #[test]
    fn parses_fenced_code_blocks_as_source_listings() {
        let document = parse_document("```rust,linenums\nfn main() {}\n```");

        let [Block::Listing(listing)] = document.blocks.as_slice() else {
            panic!("expected listing");
        };
        assert_eq!(listing.lines, vec!["fn main() {}"]);
        assert_eq!(listing.metadata.style.as_deref(), Some("source"));
        assert_eq!(
            listing
                .metadata
                .attributes
                .get("language")
                .map(String::as_str),
            Some("rust")
        );
        assert!(
            listing
                .metadata
                .options
                .iter()
                .any(|option| option == "linenums")
        );
        assert_eq!(
            listing
                .metadata
                .attributes
                .get("cloaked-context")
                .map(String::as_str),
            Some("fenced_code")
        );
    }

    #[test]
    fn does_not_recognize_fenced_code_blocks_with_more_than_three_backticks() {
        let document = parse_document("````rust\nfn main() {}\n````");

        let [Block::Paragraph(paragraph)] = document.blocks.as_slice() else {
            panic!("expected paragraph");
        };
        assert_eq!(paragraph.plain_text(), "````rust\nfn main() {}\n````");
    }

    #[test]
    fn parses_tables_with_header_option() {
        let document = parse_document(
            ".Agents\n[%header,cols=\"30%,\"]\n|===\n|Name|Email\n|Peter|peter@example.com\n|Adam|adam@example.com\n|===",
        );

        let [Block::Table(table)] = document.blocks.as_slice() else {
            panic!("expected table");
        };
        assert_eq!(table.metadata.title.as_deref(), Some("Agents"));
        assert_eq!(table.metadata.options, vec!["header"]);
        assert_eq!(table.header.as_ref().map(|row| row.cells.len()), Some(2));
        assert_eq!(
            table
                .header
                .as_ref()
                .map(|row| row.cells[0].content.as_str()),
            Some("Name")
        );
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[0].cells[1].content, "peter@example.com");
    }

    #[test]
    fn parses_tables_with_bang_delimiters() {
        let document = parse_document("!===\n!Name!Email\n!Peter!peter@example.com\n!===");

        let [Block::Table(table)] = document.blocks.as_slice() else {
            panic!("expected table");
        };
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[0].cells[0].content, "Name");
        assert_eq!(table.rows[1].cells[1].content, "peter@example.com");
    }

    #[test]
    fn parses_tables_with_custom_ascii_doc_separator() {
        let document = parse_document(
            "[cols=\"1,1\",separator=!]\n|===\n!Pipe output to vim\na!\n----\nasciidoctor -o - -s test.adoc | view -\n----\n|===",
        );

        let [Block::Table(table)] = document.blocks.as_slice() else {
            panic!("expected table");
        };
        assert_eq!(table.rows.len(), 1);
        assert_eq!(table.rows[0].cells[0].content, "Pipe output to vim");
        assert_eq!(table.rows[0].cells[1].style.as_deref(), Some("asciidoc"));
        let Block::Listing(listing) = &table.rows[0].cells[1].blocks[0] else {
            panic!("expected listing in AsciiDoc cell");
        };
        assert_eq!(listing.lines[0], "asciidoctor -o - -s test.adoc | view -");
    }

    #[test]
    fn parses_csv_tables_with_shorthand_delimiter() {
        let document =
            parse_document(",===\nArtist,Track,Genre\nBaauer,Harlem Shake,Hip Hop\n,===");

        let [Block::Table(table)] = document.blocks.as_slice() else {
            panic!("expected table");
        };
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[0].cells.len(), 3);
        assert_eq!(table.rows[1].cells[1].content, "Harlem Shake");
    }

    #[test]
    fn parses_dsv_tables_with_shorthand_delimiter() {
        let document = parse_document(":===\nArtist:Track:Genre\nRobyn:Indestructible:Dance\n:===");

        let [Block::Table(table)] = document.blocks.as_slice() else {
            panic!("expected table");
        };
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[0].cells.len(), 3);
        assert_eq!(table.rows[1].cells[2].content, "Dance");
    }

    #[test]
    fn parses_tables_with_cells_stacked_across_lines() {
        let document = parse_document(
            ".Agents\n[%header,cols=\"30%,70%\"]\n|===\n|Name\n|Email\n|Peter\n|peter@example.com\n|Adam\n|adam@example.com\n|===",
        );

        let [Block::Table(table)] = document.blocks.as_slice() else {
            panic!("expected table");
        };
        assert_eq!(table.header.as_ref().map(|row| row.cells.len()), Some(2));
        assert_eq!(
            table
                .header
                .as_ref()
                .map(|row| row.cells[0].content.as_str()),
            Some("Name")
        );
        assert_eq!(
            table
                .header
                .as_ref()
                .map(|row| row.cells[1].content.as_str()),
            Some("Email")
        );
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[0].cells[0].content, "Peter");
        assert_eq!(table.rows[0].cells[1].content, "peter@example.com");
    }

    #[test]
    fn parses_tables_with_stacked_cells_without_cols_when_rows_are_separated() {
        let document = parse_document(
            ".Agents\n[%header]\n|===\n|Name\n|Email\n\n|Peter\n|peter@example.com\n\n|Adam\n|adam@example.com\n|===",
        );

        let [Block::Table(table)] = document.blocks.as_slice() else {
            panic!("expected table");
        };
        assert_eq!(table.header.as_ref().map(|row| row.cells.len()), Some(2));
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[0].cells[0].content, "Peter");
        assert_eq!(table.rows[0].cells[1].content, "peter@example.com");
        assert_eq!(table.rows[1].cells[0].content, "Adam");
        assert_eq!(table.rows[1].cells[1].content, "adam@example.com");
    }

    #[test]
    fn parses_block_content_inside_table_cells() {
        let document = parse_document(
            ".Services\n[%header,cols=\"1,3\"]\n|===\n|Name\n|Details\n|API\n|First paragraph.\n\n* fast\n* typed\n|===",
        );

        let [Block::Table(table)] = document.blocks.as_slice() else {
            panic!("expected table");
        };
        let detail_cell = &table.rows[0].cells[1];
        assert_eq!(detail_cell.blocks.len(), 2);
        let Block::Paragraph(paragraph) = &detail_cell.blocks[0] else {
            panic!("expected paragraph in table cell");
        };
        assert_eq!(paragraph.plain_text(), "First paragraph.");
        let Block::UnorderedList(list) = &detail_cell.blocks[1] else {
            panic!("expected unordered list in table cell");
        };
        assert_eq!(list.items.len(), 2);
    }

    #[test]
    fn parses_table_cell_specs_for_rowspan_and_asciidoc_style() {
        let document = parse_document(
            "[%header,cols=\"1,2\"]\n|===\nh|Area\n|Description\n\n.2+|Shared\na|First paragraph.\n+\nSecond paragraph.\n\n|Another description\n|===",
        );

        let [Block::Table(table)] = document.blocks.as_slice() else {
            panic!("expected table");
        };
        assert_eq!(
            table
                .header
                .as_ref()
                .map(|row| row.cells[0].style.as_deref()),
            Some(Some("header"))
        );
        assert_eq!(table.rows[0].cells[0].rowspan, 2);
        assert_eq!(table.rows[0].cells[0].content, "Shared");
        assert_eq!(table.rows[0].cells[1].style.as_deref(), Some("asciidoc"));
        assert_eq!(table.rows[0].cells[1].blocks.len(), 2);
        assert_eq!(table.rows[1].cells.len(), 1);
    }

    #[test]
    fn parses_table_cell_specs_on_later_cells_in_same_line() {
        let document = parse_document(
            "[%header,cols=\"1,2\"]\n|===\n|Name|h|Email\n|API|a|First paragraph.\n+\nSecond paragraph.\n|===",
        );

        let [Block::Table(table)] = document.blocks.as_slice() else {
            panic!("expected table");
        };
        assert_eq!(table.header.as_ref().map(|row| row.cells.len()), Some(2));
        assert_eq!(
            table
                .header
                .as_ref()
                .map(|row| row.cells[1].style.as_deref()),
            Some(Some("header"))
        );
        assert_eq!(table.rows[0].cells[1].style.as_deref(), Some("asciidoc"));
        assert_eq!(table.rows[0].cells[1].blocks.len(), 2);
    }

    #[test]
    fn parses_multiline_asciidoc_cell_after_later_cell_spec_on_new_row() {
        let document = parse_document(
            "[%header,cols=\"1,2\"]\n|===\nh|Area\n|Description\n\n|North|Plain cell\n|South|a|AsciiDoc cell with a list:\n\n* first\n* second\n|===",
        );

        let [Block::Table(table)] = document.blocks.as_slice() else {
            panic!("expected table");
        };
        assert_eq!(table.header.as_ref().map(|row| row.cells.len()), Some(2));
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[0].cells[0].content, "North");
        assert_eq!(table.rows[0].cells[1].content, "Plain cell");
        assert_eq!(table.rows[1].cells[0].content, "South");
        assert_eq!(table.rows[1].cells[1].style.as_deref(), Some("asciidoc"));
        assert_eq!(table.rows[1].cells[1].blocks.len(), 2);
        let Block::Paragraph(paragraph) = &table.rows[1].cells[1].blocks[0] else {
            panic!("expected paragraph");
        };
        assert_eq!(paragraph.plain_text(), "AsciiDoc cell with a list:");
        let Block::UnorderedList(list) = &table.rows[1].cells[1].blocks[1] else {
            panic!("expected unordered list");
        };
        assert_eq!(list.items.len(), 2);
    }

    #[test]
    fn parses_nested_tables_with_alternate_delimiters() {
        let document = parse_document(
            "[cols=\"1,2a\"]\n|===\n|Normal cell\n|Cell with nested table\n[cols=\"2,1\"]\n!===\n!Nested table cell 1 !Nested table cell 2\n!===\n|===",
        );

        let [Block::Table(table)] = document.blocks.as_slice() else {
            panic!("expected table");
        };
        let nested_table = table.rows[0].cells[1]
            .blocks
            .iter()
            .find_map(|block| match block {
                Block::Table(table) => Some(table),
                _ => None,
            })
            .expect("expected nested table");
        assert_eq!(nested_table.rows.len(), 1);
        assert_eq!(nested_table.rows[0].cells[0].content, "Nested table cell 1");
        assert_eq!(nested_table.rows[0].cells[1].content, "Nested table cell 2");
    }

    #[test]
    fn infers_table_grid_for_spans_without_cols() {
        let document =
            parse_document("|===\n|Name|Description\n\n.2+|Shared\n|First\n\n|Second\n|===");

        let [Block::Table(table)] = document.blocks.as_slice() else {
            panic!("expected table");
        };
        assert_eq!(table.rows.len(), 3);
        assert_eq!(table.rows[1].cells[0].rowspan, 2);
        assert_eq!(table.rows[1].cells[1].content, "First");
        assert_eq!(table.rows[2].cells.len(), 1);
        assert_eq!(table.rows[2].cells[0].content, "Second");
    }

    #[test]
    fn applies_anchor_to_tables() {
        let document = parse_document(
            "[[deploy-table,Deployment Table]]\n|===\n|Name|Email\n|Peter|peter@example.com\n|===",
        );

        let [Block::Table(table)] = document.blocks.as_slice() else {
            panic!("expected table");
        };
        assert_eq!(table.metadata.id.as_deref(), Some("deploy-table"));
        assert_eq!(table.reftext.as_deref(), Some("Deployment Table"));
    }

    #[test]
    fn applies_anchor_to_delimited_listing_blocks() {
        let document = parse_document("[[code-sample]]\n----\nputs 'hello'\n----");

        let [Block::Listing(listing)] = document.blocks.as_slice() else {
            panic!("expected listing");
        };
        assert_eq!(listing.metadata.id.as_deref(), Some("code-sample"));
        assert_eq!(listing.reftext, None);
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
                                metadata: BlockMetadata::default(),
                            })],
                        },
                        ListItem {
                            blocks: vec![Block::Paragraph(Paragraph {
                                inlines: vec![Inline::Text("two".into())],
                                lines: vec!["two".into()],
                                id: None,
                                reftext: None,
                                metadata: BlockMetadata::default(),
                            })],
                        },
                    ],
                    reftext: None,
                    metadata: BlockMetadata::default(),
                })],
                reftext: None,
                context: None,
                metadata: BlockMetadata::default(),
            })]
        );
    }

    #[test]
    fn applies_anchor_to_delimited_sidebar_blocks() {
        let document = parse_document("[[callouts]]\n****\ninside\n****");

        let [Block::Sidebar(sidebar)] = document.blocks.as_slice() else {
            panic!("expected sidebar");
        };
        assert_eq!(sidebar.metadata.id.as_deref(), Some("callouts"));
        assert_eq!(sidebar.reftext, None);
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
                    metadata: BlockMetadata::default(),
                })],
                reftext: None,
                context: None,
                metadata: BlockMetadata::default(),
            })]
        );
    }

    #[test]
    fn parses_nested_example_blocks_with_longer_child_delimiters() {
        let document = parse_document("====\n======\ninside\n======\n====");

        let [Block::Example(example)] = document.blocks.as_slice() else {
            panic!("expected example");
        };
        let [Block::Example(inner)] = example.blocks.as_slice() else {
            panic!("expected nested example");
        };
        let [Block::Paragraph(paragraph)] = inner.blocks.as_slice() else {
            panic!("expected nested paragraph");
        };
        assert_eq!(paragraph.plain_text(), "inside");
    }

    #[test]
    fn parses_nested_example_blocks_with_shorter_child_delimiters() {
        let document = parse_document("======\n====\ninside\n====\n======");

        let [Block::Example(example)] = document.blocks.as_slice() else {
            panic!("expected example");
        };
        let [Block::Example(inner)] = example.blocks.as_slice() else {
            panic!("expected nested example");
        };
        let [Block::Paragraph(paragraph)] = inner.blocks.as_slice() else {
            panic!("expected nested paragraph");
        };
        assert_eq!(paragraph.plain_text(), "inside");
    }

    #[test]
    fn does_not_close_delimited_block_with_mismatched_delimiter_length() {
        let document = parse_document("====\ninside\n======");

        let [Block::Paragraph(paragraph)] = document.blocks.as_slice() else {
            panic!("expected paragraph");
        };
        assert_eq!(paragraph.lines, vec!["====", "inside", "======"]);
    }

    #[test]
    fn ignores_comment_blocks_with_longer_delimiters() {
        let document = parse_document("//////\nignore me\n//////\n\nvisible");

        let [Block::Paragraph(paragraph)] = document.blocks.as_slice() else {
            panic!("expected paragraph");
        };
        assert_eq!(paragraph.plain_text(), "visible");
    }

    #[test]
    fn applies_anchor_to_delimited_example_blocks() {
        let document = parse_document("[[walkthrough]]\n====\nA paragraph.\n====");

        let [Block::Example(example)] = document.blocks.as_slice() else {
            panic!("expected example");
        };
        assert_eq!(example.metadata.id.as_deref(), Some("walkthrough"));
        assert_eq!(example.reftext, None);
    }

    #[test]
    fn parses_delimited_listing_block_title_and_attributes() {
        let document = parse_document(".Exhibit A\n[source,rust]\n----\nfn main() {}\n----");

        let [Block::Listing(listing)] = document.blocks.as_slice() else {
            panic!("expected listing");
        };
        assert_eq!(listing.metadata.title.as_deref(), Some("Exhibit A"));
        assert_eq!(listing.metadata.style.as_deref(), Some("source"));
        assert_eq!(
            listing.metadata.attributes.get("$1").map(String::as_str),
            Some("source")
        );
        assert_eq!(
            listing.metadata.attributes.get("$2").map(String::as_str),
            Some("rust")
        );
        assert_eq!(
            listing
                .metadata
                .attributes
                .get("language")
                .map(String::as_str),
            Some("rust")
        );
    }

    #[test]
    fn parses_delimited_listing_block_title_with_stacked_attribute_lists() {
        let document =
            parse_document(".Exhibit A\n[source]\n[rust,linenums]\n----\nfn main() {}\n----");

        let [Block::Listing(listing)] = document.blocks.as_slice() else {
            panic!("expected listing");
        };
        assert_eq!(listing.metadata.title.as_deref(), Some("Exhibit A"));
        assert_eq!(listing.metadata.style.as_deref(), Some("source"));
        assert_eq!(
            listing.metadata.attributes.get("$1").map(String::as_str),
            Some("source")
        );
        assert_eq!(
            listing.metadata.attributes.get("$2").map(String::as_str),
            Some("rust")
        );
        assert_eq!(
            listing.metadata.attributes.get("$3").map(String::as_str),
            Some("linenums")
        );
        assert_eq!(
            listing
                .metadata
                .attributes
                .get("language")
                .map(String::as_str),
            Some("rust")
        );
        assert!(
            listing
                .metadata
                .options
                .iter()
                .any(|option| option == "linenums")
        );
    }

    #[test]
    fn parses_delimited_sidebar_block_attributes() {
        let document = parse_document("[foo=bar,%open,.callout]\n****\ninside\n****");

        let [Block::Sidebar(sidebar)] = document.blocks.as_slice() else {
            panic!("expected sidebar");
        };
        assert_eq!(
            sidebar.metadata.attributes.get("foo").map(String::as_str),
            Some("bar")
        );
        assert_eq!(
            sidebar
                .metadata
                .attributes
                .get("open-option")
                .map(String::as_str),
            Some("")
        );
        assert_eq!(sidebar.metadata.role.as_deref(), Some("callout"));
        assert_eq!(sidebar.metadata.options, vec!["open"]);
        assert_eq!(sidebar.metadata.roles, vec!["callout"]);
    }

    #[test]
    fn parses_non_delimited_block_with_stacked_attribute_lists() {
        let document = parse_document(".Exhibit A\n[source]\n[rust]\nfn main() {}");

        let [Block::Listing(listing)] = document.blocks.as_slice() else {
            panic!("expected listing");
        };
        assert_eq!(listing.metadata.title.as_deref(), Some("Exhibit A"));
        assert_eq!(listing.metadata.style.as_deref(), Some("source"));
        assert_eq!(
            listing.metadata.attributes.get("$1").map(String::as_str),
            Some("source")
        );
        assert_eq!(
            listing.metadata.attributes.get("$2").map(String::as_str),
            Some("rust")
        );
        assert_eq!(
            listing
                .metadata
                .attributes
                .get("language")
                .map(String::as_str),
            Some("rust")
        );
    }

    #[test]
    fn applies_anchor_to_unordered_lists() {
        let document = parse_document("[[steps]]\n* one");

        let [Block::UnorderedList(list)] = document.blocks.as_slice() else {
            panic!("expected unordered list");
        };
        assert_eq!(list.metadata.id.as_deref(), Some("steps"));
        assert_eq!(list.reftext, None);
    }

    #[test]
    fn applies_anchor_to_ordered_lists() {
        let document = parse_document("[[recipe]]\n. one");

        let [Block::OrderedList(list)] = document.blocks.as_slice() else {
            panic!("expected ordered list");
        };
        assert_eq!(list.metadata.id.as_deref(), Some("recipe"));
        assert_eq!(list.reftext, None);
    }

    #[test]
    fn preserves_anchor_reftext_for_unordered_lists() {
        let document = parse_document("[[steps,Setup Steps]]\n* one");

        let [Block::UnorderedList(list)] = document.blocks.as_slice() else {
            panic!("expected unordered list");
        };
        assert_eq!(list.metadata.id.as_deref(), Some("steps"));
        assert_eq!(list.reftext.as_deref(), Some("Setup Steps"));
    }

    #[test]
    fn preserves_anchor_reftext_for_delimited_blocks() {
        let document = parse_document(
            "[[code-sample,Code Sample]]\n----\nputs 'hello'\n----\n\n[[aside,Important Aside]]\n****\ninside\n****",
        );

        let [Block::Listing(listing), Block::Sidebar(sidebar)] = document.blocks.as_slice() else {
            panic!("expected listing and sidebar");
        };
        assert_eq!(listing.reftext.as_deref(), Some("Code Sample"));
        assert_eq!(sidebar.reftext.as_deref(), Some("Important Aside"));
    }

    #[test]
    fn parses_admonition_paragraphs() {
        let document = parse_document("NOTE: This is _important_.");

        assert_eq!(
            document.blocks,
            vec![Block::Admonition(AdmonitionBlock {
                variant: AdmonitionVariant::Note,
                blocks: vec![Block::Paragraph(Paragraph {
                    inlines: vec![
                        Inline::Text("This is ".into()),
                        Inline::Span(InlineSpan {
                            variant: InlineVariant::Emphasis,
                            form: InlineForm::Constrained,
                            inlines: vec![Inline::Text("important".into())],
                        }),
                        Inline::Text(".".into()),
                    ],
                    lines: vec!["This is _important_.".into()],
                    id: None,
                    reftext: None,
                    metadata: BlockMetadata::default(),
                })],
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
            })]
        );
    }

    #[test]
    fn parses_styled_admonition_paragraphs() {
        let document = parse_document("[NOTE]\nRemember the milk.");

        let [Block::Admonition(admonition)] = document.blocks.as_slice() else {
            panic!("expected admonition");
        };
        assert_eq!(admonition.variant, AdmonitionVariant::Note);
        assert_eq!(admonition.id, None);
        assert_eq!(admonition.reftext, None);
        assert_eq!(admonition.metadata.style.as_deref(), Some("NOTE"));
        let [Block::Paragraph(paragraph)] = admonition.blocks.as_slice() else {
            panic!("expected paragraph");
        };
        assert_eq!(paragraph.plain_text(), "Remember the milk.");
    }

    #[test]
    fn parses_styled_delimited_admonition_blocks() {
        let document = parse_document("[TIP]\n====\nRemember the milk.\n====");

        let [Block::Admonition(admonition)] = document.blocks.as_slice() else {
            panic!("expected admonition");
        };
        assert_eq!(admonition.variant, AdmonitionVariant::Tip);
        assert_eq!(admonition.id, None);
        assert_eq!(admonition.reftext, None);
        assert_eq!(admonition.metadata.style.as_deref(), Some("TIP"));
        let [Block::Paragraph(paragraph)] = admonition.blocks.as_slice() else {
            panic!("expected paragraph");
        };
        assert_eq!(paragraph.plain_text(), "Remember the milk.");
    }

    #[test]
    fn applies_anchor_to_admonition_paragraphs() {
        let document = parse_document("[[install-note,Install Note]]\nNOTE: Read this first.");

        let [Block::Admonition(admonition)] = document.blocks.as_slice() else {
            panic!("expected admonition");
        };
        assert_eq!(admonition.id.as_deref(), Some("install-note"));
        assert_eq!(admonition.reftext.as_deref(), Some("Install Note"));
    }

    #[test]
    fn applies_anchor_to_styled_delimited_admonitions() {
        let document =
            parse_document("[[ship-tip,Shipping Tip]]\n[TIP]\n====\nShip it carefully.\n====");

        let [Block::Admonition(admonition)] = document.blocks.as_slice() else {
            panic!("expected admonition");
        };
        assert_eq!(admonition.id.as_deref(), Some("ship-tip"));
        assert_eq!(admonition.reftext.as_deref(), Some("Shipping Tip"));
        assert_eq!(admonition.variant, AdmonitionVariant::Tip);
    }

    #[test]
    fn parses_block_image_with_alt_text() {
        let document = parse_document("image::tiger.png[Tiger]");

        let [Block::Image(image)] = document.blocks.as_slice() else {
            panic!("expected image block, got: {:?}", document.blocks);
        };
        assert_eq!(image.target, "tiger.png");
        assert_eq!(image.alt, "Tiger");
        assert_eq!(image.width, None);
        assert_eq!(image.height, None);
    }

    #[test]
    fn parses_block_image_with_dimensions() {
        let document = parse_document("image::images/tiger.png[Tiger, 200, 300]");

        let [Block::Image(image)] = document.blocks.as_slice() else {
            panic!("expected image block");
        };
        assert_eq!(image.target, "images/tiger.png");
        assert_eq!(image.alt, "Tiger");
        assert_eq!(image.width.as_deref(), Some("200"));
        assert_eq!(image.height.as_deref(), Some("300"));
    }

    #[test]
    fn parses_block_image_with_auto_generated_alt() {
        let document = parse_document("image::images/lions-and-tigers.png[]");

        let [Block::Image(image)] = document.blocks.as_slice() else {
            panic!("expected image block");
        };
        assert_eq!(image.target, "images/lions-and-tigers.png");
        assert_eq!(image.alt, "lions and tigers");
    }

    #[test]
    fn parses_block_image_with_named_attributes() {
        let document = parse_document("image::tiger.png[Tiger, link='http://example.com']");

        let [Block::Image(image)] = document.blocks.as_slice() else {
            panic!("expected image block");
        };
        assert_eq!(image.target, "tiger.png");
        assert_eq!(image.alt, "Tiger");
        assert_eq!(
            image.metadata.attributes.get("link").map(String::as_str),
            Some("http://example.com")
        );
    }

    #[test]
    fn applies_prelude_to_block_image() {
        let document = parse_document(".The AsciiDoc Tiger\nimage::tiger.png[Tiger]");

        let [Block::Image(image)] = document.blocks.as_slice() else {
            panic!("expected image block");
        };
        assert_eq!(image.metadata.title.as_deref(), Some("The AsciiDoc Tiger"));
        assert_eq!(image.target, "tiger.png");
    }

    #[test]
    fn parses_block_image_with_subdirectory_path() {
        let document = parse_document("image::assets/mupdate-update-flow/initial-inventory.png[]");

        let [Block::Image(image)] = document.blocks.as_slice() else {
            panic!("expected image block");
        };
        assert_eq!(
            image.target,
            "assets/mupdate-update-flow/initial-inventory.png"
        );
        assert_eq!(image.alt, "initial inventory");
    }

    #[test]
    fn parses_quote_block() {
        let document =
            parse_document("[quote, Abraham Lincoln, Gettysburg Address]\n____\nFour score.\n____");

        let [Block::Quote(quote)] = document.blocks.as_slice() else {
            panic!("expected quote block");
        };
        assert!(!quote.is_verse);
        assert_eq!(quote.attribution.as_deref(), Some("Abraham Lincoln"));
        assert_eq!(quote.citetitle.as_deref(), Some("Gettysburg Address"));
        assert_eq!(quote.blocks.len(), 1);
    }

    #[test]
    fn parses_quote_block_with_combined_style_id_and_positional_attributes() {
        let document = parse_document(
            "[quote#roads, Dr. Emmett Brown, Back to the Future]\n____\nRoads? Where we're going, we don't need roads.\n____",
        );

        let [Block::Quote(quote)] = document.blocks.as_slice() else {
            panic!("expected quote block");
        };
        assert_eq!(quote.metadata.style, None);
        assert_eq!(quote.metadata.id.as_deref(), Some("roads"));
        assert_eq!(quote.attribution.as_deref(), Some("Dr. Emmett Brown"));
        assert_eq!(quote.citetitle.as_deref(), Some("Back to the Future"));
    }

    #[test]
    fn parses_quote_block_without_attribution() {
        let document = parse_document("____\nSome quoted text.\n____");

        let [Block::Quote(quote)] = document.blocks.as_slice() else {
            panic!("expected quote block");
        };
        assert!(!quote.is_verse);
        assert!(quote.attribution.is_none());
        assert!(quote.citetitle.is_none());
        assert_eq!(quote.blocks.len(), 1);
    }

    #[test]
    fn parses_verse_block() {
        let document = parse_document(
            "[verse, Carl Sandburg, Fog]\n____\nThe fog comes\non little cat feet.\n____",
        );

        let [Block::Quote(quote)] = document.blocks.as_slice() else {
            panic!("expected verse block");
        };
        assert!(quote.is_verse);
        assert_eq!(quote.attribution.as_deref(), Some("Carl Sandburg"));
        assert_eq!(quote.citetitle.as_deref(), Some("Fog"));
        assert_eq!(
            quote.content.as_deref(),
            Some("The fog comes\non little cat feet.")
        );
    }

    #[test]
    fn parses_delimited_literal_block() {
        let document = parse_document("....\n  indented preformatted text\n....");

        let [Block::Literal(literal)] = document.blocks.as_slice() else {
            panic!("expected literal block");
        };
        assert_eq!(literal.lines, vec!["  indented preformatted text"]);
    }

    #[test]
    fn normalizes_trailing_spaces_before_parsing_blocks() {
        let document = parse_document("line one  \r\nline two\t\r\n");

        let [Block::Paragraph(paragraph)] = document.blocks.as_slice() else {
            panic!("expected paragraph");
        };
        assert_eq!(paragraph.lines, vec!["line one", "line two"]);
    }

    #[test]
    fn parses_literal_styled_paragraph() {
        let document = parse_document("[literal]\nThis becomes preformatted.");

        let [Block::Literal(literal)] = document.blocks.as_slice() else {
            panic!("expected literal block");
        };
        assert_eq!(literal.lines, vec!["This becomes preformatted."]);
        assert_eq!(literal.metadata.style, None);
    }

    #[test]
    fn parses_listing_styled_paragraph() {
        let document = parse_document("[listing]\nputs 'hello' <1>");

        let [Block::Listing(listing)] = document.blocks.as_slice() else {
            panic!("expected listing block");
        };
        assert_eq!(listing.lines, vec!["puts 'hello'"]);
        assert_eq!(listing.callouts, vec![(0, 1)]);
        assert_eq!(listing.metadata.style, None);
    }

    #[test]
    fn parses_source_styled_paragraph() {
        let document = parse_document("[source,rust]\nfn main() {}");

        let [Block::Listing(listing)] = document.blocks.as_slice() else {
            panic!("expected listing block");
        };
        assert_eq!(listing.lines, vec!["fn main() {}"]);
        assert_eq!(listing.metadata.style.as_deref(), Some("source"));
        assert_eq!(
            listing
                .metadata
                .attributes
                .get("language")
                .map(String::as_str),
            Some("rust")
        );
    }

    #[test]
    fn parses_source_blocks_with_positional_linenums_option() {
        let document = parse_document("[source,rust,linenums]\n----\nfn main() {}\n----");

        let [Block::Listing(listing)] = document.blocks.as_slice() else {
            panic!("expected listing block");
        };
        assert_eq!(listing.metadata.style.as_deref(), Some("source"));
        assert_eq!(
            listing
                .metadata
                .attributes
                .get("language")
                .map(String::as_str),
            Some("rust")
        );
        assert!(
            listing
                .metadata
                .options
                .iter()
                .any(|option| option == "linenums")
        );
        assert_eq!(
            listing
                .metadata
                .attributes
                .get("linenums-option")
                .map(String::as_str),
            Some("")
        );
    }

    #[test]
    fn parses_quote_styled_paragraph() {
        let document = parse_document("[quote, Abraham Lincoln, Gettysburg Address]\nFour score.");

        let [Block::Quote(quote)] = document.blocks.as_slice() else {
            panic!("expected quote block");
        };
        assert_eq!(quote.attribution.as_deref(), Some("Abraham Lincoln"));
        assert_eq!(quote.citetitle.as_deref(), Some("Gettysburg Address"));
        assert!(!quote.is_verse);
        let [Block::Paragraph(paragraph)] = quote.blocks.as_slice() else {
            panic!("expected paragraph child");
        };
        assert_eq!(paragraph.plain_text(), "Four score.");
    }

    #[test]
    fn parses_quote_styled_paragraph_with_combined_style_and_id() {
        let document = parse_document("[quote#roads]\nRoads? Where we're going, we don't need roads.");

        let [Block::Quote(quote)] = document.blocks.as_slice() else {
            panic!("expected quote block");
        };
        assert_eq!(quote.metadata.style, None);
        assert_eq!(quote.metadata.id.as_deref(), Some("roads"));
        assert!(quote.attribution.is_none());
        assert!(quote.citetitle.is_none());
        let [Block::Paragraph(paragraph)] = quote.blocks.as_slice() else {
            panic!("expected paragraph child");
        };
        assert_eq!(paragraph.plain_text(), "Roads? Where we're going, we don't need roads.");
    }

    #[test]
    fn parses_sidebar_styled_paragraph() {
        let document = parse_document("[sidebar]\nA short aside.");

        let [Block::Sidebar(sidebar)] = document.blocks.as_slice() else {
            panic!("expected sidebar block");
        };
        assert_eq!(sidebar.metadata.style, None);
        let [Block::Paragraph(paragraph)] = sidebar.blocks.as_slice() else {
            panic!("expected paragraph child");
        };
        assert_eq!(paragraph.plain_text(), "A short aside.");
    }

    #[test]
    fn parses_example_styled_paragraph() {
        let document = parse_document("[example]\nA short example.");

        let [Block::Example(example)] = document.blocks.as_slice() else {
            panic!("expected example block");
        };
        assert_eq!(example.metadata.style, None);
        let [Block::Paragraph(paragraph)] = example.blocks.as_slice() else {
            panic!("expected paragraph child");
        };
        assert_eq!(paragraph.plain_text(), "A short example.");
    }

    #[test]
    fn parses_verse_styled_paragraph() {
        let document = parse_document("[verse, Carl Sandburg, Fog]\nThe fog comes\non little cat feet.");

        let [Block::Quote(quote)] = document.blocks.as_slice() else {
            panic!("expected verse block");
        };
        assert!(quote.is_verse);
        assert_eq!(quote.metadata.style, None);
        assert_eq!(
            quote.content.as_deref(),
            Some("The fog comes\non little cat feet.")
        );
        assert_eq!(quote.attribution.as_deref(), Some("Carl Sandburg"));
        assert_eq!(quote.citetitle.as_deref(), Some("Fog"));
    }

    #[test]
    fn parses_abstract_styled_paragraph_as_open_block() {
        let document = parse_document("[abstract]\nAn abstract for the article.");

        let [Block::Open(open)] = document.blocks.as_slice() else {
            panic!("expected open block");
        };
        assert_eq!(open.metadata.style, None);
        assert_eq!(open.context, Some(OpenBlockContext::Abstract));
        let [Block::Paragraph(paragraph)] = open.blocks.as_slice() else {
            panic!("expected paragraph child");
        };
        assert_eq!(paragraph.plain_text(), "An abstract for the article.");
    }

    #[test]
    fn parses_partintro_styled_paragraph_as_open_block() {
        let document = parse_document("[partintro]\nRead this first.");

        let [Block::Open(open)] = document.blocks.as_slice() else {
            panic!("expected open block");
        };
        assert_eq!(open.metadata.style, None);
        assert_eq!(open.context, Some(OpenBlockContext::PartIntro));
        let [Block::Paragraph(paragraph)] = open.blocks.as_slice() else {
            panic!("expected paragraph child");
        };
        assert_eq!(paragraph.plain_text(), "Read this first.");
    }

    #[test]
    fn parses_comment_styled_paragraph_as_comment_open_block() {
        let document = parse_document("[comment]\nThis should stay hidden.");

        let [Block::Open(open)] = document.blocks.as_slice() else {
            panic!("expected open block");
        };
        assert_eq!(open.metadata.style, None);
        assert_eq!(open.context, Some(OpenBlockContext::Comment));
        let [Block::Paragraph(paragraph)] = open.blocks.as_slice() else {
            panic!("expected paragraph child");
        };
        assert_eq!(paragraph.plain_text(), "This should stay hidden.");
    }

    #[test]
    fn parses_normal_styled_indented_paragraph_as_paragraph() {
        let document = parse_document("[normal]\n indented but not literal");

        let [Block::Paragraph(paragraph)] = document.blocks.as_slice() else {
            panic!("expected paragraph");
        };
        assert_eq!(paragraph.metadata.style, None);
        assert_eq!(paragraph.plain_text(), " indented but not literal");
    }

    #[test]
    fn parses_pass_styled_paragraph() {
        let document = parse_document("[pass]\n<span>ok</span>");

        let [Block::Passthrough(passthrough)] = document.blocks.as_slice() else {
            panic!("expected passthrough block");
        };
        assert_eq!(passthrough.content, "<span>ok</span>");
        assert_eq!(passthrough.metadata.style, None);
    }

    #[test]
    fn parses_stem_delimited_passthrough_block() {
        let document = parse_document("[stem]\n++++\nsqrt(4) = 2\n++++");

        let [Block::Passthrough(passthrough)] = document.blocks.as_slice() else {
            panic!("expected passthrough block");
        };
        assert_eq!(passthrough.content, "sqrt(4) = 2");
        assert_eq!(passthrough.metadata.style.as_deref(), Some("stem"));
    }

    #[test]
    fn parses_indented_paragraph_as_literal() {
        let document = parse_document(" indented line one\n indented line two");

        let [Block::Literal(literal)] = document.blocks.as_slice() else {
            panic!("expected literal block");
        };
        assert_eq!(
            literal.lines,
            vec![" indented line one", " indented line two"]
        );
    }

    #[test]
    fn parses_bare_open_block() {
        let document = parse_document("--\nparagraph one\n\nparagraph two\n--");

        let [Block::Open(open)] = document.blocks.as_slice() else {
            panic!("expected open block");
        };
        assert_eq!(open.blocks.len(), 2);
    }

    #[test]
    fn parses_styled_open_block_as_sidebar() {
        let document = parse_document("[sidebar]\n--\ninside\n--");

        let [Block::Sidebar(_)] = document.blocks.as_slice() else {
            panic!("expected sidebar");
        };
    }

    #[test]
    fn parses_styled_open_block_as_admonition() {
        let document = parse_document("[NOTE]\n--\nRemember this.\n--");

        let [Block::Admonition(admonition)] = document.blocks.as_slice() else {
            panic!("expected admonition");
        };
        assert_eq!(admonition.variant, AdmonitionVariant::Note);
        assert_eq!(admonition.blocks.len(), 1);
    }

    #[test]
    fn parses_source_styled_open_block_as_listing() {
        let document = parse_document("[source,rust]\n--\nfn main() {} <1>\n--");

        let [Block::Listing(listing)] = document.blocks.as_slice() else {
            panic!("expected listing");
        };
        assert_eq!(listing.metadata.style.as_deref(), Some("source"));
        assert_eq!(
            listing
                .metadata
                .attributes
                .get("language")
                .map(String::as_str),
            Some("rust")
        );
        assert_eq!(listing.lines, vec!["fn main() {}"]);
        assert_eq!(listing.callouts, vec![(0, 1)]);
    }

    #[test]
    fn parses_listing_styled_open_block_as_listing() {
        let document = parse_document("[listing]\n--\nputs 'hello' <1>\n--");

        let [Block::Listing(listing)] = document.blocks.as_slice() else {
            panic!("expected listing");
        };
        assert_eq!(listing.metadata.style, None);
        assert_eq!(listing.lines, vec!["puts 'hello'"]);
        assert_eq!(listing.callouts, vec![(0, 1)]);
    }

    #[test]
    fn parses_literal_styled_open_block_as_literal() {
        let document = parse_document("[literal]\n--\n  preserved text\n--");

        let [Block::Literal(literal)] = document.blocks.as_slice() else {
            panic!("expected literal");
        };
        assert_eq!(literal.metadata.style, None);
        assert_eq!(literal.lines, vec!["  preserved text"]);
        assert!(literal.callouts.is_empty());
    }

    #[test]
    fn parses_abstract_styled_open_block_as_open() {
        let document = parse_document("[abstract]\n--\nAbstract.\n--");

        let [Block::Open(open)] = document.blocks.as_slice() else {
            panic!("expected open block");
        };
        assert_eq!(open.metadata.style, None);
        assert_eq!(open.context, Some(OpenBlockContext::Abstract));
        assert_eq!(open.blocks.len(), 1);
    }

    #[test]
    fn parses_partintro_styled_open_block_as_open() {
        let document = parse_document("[partintro]\n--\nIntro.\n--");

        let [Block::Open(open)] = document.blocks.as_slice() else {
            panic!("expected open block");
        };
        assert_eq!(open.metadata.style, None);
        assert_eq!(open.context, Some(OpenBlockContext::PartIntro));
        assert_eq!(open.blocks.len(), 1);
    }

    #[test]
    fn parses_comment_styled_open_block_as_open() {
        let document = parse_document("[comment]\n--\nHidden.\n--");

        let [Block::Open(open)] = document.blocks.as_slice() else {
            panic!("expected open block");
        };
        assert_eq!(open.metadata.style, None);
        assert_eq!(open.context, Some(OpenBlockContext::Comment));
        assert_eq!(open.blocks.len(), 1);
    }

    #[test]
    fn parses_pass_styled_open_block_as_passthrough() {
        let document = parse_document("[pass]\n--\n<span>ok</span>\n--");

        let [Block::Passthrough(passthrough)] = document.blocks.as_slice() else {
            panic!("expected passthrough block");
        };
        assert_eq!(passthrough.content, "<span>ok</span>");
        assert_eq!(passthrough.metadata.style, None);
    }

    #[test]
    fn parses_stem_styled_open_block_as_passthrough() {
        let document = parse_document("[stem]\n--\nsqrt(4) = 2\n--");

        let [Block::Passthrough(passthrough)] = document.blocks.as_slice() else {
            panic!("expected passthrough block");
        };
        assert_eq!(passthrough.content, "sqrt(4) = 2");
        assert_eq!(passthrough.metadata.style.as_deref(), Some("stem"));
    }

    #[test]
    fn parses_latexmath_styled_open_block_as_passthrough() {
        let document = parse_document("[latexmath]\n--\n\\alpha + \\beta\n--");

        let [Block::Passthrough(passthrough)] = document.blocks.as_slice() else {
            panic!("expected passthrough block");
        };
        assert_eq!(passthrough.content, "\\alpha + \\beta");
        assert_eq!(passthrough.metadata.style.as_deref(), Some("latexmath"));
    }

    #[test]
    fn parses_asciimath_styled_open_block_as_passthrough() {
        let document = parse_document("[asciimath]\n--\nsqrt(4) = 2\n--");

        let [Block::Passthrough(passthrough)] = document.blocks.as_slice() else {
            panic!("expected passthrough block");
        };
        assert_eq!(passthrough.content, "sqrt(4) = 2");
        assert_eq!(passthrough.metadata.style.as_deref(), Some("asciimath"));
    }

    #[test]
    fn parses_listing_styled_literal_block_as_listing() {
        let document = parse_document("[listing]\n....\nputs 'hello' <1>\n....");

        let [Block::Listing(listing)] = document.blocks.as_slice() else {
            panic!("expected listing");
        };
        assert_eq!(listing.metadata.style, None);
        assert_eq!(listing.lines, vec!["puts 'hello'"]);
        assert_eq!(listing.callouts, vec![(0, 1)]);
    }

    #[test]
    fn parses_source_styled_literal_block_as_listing() {
        let document = parse_document("[source,rust]\n....\nfn main() {} <1>\n....");

        let [Block::Listing(listing)] = document.blocks.as_slice() else {
            panic!("expected listing");
        };
        assert_eq!(listing.metadata.style.as_deref(), Some("source"));
        assert_eq!(listing.lines, vec!["fn main() {}"]);
        assert_eq!(listing.callouts, vec![(0, 1)]);
        assert_eq!(
            listing
                .metadata
                .attributes
                .get("language")
                .map(String::as_str),
            Some("rust")
        );
    }

    #[test]
    fn parses_literal_styled_listing_block_as_literal() {
        let document = parse_document("[literal]\n----\nputs 'hello' <1>\n----");

        let [Block::Literal(literal)] = document.blocks.as_slice() else {
            panic!("expected literal");
        };
        assert_eq!(literal.metadata.style, None);
        assert_eq!(literal.lines, vec!["puts 'hello' <1>"]);
        assert!(literal.callouts.is_empty());
    }

    #[test]
    fn leaves_unknown_styled_open_block_as_open() {
        let document = parse_document("[custom]\n--\ninside\n--");

        let [Block::Open(open)] = document.blocks.as_slice() else {
            panic!("expected open block");
        };
        assert_eq!(open.metadata.style.as_deref(), Some("custom"));
        assert_eq!(open.blocks.len(), 1);
    }
}
