param([string]$Executable = 'src-tauri/target/debug/examples/window_probe.exe')
$ErrorActionPreference = 'Stop'
$sdkRoot = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits/10/bin'
$manifestTool = Get-ChildItem -LiteralPath $sdkRoot -Filter mt.exe -Recurse |
    Where-Object { $_.Directory.Name -eq 'x64' } |
    Sort-Object FullName | Select-Object -Last 1
if (-not $manifestTool) { throw 'Windows SDK manifest tool was not found.' }
$binary = (Resolve-Path -LiteralPath $Executable).Path
$artifactRoot = Join-Path $PSScriptRoot '../artifacts'
New-Item -ItemType Directory -Path $artifactRoot -Force | Out-Null
$manifestPath = Join-Path (Resolve-Path -LiteralPath $artifactRoot).Path 'native.manifest'
& $manifestTool.FullName "-inputresource:$binary;#1" "-out:$manifestPath"
if ($LASTEXITCODE -ne 0) { throw 'Native executable is missing its embedded manifest.' }
[xml]$manifest = Get-Content -LiteralPath $manifestPath -Raw
$controls = $manifest.SelectSingleNode("//*[local-name()='assemblyIdentity' and @name='Microsoft.Windows.Common-Controls' and @version='6.0.0.0']")
if (-not $controls) { throw 'Common Controls v6 dependency is missing from the native manifest.' }
Write-Output 'Native manifest: Common Controls v6 verified.'
