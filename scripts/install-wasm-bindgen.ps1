$ErrorActionPreference = "Stop"

$Version = "0.2.115"
$Root = Split-Path -Parent $PSScriptRoot
$ToolsDir = Join-Path $Root ".tools\wasm-bindgen"
$ArchiveName = "wasm-bindgen-$Version-x86_64-pc-windows-msvc.tar.gz"
$ArchivePath = Join-Path $ToolsDir $ArchiveName
$Url = "https://github.com/rustwasm/wasm-bindgen/releases/download/$Version/$ArchiveName"

New-Item -ItemType Directory -Path $ToolsDir -Force | Out-Null

Write-Host "Downloading $Url"
Invoke-WebRequest -Uri $Url -OutFile $ArchivePath

Write-Host "Extracting wasm-bindgen"
tar -xzf $ArchivePath -C $ToolsDir

Write-Host "Installed to $ToolsDir"
