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
