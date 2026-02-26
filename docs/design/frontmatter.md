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
---
```

`tags` など他のYAMLキーはユーザーが自由に追加できる。sprout はこれらを読み取りのみ行い、書き戻し時に変更しない。

## フィールド定義

| フィールド | 型 | デフォルト | 説明 |
|-----------|-----|-----------|------|
| `maturity` | string | `"seedling"` | ノートの成熟度: `seedling`, `budding`, `evergreen` |
| `created` | date | 初期化日 | ノート作成日 |
| `last_review` | date | 初期化日 | 最後のレビュー日 |
| `review_interval` | u32 | `1` | 現在のレビュー間隔（日数） |
| `next_review` | date | 翌日 | 次のレビュー予定日 |
| `ease` | f64 | `2.5` | ease factor |
| `tags` | list | `["review"]` | タグリスト。sprout は読み取りのみ、書き戻し対象外 |

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
2. `\r\n` を `\n` に正規化する（書き戻し時も `\n` のみで出力する）
3. `---\n` で始まるか確認
4. 閉じ `---\n`（2番目の出現）を検索
5. デリミタ間のYAMLテキストを `frontmatter_raw` として保存
6. `serde_yaml_ng::from_str` で `SproutFrontmatter` にデシリアライズ（未知キーは `#[serde(deny_unknown_fields)]` なしで無視）
7. body（2番目の `---` 以降のすべて）を保存

## 書き戻しアルゴリズム

`frontmatter_raw` に対して文字列操作で sprout フィールドのみ更新する。YAML全体の再シリアライズは行わない。

1. `frontmatter_raw`（元のYAMLテキスト）を取得
2. sprout 管理キー (`maturity`, `last_review`, `review_interval`, `next_review`, `ease`) について（`ease` は `{:.2}` 小数2桁でフォーマットする。±0.15の離散変動のみのため2桁で十分かつ f64 丸め誤差を回避）:
   - キーが既存なら、該当行を正規表現で値部分のみ置換（`key: old_value` → `key: new_value`）
   - キーが存在しなければ、YAMLブロック末尾に行を追加
3. 再構築: `---\n{updated_yaml}\n---\n{body}`

### 行置換の正規表現パターン

3グループパターンでインラインコメントを保持する:

```
^({key}\s*:\s*)(\S+)(.*)$
```

- Group 1: キー＋コロン＋空白（保持）
- Group 2: 値（空白なしの単一トークン — sprout管理値はすべてこの条件を満たす）
- Group 3: 値以降の残り（空白＋インラインコメント、保持）

置換結果: `${1}{new_value}${3}`

```rust
/// 既存キーの値を置換する。インラインコメントは保持される。
/// 例: "review_interval: 3  # 日数" → "review_interval: 7  # 日数"
fn replace_field(yaml: &str, key: &str, new_value: &str) -> String;

/// キーが存在しない場合、YAMLブロック末尾に `{key}: {value}` 行を追加する。
fn append_field(yaml: &str, key: &str, value: &str) -> String;
```

### `sprout init` の3ケース

| ケース | 状態 | init の挙動 |
|---|---|---|
| A | フロントマターなし | `---` ブロックを新規作成し、全sproutフィールドを挿入 |
| B | フロントマターあり、sproutフィールドなし | 既存ブロック末尾（閉じ `---` の直前）にsproutフィールドを追加 |
| C | sproutフィールドあり | `already_initialized` エラー |

「sproutフィールドあり」の判定基準: `maturity` キーの存在。

`sprout init` が書くフィールド: `maturity`, `created`, `last_review`, `review_interval`, `next_review`, `ease`。

### 書き戻し時の注意事項

- **クォートスタイル**: sprout はクォートなしで値を書く。元のクォートスタイル（`"seedling"` vs `seedling`）は保持しない
- **複数フィールド同時更新**: `replace_field` を順次適用する。各フィールドは独立に「置換 or 追加」にフォールバックする
