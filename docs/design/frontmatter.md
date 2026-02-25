# フロントマター形式

## 形式仕様

```yaml
---
maturity: seedling       # seedling | budding | evergreen
created: 2026-02-25
last_review: 2026-02-25
review_interval: 1       # 日数
next_review: 2026-02-26
ease: 2.5                # ease factor
tags:
  - review
---
```

## フィールド定義

| フィールド | 型 | デフォルト | 説明 |
|-----------|-----|-----------|------|
| `maturity` | string | `"seedling"` | ノートの成熟度: `seedling`, `budding`, `evergreen` |
| `created` | date | 初期化日 | ノート作成日 |
| `last_review` | date | 初期化日 | 最後のレビュー日 |
| `review_interval` | u32 | `1` | 現在のレビュー間隔（日数） |
| `next_review` | date | 翌日 | 次のレビュー予定日 |
| `ease` | f64 | `2.5` | ease factor |
| `tags` | list | `["review"]` | タグリスト |

## Obsidian互換性

未知のYAMLキー（`aliases`, `cssclasses` など）はラウンドトリップ時に保持される必要がある。これは二重パーシングにより実現する: `serde_yaml::Value` + 型付きstruct。

## Rust実装

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct SproutFrontmatter {
    pub maturity: Option<String>,          // "seedling" | "budding" | "evergreen"
    pub created: Option<NaiveDate>,
    pub last_review: Option<NaiveDate>,
    pub review_interval: Option<u32>,       // 日数
    pub next_review: Option<NaiveDate>,
    pub ease: Option<f64>,                  // ease factor, default 2.5
    #[serde(default)]
    pub tags: Vec<String>,
}

pub struct ParsedNote {
    pub frontmatter_raw: String,           // 元のYAMLテキスト（ラウンドトリップ用）
    pub frontmatter: serde_yaml::Value,    // 全YAMLバリュー（未知キー保持）
    pub sprout: SproutFrontmatter,         // sproutフィールド抽出
    pub body: String,                      // 閉じ --- 以降のすべて
}
```

## パーシングアルゴリズム

1. ファイルを文字列として読み込み
2. `---\n` で始まるか確認
3. 閉じ `---\n`（2番目の出現）を検索
4. デリミタ間のYAMLを抽出
5. `serde_yaml::Value` にデシリアライズ（全キー保持）
6. `SproutFrontmatter` にオーバーレイデシリアライズ（sproutキー抽出）
7. body（2番目の `---` 以降のすべて）を保存

## 書き戻しアルゴリズム

1. 既存の `serde_yaml::Value` マップを取得
2. sprout関連キーのみ更新
3. YAMLにシリアライズ
4. 再構築: `---\n{yaml}\n---\n{body}`

フロントマターのないファイルはsproutフィールドに `None` を返し、`sprout init` でフロントマターブロックを追加する。
