use crate::prepare::{
    DocumentBlock, DocumentSection, PreparedBlock, PreparedInline, prepare_document,
};
use std::{cell::RefCell, collections::BTreeMap, sync::OnceLock};
use syntect::{
    easy::HighlightLines,
    highlighting::{Theme, ThemeSet},
    html::{IncludeBackground, styled_line_to_highlighted_html},
    parsing::{SyntaxReference, SyntaxSet},
};

struct RenderContext<'a> {
    document_attributes: &'a std::collections::BTreeMap<String, String>,
    sections: &'a [DocumentSection],
    caption_counters: RefCell<BTreeMap<String, CounterState>>,
}

#[derive(Clone, Copy)]
enum CaptionKind {
    Example,
    Listing,
    Table,
    Image,
}

#[derive(Clone)]
enum CounterState {
    Numeric(u32),
    Alpha { index: u32, uppercase: bool },
}

pub fn render_html(document: &crate::ast::Document) -> String {
    render_prepared_html(&prepare_document(document))
}

pub fn render_prepared_html(document: &DocumentBlock) -> String {
    let mut html = String::new();
    let ctx = RenderContext {
        document_attributes: &document.attributes,
        sections: &document.sections,
        caption_counters: RefCell::new(BTreeMap::new()),
    };

    html.push_str("<div id=\"header\">\n");
    if !document.title.is_empty() {
        html.push_str(&format!("<h1>{}</h1>\n", escape_html(&document.title)));
    }

    // Auto-place TOC when toc attribute is set but not "macro"
    let toc_placement = document.attributes.get("toc").map(String::as_str);
    if matches!(toc_placement, Some(v) if v != "macro") {
        render_toc(&mut html, &document.sections, ctx.document_attributes);
    }

    html.push_str("</div>\n");

    html.push_str("<div id=\"content\">\n");
    for block in &document.blocks {
        render_block(&mut html, block, &ctx);
    }
    html.push_str("</div>\n");
    render_footnotes(&mut html, &document.footnotes);
    html
}

fn render_toc(
    html: &mut String,
    sections: &[DocumentSection],
    document_attributes: &std::collections::BTreeMap<String, String>,
) {
    if sections.is_empty() {
        return;
    }
    let title = document_attributes
        .get("toctitle")
        .map(String::as_str)
        .filter(|t| !t.is_empty())
        .unwrap_or("Table of Contents");
    let max_level: u8 = document_attributes
        .get("toclevels")
        .and_then(|v| v.parse().ok())
        .unwrap_or(2);
    html.push_str("<div id=\"toc\" class=\"toc\">\n");
    html.push_str(&format!(
        "<div id=\"toctitle\">{}</div>\n",
        escape_html(title)
    ));
    render_toc_sections(html, sections, 1, max_level);
    html.push_str("</div>\n");
}

fn render_toc_sections(html: &mut String, sections: &[DocumentSection], level: u8, max_level: u8) {
    if level > max_level || sections.is_empty() {
        return;
    }
    html.push_str(&format!("<ul class=\"sectlevel{}\">\n", level));
    for section in sections {
        html.push_str("<li>");
        html.push_str(&format!(
            "<a href=\"#{}\">{}",
            escape_html(&section.id),
            escape_html(&section.title)
        ));
        html.push_str("</a>");
        if !section.sections.is_empty() && level < max_level {
            html.push('\n');
            render_toc_sections(html, &section.sections, level + 1, max_level);
        }
        html.push_str("</li>\n");
    }
    html.push_str("</ul>\n");
}

fn trimmed_delimited_content_lines(content: &str) -> (usize, Vec<&str>) {
    let lines = if content.is_empty() {
        Vec::new()
    } else {
        content.split('\n').collect::<Vec<_>>()
    };

    let start = lines
        .iter()
        .position(|line| !line.trim().is_empty())
        .unwrap_or(lines.len());
    let end = lines
        .iter()
        .rposition(|line| !line.trim().is_empty())
        .map(|idx| idx + 1)
        .unwrap_or(start);

    (start, lines[start..end].to_vec())
}

fn render_block(html: &mut String, block: &PreparedBlock, ctx: &RenderContext<'_>) {
    match block {
        PreparedBlock::Preamble(preamble) => {
            html.push_str("<div id=\"preamble\">\n<div class=\"sectionbody\">\n");
            for block in &preamble.blocks {
                render_block(html, block, ctx);
            }
            html.push_str("</div>\n</div>\n");
        }
        PreparedBlock::Paragraph(paragraph) => render_paragraph(html, paragraph, ctx),
        PreparedBlock::Admonition(admonition) => render_admonition(html, admonition, ctx),
        PreparedBlock::UnorderedList(list) => render_unordered_list(html, list, ctx),
        PreparedBlock::OrderedList(list) => render_ordered_list(html, list, ctx),
        PreparedBlock::DescriptionList(list) => render_description_list(html, list, ctx),
        PreparedBlock::Table(table) => render_table(html, table, ctx),
        PreparedBlock::Listing(listing) => render_listing(html, listing, ctx),
        PreparedBlock::Literal(literal) => render_literal(html, literal, ctx),
        PreparedBlock::CalloutList(colist) => render_callout_list(html, colist),
        PreparedBlock::Example(example) => render_compound(
            html,
            "exampleblock",
            example,
            ctx,
            Some(CaptionKind::Example),
        ),
        PreparedBlock::Sidebar(sidebar) => render_sidebar(html, sidebar, ctx),
        PreparedBlock::Open(open) => render_open(html, open, ctx),
        PreparedBlock::Quote(quote) => render_quote(html, quote, ctx),
        PreparedBlock::Passthrough(p) => render_passthrough(html, p, ctx.document_attributes, ctx),
        PreparedBlock::Image(image) => render_image_block(html, image, ctx),
        PreparedBlock::Section(section) => {
            let level = usize::from(section.level) + 1;
            html.push_str(&format!(
                "<div class=\"sect{}\" id=\"{}\">\n",
                section.level,
                escape_html(&section.id)
            ));
            html.push_str(&format!(
                "<h{level}>{}</h{level}>\n",
                escape_html(&section.title)
            ));
            html.push_str("<div class=\"sectionbody\">\n");
            for block in &section.blocks {
                render_block(html, block, ctx);
            }
            html.push_str("</div>\n</div>\n");
        }
        PreparedBlock::Toc(_) => {
            render_toc(html, ctx.sections, ctx.document_attributes);
        }
    }
}

fn render_unordered_list(
    html: &mut String,
    list: &crate::prepare::ListBlock,
    ctx: &RenderContext<'_>,
) {
    html.push_str("<div class=\"ulist\"");
    if let Some(id) = &list.id {
        html.push_str(&format!(" id=\"{}\"", escape_html(id)));
    }
    html.push_str(">\n");
    if let Some(title) = expanded_block_title(list.title.as_deref(), ctx) {
        html.push_str(&format!(
            "<div class=\"title\">{}</div>\n",
            escape_html(&title)
        ));
    }
    html.push_str("<ul>\n");
    for item in &list.items {
        html.push_str("<li>\n");
        for block in &item.blocks {
            render_block(html, block, ctx);
        }
        html.push_str("</li>\n");
    }
    html.push_str("</ul>\n</div>\n");
}

fn render_ordered_list(
    html: &mut String,
    list: &crate::prepare::ListBlock,
    ctx: &RenderContext<'_>,
) {
    html.push_str("<div class=\"olist arabic\"");
    if let Some(id) = &list.id {
        html.push_str(&format!(" id=\"{}\"", escape_html(id)));
    }
    html.push_str(">\n");
    if let Some(title) = expanded_block_title(list.title.as_deref(), ctx) {
        html.push_str(&format!(
            "<div class=\"title\">{}</div>\n",
            escape_html(&title)
        ));
    }
    html.push_str("<ol class=\"arabic\">\n");
    for item in &list.items {
        html.push_str("<li>\n");
        for block in &item.blocks {
            render_block(html, block, ctx);
        }
        html.push_str("</li>\n");
    }
    html.push_str("</ol>\n</div>\n");
}

fn render_description_list(
    html: &mut String,
    list: &crate::prepare::DescriptionListBlock,
    ctx: &RenderContext<'_>,
) {
    html.push_str("<div class=\"dlist\"");
    if let Some(id) = &list.id {
        html.push_str(&format!(" id=\"{}\"", escape_html(id)));
    }
    html.push_str(">\n");
    if let Some(title) = expanded_block_title(list.title.as_deref(), ctx) {
        html.push_str(&format!(
            "<div class=\"title\">{}</div>\n",
            escape_html(&title)
        ));
    }
    html.push_str("<dl>\n");
    for item in &list.items {
        for term in &item.terms {
            html.push_str("<dt class=\"hdlist1\">");
            render_inlines(html, &term.inlines);
            html.push_str("</dt>\n");
        }
        if let Some(desc) = &item.description {
            html.push_str("<dd>\n");
            for block in &desc.blocks {
                render_block(html, block, ctx);
            }
            html.push_str("</dd>\n");
        }
    }
    html.push_str("</dl>\n</div>\n");
}

fn render_table(html: &mut String, table: &crate::prepare::TableBlock, ctx: &RenderContext<'_>) {
    html.push_str("<table class=\"tableblock frame-all grid-all stretch\"");
    if let Some(id) = &table.id {
        html.push_str(&format!(" id=\"{}\"", escape_html(id)));
    }
    html.push_str(">\n");
    if let Some(title) = captioned_block_title(
        table.title.as_deref(),
        &table.attributes,
        ctx,
        CaptionKind::Table,
    ) {
        html.push_str(&format!(
            "<caption class=\"title\">{}</caption>\n",
            escape_html(&title)
        ));
    }
    if let Some(header) = &table.header {
        html.push_str("<thead>\n<tr>\n");
        for cell in &header.cells {
            render_table_cell(html, cell, true, ctx);
        }
        html.push_str("</tr>\n</thead>\n");
    }
    html.push_str("<tbody>\n");
    for row in &table.rows {
        html.push_str("<tr>\n");
        for cell in &row.cells {
            render_table_cell(html, cell, false, ctx);
        }
        html.push_str("</tr>\n");
    }
    html.push_str("</tbody>\n</table>\n");
}

fn render_table_cell(
    html: &mut String,
    cell: &crate::prepare::TableCell,
    header: bool,
    ctx: &RenderContext<'_>,
) {
    let tag = if header || cell.style.as_deref() == Some("header") {
        "th"
    } else {
        "td"
    };
    html.push_str(&format!(
        "<{tag} class=\"tableblock halign-left valign-top\""
    ));
    if cell.colspan > 1 {
        html.push_str(&format!(" colspan=\"{}\"", cell.colspan));
    }
    if cell.rowspan > 1 {
        html.push_str(&format!(" rowspan=\"{}\"", cell.rowspan));
    }
    html.push('>');
    render_table_cell_content(html, cell, tag == "th", ctx);
    html.push_str(&format!("</{tag}>\n"));
}

fn render_table_cell_content(
    html: &mut String,
    cell: &crate::prepare::TableCell,
    header: bool,
    ctx: &RenderContext<'_>,
) {
    if cell.blocks.len() == 1
        && matches!(
            cell.blocks.first(),
            Some(crate::prepare::PreparedBlock::Paragraph(_))
        )
    {
        if let Some(crate::prepare::PreparedBlock::Paragraph(paragraph)) = cell.blocks.first() {
            if header {
                render_inlines(html, &paragraph.inlines);
            } else {
                html.push_str("<p class=\"tableblock\">");
                render_inlines(html, &paragraph.inlines);
                html.push_str("</p>");
            }
            return;
        }
    }

    for block in &cell.blocks {
        render_block(html, block, ctx);
    }
}

fn render_listing(
    html: &mut String,
    listing: &crate::prepare::ListingBlock,
    ctx: &RenderContext<'_>,
) {
    html.push_str("<div class=\"listingblock\"");
    if let Some(id) = &listing.id {
        html.push_str(&format!(" id=\"{}\"", escape_html(id)));
    }
    html.push_str(">\n");
    if let Some(title) = captioned_block_title(
        listing.title.as_deref(),
        &listing.attributes,
        ctx,
        CaptionKind::Listing,
    ) {
        html.push_str(&format!(
            "<div class=\"title\">{}</div>\n",
            escape_html(&title)
        ));
    }
    let lang = listing.attributes.get("language").map(String::as_str);
    let is_source =
        listing.style.as_deref() == Some("source") && lang.is_some_and(|l| !l.is_empty());
    let (line_offset, lines) = trimmed_delimited_content_lines(&listing.content);
    let rendered_lines = render_listing_lines(
        listing,
        &lines,
        line_offset,
        lang,
        is_source,
        ctx.document_attributes,
    );
    let rendered_content = rendered_lines.join("\n");

    html.push_str("<div class=\"content\">\n");
    if let Some(start) = listing.line_number {
        let numbered_lines = rendered_content.split('\n').collect::<Vec<_>>();
        html.push_str("<table class=\"linenotable\">\n<tbody>\n");
        for (offset, line) in numbered_lines.iter().enumerate() {
            let number = start.saturating_add(offset as u32);
            html.push_str("<tr>\n<td class=\"linenos\"><pre>");
            html.push_str(&number.to_string());
            html.push_str("</pre></td>\n<td class=\"code\">");
            if is_source {
                let l = lang.unwrap();
                html.push_str(&format!(
                    "<pre><code class=\"language-{l}\" data-lang=\"{l}\">{line}</code></pre>"
                ));
            } else {
                html.push_str("<pre>");
                html.push_str(line);
                html.push_str("</pre>");
            }
            html.push_str("</td>\n</tr>\n");
        }
        html.push_str("</tbody>\n</table>\n");
    } else if is_source {
        let l = lang.unwrap();
        html.push_str(&format!(
            "<pre class=\"highlight\"><code class=\"language-{l}\" data-lang=\"{l}\">"
        ));
        html.push_str(&rendered_content);
        html.push_str("</code></pre>\n");
    } else {
        html.push_str("<pre>");
        html.push_str(&rendered_content);
        html.push_str("</pre>\n");
    }
    html.push_str("</div>\n</div>\n");
}

fn render_passthrough(
    html: &mut String,
    passthrough: &crate::prepare::PassthroughBlock,
    document_attributes: &std::collections::BTreeMap<String, String>,
    ctx: &RenderContext<'_>,
) {
    if let Some(stem_style) = stem_style(passthrough.style.as_deref(), document_attributes) {
        html.push_str("<div");
        if let Some(id) = &passthrough.id {
            html.push_str(&format!(" id=\"{}\"", escape_html(id)));
        }
        html.push_str(" class=\"stemblock");
        if let Some(role) = &passthrough.role {
            html.push(' ');
            html.push_str(&escape_html(role));
        }
        html.push_str("\">\n");
        if let Some(title) = expanded_block_title(passthrough.title.as_deref(), ctx) {
            html.push_str(&format!(
                "<div class=\"title\">{}</div>\n",
                escape_html(&title)
            ));
        }
        html.push_str("<div class=\"content\">\n");
        html.push_str(&stem_equation(&passthrough.content, stem_style));
        html.push_str("\n</div>\n</div>\n");
    } else {
        html.push_str(
            &trimmed_delimited_content_lines(&passthrough.content)
                .1
                .join("\n"),
        );
        html.push('\n');
    }
}

fn render_listing_lines(
    listing: &crate::prepare::ListingBlock,
    lines: &[&str],
    line_offset: usize,
    lang: Option<&str>,
    is_source: bool,
    document_attributes: &std::collections::BTreeMap<String, String>,
) -> Vec<String> {
    let conum_map: std::collections::HashMap<usize, u32> =
        listing.callout_lines.iter().copied().collect();
    let highlighted_lines = if is_source {
        lang.and_then(|language| syntect_highlight_lines(language, lines, document_attributes))
    } else {
        None
    };

    lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let rendered = highlighted_lines
                .as_ref()
                .and_then(|rendered| rendered.get(i).cloned())
                .unwrap_or_else(|| escape_html(line));
            match conum_map.get(&(i + line_offset)) {
                Some(n) => {
                    format!("{rendered}<i class=\"conum\" data-value=\"{n}\"></i><b>{n}</b>")
                }
                None => rendered,
            }
        })
        .collect()
}

fn syntect_highlight_lines(
    language: &str,
    lines: &[&str],
    document_attributes: &std::collections::BTreeMap<String, String>,
) -> Option<Vec<String>> {
    if !uses_syntect(document_attributes) || language.eq_ignore_ascii_case("text") {
        return None;
    }

    let syntax_set = syntect_syntax_set();
    let syntax = syntect_syntax_for_language(syntax_set, language)?;
    let mut highlighter = HighlightLines::new(syntax, syntect_theme());

    let mut rendered = Vec::with_capacity(lines.len());
    for line in lines {
        let ranges = highlighter.highlight_line(line, syntax_set).ok()?;
        let html = styled_line_to_highlighted_html(&ranges, IncludeBackground::No).ok()?;
        rendered.push(html);
    }
    Some(rendered)
}

fn uses_syntect(document_attributes: &std::collections::BTreeMap<String, String>) -> bool {
    matches!(
        document_attributes
            .get("source-highlighter")
            .map(|value| value.trim().to_ascii_lowercase()),
        Some(value) if value == "syntect"
    )
}

fn syntect_syntax_set() -> &'static SyntaxSet {
    static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
    SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn syntect_theme() -> &'static Theme {
    static THEME_SET: OnceLock<ThemeSet> = OnceLock::new();
    static THEME_NAME: OnceLock<String> = OnceLock::new();

    let theme_set = THEME_SET.get_or_init(ThemeSet::load_defaults);
    let theme_name = THEME_NAME.get_or_init(|| {
        if theme_set.themes.contains_key("InspiredGitHub") {
            "InspiredGitHub".to_owned()
        } else {
            theme_set
                .themes
                .keys()
                .next()
                .cloned()
                .expect("syntect should provide at least one default theme")
        }
    });
    theme_set
        .themes
        .get(theme_name)
        .expect("selected syntect theme should exist")
}

fn syntect_syntax_for_language<'a>(
    syntax_set: &'a SyntaxSet,
    language: &str,
) -> Option<&'a SyntaxReference> {
    syntax_set
        .find_syntax_by_token(language)
        .or_else(|| syntax_set.find_syntax_by_extension(language))
}

fn stem_style<'a>(
    style: Option<&'a str>,
    document_attributes: &'a std::collections::BTreeMap<String, String>,
) -> Option<&'a str> {
    match style {
        Some("asciimath" | "latexmath") => style,
        Some("stem") => match document_attributes.get("stem").map(String::as_str) {
            Some("latexmath") => Some("latexmath"),
            _ => Some("asciimath"),
        },
        _ => None,
    }
}

fn stem_equation(content: &str, stem_style: &str) -> String {
    let equation = trimmed_delimited_content_lines(content).1.join("\n");
    let (open, close) = match stem_style {
        "latexmath" => (r"\[", r"\]"),
        _ => (r"\$", r"\$"),
    };
    if equation.starts_with(open) && equation.ends_with(close) {
        equation
    } else {
        format!("{open}{equation}{close}")
    }
}

fn render_callout_list(html: &mut String, colist: &crate::prepare::CalloutListBlock) {
    html.push_str("<div class=\"colist arabic\">\n<table>\n<tbody>\n");
    for item in &colist.items {
        let n = item.number;
        html.push_str(&format!(
            "<tr>\n<td><i class=\"conum\" data-value=\"{n}\"></i><b>{n}</b></td>\n<td>"
        ));
        render_inlines(html, &item.inlines);
        html.push_str("</td>\n</tr>\n");
    }
    html.push_str("</tbody>\n</table>\n</div>\n");
}

fn render_literal(
    html: &mut String,
    literal: &crate::prepare::ListingBlock,
    ctx: &RenderContext<'_>,
) {
    html.push_str("<div class=\"literalblock\"");
    if let Some(id) = &literal.id {
        html.push_str(&format!(" id=\"{}\"", escape_html(id)));
    }
    html.push_str(">\n");
    if let Some(title) = expanded_block_title(literal.title.as_deref(), ctx) {
        html.push_str(&format!(
            "<div class=\"title\">{}</div>\n",
            escape_html(&title)
        ));
    }
    html.push_str("<div class=\"content\">\n<pre>");
    html.push_str(&escape_html(
        &trimmed_delimited_content_lines(&literal.content)
            .1
            .join("\n"),
    ));
    html.push_str("</pre>\n</div>\n</div>\n");
}

fn render_compound(
    html: &mut String,
    class_name: &str,
    block: &crate::prepare::CompoundBlock,
    ctx: &RenderContext<'_>,
    caption_kind: Option<CaptionKind>,
) {
    html.push_str(&format!("<div class=\"{class_name}\""));
    if let Some(id) = &block.id {
        html.push_str(&format!(" id=\"{}\"", escape_html(id)));
    }
    html.push_str(">\n");
    let title = match caption_kind {
        Some(kind) => captioned_block_title(block.title.as_deref(), &block.attributes, ctx, kind),
        None => expanded_block_title(block.title.as_deref(), ctx),
    };
    if let Some(title) = title {
        html.push_str(&format!(
            "<div class=\"title\">{}</div>\n",
            escape_html(&title)
        ));
    }
    html.push_str("<div class=\"content\">\n");
    for child in &block.blocks {
        render_block(html, child, ctx);
    }
    html.push_str("</div>\n</div>\n");
}

fn render_sidebar(
    html: &mut String,
    block: &crate::prepare::CompoundBlock,
    ctx: &RenderContext<'_>,
) {
    html.push_str("<div class=\"sidebarblock\"");
    if let Some(id) = &block.id {
        html.push_str(&format!(" id=\"{}\"", escape_html(id)));
    }
    html.push_str(">\n<div class=\"content\">\n");
    if let Some(title) = expanded_block_title(block.title.as_deref(), ctx) {
        html.push_str(&format!(
            "<div class=\"title\">{}</div>\n",
            escape_html(&title)
        ));
    }
    for child in &block.blocks {
        render_block(html, child, ctx);
    }
    html.push_str("</div>\n</div>\n");
}

fn render_open(html: &mut String, block: &crate::prepare::CompoundBlock, ctx: &RenderContext<'_>) {
    html.push_str("<div class=\"openblock\"");
    if let Some(id) = &block.id {
        html.push_str(&format!(" id=\"{}\"", escape_html(id)));
    }
    html.push_str(">\n");
    if let Some(title) = expanded_block_title(block.title.as_deref(), ctx) {
        html.push_str(&format!(
            "<div class=\"title\">{}</div>\n",
            escape_html(&title)
        ));
    }
    html.push_str("<div class=\"content\">\n");
    for child in &block.blocks {
        render_block(html, child, ctx);
    }
    html.push_str("</div>\n</div>\n");
}

fn render_quote(html: &mut String, block: &crate::prepare::QuoteBlock, ctx: &RenderContext<'_>) {
    let div_class = if block.is_verse {
        "verseblock"
    } else {
        "quoteblock"
    };
    html.push_str(&format!("<div class=\"{div_class}\""));
    if let Some(id) = &block.id {
        html.push_str(&format!(" id=\"{}\"", escape_html(id)));
    }
    html.push_str(">\n");
    if let Some(title) = expanded_block_title(block.title.as_deref(), ctx) {
        html.push_str(&format!(
            "<div class=\"title\">{}</div>\n",
            escape_html(&title)
        ));
    }
    if block.is_verse {
        html.push_str("<pre class=\"content\">");
        html.push_str(&escape_html(
            &trimmed_delimited_content_lines(&block.content).1.join("\n"),
        ));
        html.push_str("</pre>\n");
    } else {
        html.push_str("<blockquote>\n");
        for child in &block.blocks {
            render_block(html, child, ctx);
        }
        html.push_str("</blockquote>\n");
    }
    if block.attribution.is_some() || block.citetitle.is_some() {
        html.push_str("<div class=\"attribution\">\n");
        html.push_str("&#8212; ");
        if let Some(attribution) = &block.attribution {
            html.push_str(&escape_html(attribution));
        }
        if let Some(citetitle) = &block.citetitle {
            html.push_str(&format!("<br>\n<cite>{}</cite>\n", escape_html(citetitle)));
        }
        html.push_str("</div>\n");
    }
    html.push_str("</div>\n");
}

fn render_paragraph(
    html: &mut String,
    paragraph: &crate::prepare::ParagraphBlock,
    ctx: &RenderContext<'_>,
) {
    html.push_str("<div class=\"paragraph\"");
    if let Some(id) = &paragraph.id {
        html.push_str(&format!(" id=\"{}\"", escape_html(id)));
    }
    html.push_str(">\n");
    if let Some(title) = expanded_block_title(paragraph.title.as_deref(), ctx) {
        html.push_str(&format!(
            "<div class=\"title\">{}</div>\n",
            escape_html(&title)
        ));
    }
    html.push_str("<p>");
    render_inlines(html, &paragraph.inlines);
    html.push_str("</p>\n</div>\n");
}

fn render_admonition(
    html: &mut String,
    admonition: &crate::prepare::AdmonitionBlock,
    ctx: &RenderContext<'_>,
) {
    let label = admonition_label(
        &admonition.variant,
        &admonition.attributes,
        ctx.document_attributes,
    );
    html.push_str(&format!(
        "<div class=\"admonitionblock {}\"",
        escape_html(&admonition.variant)
    ));
    if let Some(id) = &admonition.id {
        html.push_str(&format!(" id=\"{}\"", escape_html(id)));
    }
    html.push_str(">\n<table>\n<tr>\n<td class=\"icon\">\n");
    if let Some(font_icon_class) = admonition_font_icon_class(
        &admonition.variant,
        &admonition.attributes,
        ctx.document_attributes,
    ) {
        html.push_str(&format!(
            "<i class=\"fa {}\" title=\"{}\"></i>\n",
            escape_html(font_icon_class),
            escape_html(label)
        ));
    } else if let Some(icon_target) = admonition_icon_target(
        &admonition.variant,
        &admonition.attributes,
        ctx.document_attributes,
    ) {
        html.push_str(&format!(
            "<img src=\"{}\" alt=\"{}\">\n",
            escape_html(&icon_target),
            escape_html(label)
        ));
    } else {
        html.push_str(&format!(
            "<div class=\"title\">{}</div>\n",
            escape_html(label)
        ));
    }
    html.push_str("</td>\n<td class=\"content\">\n");
    if let Some(title) = expanded_block_title(admonition.title.as_deref(), ctx) {
        html.push_str(&format!(
            "<div class=\"title\">{}</div>\n",
            escape_html(&title)
        ));
    }
    for block in &admonition.blocks {
        render_block(html, block, ctx);
    }
    html.push_str("</td>\n</tr>\n</table>\n</div>\n");
}

fn admonition_label<'a>(
    variant: &'a str,
    block_attributes: &'a std::collections::BTreeMap<String, String>,
    document_attributes: &'a std::collections::BTreeMap<String, String>,
) -> &'a str {
    if let Some(caption) = block_attributes
        .get("caption")
        .filter(|caption| !caption.is_empty())
    {
        return caption;
    }
    let key = format!("{variant}-caption");
    if let Some(caption) = document_attributes
        .get(&key)
        .filter(|caption| !caption.is_empty())
    {
        return caption;
    }
    match variant {
        "note" => "Note",
        "tip" => "Tip",
        "important" => "Important",
        "caution" => "Caution",
        "warning" => "Warning",
        _ => variant,
    }
}

fn admonition_font_icon_class<'a>(
    variant: &'a str,
    block_attributes: &'a std::collections::BTreeMap<String, String>,
    document_attributes: &'a std::collections::BTreeMap<String, String>,
) -> Option<&'a str> {
    let icons = named_attribute(block_attributes, document_attributes, "icons")?;
    if icons == "font" {
        Some(match variant {
            "note" => "icon-note",
            "tip" => "icon-tip",
            "important" => "icon-important",
            "caution" => "icon-caution",
            "warning" => "icon-warning",
            _ => "icon-note",
        })
    } else {
        None
    }
}

fn admonition_icon_target(
    variant: &str,
    block_attributes: &std::collections::BTreeMap<String, String>,
    document_attributes: &std::collections::BTreeMap<String, String>,
) -> Option<String> {
    let icons = named_attribute(block_attributes, document_attributes, "icons")?;
    if icons == "font" {
        return None;
    }

    let icon_name = named_attribute(block_attributes, document_attributes, "icon")
        .filter(|icon| !icon.is_empty())
        .unwrap_or(variant);
    let iconsdir = named_attribute(block_attributes, document_attributes, "iconsdir")
        .filter(|iconsdir| !iconsdir.is_empty())
        .unwrap_or("./images/icons");
    let separator = if iconsdir.ends_with('/') || iconsdir.ends_with('\\') {
        ""
    } else {
        "/"
    };

    if icon_name_has_extension(icon_name) {
        return Some(format!("{iconsdir}{separator}{icon_name}"));
    }

    let extension = named_attribute(block_attributes, document_attributes, "icontype")
        .filter(|icontype| !icontype.is_empty())
        .or_else(|| match icons {
            "" | "image" => None,
            other => Some(other),
        })
        .unwrap_or("png");

    Some(format!("{iconsdir}{separator}{icon_name}.{extension}"))
}

fn named_attribute<'a>(
    block_attributes: &'a std::collections::BTreeMap<String, String>,
    document_attributes: &'a std::collections::BTreeMap<String, String>,
    name: &str,
) -> Option<&'a str> {
    block_attributes
        .get(name)
        .map(String::as_str)
        .or_else(|| document_attributes.get(name).map(String::as_str))
}

fn icon_name_has_extension(icon_name: &str) -> bool {
    let file_name = icon_name.rsplit(['/', '\\']).next().unwrap_or(icon_name);
    file_name.contains('.')
}

fn render_image_block(
    html: &mut String,
    image: &crate::prepare::ImageBlock,
    ctx: &RenderContext<'_>,
) {
    let document_attributes = ctx.document_attributes;
    let mut classes = vec!["imageblock".to_owned()];
    if let Some(float) = &image.float {
        classes.push(float.clone());
    }
    if let Some(align) = &image.align {
        classes.push(format!("text-{}", align));
    }
    if let Some(role) = &image.role {
        classes.push(role.clone());
    }

    html.push_str(&format!("<div class=\"{}\"", classes.join(" ")));
    if let Some(id) = &image.id {
        html.push_str(&format!(" id=\"{}\"", escape_html(id)));
    }
    html.push_str(">\n<div class=\"content\">\n");

    let src = resolve_image_src(&image.target, document_attributes);

    let link = image.link.as_deref().map(|l| {
        if l == "self" {
            src.clone()
        } else {
            l.to_owned()
        }
    });

    if let Some(href) = &link {
        html.push_str(&format!(
            "<a class=\"image\" href=\"{}\">",
            escape_html(href)
        ));
    }

    html.push_str(&format!(
        "<img src=\"{}\" alt=\"{}\"",
        escape_html(&src),
        escape_html(&image.alt)
    ));
    if let Some(width) = &image.width {
        html.push_str(&format!(" width=\"{}\"", escape_html(width)));
    }
    if let Some(height) = &image.height {
        html.push_str(&format!(" height=\"{}\"", escape_html(height)));
    }
    html.push_str(">");

    if link.is_some() {
        html.push_str("</a>");
    }

    html.push_str("\n</div>\n");

    if let Some(title) = captioned_block_title(
        image.title.as_deref(),
        &image.attributes,
        ctx,
        CaptionKind::Image,
    ) {
        html.push_str(&format!(
            "<div class=\"title\">{}</div>\n",
            escape_html(&title)
        ));
    }

    html.push_str("</div>\n");
}

fn captioned_block_title(
    title: Option<&str>,
    block_attributes: &std::collections::BTreeMap<String, String>,
    ctx: &RenderContext<'_>,
    kind: CaptionKind,
) -> Option<String> {
    let title = expand_counter_macros(title?, ctx);
    if let Some(caption) = block_attributes
        .get("caption")
        .filter(|caption| !caption.is_empty())
    {
        return Some(format!("{}{title}", expand_counter_macros(caption, ctx)));
    }

    let Some(label) = caption_label(kind, ctx.document_attributes) else {
        return Some(title.to_owned());
    };
    let number = next_counter_value(
        ctx,
        counter_attribute_name(kind),
        ctx.document_attributes
            .get(counter_attribute_name(kind))
            .map(String::as_str),
    );
    Some(format!("{label} {number}. {title}"))
}

fn expanded_block_title(title: Option<&str>, ctx: &RenderContext<'_>) -> Option<String> {
    title.map(|title| expand_counter_macros(title, ctx))
}

fn caption_label(
    kind: CaptionKind,
    document_attributes: &std::collections::BTreeMap<String, String>,
) -> Option<String> {
    let key = caption_attribute_name(kind);
    if let Some(caption) = document_attributes
        .get(key)
        .filter(|caption| !caption.is_empty())
    {
        return Some(caption.trim_end().to_owned());
    }

    match kind {
        CaptionKind::Example => Some("Example".into()),
        CaptionKind::Table => Some("Table".into()),
        CaptionKind::Image => Some("Figure".into()),
        CaptionKind::Listing => None,
    }
}

fn caption_attribute_name(kind: CaptionKind) -> &'static str {
    match kind {
        CaptionKind::Example => "example-caption",
        CaptionKind::Listing => "listing-caption",
        CaptionKind::Table => "table-caption",
        CaptionKind::Image => "figure-caption",
    }
}

fn counter_attribute_name(kind: CaptionKind) -> &'static str {
    match kind {
        CaptionKind::Example => "example-number",
        CaptionKind::Listing => "listing-number",
        CaptionKind::Table => "table-number",
        CaptionKind::Image => "figure-number",
    }
}

fn expand_counter_macros(input: &str, ctx: &RenderContext<'_>) -> String {
    let mut output = String::new();
    let mut cursor = 0;

    while let Some(start) = input[cursor..].find("{counter:") {
        let start = cursor + start;
        output.push_str(&input[cursor..start]);

        let macro_start = start + "{counter:".len();
        let Some(end) = input[macro_start..].find('}') else {
            output.push_str(&input[start..]);
            return output;
        };
        let end = macro_start + end;
        let body = &input[macro_start..end];
        let mut parts = body.splitn(2, ':');
        let name = parts.next().unwrap_or("").trim();
        if name.is_empty() {
            output.push_str(&input[start..=end]);
            cursor = end + 1;
            continue;
        }

        let seed = parts.next().map(str::trim).filter(|seed| !seed.is_empty());
        output.push_str(&next_counter_value(ctx, name, seed));
        cursor = end + 1;
    }

    output.push_str(&input[cursor..]);
    output
}

fn next_counter_value(ctx: &RenderContext<'_>, counter_name: &str, seed: Option<&str>) -> String {
    let mut counters = ctx.caption_counters.borrow_mut();
    if let Some(counter) = counters.get_mut(counter_name) {
        let current = counter.display();
        counter.increment();
        return current;
    }

    let mut counter = seed
        .and_then(parse_counter_seed)
        .unwrap_or(CounterState::Numeric(1));
    let current = counter.display();
    counter.increment();
    counters.insert(counter_name.to_owned(), counter);
    current
}

fn parse_counter_seed(seed: &str) -> Option<CounterState> {
    if let Ok(value) = seed.parse::<u32>() {
        return (value > 0).then_some(CounterState::Numeric(value));
    }

    let uppercase = seed.chars().all(|ch| ch.is_ascii_uppercase());
    let lowercase = seed.chars().all(|ch| ch.is_ascii_lowercase());
    if !(uppercase || lowercase) {
        return None;
    }

    alphabetic_to_index(seed).map(|index| CounterState::Alpha { index, uppercase })
}

fn alphabetic_to_index(seed: &str) -> Option<u32> {
    let mut value = 0u32;
    for ch in seed.chars() {
        let digit = match ch {
            'A'..='Z' => ch as u32 - 'A' as u32 + 1,
            'a'..='z' => ch as u32 - 'a' as u32 + 1,
            _ => return None,
        };
        value = value.checked_mul(26)?.checked_add(digit)?;
    }
    Some(value)
}

fn index_to_alphabetic(mut index: u32, uppercase: bool) -> String {
    let base = if uppercase { b'A' } else { b'a' };
    let mut chars = Vec::new();
    while index > 0 {
        index -= 1;
        chars.push((base + (index % 26) as u8) as char);
        index /= 26;
    }
    chars.into_iter().rev().collect()
}

impl CounterState {
    fn display(&self) -> String {
        match self {
            Self::Numeric(value) => value.to_string(),
            Self::Alpha { index, uppercase } => index_to_alphabetic(*index, *uppercase),
        }
    }

    fn increment(&mut self) {
        match self {
            Self::Numeric(value) => *value = value.saturating_add(1),
            Self::Alpha { index, .. } => *index = index.saturating_add(1),
        }
    }
}

fn resolve_image_src(
    target: &str,
    document_attributes: &std::collections::BTreeMap<String, String>,
) -> String {
    if target.starts_with("http://")
        || target.starts_with("https://")
        || target.starts_with("data:")
        || target.starts_with('/')
    {
        return target.to_owned();
    }

    if let Some(imagesdir) = document_attributes
        .get("imagesdir")
        .filter(|d| !d.is_empty())
    {
        let dir = imagesdir.trim_end_matches('/');
        format!("{}/{}", dir, target)
    } else {
        target.to_owned()
    }
}

fn render_inlines(html: &mut String, inlines: &[PreparedInline]) {
    for inline in inlines {
        match inline {
            PreparedInline::Text(text) => html.push_str(&escape_html(&text.value)),
            PreparedInline::Span(span) => {
                let tag = match span.variant.as_str() {
                    "strong" => "strong",
                    "emphasis" => "em",
                    "monospace" => "code",
                    "subscript" => "sub",
                    "superscript" => "sup",
                    _ => "span",
                };
                html.push_str(&format!("<{tag}>"));
                render_inlines(html, &span.inlines);
                html.push_str(&format!("</{tag}>"));
            }
            PreparedInline::Link(link) => {
                html.push_str(&format!("<a href=\"{}\"", escape_html(&link.target)));
                if link.bare {
                    html.push_str(" class=\"bare\"");
                }
                if let Some(window) = &link.window {
                    html.push_str(&format!(" target=\"{}\"", escape_html(window)));
                    if window == "_blank" {
                        html.push_str(" rel=\"noopener\"");
                    }
                }
                html.push('>');
                render_inlines(html, &link.inlines);
                html.push_str("</a>");
            }
            PreparedInline::Xref(xref) => {
                html.push_str(&format!("<a href=\"{}\">", escape_html(&xref.href)));
                render_inlines(html, &xref.inlines);
                html.push_str("</a>");
            }
            PreparedInline::Anchor(anchor) => {
                html.push_str(&format!("<a id=\"{}\"></a>", escape_html(&anchor.id)));
                render_inlines(html, &anchor.inlines);
            }
            PreparedInline::Passthrough(p) => html.push_str(&p.value),
            PreparedInline::Icon(icon) => {
                let mut classes = format!("fa fa-{}", escape_html(&icon.name));
                // Append Font Awesome size modifier if provided
                if let Some(size) = &icon.size {
                    classes.push(' ');
                    classes.push_str(&format!("fa-{}", escape_html(size)));
                }
                // Append role as additional CSS classes
                if let Some(role) = &icon.role {
                    classes.push(' ');
                    classes.push_str(&escape_html(role));
                }
                let title_attr = icon
                    .title
                    .as_deref()
                    .map(|t| format!(" title=\"{}\"", escape_html(t)))
                    .unwrap_or_default();
                html.push_str("<span class=\"icon\">");
                html.push_str(&format!("<i class=\"{classes}\"{title_attr}></i>"));
                html.push_str("</span>");
            }
            PreparedInline::Image(image) => {
                html.push_str("<span class=\"image\">");
                html.push_str(&format!(
                    "<img src=\"{}\" alt=\"{}\"",
                    escape_html(&image.target),
                    escape_html(&image.alt)
                ));
                if let Some(width) = &image.width {
                    html.push_str(&format!(" width=\"{}\"", escape_html(width)));
                }
                if let Some(height) = &image.height {
                    html.push_str(&format!(" height=\"{}\"", escape_html(height)));
                }
                html.push_str(">");
                html.push_str("</span>");
            }
            PreparedInline::Footnote(footnote) => {
                let index = footnote.index.unwrap_or(0);
                html.push_str(&format!(
                    "<sup class=\"footnote\" id=\"_footnoteref_{index}\"><a href=\"#_footnotedef_{index}\">{index}</a></sup>"
                ));
            }
        }
    }
}

fn render_footnotes(html: &mut String, footnotes: &[crate::prepare::Footnote]) {
    if footnotes.is_empty() {
        return;
    }

    html.push_str("<div id=\"footnotes\">\n<hr>\n");
    for footnote in footnotes {
        let index = footnote.index.unwrap_or(0);
        html.push_str(&format!(
            "<div class=\"footnote\" id=\"_footnotedef_{index}\"><a href=\"#_footnoteref_{index}\">{index}</a>. "
        ));
        render_inlines(html, &footnote.inlines);
        html.push_str("</div>\n");
    }
    html.push_str("</div>\n");
}

fn escape_html(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());

    for ch in input.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(ch),
        }
    }

    escaped
}

#[cfg(test)]
mod tests {
    use crate::ast::{
        Block, BlockMetadata, CompoundBlock, Document, Heading, Inline, InlineAnchor,
        InlineFootnote, InlineForm, InlineLink, InlineSpan, InlineVariant, InlineXref, ListItem,
        Listing, OrderedList, Paragraph, UnorderedList,
    };
    use crate::prepare::prepare_document;
    use crate::render::render_html;

    #[test]
    fn renders_document_title_sections_and_paragraphs() {
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
                Block::Heading(Heading {
                    level: 1,
                    title: "Section One".into(),
                    id: None,
                    reftext: None,
                    metadata: BlockMetadata::default(),
                }),
                Block::Paragraph(Paragraph {
                    inlines: vec![Inline::Text("first line\nsecond line".into())],
                    lines: vec!["first line".into(), "second line".into()],
                    id: None,
                    reftext: None,
                    metadata: BlockMetadata::default(),
                }),
            ],
        };

        let html = render_html(&document);

        assert_eq!(
            html,
            concat!(
                "<div id=\"header\">\n",
                "<h1>Document Title</h1>\n",
                "</div>\n",
                "<div id=\"content\">\n",
                "<div class=\"sect1\" id=\"_section_one\">\n",
                "<h2>Section One</h2>\n",
                "<div class=\"sectionbody\">\n",
                "<div class=\"paragraph\">\n",
                "<p>first line\nsecond line</p>\n",
                "</div>\n",
                "</div>\n",
                "</div>\n",
                "</div>\n"
            )
        );
    }

    #[test]
    fn escapes_html_in_text_nodes() {
        let document = Document {
            attributes: Default::default(),
            title: Some(Heading {
                level: 0,
                title: "Fish & Chips".into(),
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
            }),
            blocks: vec![Block::Paragraph(Paragraph {
                inlines: vec![Inline::Text("<tag> \"quoted\" and 'apostrophe'".into())],
                lines: vec!["<tag> \"quoted\" and 'apostrophe'".into()],
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
            })],
        };

        let html = render_html(&document);

        assert!(html.contains("<h1>Fish &amp; Chips</h1>"));
        assert!(html.contains("<p>&lt;tag&gt; &quot;quoted&quot; and &#39;apostrophe&#39;</p>"));
    }

    #[test]
    fn rendering_prepared_document_keeps_nested_sections() {
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
                    title: "Section A".into(),
                    id: None,
                    reftext: None,
                    metadata: BlockMetadata::default(),
                }),
                Block::Heading(Heading {
                    level: 2,
                    title: "Section B".into(),
                    id: None,
                    reftext: None,
                    metadata: BlockMetadata::default(),
                }),
            ],
        };

        let prepared = prepare_document(&document);
        let html = super::render_prepared_html(&prepared);

        assert!(html.contains("<div class=\"sect1\" id=\"_section_a\">"));
        assert!(html.contains("<div class=\"sect2\" id=\"_section_b\">"));
        assert!(html.contains("<h3>Section B</h3>"));
    }

    #[test]
    fn renders_strong_and_emphasis_inline_markup() {
        let document = Document {
            attributes: Default::default(),
            title: None,
            blocks: vec![Block::Paragraph(Paragraph {
                lines: vec!["before *strong* and _emphasis_ after".into()],
                inlines: vec![
                    Inline::Text("before ".into()),
                    Inline::Span(InlineSpan {
                        variant: InlineVariant::Strong,
                        form: InlineForm::Constrained,
                        inlines: vec![Inline::Text("strong".into())],
                    }),
                    Inline::Text(" and ".into()),
                    Inline::Span(InlineSpan {
                        variant: InlineVariant::Emphasis,
                        form: InlineForm::Constrained,
                        inlines: vec![Inline::Text("emphasis".into())],
                    }),
                    Inline::Text(" after".into()),
                ],
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
            })],
        };

        let html = render_html(&document);

        assert!(html.contains("<p>before <strong>strong</strong> and <em>emphasis</em> after</p>"));
    }

    #[test]
    fn renders_monospace_inline_markup() {
        let document = Document {
            attributes: Default::default(),
            title: None,
            blocks: vec![Block::Paragraph(Paragraph {
                lines: vec!["Run `cargo test` now".into()],
                inlines: vec![
                    Inline::Text("Run ".into()),
                    Inline::Span(InlineSpan {
                        variant: InlineVariant::Monospace,
                        form: InlineForm::Constrained,
                        inlines: vec![Inline::Text("cargo test".into())],
                    }),
                    Inline::Text(" now".into()),
                ],
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
            })],
        };

        let html = render_html(&document);
        assert!(html.contains("<p>Run <code>cargo test</code> now</p>"));
    }

    #[test]
    fn renders_escaped_markup_as_literal_text() {
        let document = Document {
            attributes: Default::default(),
            title: None,
            blocks: vec![Block::Paragraph(Paragraph {
                lines: vec![r"\*not strong* and \_not emphasis_".into()],
                inlines: vec![Inline::Text("*not strong* and _not emphasis_".into())],
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
            })],
        };

        let html = render_html(&document);

        assert!(html.contains("<p>*not strong* and _not emphasis_</p>"));
    }

    #[test]
    fn renders_links() {
        let document = Document {
            attributes: Default::default(),
            title: None,
            blocks: vec![Block::Paragraph(Paragraph {
                lines: vec!["See https://example.org[example] and http://foo.com".into()],
                inlines: vec![
                    Inline::Text("See ".into()),
                    Inline::Link(InlineLink {
                        target: "https://example.org".into(),
                        text: vec![Inline::Text("example".into())],
                        bare: false,
                        window: None,
                    }),
                    Inline::Text(" and ".into()),
                    Inline::Link(InlineLink {
                        target: "http://foo.com".into(),
                        text: vec![Inline::Text("http://foo.com".into())],
                        bare: true,
                        window: None,
                    }),
                ],
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
            })],
        };

        let html = render_html(&document);

        assert!(html.contains("<a href=\"https://example.org\">example</a>"));
        assert!(html.contains("<a href=\"http://foo.com\" class=\"bare\">http://foo.com</a>"));
    }

    #[test]
    fn renders_links_with_window_targets() {
        let document = Document {
            attributes: Default::default(),
            title: None,
            blocks: vec![Block::Paragraph(Paragraph {
                lines: vec!["See https://example.org[example^]".into()],
                inlines: vec![Inline::Link(InlineLink {
                    target: "https://example.org".into(),
                    text: vec![Inline::Text("example".into())],
                    bare: false,
                    window: Some("_blank".into()),
                })],
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
            })],
        };

        let html = render_html(&document);
        assert!(html.contains(
            "<a href=\"https://example.org\" target=\"_blank\" rel=\"noopener\">example</a>"
        ));
    }

    #[test]
    fn renders_xrefs() {
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

        let html = render_html(&document);
        assert!(html.contains("<a href=\"#install\">Installation</a>"));
    }

    #[test]
    fn renders_xrefs_with_resolved_section_ids() {
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

        let html = render_html(&document);
        assert!(html.contains("<a href=\"#_first_section\">First Section</a>"));
    }

    #[test]
    fn renders_paragraph_anchor_ids() {
        let document = Document {
            attributes: Default::default(),
            title: None,
            blocks: vec![Block::Paragraph(Paragraph {
                lines: vec!["Hello".into()],
                inlines: vec![Inline::Text("Hello".into())],
                id: Some("intro".into()),
                reftext: Some("Introduction".into()),
                metadata: BlockMetadata::default(),
            })],
        };

        let html = render_html(&document);
        assert!(html.contains("<div class=\"paragraph\" id=\"intro\">"));
    }

    #[test]
    fn renders_inline_anchor_points() {
        let document = Document {
            attributes: Default::default(),
            title: None,
            blocks: vec![Block::Paragraph(Paragraph {
                lines: vec!["[[bookmark-a]]look here".into()],
                inlines: vec![
                    Inline::Anchor(InlineAnchor {
                        id: "bookmark-a".into(),
                        reftext: None,
                        inlines: Vec::new(),
                    }),
                    Inline::Text("look here".into()),
                ],
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
            })],
        };

        let html = render_html(&document);
        assert!(html.contains("<a id=\"bookmark-a\"></a>look here"));
    }

    #[test]
    fn renders_phrase_applied_inline_anchor_text() {
        let document = Document {
            attributes: Default::default(),
            title: None,
            blocks: vec![Block::Paragraph(Paragraph {
                lines: vec!["[#bookmark-b]#visible text#".into()],
                inlines: vec![Inline::Anchor(InlineAnchor {
                    id: "bookmark-b".into(),
                    reftext: None,
                    inlines: vec![Inline::Text("visible text".into())],
                })],
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
            })],
        };

        let html = render_html(&document);
        assert!(html.contains("<a id=\"bookmark-b\"></a>visible text"));
    }

    #[test]
    fn renders_empty_header_div_when_no_title() {
        let document = Document {
            attributes: Default::default(),
            title: None,
            blocks: vec![Block::Paragraph(Paragraph {
                lines: vec!["hello".into()],
                inlines: vec![Inline::Text("hello".into())],
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
            })],
        };

        let html = render_html(&document);

        assert!(html.contains("<div id=\"header\">\n</div>"));
        assert!(!html.contains("<h1>"));
    }

    #[test]
    fn renders_preamble_with_correct_html_structure() {
        let document = Document {
            attributes: Default::default(),
            title: Some(Heading {
                level: 0,
                title: "My Doc".into(),
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
            }),
            blocks: vec![
                Block::Paragraph(Paragraph {
                    lines: vec!["Intro paragraph.".into()],
                    inlines: vec![Inline::Text("Intro paragraph.".into())],
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

        let html = render_html(&document);

        assert!(html.contains("<div id=\"preamble\">"));
        assert!(html.contains("<div class=\"sectionbody\">"));
        assert!(html.contains("<p>Intro paragraph.</p>"));
        // preamble closes before the section starts
        let preamble_end = html.find("</div>\n</div>").unwrap();
        let section_start = html.find("<div class=\"sect1\"").unwrap();
        assert!(preamble_end < section_start);
    }

    #[test]
    fn renders_ordered_lists() {
        let document = Document {
            attributes: Default::default(),
            title: None,
            blocks: vec![Block::OrderedList(OrderedList {
                items: vec![
                    ListItem {
                        blocks: vec![Block::Paragraph(Paragraph {
                            lines: vec!["first item".into()],
                            inlines: vec![Inline::Text("first item".into())],
                            id: None,
                            reftext: None,
                            metadata: BlockMetadata::default(),
                        })],
                    },
                    ListItem {
                        blocks: vec![Block::Paragraph(Paragraph {
                            lines: vec!["second item".into()],
                            inlines: vec![Inline::Text("second item".into())],
                            id: None,
                            reftext: None,
                            metadata: BlockMetadata::default(),
                        })],
                    },
                ],
                reftext: None,
                metadata: BlockMetadata::default(),
            })],
        };

        let html = render_html(&document);
        assert!(html.contains("<div class=\"olist arabic\">"));
        assert!(html.contains("<ol class=\"arabic\">"));
        assert!(html.contains("<li>"));
        assert!(html.contains("<p>first item</p>"));
        assert!(html.contains("<p>second item</p>"));
    }

    #[test]
    fn renders_description_lists() {
        let html = render_html(&crate::parser::parse_document(
            "Term:: Definition\nSecond::\nThird::\nMore definition",
        ));
        println!("HTML: {}", html);

        assert!(html.contains("<div class=\"dlist\">"));
        assert!(html.contains("<dt class=\"hdlist1\">Term</dt>"));
        assert!(html.contains("<p>Definition</p>"));
        assert!(html.contains("<dt class=\"hdlist1\">Second</dt>"));
        assert!(html.contains("<dt class=\"hdlist1\">Third</dt>"));
        assert!(html.contains("<p>More definition</p>"));
    }

    #[test]
    fn renders_unordered_lists() {
        let document = Document {
            attributes: Default::default(),
            title: None,
            blocks: vec![Block::UnorderedList(UnorderedList {
                items: vec![ListItem {
                    blocks: vec![Block::Paragraph(Paragraph {
                        lines: vec!["first item".into()],
                        inlines: vec![Inline::Text("first item".into())],
                        id: None,
                        reftext: None,
                        metadata: BlockMetadata::default(),
                    })],
                }],
                reftext: None,
                metadata: BlockMetadata::default(),
            })],
        };

        let html = render_html(&document);
        assert!(html.contains("<div class=\"ulist\">"));
        assert!(html.contains("<ul>"));
        assert!(html.contains("<li>"));
        assert!(html.contains("<p>first item</p>"));
    }

    #[test]
    fn renders_nested_lists_and_item_continuations() {
        let document = Document {
            attributes: Default::default(),
            title: None,
            blocks: vec![Block::OrderedList(OrderedList {
                items: vec![
                    ListItem {
                        blocks: vec![
                            Block::Paragraph(Paragraph {
                                lines: vec!["first item".into()],
                                inlines: vec![Inline::Text("first item".into())],
                                id: None,
                                reftext: None,
                                metadata: BlockMetadata::default(),
                            }),
                            Block::UnorderedList(UnorderedList {
                                items: vec![ListItem {
                                    blocks: vec![Block::Paragraph(Paragraph {
                                        lines: vec!["nested item".into()],
                                        inlines: vec![Inline::Text("nested item".into())],
                                        id: None,
                                        reftext: None,
                                        metadata: BlockMetadata::default(),
                                    })],
                                }],
                                reftext: None,
                                metadata: BlockMetadata::default(),
                            }),
                            Block::Paragraph(Paragraph {
                                lines: vec!["continued paragraph".into()],
                                inlines: vec![Inline::Text("continued paragraph".into())],
                                id: None,
                                reftext: None,
                                metadata: BlockMetadata::default(),
                            }),
                        ],
                    },
                    ListItem {
                        blocks: vec![Block::Paragraph(Paragraph {
                            lines: vec!["second item".into()],
                            inlines: vec![Inline::Text("second item".into())],
                            id: None,
                            reftext: None,
                            metadata: BlockMetadata::default(),
                        })],
                    },
                ],
                reftext: None,
                metadata: BlockMetadata::default(),
            })],
        };

        let html = render_html(&document);

        assert!(html.contains("<div class=\"olist arabic\">"));
        assert!(html.contains("<div class=\"ulist\">"));
        assert!(html.contains("<p>nested item</p>"));
        assert!(html.contains("<p>continued paragraph</p>"));
        assert!(html.contains("<p>second item</p>"));
    }

    #[test]
    fn renders_delimited_blocks() {
        let document = Document {
            attributes: Default::default(),
            title: None,
            blocks: vec![
                Block::Listing(Listing {
                    lines: vec!["puts 'hello'".into()],
                    callouts: vec![],
                    reftext: None,
                    metadata: BlockMetadata::default(),
                }),
                Block::Sidebar(CompoundBlock {
                    blocks: vec![Block::Paragraph(Paragraph {
                        lines: vec!["inside sidebar".into()],
                        inlines: vec![Inline::Text("inside sidebar".into())],
                        id: None,
                        reftext: None,
                        metadata: BlockMetadata::default(),
                    })],
                    reftext: None,
                    metadata: BlockMetadata::default(),
                }),
                Block::Example(CompoundBlock {
                    blocks: vec![Block::Paragraph(Paragraph {
                        lines: vec!["inside example".into()],
                        inlines: vec![Inline::Text("inside example".into())],
                        id: None,
                        reftext: None,
                        metadata: BlockMetadata::default(),
                    })],
                    reftext: None,
                    metadata: BlockMetadata::default(),
                }),
            ],
        };

        let html = render_html(&document);

        assert!(html.contains("<div class=\"listingblock\">"));
        assert!(html.contains("<pre>puts &#39;hello&#39;</pre>"));
        assert!(html.contains("<div class=\"sidebarblock\">"));
        assert!(html.contains("<p>inside sidebar</p>"));
        assert!(html.contains("<div class=\"exampleblock\">"));
        assert!(html.contains("<p>inside example</p>"));
    }

    #[test]
    fn renders_default_captioned_titles_for_examples_tables_and_images() {
        let html = render_html(&crate::parser::parse_document(
            "= Demo\n\n.First Example\n====\ncontent\n====\n\n.Second Example\n====\nmore\n====\n\n.Agents\n|===\n|Name\n|Ada\n|===\n\n.The Tiger\nimage::images/tiger.png[Tiger]",
        ));

        assert!(html.contains("<div class=\"title\">Example 1. First Example</div>"));
        assert!(html.contains("<div class=\"title\">Example 2. Second Example</div>"));
        assert!(html.contains("<caption class=\"title\">Table 1. Agents</caption>"));
        assert!(html.contains("<div class=\"title\">Figure 1. The Tiger</div>"));
    }

    #[test]
    fn renders_listing_captions_when_enabled_and_respects_counter_start() {
        let html = render_html(&crate::parser::parse_document(
            "= Demo\n:listing-caption: Listing\n:listing-number: 3\n\n.First\n[source,rust]\n----\nfn main() {}\n----\n\n.Second\n----\nputs 'hi'\n----",
        ));

        assert!(html.contains("<div class=\"title\">Listing 3. First</div>"));
        assert!(html.contains("<div class=\"title\">Listing 4. Second</div>"));
    }

    #[test]
    fn block_caption_overrides_generated_captioned_title() {
        let html = render_html(&crate::parser::parse_document(
            "= Demo\n\n.Block Title\n[caption=\"Example A: \"]\n====\nBlock content\n====",
        ));

        assert!(html.contains("<div class=\"title\">Example A: Block Title</div>"));
        assert!(!html.contains("Example 1. Block Title"));
    }

    #[test]
    fn block_caption_override_expands_custom_counters() {
        let html = render_html(&crate::parser::parse_document(
            "= Demo\n\n.First\n[caption=\"Example {counter:my-example-number:A}: \"]\n====\nOne\n====\n\n.Second\n[caption=\"Example {counter:my-example-number}: \"]\n====\nTwo\n====",
        ));

        assert!(html.contains("<div class=\"title\">Example A: First</div>"));
        assert!(html.contains("<div class=\"title\">Example B: Second</div>"));
    }

    #[test]
    fn plain_block_titles_expand_custom_counters() {
        let html = render_html(&crate::parser::parse_document(
            "= Demo\n\n.Step {counter:task-number:1}\nterm:: first\n\n.Step {counter:task-number}\nnext:: second",
        ));

        assert!(html.contains("<div class=\"title\">Step 1</div>"));
        assert!(html.contains("<div class=\"title\">Step 2</div>"));
    }

    #[test]
    fn renders_fenced_code_blocks_with_language_class() {
        let html = render_html(&crate::parser::parse_document("```rust\nfn main() {}\n```"));

        assert!(html.contains("<div class=\"listingblock\">"));
        assert!(html.contains("class=\"language-rust\""));
        assert!(html.contains("data-lang=\"rust\""));
        assert!(html.contains("fn main() {}"));
    }

    #[test]
    fn renders_numbered_listings_with_linenotable_markup() {
        let html = render_html(&crate::parser::parse_document(
            "[source,rust,start=7,%linenums]\n----\nfn main() {}\nprintln!(\"done\");\n----",
        ));

        assert!(html.contains("<table class=\"linenotable\">"));
        assert!(html.contains("<tbody>"));
        assert!(html.contains("<td class=\"linenos\"><pre>7</pre></td>"));
        assert!(html.contains("<td class=\"linenos\"><pre>8</pre></td>"));
        assert!(html.contains("<td class=\"code\"><pre><code class=\"language-rust\" data-lang=\"rust\">fn main() {}</code></pre></td>"));
    }

    #[test]
    fn renders_syntect_highlighted_source_blocks() {
        let html = render_html(&crate::parser::parse_document(
            "= Demo\n:source-highlighter: syntect\n\n[source,rust]\n----\nfn main() {}\n----",
        ));

        assert!(html.contains(
            "<pre class=\"highlight\"><code class=\"language-rust\" data-lang=\"rust\">"
        ));
        assert!(html.contains("<span style="));
        assert!(html.contains("fn "));
    }

    #[test]
    fn does_not_highlight_for_synctect_typo() {
        let html = render_html(&crate::parser::parse_document(
            "= Demo\n:source-highlighter: synctect\n\n[source,rust]\n----\nfn main() {}\n----",
        ));

        assert!(!html.contains("<span style="));
    }

    #[test]
    fn renders_delimited_block_titles() {
        let document = Document {
            attributes: Default::default(),
            title: None,
            blocks: vec![Block::Listing(Listing {
                lines: vec!["puts 'hello'".into()],
                callouts: vec![],
                reftext: None,
                metadata: BlockMetadata {
                    title: Some("Exhibit A".into()),
                    ..Default::default()
                },
            })],
        };

        let html = render_html(&document);

        assert!(html.contains("<div class=\"title\">Exhibit A</div>"));
    }

    #[test]
    fn renders_tables() {
        let html = render_html(&crate::parser::parse_document(
            ".Agents\n[%header,cols=\"30%,\"]\n|===\n|Name|Email\n|Peter|peter@example.com\n|Adam|adam@example.com\n|===",
        ));

        assert!(html.contains("<table class=\"tableblock frame-all grid-all stretch\">"));
        assert!(html.contains("<caption class=\"title\">Table 1. Agents</caption>"));
        assert!(html.contains("<thead>"));
        assert!(html.contains("<th class=\"tableblock halign-left valign-top\">Name</th>"));
        assert!(html.contains(
            "<td class=\"tableblock halign-left valign-top\"><p class=\"tableblock\">Peter</p></td>"
        ));
    }

    #[test]
    fn renders_tables_with_stacked_cells() {
        let html = render_html(&crate::parser::parse_document(
            ".Agents\n[%header,cols=\"30%,70%\"]\n|===\n|Name\n|Email\n|Peter\n|peter@example.com\n|Adam\n|adam@example.com\n|===",
        ));

        assert!(html.contains("<th class=\"tableblock halign-left valign-top\">Name</th>"));
        assert!(html.contains("<th class=\"tableblock halign-left valign-top\">Email</th>"));
        assert!(html.contains(
            "<td class=\"tableblock halign-left valign-top\"><p class=\"tableblock\">Peter</p></td>"
        ));
        assert!(html.contains("<td class=\"tableblock halign-left valign-top\"><p class=\"tableblock\">peter@example.com</p></td>"));
    }

    #[test]
    fn renders_tables_with_stacked_cells_without_cols() {
        let html = render_html(&crate::parser::parse_document(
            ".Agents\n[%header]\n|===\n|Name\n|Email\n\n|Peter\n|peter@example.com\n\n|Adam\n|adam@example.com\n|===",
        ));

        assert!(html.contains("<th class=\"tableblock halign-left valign-top\">Name</th>"));
        assert!(html.contains("<th class=\"tableblock halign-left valign-top\">Email</th>"));
        assert!(html.contains(
            "<td class=\"tableblock halign-left valign-top\"><p class=\"tableblock\">Adam</p></td>"
        ));
        assert!(html.contains("<td class=\"tableblock halign-left valign-top\"><p class=\"tableblock\">adam@example.com</p></td>"));
    }

    #[test]
    fn renders_block_content_inside_table_cells() {
        let html = render_html(&crate::parser::parse_document(
            ".Services\n[%header,cols=\"1,3\"]\n|===\n|Name\n|Details\n|API\n|First paragraph.\n\n* fast\n* typed\n|===",
        ));

        assert!(
            html.contains(
                "<td class=\"tableblock halign-left valign-top\"><div class=\"paragraph\">"
            )
        );
        assert!(html.contains("<p>First paragraph.</p>"));
        assert!(html.contains("<div class=\"ulist\">"));
        assert!(html.contains("<p>fast</p>"));
        assert!(html.contains("<p>typed</p>"));
    }

    #[test]
    fn renders_table_cell_specs_for_rowspan_and_asciidoc_style() {
        let html = render_html(&crate::parser::parse_document(
            "[%header,cols=\"1,2\"]\n|===\nh|Area\n|Description\n\n.2+|Shared\na|First paragraph.\n+\nSecond paragraph.\n\n|Another description\n|===",
        ));

        assert!(html.contains("<th class=\"tableblock halign-left valign-top\">Area</th>"));
        assert!(html.contains("<td class=\"tableblock halign-left valign-top\" rowspan=\"2\"><p class=\"tableblock\">Shared</p></td>"));
        assert!(html.contains("<p>First paragraph.</p>"));
        assert!(html.contains("<p>Second paragraph.</p>"));
    }

    #[test]
    fn renders_admonition_paragraphs() {
        let document = Document {
            attributes: Default::default(),
            title: None,
            blocks: vec![Block::Admonition(crate::ast::AdmonitionBlock {
                variant: crate::ast::AdmonitionVariant::Note,
                blocks: vec![Block::Paragraph(Paragraph {
                    lines: vec!["This is just a note.".into()],
                    inlines: vec![Inline::Text("This is just a note.".into())],
                    id: None,
                    reftext: None,
                    metadata: BlockMetadata::default(),
                })],
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
            })],
        };

        let html = render_html(&document);

        assert!(html.contains("<div class=\"admonitionblock note\">"));
        assert!(html.contains("<td class=\"icon\">"));
        assert!(html.contains("<div class=\"title\">Note</div>"));
        assert!(html.contains("<td class=\"content\">"));
        assert!(html.contains("<p>This is just a note.</p>"));
    }

    #[test]
    fn uses_document_caption_for_admonition_label() {
        let document = Document {
            attributes: [("tip-caption".to_string(), "Pro Tip".to_string())]
                .into_iter()
                .collect(),
            title: None,
            blocks: vec![Block::Admonition(crate::ast::AdmonitionBlock {
                id: None,
                reftext: None,
                variant: crate::ast::AdmonitionVariant::Tip,
                blocks: vec![Block::Paragraph(Paragraph {
                    lines: vec!["Ship it carefully.".into()],
                    inlines: vec![Inline::Text("Ship it carefully.".into())],
                    id: None,
                    reftext: None,
                    metadata: BlockMetadata::default(),
                })],
                metadata: BlockMetadata::default(),
            })],
        };

        let html = render_html(&document);

        assert!(html.contains("<div class=\"title\">Pro Tip</div>"));
        assert!(!html.contains("<td class=\"icon\">\n<div class=\"title\">Tip</div>"));
    }

    #[test]
    fn block_caption_overrides_document_caption_for_admonition_label() {
        let document = Document {
            attributes: [("tip-caption".to_string(), "Pro Tip".to_string())]
                .into_iter()
                .collect(),
            title: None,
            blocks: vec![Block::Admonition(crate::ast::AdmonitionBlock {
                id: None,
                reftext: None,
                variant: crate::ast::AdmonitionVariant::Tip,
                blocks: vec![Block::Paragraph(Paragraph {
                    lines: vec!["Ship it carefully.".into()],
                    inlines: vec![Inline::Text("Ship it carefully.".into())],
                    id: None,
                    reftext: None,
                    metadata: BlockMetadata::default(),
                })],
                metadata: BlockMetadata {
                    attributes: [("caption".to_string(), "Custom Tip".to_string())]
                        .into_iter()
                        .collect(),
                    ..BlockMetadata::default()
                },
            })],
        };

        let html = render_html(&document);

        assert!(html.contains("<div class=\"title\">Custom Tip</div>"));
        assert!(!html.contains("<td class=\"icon\">\n<div class=\"title\">Pro Tip</div>"));
    }

    #[test]
    fn renders_image_admonition_icons_from_document_attributes() {
        let document = Document {
            attributes: [
                ("icons".to_string(), String::new()),
                ("iconsdir".to_string(), "assets/icons".to_string()),
            ]
            .into_iter()
            .collect(),
            title: None,
            blocks: vec![Block::Admonition(crate::ast::AdmonitionBlock {
                id: None,
                reftext: None,
                variant: crate::ast::AdmonitionVariant::Tip,
                blocks: vec![Block::Paragraph(Paragraph {
                    lines: vec!["Ship it carefully.".into()],
                    inlines: vec![Inline::Text("Ship it carefully.".into())],
                    id: None,
                    reftext: None,
                    metadata: BlockMetadata::default(),
                })],
                metadata: BlockMetadata::default(),
            })],
        };

        let html = render_html(&document);

        assert!(html.contains("<img src=\"assets/icons/tip.png\" alt=\"Tip\">"));
        assert!(!html.contains("<div class=\"title\">Tip</div>"));
    }

    #[test]
    fn renders_font_admonition_icons_from_document_attributes() {
        let document = Document {
            attributes: [("icons".to_string(), "font".to_string())]
                .into_iter()
                .collect(),
            title: None,
            blocks: vec![Block::Admonition(crate::ast::AdmonitionBlock {
                id: None,
                reftext: None,
                variant: crate::ast::AdmonitionVariant::Tip,
                blocks: vec![Block::Paragraph(Paragraph {
                    lines: vec!["Ship it carefully.".into()],
                    inlines: vec![Inline::Text("Ship it carefully.".into())],
                    id: None,
                    reftext: None,
                    metadata: BlockMetadata::default(),
                })],
                metadata: BlockMetadata::default(),
            })],
        };

        let html = render_html(&document);

        assert!(html.contains("<i class=\"fa icon-tip\" title=\"Tip\"></i>"));
        assert!(!html.contains("<div class=\"title\">Tip</div>"));
        assert!(!html.contains("<img"));
    }

    #[test]
    fn uses_caption_as_font_admonition_icon_title() {
        let document = Document {
            attributes: [("icons".to_string(), "font".to_string())]
                .into_iter()
                .collect(),
            title: None,
            blocks: vec![Block::Admonition(crate::ast::AdmonitionBlock {
                id: None,
                reftext: None,
                variant: crate::ast::AdmonitionVariant::Tip,
                blocks: vec![Block::Paragraph(Paragraph {
                    lines: vec!["Ship it carefully.".into()],
                    inlines: vec![Inline::Text("Ship it carefully.".into())],
                    id: None,
                    reftext: None,
                    metadata: BlockMetadata::default(),
                })],
                metadata: BlockMetadata {
                    attributes: [("caption".to_string(), "Custom Tip".to_string())]
                        .into_iter()
                        .collect(),
                    ..Default::default()
                },
            })],
        };

        let html = render_html(&document);

        assert!(html.contains("<i class=\"fa icon-tip\" title=\"Custom Tip\"></i>"));
        assert!(!html.contains("<div class=\"title\">Custom Tip</div>"));
    }

    #[test]
    fn block_icon_attributes_override_default_admonition_icon() {
        let document = Document {
            attributes: [("icons".to_string(), String::new())].into_iter().collect(),
            title: None,
            blocks: vec![Block::Admonition(crate::ast::AdmonitionBlock {
                id: None,
                reftext: None,
                variant: crate::ast::AdmonitionVariant::Tip,
                blocks: vec![Block::Paragraph(Paragraph {
                    lines: vec!["Ship it carefully.".into()],
                    inlines: vec![Inline::Text("Ship it carefully.".into())],
                    id: None,
                    reftext: None,
                    metadata: BlockMetadata::default(),
                })],
                metadata: BlockMetadata {
                    attributes: [
                        ("icon".to_string(), "hint".to_string()),
                        ("iconsdir".to_string(), "custom/icons".to_string()),
                        ("icontype".to_string(), "svg".to_string()),
                        ("caption".to_string(), "Custom Tip".to_string()),
                    ]
                    .into_iter()
                    .collect(),
                    ..BlockMetadata::default()
                },
            })],
        };

        let html = render_html(&document);

        assert!(html.contains("<img src=\"custom/icons/hint.svg\" alt=\"Custom Tip\">"));
    }

    #[test]
    fn renders_block_passthrough_as_raw_html() {
        let html = render_html(&crate::parser::parse_document(
            "++++\n<video src=\"video.mp4\" controls></video>\n++++\n",
        ));
        assert!(html.contains("<video src=\"video.mp4\" controls></video>"));
        assert!(!html.contains("&lt;video"));
    }

    #[test]
    fn renders_stem_blocks_with_asciimath_delimiters() {
        let html = render_html(&crate::parser::parse_document(
            "= Demo\n:stem:\n\n[stem]\n++++\nsqrt(4) = 2\n++++",
        ));

        assert!(html.contains("<div class=\"stemblock\">"));
        assert!(html.contains("\\$sqrt(4) = 2\\$"));
    }

    #[test]
    fn renders_stem_blocks_with_latexmath_delimiters() {
        let html = render_html(&crate::parser::parse_document(
            "= Demo\n:stem: latexmath\n\n[stem]\n++++\n\\alpha + \\beta\n++++",
        ));

        assert!(html.contains("\\[\\alpha + \\beta\\]"));
    }

    #[test]
    fn renders_explicit_latexmath_blocks() {
        let html = render_html(&crate::parser::parse_document(
            "[latexmath]\n++++\n\\alpha + \\beta\n++++",
        ));

        assert!(html.contains("<div class=\"stemblock\">"));
        assert!(html.contains("\\[\\alpha + \\beta\\]"));
    }

    #[test]
    fn trims_outer_blank_lines_in_listing_blocks_when_rendering() {
        let html = render_html(&crate::parser::parse_document(
            "----\n\nputs 'hello' <1>\n\nputs 'goodbye'\n\n----\n<1> greeting\n",
        ));

        assert!(html.contains(
            "<pre>puts &#39;hello&#39;<i class=\"conum\" data-value=\"1\"></i><b>1</b>\n\nputs &#39;goodbye&#39;</pre>"
        ));
    }

    #[test]
    fn trims_outer_blank_lines_in_literal_blocks_when_rendering() {
        let html = render_html(&crate::parser::parse_document(
            "....\n\n  first line\n\nlast line\n\n....",
        ));

        assert!(html.contains("<pre>  first line\n\nlast line</pre>"));
    }

    #[test]
    fn renders_inline_triple_plus_passthrough_unescaped() {
        let html = render_html(&crate::parser::parse_document(
            "See +++<del>this</del>+++ example.\n",
        ));
        assert!(html.contains("<del>this</del>"));
        assert!(!html.contains("&lt;del&gt;"));
    }

    #[test]
    fn renders_inline_pass_macro_unescaped() {
        let html = render_html(&crate::parser::parse_document("See pass:[<br>] here.\n"));
        assert!(html.contains("<br>"));
        assert!(!html.contains("&lt;br&gt;"));
    }

    #[test]
    fn renders_block_image_with_alt_and_src() {
        let html = render_html(&crate::parser::parse_document("image::tiger.png[Tiger]"));
        assert!(html.contains("<div class=\"imageblock\">"));
        assert!(html.contains("<img src=\"tiger.png\" alt=\"Tiger\">"));
    }

    #[test]
    fn renders_block_image_with_dimensions() {
        let html = render_html(&crate::parser::parse_document(
            "image::tiger.png[Tiger, 200, 300]",
        ));
        assert!(
            html.contains("<img src=\"tiger.png\" alt=\"Tiger\" width=\"200\" height=\"300\">")
        );
    }

    #[test]
    fn renders_block_image_with_imagesdir() {
        let html = render_html(&crate::parser::parse_document(
            ":imagesdir: images\n\nimage::tiger.png[Tiger]",
        ));
        assert!(html.contains("<img src=\"images/tiger.png\" alt=\"Tiger\">"));
    }

    #[test]
    fn renders_block_image_uri_bypasses_imagesdir() {
        let html = render_html(&crate::parser::parse_document(
            ":imagesdir: images\n\nimage::http://example.com/tiger.png[Tiger]",
        ));
        assert!(html.contains("<img src=\"http://example.com/tiger.png\" alt=\"Tiger\">"));
    }

    #[test]
    fn renders_block_image_with_title() {
        let html = render_html(&crate::parser::parse_document(
            ".The AsciiDoc Tiger\nimage::tiger.png[Tiger]",
        ));
        assert!(html.contains("<div class=\"title\">Figure 1. The AsciiDoc Tiger</div>"));
    }

    #[test]
    fn renders_block_image_with_link() {
        let html = render_html(&crate::parser::parse_document(
            "image::tiger.png[Tiger, link='http://example.com']",
        ));
        assert!(html.contains("<a class=\"image\" href=\"http://example.com\">"));
        assert!(html.contains("<img src=\"tiger.png\" alt=\"Tiger\">"));
        assert!(html.contains("</a>"));
    }

    #[test]
    fn renders_block_image_with_auto_generated_alt() {
        let html = render_html(&crate::parser::parse_document(
            "image::lions-and-tigers.png[]",
        ));
        assert!(html.contains("alt=\"lions and tigers\""));
    }

    #[test]
    fn renders_inline_image() {
        let html = render_html(&crate::parser::parse_document(
            "Click image:icon.png[Icon] to continue.",
        ));
        assert!(html.contains("<span class=\"image\"><img src=\"icon.png\" alt=\"Icon\"></span>"));
    }

    #[test]
    fn renders_inline_image_with_dimensions() {
        let html = render_html(&crate::parser::parse_document(
            "image:icon.png[Icon, 16, 16]",
        ));
        assert!(html.contains("<img src=\"icon.png\" alt=\"Icon\" width=\"16\" height=\"16\">"));
    }

    #[test]
    fn renders_footnotes() {
        let document = Document {
            attributes: Default::default(),
            title: None,
            blocks: vec![Block::Paragraph(Paragraph {
                lines: vec!["A notefootnote:[Read this first.] here.".into()],
                inlines: vec![
                    Inline::Text("A note".into()),
                    Inline::Footnote(InlineFootnote {
                        inlines: vec![Inline::Text("Read this first.".into())],
                    }),
                    Inline::Text(" here.".into()),
                ],
                id: None,
                reftext: None,
                metadata: BlockMetadata::default(),
            })],
        };

        let html = render_html(&document);

        assert!(html.contains(
            "<sup class=\"footnote\" id=\"_footnoteref_1\"><a href=\"#_footnotedef_1\">1</a></sup>"
        ));
        assert!(html.contains("<div id=\"footnotes\">"));
        assert!(html.contains("<div class=\"footnote\" id=\"_footnotedef_1\"><a href=\"#_footnoteref_1\">1</a>. Read this first.</div>"));
    }

    #[test]
    fn renders_toc_macro_at_placement_location() {
        let html = render_html(&crate::parser::parse_document(
            "= Doc\n:toc: macro\n\ntoc::[]\n\n== Alpha\n\ntext\n\n== Beta\n\nmore",
        ));
        // TOC should appear in content, not the header
        assert!(html.contains("<div id=\"toc\" class=\"toc\">"));
        assert!(html.contains("<div id=\"toctitle\">Table of Contents</div>"));
        assert!(html.contains("<ul class=\"sectlevel1\">"));
        assert!(html.contains("<a href=\"#_alpha\">Alpha</a>"));
        // Second same-level section gets -2 suffix from ID generator
        assert!(html.contains("<a href=\"#_beta-2\">Beta</a>"));
        // Header div should NOT contain the TOC (toc: macro means explicit placement only)
        let content_start = html.find("<div id=\"content\">").unwrap();
        assert!(!html[..content_start].contains("id=\"toc\""));
    }

    #[test]
    fn renders_toc_auto_placed_in_header() {
        let html = render_html(&crate::parser::parse_document(
            "= Doc\n:toc:\n\n== First\n\ntext\n\n=== Sub\n\nmore",
        ));
        assert!(html.contains("<div id=\"toc\" class=\"toc\">"));
        assert!(html.contains("<a href=\"#_first\">First</a>"));
        // Nested sectlevel2
        assert!(html.contains("<ul class=\"sectlevel2\">"));
        assert!(html.contains("<a href=\"#_sub\">Sub</a>"));
        // TOC is in the header, before </div>\n<div id="content">
        let content_start = html.find("<div id=\"content\">").unwrap();
        assert!(html[..content_start].contains("<div id=\"toc\" class=\"toc\">"));
    }

    #[test]
    fn renders_toc_with_custom_title_and_levels() {
        let html = render_html(&crate::parser::parse_document(
            "= Doc\n:toc:\n:toctitle: Contents\n:toclevels: 1\n\n== Sec\n\n=== Nested\n\ntext",
        ));
        assert!(html.contains("<div id=\"toctitle\">Contents</div>"));
        assert!(html.contains("<ul class=\"sectlevel1\">"));
        // Level 2 should be suppressed
        assert!(!html.contains("sectlevel2"));
    }

    #[test]
    fn renders_open_block() {
        let html = render_html(&crate::parser::parse_document(
            "--\nFirst paragraph.\n\nSecond paragraph.\n--",
        ));
        assert!(html.contains("<div class=\"openblock\""));
        assert!(html.contains("<div class=\"content\">"));
        assert!(html.contains("First paragraph."));
        assert!(html.contains("Second paragraph."));
    }

    #[test]
    fn renders_styled_open_block_as_sidebar() {
        let html = render_html(&crate::parser::parse_document(
            "[sidebar]\n--\nSidebar content.\n--",
        ));
        assert!(html.contains("<div class=\"sidebarblock\""));
        assert!(!html.contains("openblock"));
    }

    #[test]
    fn renders_delimited_literal_block() {
        let html = render_html(&crate::parser::parse_document(
            "....\n  preformatted text\n  with indent\n....",
        ));
        assert!(html.contains("<div class=\"literalblock\""));
        assert!(html.contains("<pre>  preformatted text\n  with indent</pre>"));
    }

    #[test]
    fn renders_literal_styled_paragraph() {
        let html = render_html(&crate::parser::parse_document("[literal]\npreformatted."));
        assert!(html.contains("<div class=\"literalblock\""));
        assert!(html.contains("<pre>preformatted.</pre>"));
    }

    #[test]
    fn renders_indented_paragraph_as_literal() {
        let html = render_html(&crate::parser::parse_document(" indented text"));
        assert!(html.contains("<div class=\"literalblock\""));
        assert!(html.contains("<pre> indented text</pre>"));
    }

    #[test]
    fn renders_quote_block() {
        let html = render_html(&crate::parser::parse_document(
            "[quote, Abraham Lincoln, Gettysburg Address]\n____\nFour score.\n____",
        ));
        assert!(html.contains("<div class=\"quoteblock\""));
        assert!(html.contains("<blockquote>"));
        assert!(html.contains("Four score."));
        assert!(html.contains("</blockquote>"));
        assert!(html.contains("<div class=\"attribution\">"));
        assert!(html.contains("Abraham Lincoln"));
        assert!(html.contains("<cite>Gettysburg Address</cite>"));
    }

    #[test]
    fn renders_quote_block_without_attribution() {
        let html = render_html(&crate::parser::parse_document(
            "____\nSome quoted text.\n____",
        ));
        assert!(html.contains("<div class=\"quoteblock\""));
        assert!(html.contains("<blockquote>"));
        assert!(html.contains("Some quoted text."));
        assert!(!html.contains("<div class=\"attribution\">"));
    }

    #[test]
    fn renders_verse_block() {
        let html = render_html(&crate::parser::parse_document(
            "[verse, Carl Sandburg, Fog]\n____\nThe fog comes\non little cat feet.\n____",
        ));
        assert!(html.contains("<div class=\"verseblock\""));
        assert!(html.contains("<pre class=\"content\">"));
        assert!(html.contains("The fog comes\non little cat feet."));
        assert!(html.contains("<div class=\"attribution\">"));
        assert!(html.contains("Carl Sandburg"));
        assert!(html.contains("<cite>Fog</cite>"));
    }

    #[test]
    fn trims_outer_blank_lines_in_verse_blocks_when_rendering() {
        let html = render_html(&crate::parser::parse_document(
            "[verse]\n____\n\nline one\n\nline two\n\n____",
        ));

        assert!(html.contains("<pre class=\"content\">line one\n\nline two</pre>"));
    }
}
