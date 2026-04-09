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
    let value =
        asciidoctor_rs::prepare_document_value("= Sample Document\n:author: Jane Doe\n\nHello.\n")
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
    assert_eq!(
        get_property(&attributes, "firstname")
            .as_string()
            .as_deref(),
        Some("Jane")
    );
    assert_eq!(
        get_property(&attributes, "lastname").as_string().as_deref(),
        Some("Doe")
    );
    assert_eq!(
        get_property(&attributes, "authorinitials")
            .as_string()
            .as_deref(),
        Some("JD")
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

#[wasm_bindgen_test]
fn browser_prepare_document_exposes_revision_attributes() {
    let value = asciidoctor_rs::prepare_document_value(
        "= Sample Document\n:revnumber: 1.2\n:revdate: 2026-03-31\n:revremark: Draft\n\nHello.\n",
    )
    .expect("value export should succeed");

    let revision = get_property(&value, "revision");
    assert_eq!(
        get_property(&revision, "number").as_string().as_deref(),
        Some("1.2")
    );
    assert_eq!(
        get_property(&revision, "date").as_string().as_deref(),
        Some("2026-03-31")
    );
    assert_eq!(
        get_property(&revision, "remark").as_string().as_deref(),
        Some("Draft")
    );
}

#[wasm_bindgen_test]
fn browser_prepare_document_exposes_implicit_header_metadata() {
    let value = asciidoctor_rs::prepare_document_value(
        "= Sample Document\nStuart Rackham <founder@asciidoc.org>\nv8.6.8, 2012-07-12: See changelog.\n\nHello.\n",
    )
    .expect("value export should succeed");

    let authors = Array::from(&get_property(&value, "authors"));
    assert_eq!(authors.length(), 1);

    let first_author = authors.get(0);
    assert_eq!(
        get_property(&first_author, "name").as_string().as_deref(),
        Some("Stuart Rackham")
    );
    assert_eq!(
        get_property(&first_author, "email").as_string().as_deref(),
        Some("founder@asciidoc.org")
    );

    let revision = get_property(&value, "revision");
    assert_eq!(
        get_property(&revision, "number").as_string().as_deref(),
        Some("8.6.8")
    );
    assert_eq!(
        get_property(&revision, "date").as_string().as_deref(),
        Some("2012-07-12")
    );
    assert_eq!(
        get_property(&revision, "remark").as_string().as_deref(),
        Some("See changelog.")
    );
}

#[wasm_bindgen_test]
fn browser_prepare_document_exposes_multiple_implicit_authors() {
    let value = asciidoctor_rs::prepare_document_value(
        "= Sample Document\nDoc Writer <thedoctor@asciidoc.org>; Junior Writer <junior@asciidoctor.org>\n\nHello.\n",
    )
    .expect("value export should succeed");

    let authors = Array::from(&get_property(&value, "authors"));
    assert_eq!(authors.length(), 2);

    let first_author = authors.get(0);
    assert_eq!(
        get_property(&first_author, "name").as_string().as_deref(),
        Some("Doc Writer")
    );
    assert_eq!(
        get_property(&first_author, "email").as_string().as_deref(),
        Some("thedoctor@asciidoc.org")
    );

    let second_author = authors.get(1);
    assert_eq!(
        get_property(&second_author, "name").as_string().as_deref(),
        Some("Junior Writer")
    );
    assert_eq!(
        get_property(&second_author, "email").as_string().as_deref(),
        Some("junior@asciidoctor.org")
    );
}

#[wasm_bindgen_test]
fn browser_prepare_document_exposes_explicit_authors_metadata() {
    let value = asciidoctor_rs::prepare_document_value(
        "= Sample Document\n:authors: Doc Writer; Other Author\n\nHello.\n",
    )
    .expect("value export should succeed");

    let authors = Array::from(&get_property(&value, "authors"));
    assert_eq!(authors.length(), 2);

    let attributes = get_property(&value, "attributes");
    assert_eq!(
        get_property(&attributes, "firstname")
            .as_string()
            .as_deref(),
        Some("Doc")
    );
    assert_eq!(
        get_property(&attributes, "lastname_2")
            .as_string()
            .as_deref(),
        Some("Author")
    );
    assert_eq!(
        get_property(&attributes, "authorinitials_2")
            .as_string()
            .as_deref(),
        Some("OA")
    );
}

#[wasm_bindgen_test]
fn browser_prepare_document_exposes_delimited_blocks() {
    let value = asciidoctor_rs::prepare_document_value(
        "= Sample Document\n\n----\nputs 'hello'\n----\n\n****\n* phone\n* keys\n****\n\n====\ninside example\n====\n",
    )
    .expect("value export should succeed");

    let blocks = Array::from(&get_property(&value, "blocks"));
    let preamble = blocks.get(0);
    let preamble_blocks = Array::from(&get_property(&preamble, "blocks"));

    let listing = preamble_blocks.get(0);
    assert_eq!(
        get_property(&listing, "type").as_string().as_deref(),
        Some("listing")
    );
    assert_eq!(
        get_property(&listing, "content").as_string().as_deref(),
        Some("puts 'hello'")
    );

    let sidebar = preamble_blocks.get(1);
    assert_eq!(
        get_property(&sidebar, "type").as_string().as_deref(),
        Some("sidebar")
    );

    let example = preamble_blocks.get(2);
    assert_eq!(
        get_property(&example, "type").as_string().as_deref(),
        Some("example")
    );
}

#[wasm_bindgen_test]
fn browser_prepare_document_exposes_delimited_block_metadata() {
    let value = asciidoctor_rs::prepare_document_value(
        "= Sample Document\n\n.Exhibit A\n[source,rust]\n----\nputs 'hello'\n----\n",
    )
    .expect("value export should succeed");

    let blocks = Array::from(&get_property(&value, "blocks"));
    let preamble = blocks.get(0);
    let preamble_blocks = Array::from(&get_property(&preamble, "blocks"));

    let listing = preamble_blocks.get(0);
    assert_eq!(
        get_property(&listing, "title").as_string().as_deref(),
        Some("Exhibit A")
    );
    assert_eq!(
        get_property(&listing, "style").as_string().as_deref(),
        Some("source")
    );

    let attributes = get_property(&listing, "attributes");
    assert_eq!(
        get_property(&attributes, "language").as_string().as_deref(),
        Some("rust")
    );
}

#[wasm_bindgen_test]
fn browser_prepare_document_exposes_admonition_blocks() {
    let value =
        asciidoctor_rs::prepare_document_value("= Sample Document\n\nNOTE: This is just a note.\n")
            .expect("value export should succeed");

    let blocks = Array::from(&get_property(&value, "blocks"));
    let preamble = blocks.get(0);
    let preamble_blocks = Array::from(&get_property(&preamble, "blocks"));

    let admonition = preamble_blocks.get(0);
    assert_eq!(
        get_property(&admonition, "type").as_string().as_deref(),
        Some("admonition")
    );
    assert_eq!(
        get_property(&admonition, "variant").as_string().as_deref(),
        Some("note")
    );
}

#[wasm_bindgen_test]
fn browser_prepare_document_exposes_block_admonitions() {
    let value = asciidoctor_rs::prepare_document_value(
        "= Sample Document\n\n[TIP]\n====\nRemember the milk.\n====\n",
    )
    .expect("value export should succeed");

    let blocks = Array::from(&get_property(&value, "blocks"));
    let preamble = blocks.get(0);
    let preamble_blocks = Array::from(&get_property(&preamble, "blocks"));

    let admonition = preamble_blocks.get(0);
    assert_eq!(
        get_property(&admonition, "type").as_string().as_deref(),
        Some("admonition")
    );
    assert_eq!(
        get_property(&admonition, "variant").as_string().as_deref(),
        Some("tip")
    );
    assert_eq!(
        get_property(&admonition, "style").as_string().as_deref(),
        Some("TIP")
    );
}
