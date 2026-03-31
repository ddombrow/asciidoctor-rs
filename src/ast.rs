use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Document {
    pub title: Option<Heading>,
    pub attributes: BTreeMap<String, String>,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Heading(Heading),
    Paragraph(Paragraph),
    UnorderedList(UnorderedList),
    OrderedList(OrderedList),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heading {
    pub level: u8,
    pub title: String,
    pub id: Option<String>,
    pub reftext: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paragraph {
    pub lines: Vec<String>,
    pub inlines: Vec<Inline>,
    pub id: Option<String>,
    pub reftext: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnorderedList {
    pub items: Vec<ListItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderedList {
    pub items: Vec<ListItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListItem {
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inline {
    Text(String),
    Span(InlineSpan),
    Link(InlineLink),
    Xref(InlineXref),
    Anchor(InlineAnchor),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineSpan {
    pub variant: InlineVariant,
    pub form: InlineForm,
    pub inlines: Vec<Inline>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineLink {
    pub target: String,
    pub text: Vec<Inline>,
    pub bare: bool,
    pub window: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineXref {
    pub target: String,
    pub text: Vec<Inline>,
    pub shorthand: bool,
    pub explicit_text: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineAnchor {
    pub id: String,
    pub reftext: Option<String>,
    pub inlines: Vec<Inline>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineVariant {
    Strong,
    Emphasis,
    Monospace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineForm {
    Constrained,
    Unconstrained,
}

impl Inline {
    pub fn plain_text(&self) -> String {
        match self {
            Self::Text(text) => text.clone(),
            Self::Span(span) => span
                .inlines
                .iter()
                .map(Self::plain_text)
                .collect::<Vec<_>>()
                .join(""),
            Self::Link(link) => link
                .text
                .iter()
                .map(Self::plain_text)
                .collect::<Vec<_>>()
                .join(""),
            Self::Xref(xref) => xref
                .text
                .iter()
                .map(Self::plain_text)
                .collect::<Vec<_>>()
                .join(""),
            Self::Anchor(anchor) => anchor
                .inlines
                .iter()
                .map(Self::plain_text)
                .collect::<Vec<_>>()
                .join(""),
        }
    }
}

impl Paragraph {
    pub fn plain_text(&self) -> String {
        self.inlines
            .iter()
            .map(Inline::plain_text)
            .collect::<Vec<_>>()
            .join("")
    }
}
