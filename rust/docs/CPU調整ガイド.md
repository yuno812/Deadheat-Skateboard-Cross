# CPU 調整ガイド

CPU の強さ・難易度・ウルトの使い方を調整する人向けの手引き。仕組みの全体は [DSC CPUシミュレーター仕様書](DSC%20CPUシミュレーター仕様書.md) にある。ここでは「どこを変えれば何が変わり、変えたあと何で確かめるか」をまとめる。

## 1. 全体像

CPU の判断は Rust で書いてあり、DLL（`dsc_ai_native.dll`）にして Unity から呼んでいる。調整した値も DLL に埋め込むので、**値を変えたら DLL を作り直す**（[2. 準備](#2-準備)）。

| 場所 | 中身 |
| --- | --- |
| `rust/crates/dsc-ai/` | CPU の判断。`brain.rs`（行動の決め方）、`ult.rs`（ウルト）、`handicap.rs`（難易度）、`recovery.rs`（倒れたときの起き上がり）、`landing.rs`（着地の姿勢合わせ）、`jump.rs`（ジャンプの軌道予測）、`terrain.rs`（地形） |
| `rust/crates/dsc-ai-ffi/` | DLL の窓口。C# の宣言 `Assets/Code/Cpu/Native/NativeMethods.g.cs` もビルド時にここから生成される |
| `rust/crates/dsc-cli/` | シミュレーターのコマンド `dsc`（実験・計測） |
| `rust/profiles/` | DLL に埋め込む調整済みの値（下の表） |
| `Assets/Plugins/DscAi/x86_64/dsc_ai_native.dll` | ビルドした DLL。git に入れる |
| `Assets/Code/Cpu/CpuController.cs` | CPU のキャラに付くコンポーネント。毎ステップ状況を DLL に渡し、返ってきた操作を入れる |

**調整できるもの**

| 何を | どこで | どう変える |
| --- | --- | --- |
| 難易度（よわい・ふつう・つよい） | `rust/profiles/weak.toml`・`normal.toml`・`strong.toml` | 遊びながら決めて手で書く（[3](#3-難易度の調整)） |
| 強さそのもの（行動の数値、キャラごとの起き上がり・着地） | `rust/profiles/tuned-R2500.toml` | 実験で探して `dsc tune-profile` で書く。手で直してもよい（[4](#4-強さそのものの調整)） |
| ウルトを撃つ条件 | `rust/crates/dsc-ai/src/ult.rs` の定数 | コードを直す（[5](#5-ウルトの使い方の調整)） |
| 重力反転ステージでの動き | `rust/crates/dsc-ai/src/brain.rs` の定数 | コードを直す |

## 2. 準備

- Windows、PowerShell 7（`pwsh`）
- Rust（stable。1.97 で確認）と、MSVC の C++ ビルドツール（シミュレーターが C++ の Box2D をビルドする）
- Unity 6000.1.14f1（`ProjectSettings/ProjectVersion.txt` の版）。`C:\Program Files\Unity\Hub\Editor` か、Unity Hub で設定したインストール先にあれば自動で見つかる。見つからなければ `$env:UNITY_EXE` に Unity.exe のパスを入れる

**DLL を作り直す**（`rust/` の下を変えたら必ず）

```powershell
pwsh scripts/build-ai.ps1
```

- Unity エディタを閉じてから実行する。エディタが一度 DLL を読み込むと、上書きできない
- C# の宣言（`NativeMethods.g.cs`）も作り直される。DLL・toml・コードは同じコミットに入れる

**シミュレーターのコマンドを作る**

```powershell
cd rust
cargo build --release -p dsc-cli
./target/release/dsc.exe          # 使い方の一覧
```

以下 `dsc` と書いたら `rust/target/release/dsc.exe` のこと。結果は `rust/data/results/` に出る（git には入らない）。

## 3. 難易度の調整

### 仕組み

「つよい」は調整済みの CPU そのまま。「ふつう」「よわい」は、判断の前後に3つのハンデを掛けて弱くする（`dsc-ai/src/handicap.rs`）。

| 項目 | 意味 | 効き方 |
| --- | --- | --- |
| `reaction_delay_steps` | 相手の状態を何ステップ遅れて見るか（1 ステップ = 0.02 秒） | 大きいほど、相手の動きへの反応が遅れる。自分の体の立て直しには効かない |
| `aim_noise` | 相手の位置をずらして見る大きさ（標準偏差、単位はステージの長さ。キャラの幅はおよそ 1） | 大きいほど、踏みつけ・弾・突進を外す。ずれは 0.6 秒ごとに引き直す（縦のずれは横の半分） |
| `slip` | 1 ステップごとに、判断を入れ損ねて前のステップの操作のままにする確率（0〜1） | 大きいほど、ジャンプを離すのが遅れる、着地の姿勢合わせがもたつく、ウルトを押し損ねる |

今の値（遊んで決めるための出発点）：

| 難易度 | 反応の遅れ | 狙いのブレ | もたつき |
| --- | --- | --- | --- |
| つよい | 0 | 0 | 0 |
| ふつう | 18 ステップ（0.36 秒） | 1.2 | 0.18 |
| よわい | 36 ステップ（0.72 秒） | 2.4 | 0.36 |

### 遊びながら決める

DLL を作り直さずに、プレイ中に値を変えて試せる。

1. **難易度を選ぶ**：TitleScene の `InputManager` オブジェクトの「Cpu Difficulty」を Weak・Normal・Strong にする（既定は Normal。選ぶ画面はまだ無い）。コードで決めるなら `Assets/Code/Input/InputManager.cs` の `cpuDifficulty` の初期値
2. **CPU と戦う**：デバイス選択の画面で C キーを押すと P2 が CPU になる（キャラは自動で選ばれる）
3. **ハンデを変える**：試合中にヒエラルキーで P2 のキャラ（例：`zero(Clone)`）を選び、`CpuController` の「ハンデ」の3項目を変える。その場で CPU に届き、コンソールに「[CpuController] ハンデを変更」と出る。試合の開始時には、選んだ難易度の値が入っている
4. **値を控える**：プレイモードを止めるとインスペクターの値は消える

### 値を確定させる

1. `rust/profiles/normal.toml`・`weak.toml` に書く（1 行目のコメントも、決めた経緯がわかるように直す）

   ```toml
   # ふつう（2026-11-xx に遊んで決めた）
   reaction_delay_steps = 18
   aim_noise = 1.2
   slip = 0.18
   ```

2. Unity エディタを閉じて `pwsh scripts/build-ai.ps1`
3. `cargo test -p dsc-ai`（よわい のハンデが ふつう 以上であることを確かめるテストがある）
4. 必要なら [6. 確かめ方](#6-確かめ方) の Unity テスト（`EveryDifficultyPlays`）
5. toml と DLL を一緒にコミットする

### シミュレーターで測る

```powershell
dsc exp e6           # 今の値で、難易度どうし・ランダム相手の勝率を測る（data/results/e6-R2500.md）
```

- キャラの組み合わせ全部 × P1/P2 × 6 種の 588 試合ずつ。ウルトは再現しない
- **差は出にくい**。シミュレーターの試合は運の要素が大きく、つよい でもランダム相手に 75% しか勝てない。仕様書の目標（上の難易度が下に 65〜80% で勝つ）は、ハンデを大きくしても届かなかった（つよい 対 ふつう で最大 58%）。そのため値は遊んで決め、E6 は大きく崩れていないかを見る程度に使う
- `dsc exp e6-search` は、目標に向けてハンデの量を探し、**`normal.toml`・`weak.toml` を上書きする**。遊んで決めた値も消えるので注意

## 4. 強さそのものの調整

ゲームの CPU は、全ステージで `rust/profiles/tuned-R2500.toml` の値を使う。

### 行動の数値（`[tactics]`）

| 項目 | 意味 |
| --- | --- |
| `upright_deg` | これより傾いていたら倒れているとみなし、起き上がりに入る（度） |
| `dodge_radius` | 防御：空中の相手の予測着地点が、自分からこの距離以内なら横へ逃げる |
| `stomp_dx` | 踏みつけ：今跳ぶと、相手の真上（横の差がこの値以内）を下りながら通る予測ならジャンプを離す |
| `stomp_dy` | 踏みつけ：そのとき、相手の中心からこの高さの範囲（下限、上限）を通ること |
| `min_hold`・`max_hold` | ジャンプを溜める最短・最長のステップ数（`min_hold` は 2 以上。1 だと跳べない） |
| `tilt_per_unit`・`max_tilt_deg` | 溜めながら相手の方へ傾ける角度：相手との横の距離 1 あたりの度数と、その上限 |
| `travel_dx` | 相手がこれより遠ければ、傾ききったところで離して近づく |
| `bounds_margin` | 場外の境界から、この距離より内側にいる |
| `on_ground_height`・`on_ground_vy` | 床からの高さが「車輪の高さ × `on_ground_height`」より低く、上下の速さが `on_ground_vy` 未満なら、寝ていても地面にいるとみなす |

### キャラごとの値（`[characters.<名前>]`）

- `ride_height`・`center_of_mass`：キャラの形から求めた値（体の中心から車輪の下端まで、重心）。手で直さない
- `recovery`（倒れたときの起き上がり）：`gain` 効きの強さ、`damping` 行き過ぎ防止、`deadband_deg` 何もしない角度の幅、`stall_steps` 真上に近づかなくなってから揺らし始めるまでのステップ数、`timeout_steps` いったん離してやり直すまでのステップ数
- `landing`（空中で着地点の傾きに板を合わせる）：`gain`・`damping`・`deadband_deg`（意味は起き上がりと同じ）

### 変え方

**手で直す**：`tuned-R2500.toml` を直して DLL を作り直す。いちばん手軽。ただし次に `dsc tune-profile` を流すと上書きされる（1 行目に「手で直さない」とあるのはそのため）。手で直した値を残すなら、下の実験の結果のファイルにも同じ値を入れておく。

**実験で探す**：数値をシミュレーターで探し、まとめて書く。順番に流す（ランダム探索なので時間がかかるものがある）。`dsc tune-profile` は 3 つの結果のファイルがそろっていないと失敗する。

```powershell
dsc exp e2          # 起き上がりの数値（data/results/e2-R2500-best.toml）
dsc exp e3          # 着地合わせの数値（e3-R2500-best.toml）
dsc exp e5-search   # 行動の数値（e5-R2500-tactics.toml）
dsc tune-profile    # 3 つをまとめて rust/profiles/tuned-R2500.toml に書く
```

**強さを測る**：

```powershell
dsc exp e5          # CPU 対 何もしない／ランダム／突進／CPU（data/results/e5-R2500.md）
dsc exp e5-trace zero cubu random   # 1 試合の経過を出す（調査用）
```

- `dsc exp e5` は `data/results/` の実験の結果を読む。無ければ `tuned-R2500.toml`（ゲームと同じ値）を使い、そのことを表示する
- 目安：今の値で ランダム相手に 勝ち 71.6%・引き分け 6.3%・負け 22.0%。CPU が受けるダメージの約 6 割は空中にいるとき
- 強さの基準は、実際に遊んだ手応えが最優先（今の「つよい」は、かなりうまい人より強いという評価）

## 5. ウルトの使い方の調整

キャラごとの使い方は `ult.rs` の `ult_kind` で決まる。

| キャラ | 使い方 |
| --- | --- |
| cubu・maiden・meme | ゲージが溜まったらすぐ撃つ |
| hitsuji | 空中ジャンプ。体がほぼ立って下りているときに、場外へ落ちそうなら復帰に、撃てば相手の真上を下りながら通るなら踏みつけに使う |
| zero | 止まる 0.5 秒と弾の時間のあと、弾の線上に相手が来ると予測できたら撃つ。相手が上から近くへ降りてくるときは撃たない |
| nagisa | 突進の途中で自分の板が相手の胴体に最初に当たると予測できたら攻撃に、場外へ落ちそうなら復帰に使う。突進の後も場外に出ないときだけ |
| okuda | ウルトなし |

**調整できる定数**（`rust/crates/dsc-ai/src/ult.rs`）

| 定数 | 今の値 | 意味 |
| --- | --- | --- |
| `AIR_JUMP_UPRIGHT_DEG` | 50 | hitsuji：これより傾いていたら空中ジャンプしない（度） |
| `RISING_HIT_DX`・`RISING_HIT_DY` | 1.0・(-1.6, 0.4) | hitsuji：上りで相手の板に胴体が当たるとみなす範囲 |
| `BULLET_HIT_DIST` | 0.9 | zero：弾の中心と相手の体の線がこれ以内なら当たるとみなす。大きくすると撃つ回数が増え、外れも増える |
| `SHOOT_MAX_S` | 1.2 | zero：これより先（秒）の当たりは狙わない |
| `DASH_MARGIN` | 0.15 | nagisa：相手の板は広め、相手の胴体は狭めに見る余裕。大きくすると慎重になる |
| `DASH_MIN_ABOVE` | 0.3 | nagisa：板が当たる瞬間、自分が相手よりこれだけ高くなければ狙わない |
| `TERRAIN_LOOKUP` | 2.5 | nagisa：突進が地形にぶつかるかを調べるとき、どれだけ上から見るか |

**ゲームの値を写した定数**：`SHOOT_STOP_STEPS`・`BULLET_SPEED`・`BULLET_SPAWN`（zero）、`DASH_SPEED`・`DASH_STEPS`・`DASH_AFTER`（nagisa）、`BOARD`・`BODY`（体の形）は、Prefab とウルトのスクリプトの値をそのまま写したもの。**Prefab 側でウルトの値を変えたら、ここも合わせる**。

**測る**：シミュレーターはウルトを再現しないので、Unity で測る（[6](#6-確かめ方)）。

- `CpuFiresItsUltimate`：cubu・maiden・meme・hitsuji が 40 秒で撃つか
- `DscSim.CpuUltMeasureTests`（計測用、5 分ほど）：ランダムに動く zero 相手に、zero と nagisa を 60 秒 × 20 種。撃った回数・当たった回数・自分もダメージを受けた回数と、ウルトを使わない場合（`CpuController.useUltimate = false`）との与えた／受けたダメージを出す。今の値では zero は 33 回中 25 回、nagisa は 24 回中 14 回当たる

## 6. 確かめ方

変えたら、関係するものを流す。

| 確かめること | コマンド |
| --- | --- |
| Rust のテスト | `cd rust; cargo test --workspace` |
| 警告 | `cargo clippy --workspace --all-targets` |
| ゲームのコードと書き出しデータが、シミュレーターの前提と合っているか | `dsc check` |
| 強さ（ウルトなし） | `dsc exp e5`（前の `data/results/e5-R2500.md` と比べる） |
| 難易度 | `dsc exp e6` |
| ゲームの中での動き | Unity テスト（下） |

**Unity テスト**（Unity エディタを閉じて、リポジトリのルートで）

```powershell
. ./scripts/unity-common.ps1
$r = Invoke-UnityTests 'DscSim.CpuMatchTests' 'cpu-match'
$r.Failures
Select-String -Path $r.Log -Pattern '\[DscSim\]' | ForEach-Object { $_.Line }
```

| テスト（`DscSim.CpuMatchTests.` の後ろ） | 見ているもの |
| --- | --- |
| `CpuDamagesAnIdleOpponent` | CPU が動いて、何もしない相手にダメージを与えるか |
| `CpuFiresItsUltimate` | ウルトを撃つか |
| `CpuKeepsPlayingWithReversedGravity` | 重力反転の Rugby で止まらずに戦えるか |
| `EveryDifficultyPlays` | 3 つの難易度が CPU に渡り、それぞれ戦えるか。プレイ中にハンデを変えると届くか |

- ウルトの当たり具合の計測は別のクラス：`Invoke-UnityTests 'DscSim.CpuUltMeasureTests' 'cpu-aimed'`（[5](#5-ウルトの使い方の調整)）
- 難易度のテスト以外は、判断そのものを見るために「つよい」で流す
- `CpuMatchTests`・`CpuUltMeasureTests` と、データを書き出す `ExportTests`・`TraceTests` は時間がかかるか git のファイルを書き換えるので、Unity の Test Runner の「Run All」では動かない（`Explicit`）。上のように名前を指定するか、Test Runner でテストを選んで実行する。Run All で動くのは `AiCpuTests`（DLL を読み込めるか）だけ
- ログは `Logs/<名前>.log`。Unity のバッチ実行は `.vscode/settings.json` を書き換えるので、終わったら `git checkout -- .vscode/settings.json` で戻す
- hitsuji の試合は Unity の中で毎回少し結果が変わる（ほかのキャラは同じ結果になる）。まれに、テストを続けて流したときだけ別のキャラの結果も変わることがある

## 7. ゲーム側を変えたとき

- **キャラの能力値・当たり判定・ステージを変えた**：Unity エディタを閉じて `pwsh scripts/export-sim-data.ps1` で書き出し直し、`dsc check` で確かめる。形や能力値が大きく変わったなら、[4](#4-強さそのものの調整) の実験を流し直して `dsc tune-profile`
- **`MovePlayer.cs` などシミュレーターに移したコードを変えた**：`dsc check` が「CHANGED」と出す。物理の動きに関わる変更なら `rust/crates/dsc-core/` の移植も直し、関わらない変更なら確かめたうえで `dsc-core/src/source.rs` の `PORTED_SOURCES` のハッシュを `dsc check` が出す値に更新する
- **ウルトの値（弾の速さ、突進の速さなど）を変えた**：`ult.rs` の写した定数を合わせる
- **キャラを足した**：`ult.rs` の `ult_kind` に使い方を足す。`tuned-R2500.toml` に無いキャラは仮の値で動くので、書き出し直して実験を流し直す
- **ステージを足した**：CPU は試合開始時の当たり判定の形を地形として覚える。動く床・フリッパーなど、動く仕掛けの位置は開始時のまま

## 8. 分かっている制限

- シミュレーターはウルトとステージの仕掛け（動く床、フリッパー、バンパー、重力反転）を再現しない。これらは Unity で確かめる
- Pinball では CPU をまだ試していない（Spring は遊んでみて普通に動いていそうとのこと。テストは R2500 と Rugby だけ）
- Rugby：重力が切り替わる線を y = 0 と決め打ちしている。境目をまたぐジャンプの軌道は予測できていない
- 難易度を選ぶ画面はまだ無い（`InputManager.cpuDifficulty` に値を入れれば効く）。LAN 対戦で CPU を使えないようにする処理もまだ
- ゲーム側の挙動で気になった点（CPU の作業とは別）：cubu のウルトの回復が最大 HP を超える。maiden のウルトのエリアが、コメントの「画面半分」ではなく画面全体の大きさ
