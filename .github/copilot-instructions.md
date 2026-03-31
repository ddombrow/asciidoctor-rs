# Copilot instructions for `asciidoctor-rs`

## Project context

- This repository is a Rust port of Asciidoctor/Asciidoctor.js. Preserve upstream behavior where practical, but grow support incrementally behind tests.
- Local sibling repositories are part of the working context for parity work:
  - `..\asciidoctor`
  - `..\asciidoc-lang`
  - `..\asciidoc-tck`
  - `..\react-asciidoc`
- Rendering the `.adoc` files under `examples\` is an explicit project goal, not just sample data.

## Build and test commands

### Rust CLI and core tests

- Run the CLI on a sample document:
  - `cargo run -- examples\sample.adoc`
  - `cargo run -- --format json examples\sample.adoc`
  - `cargo run -- --format tck-json examples\sample.adoc`
- Run the full Rust test suite:
  - `cargo test`
- Run a single Rust test:
  - `cargo test renders_links`

### WASM exports and browser-facing build

- Run the wasm-target Rust smoke tests:
  - `cargo test --target wasm32-unknown-unknown --features wasm`
- In this Windows environment, that command builds successfully but needs a configured WASM test runner/browser runner to execute the produced `.wasm` tests. Without that extra runner setup, Cargo can fail at execution time with `os error 193`.
- Run a single wasm-target integration test from `tests\browser_exports.rs`:
  - `cargo test --test browser_exports browser_prepare_document_json_smoke_test --target wasm32-unknown-unknown --features wasm`
- Install the local `wasm-bindgen` binary expected by the browser build:
  - `npm run install:wasm-bindgen`
- Build the browser test package:
  - `npm run build:wasm:test`
- Preview the browser workbench:
  - `npm run preview:browser`
- Run the full Playwright browser suite:
  - `npm run test:browser`
- Run a single Playwright test:
  - `npx playwright test tests\browser\browser.spec.js -g "exports prepared document as JSON"`
- Run the browser suite headed:
  - `npm run test:browser:headed`

### TCK adapter and smoke coverage

- Run the local TCK smoke suite:
  - `npm run test:tck:smoke`
- The smoke runner uses `scripts\run-tck-smoke.mjs` and expects the TCK harness at `..\asciidoc-tck` by default. Override with `ASCIIDOC_TCK_ROOT` if needed.
- To exercise the adapter directly through stdin:
  - `Get-Content request.json | cargo run -- --format tck-json --stdin`

### Build-time environment knobs

- `ASCIIDOCTOR_RS_WASM_BUILD_OFFLINE=1` forces the WASM browser build to stay offline instead of retrying online when Cargo cache entries are missing.

## High-level architecture

- The main document pipeline is:
  1. `src\parser.rs` + `src\inline.rs` parse source into the small Rust AST in `src\ast.rs`.
  2. `src\prepare.rs` converts that AST into the browser-facing prepared model (`DocumentBlock`, `PreparedBlock`, `PreparedInline`).
  3. Downstream consumers use the prepared model:
     - `src\render.rs` renders HTML from the prepared tree.
     - `src\wasm.rs` exposes prepared JSON / JS values across the WASM boundary.
     - `tests\browser\site\app.js` renders the prepared JS value in the browser workbench.

- The prepared model is the main integration contract. Its serialized shape is designed for browser and future `react-asciidoc` interop, so changes in `src\prepare.rs` usually ripple into:
  - `src\render.rs`
  - `src\wasm.rs`
  - `tests\browser\site\app.js`
  - `tests\browser\browser.spec.js`

- The browser preview does **not** use the Rust HTML renderer. Playwright validates the WASM exports plus the JS-side preview renderer in `tests\browser\site\app.js`, while Rust renderer behavior is covered in `src\render.rs` tests.

- `src\tck.rs` is a parallel output pipeline for the AsciiDoc TCK. It has its own ASG/TCK JSON model and block parsing logic, but it reuses the spanned inline parser from `src\inline.rs` for source locations. Changes to headings, lists, or inline spans often need to be reflected in both the main parser/preparer flow and the TCK flow.

## Key conventions

- The first level-0 heading becomes the document header/title, not a section, but only when it appears before other blocks.

- Blocks before the first section are wrapped into a top-level prepared `preamble`. That wrapping happens in `src\prepare.rs`, not in the parser.

- Cross-reference resolution happens during preparation, not during parsing. `src\parser.rs` and `src\inline.rs` keep xrefs as structural nodes; `src\prepare.rs` is where section IDs, block refs, inline anchor refs, and final `href`/display text are resolved.

- Anchor handling is intentionally split:
  - block anchors attach to the next heading or paragraph in `src\parser.rs`
  - inline and phrase anchors are parsed in `src\inline.rs`
  - xref lookup across both kinds of anchors is resolved later in `src\prepare.rs`

- Inline parsing order matters. `src\inline.rs` tries anchor parsing before xrefs, xrefs before links, links before spans, and spans before plain text accumulation. Be careful changing that order because TCK location mapping depends on the spanned inline offsets.

- The prepared JSON/WASM schema is contract-sensitive:
  - structs use `camelCase`
  - block and inline enums serialize with a `"type"` discriminator
  - variant names such as `unordered_list` / `ordered_list` are consumed directly by the browser app

- Several prepared fields are present for forward compatibility but are mostly defaulted today (`attributes`, `footnotes`, `authors`, `numbered`, `num`, `lineNumber`, etc.). Do not treat them as fully implemented features without checking the call sites and tests.

- The browser WASM build writes generated artifacts into `tests\browser\pkg` and copies `examples\sample.adoc` into the browser site. If you change exported WASM names or the prepared JSON shape, update the browser harness and Playwright coverage in the same change.
