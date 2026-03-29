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
}
