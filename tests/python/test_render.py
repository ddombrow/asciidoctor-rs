import json
import asciidoctor_rs

def test_render_html_basic():
    result = asciidoctor_rs.render_html("hello _world_")
    assert "<em>world</em>" in result

def test_prepare_document_json():
    result = asciidoctor_rs.prepare_document_json("= Title\n\n== Section\n\nPara")
    doc = json.loads(result)
    assert doc["title"] == "Title"

def test_render_tck_json():
    result = asciidoctor_rs.render_tck_json("hello _world_")
    data = json.loads(result)
    assert data is not None

def test_render_html_returns_string_on_empty():
    result = asciidoctor_rs.render_html("")
    assert isinstance(result, str)