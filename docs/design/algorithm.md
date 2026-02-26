# SRS アルゴリズム設計

## 基本計算

初期値: `ease = 2.5`, `interval = 1`

`today` はシステムローカルタイムの日付（`chrono::Local::now().date_naive()`）とする。フロントマターの日付フィールドは `NaiveDate` であり、タイムゾーン情報を持たない。

計算は全て f64 で行う。`delayed / 4` 等の除算も浮動小数点除算とし、中間結果を整数に丸めない。

```
delayed = max(0, today - next_review)  # レビュー遅延日数 (f64)

Hard:
  new_interval = (interval + delayed / 4) × 0.5       (min 1)
  new_ease     = ease - 0.15                           (min 1.3)

Good:
  new_interval = (interval + delayed / 2) × effective_ease × 0.8
  new_ease     = ease                                  (変更なし)

Easy:
  new_interval = (interval + delayed) × effective_ease
  new_ease     = ease + 0.15

全評価共通:
  new_interval は max_interval (デフォルト90日) で上限
  last_review  = today
```

### インターバルの丸め

`new_interval` の f64 → u32 変換は **負荷分散の直前に1回だけ `round`（四捨五入）** で行う。計算パイプライン中に複数回丸めると誤差が蓄積するため、変換は以下のタイミングで行う:

```
1. effective_ease 算出 → f64（リンクファクターを加味した一時値。格納しない）
2. SRS基本計算         → f64 のまま（interval 式で effective_ease を使用）
3. max_interval clamp  → f64 のまま
4. round → u32         ← ここで1回だけ丸める
5. 負荷分散 fuzzing     → u32 の整数日で範囲計算
6. last_review = today  ← 日付フィールドの更新
7. frontmatter 書き出し → ease = new_ease（rating 調整のみ）, interval, next_review
```

## 遅延日数の考慮

予定日を過ぎてレビューした場合、遅延日数を次のインターバルに反映する:

- **Easy**: 遅延の全量を加算（遅延があっても理解できた = 十分定着している）
- **Good**: 遅延の半分を加算
- **Hard**: 遅延の1/4を加算（遅延があっても理解が弱い = 最小限のクレジット）

エバーグリーンノートの文脈では、「1週間放置したが再訪時に理解できた」ということは、そのノートは1週間長いインターバルでも大丈夫ということ。これが次のレビューインターバルに反映される。

## リンクファクター

ノート本文（frontmatter を除く）からリンクを抽出し、接続度を interval 計算に反映する:

```
link_count    = ノート本文中のユニークな内部リンク数
link_factor   = min(1.0, ln(link_count + 0.5) / ln(64))  # 0.0-1.0に正規化
link_weight   = config.link_weight (デフォルト 0.1)

effective_ease = ease × (1.0 + link_weight × link_factor)
```

`effective_ease` は interval 計算にのみ使用する一時値であり、frontmatter には書き戻さない。frontmatter の `ease` は rating による調整（±0.15）のみを反映する。これにより、リンクファクターが複利的に蓄積することを防ぎ、リンク数の変化が常に現在の状態から正しく反映される。

### カウント規則

- **対象形式**: `[[wiki-link]]` および `[text](path)` の両方
- **display text**: `[[target|display text]]` 形式では `|` 以前の `target` をリンク先として抽出する（`[[foo]]` と `[[foo|Foo]]` は同一リンク先）
- **重複排除**: 同じリンク先が複数回出現してもユニーク数で1と数える。リンク先の文字列をそのまま比較し、パスの正規化（`./` 除去、`..` 解決等）は行わない
- **本文のみ**: YAML frontmatter 内のリンクは除外する
- **外部URL除外**: `http://` または `https://` で始まるリンク先は除外する

### 値の例

| link_count | link_factor | effective_ease (ease=2.5) |
|------------|-------------|--------------------------|
| 0          | 0           | 2.5 (変更なし)            |
| 8          | ≈0.5        | 2.625 (5%増)             |
| 64         | 1.0         | 2.75 (10%増)             |

### 設計根拠

Zettelkassenでは、多くのリンクを持つノートは他のノートとコンテキストを共有しており、再訪時に内容を思い出しやすい。obsidian-srはリンク先ノートの平均easeを参照するが、sproutは単純なリンク数を使用する（vault全体スキャンのコスト削減のため）。

`link_weight=0` に設定するとリンク考慮を無効化できる。

## 負荷分散

計算されたインターバルにファジングを追加し、特定の日にレビューが集中するのを防ぐ:

```
ファジング範囲:
  interval 1-7日    → なし
  interval 8-21日   → ±1日
  interval 22-90日  → ±3日 または interval × 5% の小さい方

ファジング範囲内で、既存のノートの next_review 日付が最も少ない日を選択。
```

### 実装

`sprout done` 実行時に、vault内の全ノートの `next_review` 日付を集計し、ファジング範囲内で最も負荷の低い日に `next_review` を設定する。`review` や `list` でもvault全体スキャンが行われるため、追加コストは最小限。

`config.load_balance = false` の場合、fuzzing をスキップし、丸めた interval をそのまま使用する（`next_review = today + interval`）。
