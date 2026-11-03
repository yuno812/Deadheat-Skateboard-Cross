# CPUシミュレーター用のステージ・キャラのデータを、Unityをバッチモードで動かして書き出す。
# 出力先は rust/data/export/<ステージ>/（環境変数 DSC_DATA で変更可）。
# Unityエディタでこのプロジェクトを開いている間は実行できない。
#
#   pwsh scripts/export-sim-data.ps1
#   $env:UNITY_EXE = 'D:\Unity\6000.1.14f1\Editor\Unity.exe'; pwsh scripts/export-sim-data.ps1

. (Join-Path $PSScriptRoot 'unity-common.ps1')

Assert-EditorClosed
$r = Invoke-UnityTests 'DscSim.ExportTests' 'dsc-export'

Select-String -Path $r.Log -Pattern '\[DscSim\] wrote ' | ForEach-Object { $_.Line }
if ($r.ExitCode -ne 0) {
    $r.Failures | ForEach-Object { Write-Host $_ }
    Write-Error "書き出しに失敗しました（exit $($r.ExitCode)）。ログ: $($r.Log)"
    exit $r.ExitCode
}
Write-Host "done. log: $($r.Log)"
