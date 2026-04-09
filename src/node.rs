use napi::Error;
use napi::bindgen_prelude::Result;
use napi_derive::napi;

#[napi]
pub fn render_html(input: String) -> String {
    crate::render_html(&crate::parse_document(&input))
}

#[napi]
pub fn prepare_document_json(input: String) -> Result<String> {
    let document = crate::parse_document(&input);
    let prepared = crate::prepare_document(&document);
    crate::prepare::prepared_document_to_json(&prepared)
        .map_err(|error| Error::from_reason(error.to_string()))
}

#[napi]
pub fn render_tck_json(input: String) -> Result<String> {
    crate::render_tck_json(&input).map_err(|error| Error::from_reason(error.to_string()))
}
