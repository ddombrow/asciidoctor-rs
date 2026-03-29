use crate::prepare::{DocumentBlock, PreparedBlock, PreparedInline, prepare_document};

pub fn render_html(document: &crate::ast::Document) -> String {
    render_prepared_html(&prepare_document(document))
}

pub fn render_prepared_html(document: &DocumentBlock) -> String {
    let mut html = String::new();
    html.push_str("<div id=\"header\">\n");

    if !document.title.is_empty() {
        html.push_str(&format!("<h1>{}</h1>\n", escape_html(&document.title)));
    }
    html.push_str("</div>\n");

    html.push_str("<div id=\"content\">\n");
    for block in &document.blocks {
        render_block(&mut html, block);
    }
    html.push_str("</div>\n");
    html
}

fn render_block(html: &mut String, block: &PreparedBlock) {
    match block {
        PreparedBlock::Preamble(preamble) => {
            html.push_str("<div id=\"preamble\">\n<div class=\"sectionbody\">\n");
            for block in &preamble.blocks {
                render_block(html, block);
            }
            html.push_str("</div>\n</div>\n");
        }
        PreparedBlock::Paragraph(paragraph) => render_paragraph(html, &paragraph.inlines),
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
                render_block(html, block);
            }

            html.push_str("</div>\n</div>\n");
        }
    }
}

fn render_paragraph(html: &mut String, inlines: &[PreparedInline]) {
    html.push_str("<div class=\"paragraph\">\n<p>");
    render_inlines(html, inlines);
    html.push_str("</p>\n</div>\n");
}

fn render_inlines(html: &mut String, inlines: &[PreparedInline]) {
    for inline in inlines {
        match inline {
            PreparedInline::Text(text) => html.push_str(&escape_html(&text.value)),
            PreparedInline::Span(span) => {
                let tag = match span.variant.as_str() {
                    "strong" => "strong",
                    "emphasis" => "em",
                    _ => "span",
                };
                html.push_str(&format!("<{tag}>"));
                render_inlines(html, &span.inlines);
                html.push_str(&format!("</{tag}>"));
            }
        }
    }
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
        Block, Document, Heading, Inline, InlineForm, InlineSpan, InlineVariant, Paragraph,
    };
    use crate::prepare::prepare_document;
    use crate::render::render_html;

    #[test]
    fn renders_document_title_sections_and_paragraphs() {
        let document = Document {
            title: Some(Heading {
                level: 0,
                title: "Document Title".into(),
            }),
            blocks: vec![
                Block::Heading(Heading {
                    level: 1,
                    title: "Section One".into(),
                }),
                Block::Paragraph(Paragraph {
                    inlines: vec![Inline::Text("first line\nsecond line".into())],
                    lines: vec!["first line".into(), "second line".into()],
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
            title: Some(Heading {
                level: 0,
                title: "Fish & Chips".into(),
            }),
            blocks: vec![Block::Paragraph(Paragraph {
                inlines: vec![Inline::Text("<tag> \"quoted\" and 'apostrophe'".into())],
                lines: vec!["<tag> \"quoted\" and 'apostrophe'".into()],
            })],
        };

        let html = render_html(&document);

        assert!(html.contains("<h1>Fish &amp; Chips</h1>"));
        assert!(html.contains("<p>&lt;tag&gt; &quot;quoted&quot; and &#39;apostrophe&#39;</p>"));
    }

    #[test]
    fn rendering_prepared_document_keeps_nested_sections() {
        let document = Document {
            title: Some(Heading {
                level: 0,
                title: "Doc".into(),
            }),
            blocks: vec![
                Block::Heading(Heading {
                    level: 1,
                    title: "Section A".into(),
                }),
                Block::Heading(Heading {
                    level: 2,
                    title: "Section B".into(),
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
            })],
        };

        let html = render_html(&document);

        assert!(html.contains("<p>before <strong>strong</strong> and <em>emphasis</em> after</p>"));
    }
}
