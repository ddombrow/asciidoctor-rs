# asciidoctor-rs

`asciidoctor-rs` is a Rust port of the upstream
[Asciidoctor](C:\Users\ddomb\src\asciidoctor) project.

## Goals

- preserve Asciidoctor behavior where practical
- grow feature coverage incrementally behind tests
- keep the core parser and document model ergonomic in Rust

## Initial layout

- `src/lib.rs`: library entry points
- `src/ast.rs`: document model types
- `src/parser.rs`: small parser surface to expand over time
- `src/prepare.rs`: WASM-friendly prepared document model
- `src/render.rs`: HTML rendering for the parsed document
- `src/main.rs`: CLI entry point

## Near-term plan

1. Port the document model and line-oriented parser skeleton.
2. Add fixtures based on upstream AsciiDoc samples.
3. Grow the prepared document and HTML renderer toward `react-asciidoc` compatibility.

## Running

```powershell
cargo run -- examples\sample.adoc
cargo run -- --format json examples\sample.adoc
cargo run -- --format tck-json examples\sample.adoc
cargo test
```

## TCK Adapter

There is an initial AsciiDoc TCK adapter mode for block and simple inline parsing. It can read
the TCK stdin envelope and emit ASG JSON on stdout:

```powershell
Get-Content request.json | cargo run -- --format tck-json --stdin
node C:\Users\ddomb\src\asciidoc-tck\harness\bin\asciidoc-tck.js cli --adapter-command "C:\Users\ddomb\src\asciidoctor-rs\scripts\tck-adapter.cmd"
```

There is also a local smoke suite wired to the real TCK harness:

```powershell
npm.cmd run test:tck:smoke
```

## WASM Direction

The prepared document is designed to move across a future WASM boundary in a shape that is
closer to `react-asciidoc`'s `DocumentBlock` and `Block` types. A feature-gated WASM API now
exists for:

- `prepare_document_json(input)`
- `prepare_document_value(input)`

Browser-oriented smoke tests live in `tests/browser_exports.rs` and are intended to run with:

```powershell
cargo test --target wasm32-unknown-unknown --features wasm
```

There is also a Playwright-based browser integration harness that exercises the generated WASM
module through a real page:

```powershell
npm.cmd install
npm.cmd run build:wasm:test
npm.cmd run test:browser
```

The browser build currently expects a locally downloaded `wasm-bindgen.exe`. Install it with:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\install-wasm-bindgen.ps1
```
