pub mod ast;
pub mod inline;
pub mod parser;
pub mod prepare;
pub mod preprocessor;
pub mod render;
pub mod tck;

#[cfg(feature = "wasm")]
pub mod wasm;

#[cfg(feature = "python")]
pub mod python;

#[cfg(feature = "node")]
pub mod node;

pub use ast::{
    AdmonitionBlock, AdmonitionVariant, Block, Document, Heading, Inline, InlineAnchor,
    InlineFootnote, InlineForm, InlineImage, InlineLink, InlineSpan, InlineVariant, InlineXref,
    Paragraph,
};
pub use inline::{SpannedInline, parse_inlines, parse_spanned_inlines};
pub use parser::parse_document;
pub use prepare::{
    AdmonitionBlock as PreparedAdmonitionBlock, AnchorInline, Author, CompoundBlock, DocumentBlock,
    DocumentSection, Footnote, FootnoteInline, ImageBlock, ImageInline, LinkInline, ParagraphBlock,
    PassthroughBlock, PassthroughInline, PreparedBlock, PreparedInline, Revision, SectionBlock,
    SpanInline, TableBlock, TableCell, TableRow, TextInline, XrefInline, prepare_document,
    prepared_document_to_json,
};
pub use preprocessor::preprocess;
pub use render::{render_html, render_prepared_html};
pub use tck::{
    parse_tck_document, parse_tck_inlines, render_tck_inline_json, render_tck_json,
    render_tck_json_from_request,
};

#[cfg(feature = "wasm")]
pub use wasm::{prepare_document_json, prepare_document_value};
