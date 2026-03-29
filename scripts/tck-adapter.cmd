@echo off
setlocal
cargo run --quiet -- --format tck-json --stdin
