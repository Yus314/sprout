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

未知のYAMLキー（`aliases`, `cssclasses` など）、コメント、キー順序、クォートスタイルはラウンドトリップ時に完全に保持される必要がある。

### 方針: 読み取りは serde、書き戻しは文字列操作

`serde_yaml` (0.9) は非推奨・アーカイブ済みのため使用しない。後継の `serde_yaml_ng` を読み取り専用で使用する。書き戻しはYAML全体を再シリアライズせず、生テキスト上で sprout フィールドのみを文字列操作で更新する。

**この方式の利点:**
- コメント・キー順序・書式が完全に保持される（YAMLを再シリアライズしないため）
- 読み取り側は serde の型安全性を活用できる
- 純Rust依存のみ（C依存なし、Nixビルドに適合）
- sprout が管理するフィールドは6-7個と限定的で、文字列操作の複雑さが抑えられる

**不採用の代替案:**
- `serde_yml`: RUSTSEC-2025-0068（unsound/segfault）により使用禁止
- `rust-yaml` (RoundTripConstructor): v0.0.5、採用実績が極めて少なく基盤依存にはリスクが高い
- `fyaml` (libfyaml): C依存がNixパッケージングを複雑化する

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
    pub frontmatter_raw: String,           // 元のYAMLテキスト（書き戻し時の原本）
    pub sprout: SproutFrontmatter,         // serde_yaml_ng で抽出した sprout フィールド
    pub body: String,                      // 閉じ --- 以降のすべて
}
```

## パーシングアルゴリズム

1. ファイルを文字列として読み込み
2. `---\n` で始まるか確認
3. 閉じ `---\n`（2番目の出現）を検索
4. デリミタ間のYAMLテキストを `frontmatter_raw` として保存
5. `serde_yaml_ng::from_str` で `SproutFrontmatter` にデシリアライズ（未知キーは `#[serde(deny_unknown_fields)]` なしで無視）
6. body（2番目の `---` 以降のすべて）を保存

## 書き戻しアルゴリズム

`frontmatter_raw` に対して文字列操作で sprout フィールドのみ更新する。YAML全体の再シリアライズは行わない。

1. `frontmatter_raw`（元のYAMLテキスト）を取得
2. sprout 管理キー (`maturity`, `last_review`, `review_interval`, `next_review`, `ease`) について:
   - キーが既存なら、該当行を正規表現で値部分のみ置換（`key: old_value` → `key: new_value`）
   - キーが存在しなければ、YAMLブロック末尾に行を追加
3. 再構築: `---\n{updated_yaml}\n---\n{body}`

### 行置換の正規表現パターン

```rust
// 例: "review_interval: 3" → "review_interval: 7"
// パターン: ^{key}:\s+.*$ を ^{key}: {new_value}$ に置換
fn replace_field(yaml: &str, key: &str, new_value: &str) -> String;

// 例: キーが存在しない場合、末尾に追加
fn append_field(yaml: &str, key: &str, value: &str) -> String;
```

コメント付きの行（`review_interval: 3  # 日数`）でも、値部分のみが置換されインラインコメントは保持される。

フロントマターのないファイルはsproutフィールドに `None` を返し、`sprout init` でフロントマターブロックを新規追加する。
