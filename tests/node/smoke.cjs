const assert = require('node:assert/strict')
const binding = require('../../packages/node')

assert.equal(typeof binding.renderHtml, 'function')
assert.equal(typeof binding.prepareDocumentJson, 'function')
assert.equal(typeof binding.renderTckJson, 'function')
assert.equal(typeof binding.render_html, 'function')
assert.equal(typeof binding.prepare_document_json, 'function')
assert.equal(typeof binding.render_tck_json, 'function')

assert.match(binding.renderHtml('hello _world_'), /<em>world<\/em>/)
assert.match(binding.render_html('hello _world_'), /<em>world<\/em>/)

const prepared = JSON.parse(binding.prepareDocumentJson('= Title\n\n== Section\n\nPara'))
assert.equal(prepared.title, 'Title')

const tck = JSON.parse(binding.renderTckJson('hello _world_'))
assert.equal(tck.type, 'block')
