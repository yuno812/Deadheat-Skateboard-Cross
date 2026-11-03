# シナリオをUnityで再生し、トレースCSVを記録する（照合の「正解」）。
# シナリオは rust/data/scenarios/（dsc gen verify で作る）、出力は rust/data/traces/unity/。
# Unityエディタでこのプロジェクトを開いている間は実行できない。
#
#   pwsh scripts/trace-unity.ps1                       # 全シナリオ
#   pwsh scripts/trace-unity.ps1 -Filter '^S6-zero'    # id の正規表現で絞る
#   pwsh scripts/trace-unity.ps1 -CheckReproducible    # 2回流して、トレースが完全に一致するか確かめる

param(
    [string]$Filter,
    [string]$Out,
    [switch]$CheckReproducible
)

. (Join-Path $PSScriptRoot 'unity-common.ps1')

if (-not $Out) { $Out = Join-Path $ProjectRoot 'rust/data/traces/unity' }

function Invoke-Trace([string]$Dir) {
    $env:DSC_TRACE_OUT = $Dir
    $r = Invoke-UnityTests 'DscSim.TraceTests' 'dsc-trace'
    Select-String -Path $r.Log -Pattern '\[DscSim\] traced ' | ForEach-Object { $_.Line }
    if ($r.ExitCode -ne 0) {
        $r.Failures | ForEach-Object { Write-Host $_ }
        throw "再生に失敗しました（exit $($r.ExitCode)）。ログ: $($r.Log)"
    }
}

Assert-EditorClosed
$savedFilter = $env:DSC_SCENARIO_FILTER
$savedOut = $env:DSC_TRACE_OUT
try {
    $env:DSC_SCENARIO_FILTER = $Filter
    Invoke-Trace $Out

    if ($CheckReproducible) {
        $second = Join-Path ([IO.Path]::GetTempPath()) ("dsc-trace-repro-" + [guid]::NewGuid().ToString('N'))
        try {
            Invoke-Trace $second
            $diff = @(Get-ChildItem $second -Filter *.csv | Where-Object {
                $first = Join-Path $Out $_.Name
                -not (Test-Path $first) -or (Get-FileHash $first).Hash -ne (Get-FileHash $_.FullName).Hash
            } | ForEach-Object { $_.BaseName })
            if ($diff.Count -gt 0) {
                Write-Host "2回目と一致しなかったシナリオ: $($diff.Count)"
                $diff | ForEach-Object { Write-Host "  $_" }
                exit 1
            }
            Write-Host "reproducible: $((Get-ChildItem $second -Filter *.csv).Count) traces matched exactly"
        }
        finally {
            Remove-Item -Recurse -Force $second -ErrorAction SilentlyContinue
        }
    }
}
finally {
    $env:DSC_SCENARIO_FILTER = $savedFilter
    $env:DSC_TRACE_OUT = $savedOut
}
