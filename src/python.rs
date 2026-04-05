use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;

#[pyfunction]
fn render_html(input: &str) -> String {
    crate::render_html(&crate::parse_document(input))
}

#[pyfunction]
fn prepare_document_json(input: &str) -> PyResult<String> {
    let document = crate::parse_document(input);
    let prepared = crate::prepare_document(&document);
    crate::prepare::prepared_document_to_json(&prepared)
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

#[pyfunction]
fn render_tck_json(input: &str) -> PyResult<String> {
    crate::render_tck_json(input)
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

#[pymodule]
fn asciidoctor_rs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(render_html, m)?)?;
    m.add_function(wrap_pyfunction!(prepare_document_json, m)?)?;
    m.add_function(wrap_pyfunction!(render_tck_json, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use pyo3::prelude::*;

    #[test]
    fn test_render_html_smoke() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let result = super::render_html("hello _world_");
            assert!(result.contains("world"));
        });
    }
}