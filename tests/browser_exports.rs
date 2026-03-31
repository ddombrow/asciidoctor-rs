#![cfg(all(feature = "wasm", target_arch = "wasm32"))]

use js_sys::{Array, Reflect};
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

fn get_property(value: &JsValue, key: &str) -> JsValue {
    Reflect::get(value, &JsValue::from_str(key)).expect("property lookup should succeed")
}

#[wasm_bindgen_test]
fn browser_prepare_document_json_smoke_test() {
    let json =
        asciidoctor_rs::prepare_document_json("= Sample Document\n\n== First Section\n\nHello.\n")
            .expect("json export should succeed");

    assert!(json.contains("\"type\": \"document\""));
    assert!(json.contains("\"hasHeader\": true"));
    assert!(json.contains("\"title\": \"Sample Document\""));
    assert!(json.contains("\"type\": \"section\""));
}

#[wasm_bindgen_test]
fn browser_prepare_document_value_smoke_test() {
    let value =
        asciidoctor_rs::prepare_document_value("= Sample Document\n\n== First Section\n\nHello.\n")
            .expect("value export should succeed");

    assert_eq!(
        get_property(&value, "type").as_string().as_deref(),
        Some("document")
    );
    assert_eq!(
        get_property(&value, "title").as_string().as_deref(),
        Some("Sample Document")
    );

    let sections = Array::from(&get_property(&value, "sections"));
    assert_eq!(sections.length(), 1);

    let first_section = sections.get(0);
    assert_eq!(
        get_property(&first_section, "title").as_string().as_deref(),
        Some("First Section")
    );
    assert_eq!(
        get_property(&first_section, "num").as_string().as_deref(),
        Some("1")
    );
}

#[wasm_bindgen_test]
fn browser_prepare_document_ignores_header_comments() {
    let value = asciidoctor_rs::prepare_document_value(
        "// lead comment\n= Sample Document\n// header comment\n:toc: left\n\nHello.\n",
    )
    .expect("value export should succeed");

    assert_eq!(
        get_property(&value, "title").as_string().as_deref(),
        Some("Sample Document")
    );

    let attributes = get_property(&value, "attributes");
    assert_eq!(
        get_property(&attributes, "toc").as_string().as_deref(),
        Some("left")
    );

    let blocks = Array::from(&get_property(&value, "blocks"));
    assert_eq!(blocks.length(), 1);

    let preamble = blocks.get(0);
    let preamble_blocks = Array::from(&get_property(&preamble, "blocks"));
    let paragraph = preamble_blocks.get(0);
    assert_eq!(
        get_property(&paragraph, "content").as_string().as_deref(),
        Some("Hello.")
    );
}

#[wasm_bindgen_test]
fn browser_prepare_document_exposes_author_attribute() {
    let value = asciidoctor_rs::prepare_document_value(
        "= Sample Document\n:author: Jane Doe\n\nHello.\n",
    )
    .expect("value export should succeed");

    let authors = Array::from(&get_property(&value, "authors"));
    assert_eq!(authors.length(), 1);

    let first_author = authors.get(0);
    assert_eq!(
        get_property(&first_author, "name").as_string().as_deref(),
        Some("Jane Doe")
    );

    let attributes = get_property(&value, "attributes");
    assert_eq!(
        get_property(&attributes, "author").as_string().as_deref(),
        Some("Jane Doe")
    );
}

#[wasm_bindgen_test]
fn browser_prepare_document_exposes_email_attribute() {
    let value = asciidoctor_rs::prepare_document_value(
        "= Sample Document\n:author: Jane Doe\n:email: jane@example.com\n\nHello.\n",
    )
    .expect("value export should succeed");

    let authors = Array::from(&get_property(&value, "authors"));
    assert_eq!(authors.length(), 1);

    let first_author = authors.get(0);
    assert_eq!(
        get_property(&first_author, "name").as_string().as_deref(),
        Some("Jane Doe")
    );
    assert_eq!(
        get_property(&first_author, "email").as_string().as_deref(),
        Some("jane@example.com")
    );

    let attributes = get_property(&value, "attributes");
    assert_eq!(
        get_property(&attributes, "email").as_string().as_deref(),
        Some("jane@example.com")
    );
}
