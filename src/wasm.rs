#[cfg(feature = "wasm")]
use serde::Serialize as _;
#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn prepare_document_json(input: &str) -> Result<String, JsValue> {
    let document = crate::parse_document(input);
    let prepared = crate::prepare_document(&document);
    crate::prepare::prepared_document_to_json(&prepared)
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn prepare_document_value(input: &str) -> Result<JsValue, JsValue> {
    let document = crate::parse_document(input);
    let prepared = crate::prepare_document(&document);
    let serializer = serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
    prepared
        .serialize(&serializer)
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

#[cfg(all(test, feature = "wasm"))]
mod tests {
    use crate::wasm::prepare_document_json;

    #[test]
    fn wasm_json_export_smoke_test() {
        let json = prepare_document_json(
            "= Sample Document\n\n== First Section\n\nA paragraph in the first section.\n",
        )
        .expect("json export should succeed");

        assert!(json.contains("\"type\": \"document\""));
        assert!(json.contains("\"hasHeader\": true"));
        assert!(json.contains("\"title\": \"Sample Document\""));
        assert!(json.contains("\"type\": \"section\""));
    }

    #[cfg(target_arch = "wasm32")]
    use crate::prepare::{DocumentBlock, PreparedBlock};
    #[cfg(target_arch = "wasm32")]
    use crate::wasm::prepare_document_value;

    #[cfg(target_arch = "wasm32")]
    #[test]
    fn wasm_value_export_round_trips_prepared_document() {
        let value = prepare_document_value(
            "= Sample Document\n\n== First Section\n\nA paragraph in the first section.\n",
        )
        .expect("value export should succeed");

        let document: DocumentBlock =
            serde_wasm_bindgen::from_value(value).expect("value should deserialize");

        assert_eq!(document.node_type, "document");
        assert_eq!(document.title, "Sample Document");
        assert_eq!(document.sections.len(), 1);

        let PreparedBlock::Section(section) = &document.blocks[0] else {
            panic!("expected top-level section block");
        };

        assert_eq!(section.title, "First Section");
        assert_eq!(section.num, "1");
    }
}
