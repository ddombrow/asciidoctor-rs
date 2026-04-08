use crate::prepare::{
    prepare_document, DocumentBlock, DocumentSection, PreparedBlock, PreparedInline,
};

struct RenderContext<'a> {
    document_attributes: &'a std::collections::BTreeMap<String, String>,
    sections: &'a [DocumentSection],
}

pub fn render_html(document: &crate::ast::Document) -> String {
    render_prepared_html(&prepare_document(document))
}

pub fn render_prepared_html(document: &DocumentBlock) -> String {
    let mut html = String::new();
    let ctx = RenderContext {
        document_attributes: &document.attributes,
        sections: &document.sections,
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

fn render_toc_sections(
    html: &mut String,
    sections: &[DocumentSection],
    level: u8,
    max_level: u8,
) {
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

fn render_block(
    html: &mut String,
    block: &PreparedBlock,
    ctx: &RenderContext<'_>,
) {
    match block {
        PreparedBlock::Preamble(preamble) => {
            html.push_str("<div id=\"preamble\">\n<div class=\"sectionbody\">\n");
            for block in &preamble.blocks {
                render_block(html, block, ctx);
            }
            html.push_str("</div>\n</div>\n");
        }
        PreparedBlock::Paragraph(paragraph) => render_paragraph(html, paragraph),
        PreparedBlock::Admonition(admonition) => render_admonition(html, admonition, ctx),
        PreparedBlock::UnorderedList(list) => render_unordered_list(html, list, ctx),
        PreparedBlock::OrderedList(list) => render_ordered_list(html, list, ctx),
        PreparedBlock::DescriptionList(list) => render_description_list(html, list, ctx),
        PreparedBlock::Table(table) => render_table(html, table, ctx),
        PreparedBlock::Listing(listing) => render_listing(html, listing),
        PreparedBlock::Literal(literal) => render_literal(html, literal),
        PreparedBlock::Example(example) => render_compound(html, "exampleblock", example, ctx),
        PreparedBlock::Sidebar(sidebar) => render_sidebar(html, sidebar, ctx),
        PreparedBlock::Open(open) => render_open(html, open, ctx),
        PreparedBlock::Quote(quote) => render_quote(html, quote, ctx),
        PreparedBlock::Passthrough(p) => {
            html.push_str(&p.content);
            html.push('\n');
        }
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
    if let Some(title) = &list.title {
        html.push_str(&format!(
            "<div class=\"title\">{}</div>\n",
            escape_html(title)
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
    if let Some(title) = &list.title {
        html.push_str(&format!(
            "<div class=\"title\">{}</div>\n",
            escape_html(title)
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
    if let Some(title) = &list.title {
        html.push_str(&format!(
            "<div class=\"title\">{}</div>\n",
            escape_html(title)
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

fn render_table(
    html: &mut String,
    table: &crate::prepare::TableBlock,
    ctx: &RenderContext<'_>,
) {
    html.push_str("<table class=\"tableblock frame-all grid-all stretch\"");
    if let Some(id) = &table.id {
        html.push_str(&format!(" id=\"{}\"", escape_html(id)));
    }
    html.push_str(">\n");
    if let Some(title) = &table.title {
        html.push_str(&format!(
            "<caption class=\"title\">{}</caption>\n",
            escape_html(title)
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

fn render_listing(html: &mut String, listing: &crate::prepare::ListingBlock) {
    html.push_str("<div class=\"listingblock\"");
    if let Some(id) = &listing.id {
        html.push_str(&format!(" id=\"{}\"", escape_html(id)));
    }
    html.push_str(">\n");
    if let Some(title) = &listing.title {
        html.push_str(&format!(
            "<div class=\"title\">{}</div>\n",
            escape_html(title)
        ));
    }
    html.push_str("<div class=\"content\">\n<pre>");
    html.push_str(&escape_html(&listing.content));
    html.push_str("</pre>\n</div>\n</div>\n");
}

fn render_literal(html: &mut String, literal: &crate::prepare::ListingBlock) {
    html.push_str("<div class=\"literalblock\"");
    if let Some(id) = &literal.id {
        html.push_str(&format!(" id=\"{}\"", escape_html(id)));
    }
    html.push_str(">\n");
    if let Some(title) = &literal.title {
        html.push_str(&format!(
            "<div class=\"title\">{}</div>\n",
            escape_html(title)
        ));
    }
    html.push_str("<div class=\"content\">\n<pre>");
    html.push_str(&escape_html(&literal.content));
    html.push_str("</pre>\n</div>\n</div>\n");
}

fn render_compound(
    html: &mut String,
    class_name: &str,
    block: &crate::prepare::CompoundBlock,
    ctx: &RenderContext<'_>,
) {
    html.push_str(&format!("<div class=\"{class_name}\""));
    if let Some(id) = &block.id {
        html.push_str(&format!(" id=\"{}\"", escape_html(id)));
    }
    html.push_str(">\n");
    if let Some(title) = &block.title {
        html.push_str(&format!(
            "<div class=\"title\">{}</div>\n",
            escape_html(title)
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
    if let Some(title) = &block.title {
        html.push_str(&format!(
            "<div class=\"title\">{}</div>\n",
            escape_html(title)
        ));
    }
    for child in &block.blocks {
        render_block(html, child, ctx);
    }
    html.push_str("</div>\n</div>\n");
}

fn render_open(
    html: &mut String,
    block: &crate::prepare::CompoundBlock,
    ctx: &RenderContext<'_>,
) {
    html.push_str("<div class=\"openblock\"");
    if let Some(id) = &block.id {
        html.push_str(&format!(" id=\"{}\"", escape_html(id)));
    }
    html.push_str(">\n");
    if let Some(title) = &block.title {
        html.push_str(&format!(
            "<div class=\"title\">{}</div>\n",
            escape_html(title)
        ));
    }
    html.push_str("<div class=\"content\">\n");
    for child in &block.blocks {
        render_block(html, child, ctx);
    }
    html.push_str("</div>\n</div>\n");
}

fn render_quote(
    html: &mut String,
    block: &crate::prepare::QuoteBlock,
    ctx: &RenderContext<'_>,
) {
    let div_class = if block.is_verse { "verseblock" } else { "quoteblock" };
    html.push_str(&format!("<div class=\"{div_class}\""));
    if let Some(id) = &block.id {
        html.push_str(&format!(" id=\"{}\"", escape_html(id)));
    }
    html.push_str(">\n");
    if let Some(title) = &block.title {
        html.push_str(&format!(
            "<div class=\"title\">{}</div>\n",
            escape_html(title)
        ));
    }
    if block.is_verse {
        html.push_str("<pre class=\"content\">");
        html.push_str(&escape_html(&block.content));
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

fn render_paragraph(html: &mut String, paragraph: &crate::prepare::ParagraphBlock) {
    html.push_str("<div class=\"paragraph\"");
    if let Some(id) = &paragraph.id {
        html.push_str(&format!(" id=\"{}\"", escape_html(id)));
    }
    html.push_str(">\n");
    if let Some(title) = &paragraph.title {
        html.push_str(&format!(
            "<div class=\"title\">{}</div>\n",
            escape_html(title)
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
    if let Some(title) = &admonition.title {
        html.push_str(&format!(
            "<div class=\"title\">{}</div>\n",
            escape_html(title)
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

    if let Some(title) = &image.title {
        html.push_str(&format!(
            "<div class=\"title\">{}</div>\n",
            escape_html(title)
        ));
    }

    html.push_str("</div>\n");
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
    fn renders_delimited_block_titles() {
        let document = Document {
            attributes: Default::default(),
            title: None,
            blocks: vec![Block::Listing(Listing {
                lines: vec!["puts 'hello'".into()],
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
        assert!(html.contains("<caption class=\"title\">Agents</caption>"));
        assert!(html.contains("<thead>"));
        assert!(html.contains("<th class=\"tableblock halign-left valign-top\">Name</th>"));
        assert!(html.contains("<td class=\"tableblock halign-left valign-top\"><p class=\"tableblock\">Peter</p></td>"));
    }

    #[test]
    fn renders_tables_with_stacked_cells() {
        let html = render_html(&crate::parser::parse_document(
            ".Agents\n[%header,cols=\"30%,70%\"]\n|===\n|Name\n|Email\n|Peter\n|peter@example.com\n|Adam\n|adam@example.com\n|===",
        ));

        assert!(html.contains("<th class=\"tableblock halign-left valign-top\">Name</th>"));
        assert!(html.contains("<th class=\"tableblock halign-left valign-top\">Email</th>"));
        assert!(html.contains("<td class=\"tableblock halign-left valign-top\"><p class=\"tableblock\">Peter</p></td>"));
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

        assert!(html
            .contains("<td class=\"tableblock halign-left valign-top\"><div class=\"paragraph\">"));
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
        assert!(html.contains("<img src=\"tiger.png\" alt=\"Tiger\" width=\"200\" height=\"300\">"));
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
        assert!(html.contains("<div class=\"title\">The AsciiDoc Tiger</div>"));
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
}
