# Unityをバッチモードで動かすスクリプトの共通部分。各スクリプトから . で読み込む。

$ErrorActionPreference = 'Stop'
$ProjectRoot = Split-Path -Parent $PSScriptRoot

# ProjectVersion.txt のバージョンを、Unity Hub のインストール先から探す。環境変数 UNITY_EXE があればそちら
function Find-Unity {
    if ($env:UNITY_EXE) { return $env:UNITY_EXE }

    $version = (Select-String -Path (Join-Path $ProjectRoot 'ProjectSettings/ProjectVersion.txt') -Pattern '^m_EditorVersion: (.+)$').Matches[0].Groups[1].Value.Trim()
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

function Assert-EditorClosed {
    $lock = Join-Path $ProjectRoot 'Temp/UnityLockfile'
    if (-not (Test-Path $lock)) { return }
    try { [IO.File]::Open($lock, 'Open', 'ReadWrite', 'None').Close() }
    catch { throw 'Unityエディタがこのプロジェクトを開いています。閉じてから実行してください。' }
}

# EditModeテストを1つの絞り込みで実行する。ログと結果は Logs/<Name>.log, Logs/<Name>-results.xml
function Invoke-UnityTests([string]$Filter, [string]$Name) {
    $unity = Find-Unity
    $logDir = Join-Path $ProjectRoot 'Logs'
    New-Item -ItemType Directory -Force $logDir | Out-Null
    $log = Join-Path $logDir "$Name.log"
    $results = Join-Path $logDir "$Name-results.xml"
    Remove-Item $results -ErrorAction SilentlyContinue

    Write-Host "Unity: $unity ($Filter)"
    $p = Start-Process -FilePath $unity -Wait -PassThru -ArgumentList @(
        '-batchmode', '-nographics',
        '-projectPath', "`"$ProjectRoot`"",
        '-runTests', '-testPlatform', 'EditMode',
        '-testFilter', $Filter,
        '-testResults', "`"$results`"",
        '-logFile', "`"$log`""
    )

    $failures = @()
    if (Test-Path $results) {
        $failures = ([xml](Get-Content $results -Raw)).SelectNodes('//test-case[@result="Failed"]/failure/message') |
            ForEach-Object { $_.InnerText.Trim() }
    }
    [pscustomobject]@{ ExitCode = $p.ExitCode; Log = $log; Failures = $failures }
}
