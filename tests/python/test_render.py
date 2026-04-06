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


def test_block_passthrough_renders_raw_html():
    result = asciidoctor_rs.render_html("++++\n<video src=\"x.mp4\" controls></video>\n++++\n")
    assert "<video src=\"x.mp4\" controls></video>" in result
    assert "&lt;video" not in result


def test_inline_triple_plus_renders_unescaped():
    result = asciidoctor_rs.render_html("See +++<del>this</del>+++ example.\n")
    assert "<del>this</del>" in result
    assert "&lt;del&gt;" not in result


def test_inline_pass_macro_renders_unescaped():
    result = asciidoctor_rs.render_html("See pass:[<br>] here.\n")
    assert "<br>" in result
    assert "&lt;br&gt;" not in result