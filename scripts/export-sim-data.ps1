# CPUシミュレーター用のステージ・キャラのデータを、Unityをバッチモードで動かして書き出す。
# 出力先は rust/data/export/<ステージ>/（環境変数 DSC_DATA で変更可）。
# Unityエディタでこのプロジェクトを開いている間は実行できない。
#
#   pwsh scripts/export-sim-data.ps1
#   $env:UNITY_EXE = 'D:\Unity\6000.1.14f1\Editor\Unity.exe'; pwsh scripts/export-sim-data.ps1

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot

function Find-Unity {
    if ($env:UNITY_EXE) { return $env:UNITY_EXE }

    $version = (Select-String -Path (Join-Path $root 'ProjectSettings/ProjectVersion.txt') -Pattern '^m_EditorVersion: (.+)$').Matches[0].Groups[1].Value.Trim()
    $editorDirs = @('C:\Program Files\Unity\Hub\Editor')
    $secondary = Join-Path $env:APPDATA 'UnityHub/secondaryInstallPath.json'
    if (Test-Path $secondary) {
        $dir = (Get-Content $secondary -Raw | ConvertFrom-Json)
        if ($dir) { $editorDirs = @($dir) + $editorDirs }
    }
    foreach ($dir in $editorDirs) {
        $exe = Join-Path $dir "$version/Editor/Unity.exe"
        if (Test-Path $exe) { return $exe }
    }
    throw "Unity $version が見つかりません。環境変数 UNITY_EXE で Unity.exe の場所を指定してください。"
}

if (Test-Path (Join-Path $root 'Temp/UnityLockfile')) {
    $lock = Join-Path $root 'Temp/UnityLockfile'
    try { [IO.File]::Open($lock, 'Open', 'ReadWrite', 'None').Close() }
    catch { throw 'Unityエディタがこのプロジェクトを開いています。閉じてから実行してください。' }
}

$unity = Find-Unity
$logDir = Join-Path $root 'Logs'
New-Item -ItemType Directory -Force $logDir | Out-Null
$log = Join-Path $logDir 'dsc-export.log'
$results = Join-Path $logDir 'dsc-export-results.xml'

Write-Host "Unity: $unity"
$p = Start-Process -FilePath $unity -Wait -PassThru -ArgumentList @(
    '-batchmode', '-nographics',
    '-projectPath', "`"$root`"",
    '-runTests', '-testPlatform', 'EditMode',
    '-testFilter', 'DscSim.ExportTests',
    '-testResults', "`"$results`"",
    '-logFile', "`"$log`""
)

Select-String -Path $log -Pattern '\[DscSim\] wrote ' | ForEach-Object { $_.Line }
if ($p.ExitCode -ne 0) {
    Write-Error "書き出しに失敗しました（exit $($p.ExitCode)）。ログ: $log"
    exit $p.ExitCode
}
Write-Host "done. log: $log"
