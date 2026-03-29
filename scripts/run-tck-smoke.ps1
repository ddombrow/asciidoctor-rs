param(
    [string]$TckRoot = "C:\Users\ddomb\src\asciidoc-tck",
    [string]$TestsDir = "C:\Users\ddomb\src\asciidoctor-rs\tests\tck-smoke",
    [string]$AdapterCommand = "C:\Users\ddomb\src\asciidoctor-rs\scripts\tck-adapter.cmd"
)

$Harness = Join-Path $TckRoot "harness\bin\asciidoc-tck.js"

if (-not (Test-Path $Harness)) {
    Write-Error "AsciiDoc TCK harness not found at $Harness"
    exit 1
}

node $Harness cli --tests=$TestsDir --adapter-command $AdapterCommand
