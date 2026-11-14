# CPUのDLL（dsc_ai_native.dll）をビルドし、Unityのプラグインフォルダへコピーする。
# C#の宣言（Assets/Code/Cpu/Native/NativeMethods.g.cs）もビルド時に生成し直される。
#
#   pwsh scripts/build-ai.ps1           # release
#   pwsh scripts/build-ai.ps1 -Debug    # debug（panicの場所を追いたいとき）

param([switch]$Debug)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$manifest = Join-Path $root 'rust/Cargo.toml'
$profileName = if ($Debug) { 'debug' } else { 'release' }

$cargoArgs = @('build', '-p', 'dsc-ai-ffi', '--manifest-path', $manifest)
if (-not $Debug) { $cargoArgs += '--release' }
& cargo @cargoArgs
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$src = Join-Path $root "rust/target/$profileName/dsc_ai_native.dll"
$destDir = Join-Path $root 'Assets/Plugins/DscAi/x86_64'
New-Item -ItemType Directory -Force $destDir | Out-Null

try {
    Copy-Item $src (Join-Path $destDir 'dsc_ai_native.dll') -Force
}
catch [System.IO.IOException] {
    Write-Error "DLLを上書きできませんでした。Unityエディタが読み込んだままの可能性があります。エディタを閉じてからもう一度実行してください。`n$_"
    exit 1
}

Write-Host "copied $profileName dsc_ai_native.dll -> $destDir"
