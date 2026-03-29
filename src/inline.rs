use crate::ast::{Inline, InlineForm, InlineSpan, InlineVariant};

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
                .filter(|ch| is_escapable_char(*ch))
            {
                if text_start.is_none() {
                    text_start = Some(index);
                }
                text.push(escaped);
                index += 2;
                continue;
            }
        }

        if let Some((span, consumed)) = parse_span(chars, index, base) {
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

fn parse_unconstrained_span(
    chars: &[char],
    start: usize,
    base: usize,
    marker: char,
    variant: InlineVariant,
) -> Option<(SpannedInline, usize)> {
    let mut end = start + 2;
    while end + 1 < chars.len() {
        if chars[end] == marker && chars[end + 1] == marker {
            let inner = &chars[start + 2..end];
            if inner.is_empty() || inner.first()?.is_whitespace() || inner.last()?.is_whitespace() {
                return None;
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
    if start > 0 && chars[start - 1].is_alphanumeric() {
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
    use crate::ast::{Inline, InlineForm, InlineVariant};
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
}
