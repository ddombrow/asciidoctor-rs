use crate::ast::{Inline, InlineForm, InlineLink, InlineSpan, InlineVariant, InlineXref};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpannedInline {
    pub inline: Inline,
    pub start: usize,
    pub end: usize,
}

pub fn parse_inlines(input: &str) -> Vec<Inline> {
    parse_spanned_inlines(input)
        .into_iter()
        .map(|inline| inline.inline)
        .collect()
}

pub fn parse_spanned_inlines(input: &str) -> Vec<SpannedInline> {
    let chars = input.chars().collect::<Vec<_>>();
    parse_spanned_inlines_with_base(&chars, 0)
}

fn parse_spanned_inlines_with_base(chars: &[char], base: usize) -> Vec<SpannedInline> {
    let mut result = Vec::new();
    let mut text_start = None::<usize>;
    let mut text = String::new();
    let mut index = 0;

    while index < chars.len() {
        if chars[index] == '\\' {
            if let Some(escaped) = chars
                .get(index + 1)
                .copied()
                .filter(|ch| is_escapable_char(*ch) || starts_escaped_link(chars, index))
            {
                if text_start.is_none() {
                    text_start = Some(index);
                }
                text.push(escaped);
                index += 2;
                continue;
            }
        }

        if let Some((xref, consumed)) = parse_xref(chars, index, base) {
            if let Some(start) = text_start.take() {
                result.push(SpannedInline {
                    inline: Inline::Text(std::mem::take(&mut text)),
                    start: base + start,
                    end: base + index,
                });
            }

            result.push(xref);
            index += consumed;
        } else if let Some((link, consumed)) = parse_link(chars, index, base) {
            if let Some(start) = text_start.take() {
                result.push(SpannedInline {
                    inline: Inline::Text(std::mem::take(&mut text)),
                    start: base + start,
                    end: base + index,
                });
            }

            result.push(link);
            index += consumed;
        } else if let Some((span, consumed)) = parse_span(chars, index, base) {
            if let Some(start) = text_start.take() {
                result.push(SpannedInline {
                    inline: Inline::Text(std::mem::take(&mut text)),
                    start: base + start,
                    end: base + index,
                });
            }

            result.push(span);
            index += consumed;
        } else {
            if text_start.is_none() {
                text_start = Some(index);
            }
            text.push(chars[index]);
            index += 1;
        }
    }

    if let Some(start) = text_start {
        result.push(SpannedInline {
            inline: Inline::Text(text),
            start: base + start,
            end: base + chars.len(),
        });
    }

    result
}

fn is_escapable_char(ch: char) -> bool {
    matches!(
        ch,
        '\\' | '*' | '_' | '`' | '[' | ']' | '{' | '}' | '<' | '>'
    )
}

fn starts_escaped_link(chars: &[char], index: usize) -> bool {
    let Some(next) = chars.get(index + 1) else {
        return false;
    };
    *next == 'h' || *next == 'l'
}

fn parse_span(chars: &[char], start: usize, base: usize) -> Option<(SpannedInline, usize)> {
    let marker = *chars.get(start)?;
    let variant = match marker {
        '*' => InlineVariant::Strong,
        '_' => InlineVariant::Emphasis,
        _ => return None,
    };

    if chars.get(start + 1) == Some(&marker) {
        parse_unconstrained_span(chars, start, base, marker, variant)
    } else {
        parse_constrained_span(chars, start, base, marker, variant)
    }
}

fn parse_link(chars: &[char], start: usize, base: usize) -> Option<(SpannedInline, usize)> {
    parse_link_macro(chars, start, base).or_else(|| parse_raw_url(chars, start, base))
}

fn parse_xref(chars: &[char], start: usize, base: usize) -> Option<(SpannedInline, usize)> {
    parse_xref_macro(chars, start, base).or_else(|| parse_xref_shorthand(chars, start, base))
}

fn parse_xref_macro(chars: &[char], start: usize, base: usize) -> Option<(SpannedInline, usize)> {
    const PREFIX: &str = "xref:";
    if !starts_with(chars, start, PREFIX) {
        return None;
    }

    let mut target_end = start + PREFIX.len();
    while target_end < chars.len() && chars[target_end] != '[' && !chars[target_end].is_whitespace()
    {
        target_end += 1;
    }

    if target_end >= chars.len() || chars[target_end] != '[' || target_end == start + PREFIX.len() {
        return None;
    }

    let (text_start, text_end, consumed) = parse_bracket_text(chars, target_end)?;
    let target = chars[start + PREFIX.len()..target_end]
        .iter()
        .collect::<String>();
    let text_source = chars[text_start..text_end].iter().collect::<String>();
    let text = if text_source.is_empty() {
        vec![Inline::Text(target.clone())]
    } else {
        parse_spanned_inlines_with_base(&text_source.chars().collect::<Vec<_>>(), base + text_start)
            .into_iter()
            .map(|inline| inline.inline)
            .collect()
    };

    Some((
        SpannedInline {
            inline: Inline::Xref(InlineXref {
                target,
                text,
                shorthand: false,
                explicit_text: !text_source.is_empty(),
            }),
            start: base + start,
            end: base + consumed,
        },
        consumed - start,
    ))
}

fn parse_xref_shorthand(
    chars: &[char],
    start: usize,
    base: usize,
) -> Option<(SpannedInline, usize)> {
    if chars.get(start) != Some(&'<') || chars.get(start + 1) != Some(&'<') {
        return None;
    }

    let mut end = start + 2;
    while end + 1 < chars.len() {
        if chars[end] == '>' && chars[end + 1] == '>' {
            let inner = chars[start + 2..end].iter().collect::<String>();
            if inner.trim().is_empty() {
                return None;
            }
            let mut parts = inner.splitn(2, ',');
            let target = parts.next()?.trim().to_owned();
            if target.is_empty() {
                return None;
            }
            let text_source = parts.next().map(str::trim).filter(|part| !part.is_empty());
            let text = if let Some(text_source) = text_source {
                parse_spanned_inlines_with_base(
                    &text_source.chars().collect::<Vec<_>>(),
                    base + start + 2 + inner.find(text_source).unwrap_or(0),
                )
                .into_iter()
                .map(|inline| inline.inline)
                .collect()
            } else {
                vec![Inline::Text(target.clone())]
            };

            return Some((
                SpannedInline {
                    inline: Inline::Xref(InlineXref {
                        target,
                        text,
                        shorthand: true,
                        explicit_text: text_source.is_some(),
                    }),
                    start: base + start,
                    end: base + end + 2,
                },
                end + 2 - start,
            ));
        }
        end += 1;
    }

    None
}

fn parse_link_macro(chars: &[char], start: usize, base: usize) -> Option<(SpannedInline, usize)> {
    const PREFIX: &str = "link:";
    if !starts_with(chars, start, PREFIX) {
        return None;
    }

    let mut target_end = start + PREFIX.len();
    while target_end < chars.len() && chars[target_end] != '[' && !chars[target_end].is_whitespace()
    {
        target_end += 1;
    }

    if target_end >= chars.len() || chars[target_end] != '[' || target_end == start + PREFIX.len() {
        return None;
    }

    let (text_start, text_end, consumed) = parse_bracket_text(chars, target_end)?;
    let target = chars[start + PREFIX.len()..target_end]
        .iter()
        .collect::<String>();
    let attrs = parse_link_attrs(&chars[text_start..text_end]);
    let text_source = attrs.text.as_deref().unwrap_or(target.as_str());
    let text_inlines = if text_source.is_empty() {
        vec![Inline::Text(target.clone())]
    } else {
        parse_spanned_inlines_with_base(&text_source.chars().collect::<Vec<_>>(), base + text_start)
            .into_iter()
            .map(|inline| inline.inline)
            .collect()
    };

    Some((
        SpannedInline {
            inline: Inline::Link(InlineLink {
                target,
                text: text_inlines,
                bare: attrs.text.is_none(),
                window: attrs.window,
            }),
            start: base + start,
            end: base + consumed,
        },
        consumed - start,
    ))
}

fn parse_raw_url(chars: &[char], start: usize, base: usize) -> Option<(SpannedInline, usize)> {
    let scheme = if starts_with(chars, start, "http://") {
        "http://"
    } else if starts_with(chars, start, "https://") {
        "https://"
    } else if starts_with(chars, start, "irc://") {
        "irc://"
    } else {
        return None;
    };

    let mut end = start + scheme.len();
    while end < chars.len() && !chars[end].is_whitespace() && chars[end] != '[' {
        end += 1;
    }

    if end == start + scheme.len() {
        return None;
    }

    let mut bare = true;
    let mut target_end = end;
    let text = if end < chars.len() && chars[end] == '[' {
        let (text_start, text_end, consumed) = parse_bracket_text(chars, end)?;
        let attrs = parse_link_attrs(&chars[text_start..text_end]);
        bare = attrs.text.is_none();
        let text = if bare {
            vec![Inline::Text(
                chars[start..target_end].iter().collect::<String>(),
            )]
        } else {
            parse_spanned_inlines_with_base(
                &attrs
                    .text
                    .as_deref()
                    .unwrap_or_default()
                    .chars()
                    .collect::<Vec<_>>(),
                base + text_start,
            )
            .into_iter()
            .map(|inline| inline.inline)
            .collect()
        };
        let window = attrs.window;
        end = consumed;
        (text, window)
    } else {
        while end > start && matches!(chars[end - 1], ',' | '.' | ';' | ':' | ')' | '!' | '?') {
            end -= 1;
        }
        target_end = end;
        (vec![Inline::Text(chars[start..end].iter().collect())], None)
    };
    let (text, window) = text;

    let target = chars[start..target_end].iter().collect::<String>();

    Some((
        SpannedInline {
            inline: Inline::Link(InlineLink {
                target,
                text,
                bare,
                window,
            }),
            start: base + start,
            end: base + end,
        },
        end - start,
    ))
}

fn parse_bracket_text(chars: &[char], open: usize) -> Option<(usize, usize, usize)> {
    if chars.get(open) != Some(&'[') {
        return None;
    }
    let mut index = open + 1;
    while index < chars.len() {
        if chars[index] == ']' {
            return Some((open + 1, index, index + 1));
        }
        index += 1;
    }
    None
}

fn starts_with(chars: &[char], start: usize, pattern: &str) -> bool {
    let pattern = pattern.chars().collect::<Vec<_>>();
    chars.get(start..start + pattern.len()) == Some(pattern.as_slice())
}

struct LinkAttrs {
    text: Option<String>,
    window: Option<String>,
}

fn parse_link_attrs(chars: &[char]) -> LinkAttrs {
    let raw = chars.iter().collect::<String>();
    if raw.is_empty() {
        return LinkAttrs {
            text: None,
            window: None,
        };
    }

    let parts = raw.split(',').map(str::trim).collect::<Vec<_>>();
    let mut text = None;
    let mut window = None;

    for (index, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }

        if index == 0 {
            if let Some(stripped) = part.strip_suffix('^') {
                text = Some(stripped.to_owned());
                window = Some("_blank".into());
                continue;
            }

            if let Some(value) = part.strip_prefix("window=") {
                window = Some(value.trim_matches('"').to_owned());
                continue;
            }

            text = Some((*part).to_owned());
            continue;
        }

        if let Some(value) = part.strip_prefix("window=") {
            window = Some(value.trim_matches('"').to_owned());
        }
    }

    LinkAttrs { text, window }
}

fn parse_unconstrained_span(
    chars: &[char],
    start: usize,
    base: usize,
    marker: char,
    variant: InlineVariant,
) -> Option<(SpannedInline, usize)> {
    let opener = chars.get(start + 2)?;
    if opener.is_whitespace() || *opener == marker {
        return None;
    }

    let mut end = start + 2;
    while end + 1 < chars.len() {
        if chars[end] == marker && chars[end + 1] == marker {
            let inner = &chars[start + 2..end];
            if inner.is_empty()
                || inner.first()?.is_whitespace()
                || inner.last()?.is_whitespace()
                || inner.last() == Some(&marker)
            {
                end += 1;
                continue;
            }

            return Some((
                SpannedInline {
                    inline: Inline::Span(InlineSpan {
                        variant,
                        form: InlineForm::Unconstrained,
                        inlines: parse_spanned_inlines_with_base(inner, base + start + 2)
                            .into_iter()
                            .map(|inline| inline.inline)
                            .collect(),
                    }),
                    start: base + start,
                    end: base + end + 2,
                },
                end + 2 - start,
            ));
        }
        end += 1;
    }

    None
}

fn parse_constrained_span(
    chars: &[char],
    start: usize,
    base: usize,
    marker: char,
    variant: InlineVariant,
) -> Option<(SpannedInline, usize)> {
    let opener = *chars.get(start + 1)?;
    if (start > 0 && chars[start - 1].is_alphanumeric())
        || opener.is_whitespace()
        || opener == marker
    {
        return None;
    }

    let mut end = start + 1;
    while end < chars.len() {
        if chars[end] == marker {
            let inner = &chars[start + 1..end];
            let next = chars.get(end + 1);
            if inner.is_empty()
                || inner.first()?.is_whitespace()
                || inner.last()?.is_whitespace()
                || inner.last() == Some(&marker)
                || next.is_some_and(|ch| ch.is_alphanumeric())
            {
                end += 1;
                continue;
            }

            return Some((
                SpannedInline {
                    inline: Inline::Span(InlineSpan {
                        variant,
                        form: InlineForm::Constrained,
                        inlines: parse_spanned_inlines_with_base(inner, base + start + 1)
                            .into_iter()
                            .map(|inline| inline.inline)
                            .collect(),
                    }),
                    start: base + start,
                    end: base + end + 1,
                },
                end + 1 - start,
            ));
        }
        end += 1;
    }

    None
}

#[cfg(test)]
mod tests {
    use crate::ast::{Inline, InlineForm, InlineLink, InlineSpan, InlineVariant, InlineXref};
    use crate::inline::parse_inlines;

    #[test]
    fn parses_plain_text_when_no_markup_is_present() {
        assert_eq!(parse_inlines("hello"), vec![Inline::Text("hello".into())]);
    }

    #[test]
    fn parses_constrained_strong_text() {
        let inlines = parse_inlines("*strong*");
        let [Inline::Span(span)] = &inlines[..] else {
            panic!("expected strong span");
        };

        assert_eq!(span.variant, InlineVariant::Strong);
        assert_eq!(span.form, InlineForm::Constrained);
        assert_eq!(span.inlines, vec![Inline::Text("strong".into())]);
    }

    #[test]
    fn parses_unconstrained_emphasis_inside_words() {
        let inlines = parse_inlines("before__focus__after");

        assert_eq!(inlines.len(), 3);
    }

    #[test]
    fn keeps_escaped_markup_delimiters_as_literal_text() {
        assert_eq!(
            parse_inlines(r"\*not strong*"),
            vec![Inline::Text("*not strong*".into())]
        );
        assert_eq!(
            parse_inlines(r"\_not emphasis_"),
            vec![Inline::Text("_not emphasis_".into())]
        );
    }

    #[test]
    fn does_not_parse_constrained_spans_inside_words() {
        assert_eq!(
            parse_inlines("foo*bar*baz"),
            vec![Inline::Text("foo*bar*baz".into())]
        );
        assert_eq!(
            parse_inlines("foo_bar_baz"),
            vec![Inline::Text("foo_bar_baz".into())]
        );
    }

    #[test]
    fn does_not_parse_spans_with_whitespace_at_edges() {
        assert_eq!(
            parse_inlines("* not strong*"),
            vec![Inline::Text("* not strong*".into())]
        );
        assert_eq!(
            parse_inlines("*not strong *"),
            vec![Inline::Text("*not strong *".into())]
        );
        assert_eq!(
            parse_inlines("** not strong**"),
            vec![Inline::Text("** not strong**".into())]
        );
        assert_eq!(
            parse_inlines("__not emphasis __"),
            vec![Inline::Text("__not emphasis __".into())]
        );
    }

    #[test]
    fn leaves_unmatched_delimiters_as_literal_text() {
        assert_eq!(parse_inlines("****"), vec![Inline::Text("****".into())]);
        assert_eq!(
            parse_inlines("**x***"),
            vec![
                Inline::Span(InlineSpan {
                    variant: InlineVariant::Strong,
                    form: InlineForm::Unconstrained,
                    inlines: vec![Inline::Text("x".into())],
                }),
                Inline::Text("*".into()),
            ]
        );
        assert_eq!(
            parse_inlines("***x**"),
            vec![
                Inline::Text("*".into()),
                Inline::Span(InlineSpan {
                    variant: InlineVariant::Strong,
                    form: InlineForm::Unconstrained,
                    inlines: vec![Inline::Text("x".into())],
                }),
            ]
        );
    }

    #[test]
    fn parses_bare_urls_as_links() {
        assert_eq!(
            parse_inlines("http://google.com"),
            vec![Inline::Link(InlineLink {
                target: "http://google.com".into(),
                text: vec![Inline::Text("http://google.com".into())],
                bare: true,
                window: None,
            })]
        );
    }

    #[test]
    fn parses_bare_urls_with_link_text() {
        assert_eq!(
            parse_inlines("http://google.com[Google]"),
            vec![Inline::Link(InlineLink {
                target: "http://google.com".into(),
                text: vec![Inline::Text("Google".into())],
                bare: false,
                window: None,
            })]
        );
    }

    #[test]
    fn parses_link_macro_targets() {
        assert_eq!(
            parse_inlines("link:/home.html[Home]"),
            vec![Inline::Link(InlineLink {
                target: "/home.html".into(),
                text: vec![Inline::Text("Home".into())],
                bare: false,
                window: None,
            })]
        );
    }

    #[test]
    fn does_not_include_trailing_punctuation_in_bare_urls() {
        assert_eq!(
            parse_inlines("http://foo.com,"),
            vec![
                Inline::Link(InlineLink {
                    target: "http://foo.com".into(),
                    text: vec![Inline::Text("http://foo.com".into())],
                    bare: true,
                    window: None,
                }),
                Inline::Text(",".into()),
            ]
        );
    }

    #[test]
    fn keeps_escaped_raw_urls_literal() {
        assert_eq!(
            parse_inlines(r"\http://google.com"),
            vec![Inline::Text("http://google.com".into())]
        );
    }

    #[test]
    fn parses_blank_window_shorthand_on_links() {
        assert_eq!(
            parse_inlines("https://example.org[Example^]"),
            vec![Inline::Link(InlineLink {
                target: "https://example.org".into(),
                text: vec![Inline::Text("Example".into())],
                bare: false,
                window: Some("_blank".into()),
            })]
        );
    }

    #[test]
    fn parses_explicit_window_attribute_on_links() {
        assert_eq!(
            parse_inlines("link:/home.html[Home,window=_blank]"),
            vec![Inline::Link(InlineLink {
                target: "/home.html".into(),
                text: vec![Inline::Text("Home".into())],
                bare: false,
                window: Some("_blank".into()),
            })]
        );
    }

    #[test]
    fn parses_shorthand_xrefs() {
        assert_eq!(
            parse_inlines("<<install,Installation>>"),
            vec![Inline::Xref(InlineXref {
                target: "install".into(),
                text: vec![Inline::Text("Installation".into())],
                shorthand: true,
                explicit_text: true,
            })]
        );
    }

    #[test]
    fn parses_xref_macro() {
        assert_eq!(
            parse_inlines("xref:install[Installation]"),
            vec![Inline::Xref(InlineXref {
                target: "install".into(),
                text: vec![Inline::Text("Installation".into())],
                shorthand: false,
                explicit_text: true,
            })]
        );
    }
}
