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
    Admonition(AdmonitionBlock),
    UnorderedList(UnorderedList),
    OrderedList(OrderedList),
    Table(TableBlock),
    Listing(Listing),
    Example(CompoundBlock),
    Sidebar(CompoundBlock),
    Passthrough(String),
    Image(ImageBlock),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageBlock {
    pub target: String,
    pub alt: String,
    pub width: Option<String>,
    pub height: Option<String>,
    pub metadata: BlockMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heading {
    pub level: u8,
    pub title: String,
    pub id: Option<String>,
    pub reftext: Option<String>,
    pub metadata: BlockMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paragraph {
    pub lines: Vec<String>,
    pub inlines: Vec<Inline>,
    pub id: Option<String>,
    pub reftext: Option<String>,
    pub metadata: BlockMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmonitionBlock {
    pub variant: AdmonitionVariant,
    pub blocks: Vec<Block>,
    pub id: Option<String>,
    pub reftext: Option<String>,
    pub metadata: BlockMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listing {
    pub lines: Vec<String>,
    pub reftext: Option<String>,
    pub metadata: BlockMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompoundBlock {
    pub blocks: Vec<Block>,
    pub reftext: Option<String>,
    pub metadata: BlockMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableBlock {
    pub header: Option<TableRow>,
    pub rows: Vec<TableRow>,
    pub reftext: Option<String>,
    pub metadata: BlockMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableRow {
    pub cells: Vec<TableCell>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableCell {
    pub content: String,
    pub inlines: Vec<Inline>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BlockMetadata {
    pub id: Option<String>,
    pub title: Option<String>,
    pub style: Option<String>,
    pub role: Option<String>,
    pub attributes: BTreeMap<String, String>,
    pub options: Vec<String>,
    pub roles: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnorderedList {
    pub items: Vec<ListItem>,
    pub reftext: Option<String>,
    pub metadata: BlockMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderedList {
    pub items: Vec<ListItem>,
    pub reftext: Option<String>,
    pub metadata: BlockMetadata,
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
    Passthrough(String),
    Image(InlineImage),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineImage {
    pub target: String,
    pub alt: String,
    pub width: Option<String>,
    pub height: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmonitionVariant {
    Note,
    Tip,
    Important,
    Caution,
    Warning,
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
            Self::Passthrough(raw) => raw.clone(),
            Self::Image(image) => image.alt.clone(),
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

impl AdmonitionVariant {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Note => "note",
            Self::Tip => "tip",
            Self::Important => "important",
            Self::Caution => "caution",
            Self::Warning => "warning",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Note => "Note",
            Self::Tip => "Tip",
            Self::Important => "Important",
            Self::Caution => "Caution",
            Self::Warning => "Warning",
        }
    }
}
