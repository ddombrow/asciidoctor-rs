use crate::ast::{
    Inline, InlineAnchor, InlineFootnote, InlineForm, InlineImage, InlineLink, InlineSpan,
    InlineVariant, InlineXref,
};

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

        if let Some((passthrough, consumed)) = parse_passthrough(chars, index, base) {
            if let Some(start) = text_start.take() {
                result.push(SpannedInline {
                    inline: Inline::Text(std::mem::take(&mut text)),
                    start: base + start,
                    end: base + index,
                });
            }

            result.push(passthrough);
            index += consumed;
        } else if let Some((anchor, consumed)) = parse_inline_anchor(chars, index, base) {
            if let Some(start) = text_start.take() {
                result.push(SpannedInline {
                    inline: Inline::Text(std::mem::take(&mut text)),
                    start: base + start,
                    end: base + index,
                });
            }

            result.push(anchor);
            index += consumed;
        } else if let Some((xref, consumed)) = parse_xref(chars, index, base) {
            if let Some(start) = text_start.take() {
                result.push(SpannedInline {
                    inline: Inline::Text(std::mem::take(&mut text)),
                    start: base + start,
                    end: base + index,
                });
            }

            result.push(xref);
            index += consumed;
        } else if let Some((footnote, consumed)) = parse_footnote(chars, index, base) {
            if let Some(start) = text_start.take() {
                result.push(SpannedInline {
                    inline: Inline::Text(std::mem::take(&mut text)),
                    start: base + start,
                    end: base + index,
                });
            }

            result.push(footnote);
            index += consumed;
        } else if let Some((link, consumed)) = parse_inline_image(chars, index, base) {
            if let Some(start) = text_start.take() {
                result.push(SpannedInline {
                    inline: Inline::Text(std::mem::take(&mut text)),
                    start: base + start,
                    end: base + index,
                });
            }

            result.push(link);
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
        '`' => InlineVariant::Monospace,
        '~' => InlineVariant::Subscript,
        '^' => InlineVariant::Superscript,
        _ => return None,
    };

    // Subscript and superscript are constrained-only (no ~~ or ^^ unconstrained form)
    if matches!(variant, InlineVariant::Subscript | InlineVariant::Superscript) {
        return parse_constrained_span(chars, start, base, marker, variant);
    }

    if chars.get(start + 1) == Some(&marker) {
        parse_unconstrained_span(chars, start, base, marker, variant)
    } else {
        parse_constrained_span(chars, start, base, marker, variant)
    }
}

fn parse_link(chars: &[char], start: usize, base: usize) -> Option<(SpannedInline, usize)> {
    parse_link_macro(chars, start, base).or_else(|| parse_raw_url(chars, start, base))
}

fn parse_inline_image(chars: &[char], start: usize, base: usize) -> Option<(SpannedInline, usize)> {
    // Must match `image:target[attrs]` — single colon only (block uses `::`)
    let prefix: Vec<char> = "image:".chars().collect();
    if start + prefix.len() >= chars.len() {
        return None;
    }
    for (i, &expected) in prefix.iter().enumerate() {
        if chars.get(start + i).copied() != Some(expected) {
            return None;
        }
    }
    // Reject block form `image::` — that is parsed at block level
    if chars.get(start + prefix.len()).copied() == Some(':') {
        return None;
    }

    let target_start = start + prefix.len();
    let mut i = target_start;
    while i < chars.len() && chars[i] != '[' {
        i += 1;
    }
    if i >= chars.len() || chars[i] != '[' {
        return None;
    }
    let target: String = chars[target_start..i].iter().collect();
    let target = target.trim().to_owned();
    if target.is_empty() {
        return None;
    }

    let bracket_start = i;
    i += 1;
    let mut depth = 1;
    while i < chars.len() && depth > 0 {
        if chars[i] == '[' {
            depth += 1;
        } else if chars[i] == ']' {
            depth -= 1;
        }
        i += 1;
    }
    if depth != 0 {
        return None;
    }

    let attr_text: String = chars[bracket_start + 1..i - 1].iter().collect();
    let (alt, width, height) = parse_inline_image_attributes(&attr_text, &target);

    let consumed = i - start;
    Some((
        SpannedInline {
            inline: Inline::Image(InlineImage {
                target,
                alt,
                width,
                height,
            }),
            start: base + start,
            end: base + i,
        },
        consumed,
    ))
}

fn parse_inline_image_attributes(
    attr_text: &str,
    target: &str,
) -> (String, Option<String>, Option<String>) {
    let mut positional = Vec::new();

    if !attr_text.is_empty() {
        for part in attr_text.split(',') {
            let part = part.trim();
            if part.contains('=') {
                // Named attribute — skip for inline images (we don't use them yet)
                continue;
            }
            positional.push(part.to_owned());
        }
    }

    let alt = positional
        .first()
        .filter(|s| !s.is_empty())
        .cloned()
        .unwrap_or_else(|| inline_auto_generate_alt(target));
    let width = positional.get(1).filter(|s| !s.is_empty()).cloned();
    let height = positional.get(2).filter(|s| !s.is_empty()).cloned();

    (alt, width, height)
}

fn inline_auto_generate_alt(target: &str) -> String {
    let filename = target.rsplit('/').next().unwrap_or(target);
    let filename = filename.rsplit('\\').next().unwrap_or(filename);
    let stem = filename.rsplit_once('.').map(|(s, _)| s).unwrap_or(filename);
    stem.replace('-', " ").replace('_', " ")
}

fn parse_xref(chars: &[char], start: usize, base: usize) -> Option<(SpannedInline, usize)> {
    parse_xref_macro(chars, start, base).or_else(|| parse_xref_shorthand(chars, start, base))
}

fn parse_footnote(chars: &[char], start: usize, base: usize) -> Option<(SpannedInline, usize)> {
    const PREFIX: &str = "footnote:[";
    if !starts_with(chars, start, PREFIX) {
        return None;
    }

    let (text_start, text_end, consumed) = parse_bracket_text(chars, start + PREFIX.len() - 1)?;
    let source = chars[text_start..text_end].iter().collect::<String>();
    let inlines = parse_spanned_inlines_with_base(&source.chars().collect::<Vec<_>>(), base + text_start)
        .into_iter()
        .map(|inline| inline.inline)
        .collect();

    Some((
        SpannedInline {
            inline: Inline::Footnote(InlineFootnote { inlines }),
            start: base + start,
            end: base + consumed,
        },
        consumed - start,
    ))
}

fn parse_inline_anchor(
    chars: &[char],
    start: usize,
    base: usize,
) -> Option<(SpannedInline, usize)> {
    parse_phrase_anchor(chars, start, base)
        .or_else(|| parse_inline_anchor_macro(chars, start, base))
        .or_else(|| parse_inline_anchor_brackets(chars, start, base))
}

fn parse_phrase_anchor(
    chars: &[char],
    start: usize,
    base: usize,
) -> Option<(SpannedInline, usize)> {
    if !starts_with(chars, start, "[#") {
        return None;
    }

    let mut attr_end = start + 2;
    while attr_end < chars.len() && chars[attr_end] != ']' {
        attr_end += 1;
    }
    if attr_end >= chars.len() || chars.get(attr_end + 1) != Some(&'#') {
        return None;
    }

    let inner = chars[start + 2..attr_end].iter().collect::<String>();
    let mut parts = inner.split(',').map(str::trim);
    let id = parts.next()?;
    if id.is_empty() || !is_valid_anchor_id(id) {
        return None;
    }

    let reftext = parts
        .find_map(|part| {
            part.strip_prefix("reftext=")
                .map(|value| value.trim().trim_matches('"').to_owned())
        })
        .filter(|value| !value.is_empty());

    let mut text_end = attr_end + 2;
    while text_end < chars.len() {
        if chars[text_end] == '#' {
            let source = chars[attr_end + 2..text_end].iter().collect::<String>();
            let inlines = parse_spanned_inlines_with_base(
                &source.chars().collect::<Vec<_>>(),
                base + attr_end + 2,
            )
            .into_iter()
            .map(|inline| inline.inline)
            .collect();

            return Some((
                SpannedInline {
                    inline: Inline::Anchor(InlineAnchor {
                        id: id.to_owned(),
                        reftext,
                        inlines,
                    }),
                    start: base + start,
                    end: base + text_end + 1,
                },
                text_end + 1 - start,
            ));
        }
        text_end += 1;
    }

    None
}

fn parse_inline_anchor_macro(
    chars: &[char],
    start: usize,
    base: usize,
) -> Option<(SpannedInline, usize)> {
    const PREFIX: &str = "anchor:";
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
    let id = chars[start + PREFIX.len()..target_end]
        .iter()
        .collect::<String>();
    if !is_valid_anchor_id(&id) {
        return None;
    }

    let reftext = chars[text_start..text_end]
        .iter()
        .collect::<String>()
        .trim()
        .to_owned();

    Some((
        SpannedInline {
            inline: Inline::Anchor(InlineAnchor {
                id,
                reftext: if reftext.is_empty() {
                    None
                } else {
                    Some(reftext)
                },
                inlines: Vec::new(),
            }),
            start: base + start,
            end: base + consumed,
        },
        consumed - start,
    ))
}

fn parse_inline_anchor_brackets(
    chars: &[char],
    start: usize,
    base: usize,
) -> Option<(SpannedInline, usize)> {
    if chars.get(start) != Some(&'[') || chars.get(start + 1) != Some(&'[') {
        return None;
    }

    let mut end = start + 2;
    while end + 1 < chars.len() {
        if chars[end] == ']' && chars[end + 1] == ']' {
            let inner = chars[start + 2..end].iter().collect::<String>();
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

            return Some((
                SpannedInline {
                    inline: Inline::Anchor(InlineAnchor {
                        id: id.to_owned(),
                        reftext,
                        inlines: Vec::new(),
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

fn is_valid_anchor_id(id: &str) -> bool {
    id.chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | ':' | '.'))
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

fn parse_passthrough(chars: &[char], start: usize, base: usize) -> Option<(SpannedInline, usize)> {
    parse_triple_plus_passthrough(chars, start, base)
        .or_else(|| parse_pass_macro(chars, start, base))
}

fn parse_triple_plus_passthrough(
    chars: &[char],
    start: usize,
    base: usize,
) -> Option<(SpannedInline, usize)> {
    if !starts_with(chars, start, "+++") {
        return None;
    }

    let content_start = start + 3;
    let mut index = content_start;
    while index + 2 < chars.len() {
        if chars[index] == '+' && chars[index + 1] == '+' && chars[index + 2] == '+' {
            let raw = chars[content_start..index].iter().collect::<String>();
            let end = index + 3;
            return Some((
                SpannedInline {
                    inline: Inline::Passthrough(raw),
                    start: base + start,
                    end: base + end,
                },
                end - start,
            ));
        }
        index += 1;
    }

    None
}

fn parse_pass_macro(chars: &[char], start: usize, base: usize) -> Option<(SpannedInline, usize)> {
    const PREFIX: &str = "pass:[";
    if !starts_with(chars, start, PREFIX) {
        return None;
    }

    let content_start = start + PREFIX.len();
    let mut index = content_start;
    while index < chars.len() {
        if chars[index] == ']' {
            let raw = chars[content_start..index].iter().collect::<String>();
            let end = index + 1;
            return Some((
                SpannedInline {
                    inline: Inline::Passthrough(raw),
                    start: base + start,
                    end: base + end,
                },
                end - start,
            ));
        }
        index += 1;
    }

    None
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
    // Subscript (~) and superscript (^) are designed to sit adjacent to alphanumeric
    // characters (e.g. H~2~O, E=mc^2^), so the word-boundary constraints do not apply.
    let adjacent_ok = matches!(variant, InlineVariant::Subscript | InlineVariant::Superscript);

    let opener = *chars.get(start + 1)?;
    if (!adjacent_ok && start > 0 && chars[start - 1].is_alphanumeric())
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
                || (!adjacent_ok && next.is_some_and(|ch| ch.is_alphanumeric()))
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
    use crate::ast::{
        Inline, InlineAnchor, InlineFootnote, InlineForm, InlineLink, InlineSpan, InlineVariant,
        InlineXref,
    };
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
    fn parses_constrained_monospace_text() {
        let inlines = parse_inlines("`cargo test`");
        let [Inline::Span(span)] = &inlines[..] else {
            panic!("expected monospace span");
        };

        assert_eq!(span.variant, InlineVariant::Monospace);
        assert_eq!(span.form, InlineForm::Constrained);
        assert_eq!(span.inlines, vec![Inline::Text("cargo test".into())]);
    }

    #[test]
    fn parses_unconstrained_monospace_inside_words() {
        let inlines = parse_inlines("re``link``ed");

        assert_eq!(
            inlines,
            vec![
                Inline::Text("re".into()),
                Inline::Span(InlineSpan {
                    variant: InlineVariant::Monospace,
                    form: InlineForm::Unconstrained,
                    inlines: vec![Inline::Text("link".into())],
                }),
                Inline::Text("ed".into()),
            ]
        );
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

    #[test]
    fn parses_footnote_macro() {
        assert_eq!(
            parse_inlines("Look herefootnote:[Read *this* first.]"),
            vec![
                Inline::Text("Look here".into()),
                Inline::Footnote(InlineFootnote {
                    inlines: vec![
                        Inline::Text("Read ".into()),
                        Inline::Span(InlineSpan {
                            variant: InlineVariant::Strong,
                            form: InlineForm::Constrained,
                            inlines: vec![Inline::Text("this".into())],
                        }),
                        Inline::Text(" first.".into()),
                    ],
                }),
            ]
        );
    }

    #[test]
    fn parses_inline_anchor_brackets() {
        assert_eq!(
            parse_inlines("[[bookmark-a]]Inline"),
            vec![
                Inline::Anchor(InlineAnchor {
                    id: "bookmark-a".into(),
                    reftext: None,
                    inlines: Vec::new(),
                }),
                Inline::Text("Inline".into()),
            ]
        );
    }

    #[test]
    fn parses_inline_anchor_macro() {
        assert_eq!(
            parse_inlines("anchor:bookmark-c[Label]Use"),
            vec![
                Inline::Anchor(InlineAnchor {
                    id: "bookmark-c".into(),
                    reftext: Some("Label".into()),
                    inlines: Vec::new(),
                }),
                Inline::Text("Use".into()),
            ]
        );
    }

    #[test]
    fn parses_phrase_applied_inline_anchor() {
        assert_eq!(
            parse_inlines("[#bookmark-b]#visible text#"),
            vec![Inline::Anchor(InlineAnchor {
                id: "bookmark-b".into(),
                reftext: None,
                inlines: vec![Inline::Text("visible text".into())],
            })]
        );
    }

    #[test]
    fn parses_triple_plus_passthrough() {
        assert_eq!(
            parse_inlines("+++<del>strike</del>+++"),
            vec![Inline::Passthrough("<del>strike</del>".into())]
        );
    }

    #[test]
    fn parses_triple_plus_passthrough_mid_sentence() {
        assert_eq!(
            parse_inlines("See +++<br>+++ here."),
            vec![
                Inline::Text("See ".into()),
                Inline::Passthrough("<br>".into()),
                Inline::Text(" here.".into()),
            ]
        );
    }

    #[test]
    fn parses_pass_macro() {
        assert_eq!(
            parse_inlines("pass:[<br>]"),
            vec![Inline::Passthrough("<br>".into())]
        );
    }

    #[test]
    fn parses_pass_macro_mid_sentence() {
        assert_eq!(
            parse_inlines("before pass:[<em>raw</em>] after"),
            vec![
                Inline::Text("before ".into()),
                Inline::Passthrough("<em>raw</em>".into()),
                Inline::Text(" after".into()),
            ]
        );
    }
}
