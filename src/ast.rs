#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Document {
    pub title: Option<Heading>,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Heading(Heading),
    Paragraph(Paragraph),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heading {
    pub level: u8,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paragraph {
    pub lines: Vec<String>,
    pub inlines: Vec<Inline>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inline {
    Text(String),
    Span(InlineSpan),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineSpan {
    pub variant: InlineVariant,
    pub form: InlineForm,
    pub inlines: Vec<Inline>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineVariant {
    Strong,
    Emphasis,
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
