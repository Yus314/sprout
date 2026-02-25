# SRS アルゴリズム設計

## 基本計算

初期値: `ease = 2.5`, `interval = 1`

```
delayed = max(0, today - next_review)  # レビュー遅延日数

Hard:
  new_interval = (interval + delayed / 4) × 0.5       (min 1)
  new_ease     = ease - 0.15                           (min 1.3)

Good:
  new_interval = (interval + delayed / 2) × ease × 0.8
  new_ease     = ease                                  (変更なし)

Easy:
  new_interval = (interval + delayed) × ease
  new_ease     = ease + 0.15

全評価共通: new_interval は max_interval (デフォルト90日) で上限
```

## 遅延日数の考慮

予定日を過ぎてレビューした場合、遅延日数を次のインターバルに反映する:

- **Easy**: 遅延の全量を加算（遅延があっても理解できた = 十分定着している）
- **Good**: 遅延の半分を加算
- **Hard**: 遅延の1/4を加算（遅延があっても理解が弱い = 最小限のクレジット）

エバーグリーンノートの文脈では、「1週間放置したが再訪時に理解できた」ということは、そのノートは1週間長いインターバルでも大丈夫ということ。これが次のレビューインターバルに反映される。

## リンクファクター

Markdown `[[wiki-link]]` 構文を解析し、ノートの接続度をeaseに反映する:

```
link_count    = ノートからの [[wiki-link]] の数
link_factor   = min(1.0, ln(link_count + 0.5) / ln(64))  # 0.0-1.0に正規化
link_weight   = config.link_weight (デフォルト 0.1)

effective_ease = ease × (1.0 + link_weight × link_factor)
```

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
