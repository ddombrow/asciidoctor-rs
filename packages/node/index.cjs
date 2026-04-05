'use strict'

const binding = require('./index.node')

module.exports = {
  ...binding,
  render_html: binding.renderHtml,
  prepare_document_json: binding.prepareDocumentJson,
  render_tck_json: binding.renderTckJson
}
