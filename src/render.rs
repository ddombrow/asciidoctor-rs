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
        PreparedBlock::Paragraph(paragraph) => render_paragraph(html, paragraph),
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

fn render_paragraph(html: &mut String, paragraph: &crate::prepare::ParagraphBlock) {
    html.push_str("<div class=\"paragraph\"");
    if let Some(id) = &paragraph.id {
        html.push_str(&format!(" id=\"{}\"", escape_html(id)));
    }
    html.push_str(">\n<p>");
    render_inlines(html, &paragraph.inlines);
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
        Block, Document, Heading, Inline, InlineAnchor, InlineForm, InlineLink, InlineSpan,
        InlineVariant, InlineXref, Paragraph,
    };
    use crate::prepare::prepare_document;
    use crate::render::render_html;

    #[test]
    fn renders_document_title_sections_and_paragraphs() {
        let document = Document {
            title: Some(Heading {
                level: 0,
                title: "Document Title".into(),
                id: None,
                reftext: None,
            }),
            blocks: vec![
                Block::Heading(Heading {
                    level: 1,
                    title: "Section One".into(),
                    id: None,
                    reftext: None,
                }),
                Block::Paragraph(Paragraph {
                    inlines: vec![Inline::Text("first line\nsecond line".into())],
                    lines: vec!["first line".into(), "second line".into()],
                    id: None,
                    reftext: None,
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
                id: None,
                reftext: None,
            }),
            blocks: vec![Block::Paragraph(Paragraph {
                inlines: vec![Inline::Text("<tag> \"quoted\" and 'apostrophe'".into())],
                lines: vec!["<tag> \"quoted\" and 'apostrophe'".into()],
                id: None,
                reftext: None,
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
                id: None,
                reftext: None,
            }),
            blocks: vec![
                Block::Heading(Heading {
                    level: 1,
                    title: "Section A".into(),
                    id: None,
                    reftext: None,
                }),
                Block::Heading(Heading {
                    level: 2,
                    title: "Section B".into(),
                    id: None,
                    reftext: None,
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
                id: None,
                reftext: None,
            })],
        };

        let html = render_html(&document);

        assert!(html.contains("<p>before <strong>strong</strong> and <em>emphasis</em> after</p>"));
    }

    #[test]
    fn renders_escaped_markup_as_literal_text() {
        let document = Document {
            title: None,
            blocks: vec![Block::Paragraph(Paragraph {
                lines: vec![r"\*not strong* and \_not emphasis_".into()],
                inlines: vec![Inline::Text("*not strong* and _not emphasis_".into())],
                id: None,
                reftext: None,
            })],
        };

        let html = render_html(&document);

        assert!(html.contains("<p>*not strong* and _not emphasis_</p>"));
    }

    #[test]
    fn renders_links() {
        let document = Document {
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
            })],
        };

        let html = render_html(&document);

        assert!(html.contains("<a href=\"https://example.org\">example</a>"));
        assert!(html.contains("<a href=\"http://foo.com\" class=\"bare\">http://foo.com</a>"));
    }

    #[test]
    fn renders_links_with_window_targets() {
        let document = Document {
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

        let html = render_html(&document);
        assert!(html.contains("<a href=\"#install\">Installation</a>"));
    }

    #[test]
    fn renders_xrefs_with_resolved_section_ids() {
        let document = Document {
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

        let html = render_html(&document);
        assert!(html.contains("<a href=\"#_first_section\">First Section</a>"));
    }

    #[test]
    fn renders_paragraph_anchor_ids() {
        let document = Document {
            title: None,
            blocks: vec![Block::Paragraph(Paragraph {
                lines: vec!["Hello".into()],
                inlines: vec![Inline::Text("Hello".into())],
                id: Some("intro".into()),
                reftext: Some("Introduction".into()),
            })],
        };

        let html = render_html(&document);
        assert!(html.contains("<div class=\"paragraph\" id=\"intro\">"));
    }

    #[test]
    fn renders_inline_anchor_points() {
        let document = Document {
            title: None,
            blocks: vec![Block::Paragraph(Paragraph {
                lines: vec!["[[bookmark-a]]look here".into()],
                inlines: vec![
                    Inline::Anchor(InlineAnchor {
                        id: "bookmark-a".into(),
                        reftext: None,
                    }),
                    Inline::Text("look here".into()),
                ],
                id: None,
                reftext: None,
            })],
        };

        let html = render_html(&document);
        assert!(html.contains("<a id=\"bookmark-a\"></a>look here"));
    }
}
