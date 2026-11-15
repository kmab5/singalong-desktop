param(
    [Parameter(Mandatory=$true)][ValidatePattern('^2\.[0-9]+\.[0-9]+$')][string]$TauriCliVersion
)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Push-Location -LiteralPath $projectRoot
try {
    cargo install tauri-cli --version "=$TauriCliVersion" --locked
    if ($LASTEXITCODE -ne 0) { throw 'Could not install the selected Tauri CLI.' }
    cargo tauri build --features desktop --bundles nsis --config src-tauri/tauri.conf.json
    if ($LASTEXITCODE -ne 0) { throw 'Windows packaging failed.' }
} finally {
    Pop-Location
}
