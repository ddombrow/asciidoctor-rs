set shell := ["zsh", "-cu"]

default:
  just --list

bootstrap:
  uv sync --group dev
  npm install
  npm run install:wasm-bindgen
  npx playwright install

fmt:
  cargo fmt

test:
  cargo test
  uv sync --group dev
  uv run pytest tests/python/ --quiet
  npm run test:node

test-rust:
  cargo test

test-python:
  uv sync --group dev
  uv run pytest tests/python/ --quiet

test-node:
  npm run test:node

test-browser:
  npm run test:browser

test-browser-headed:
  npm run test:browser:headed

test-tck:
  npm run test:tck

test-tck-smoke:
  npm run test:tck:smoke

build-node:
  npm run build:node:module

build-python:
  uv run maturin build --features python

build-wasm:
  npm run build:wasm:test

preview-browser:
  npm run preview:browser

install-wasm-bindgen:
  npm run install:wasm-bindgen

install-browser-deps:
  npm install
  npx playwright install

sync-preview-assets:
  npm run sync:preview-assets
