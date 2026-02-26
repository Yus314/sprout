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

## Obsidian互換性

未知のYAMLキー（`aliases`, `cssclasses` など）、コメント、キー順序、クォートスタイルはラウンドトリップ時に完全に保持される必要がある。

### 方針: 分離は gray_matter、書き戻しは文字列操作

フロントマターの分離・パースには [`gray_matter`](https://lib.rs/crates/gray_matter) クレートを使用する。書き戻しはYAML全体を再シリアライズせず、生テキスト上で sprout フィールドのみを文字列操作で更新する。

`gray_matter` は `ParsedEntity` として raw YAML 文字列（`matter`）・パース済みデータ（`data`）・本文（`content`）を返すため、sprout の「読み取りは serde、書き戻しは文字列操作」方針にそのまま適合する。デリミタ検出（`---`）・エッジケース処理（末尾空白、EOF）はライブラリ側で処理される。

**この方式の利点:**
- コメント・キー順序・書式が完全に保持される（YAMLを再シリアライズしないため）
- 読み取り側は serde の型安全性を活用できる（`gray_matter` は `DeserializeOwned` 経由でデシリアライズ）
- フロントマター分離のエッジケース処理をライブラリに委ねられる
- 純Rust依存のみ（`yaml-rust2`, `serde`, `thiserror`。C依存なし、Nixビルドに適合）
- sprout が管理するフィールドは6個と限定的で、文字列操作の複雑さが抑えられる

**不採用の代替案:**
- `serde_yaml_ng` + 自前分離: フロントマター分離のエッジケースを自前で処理する必要がある
- `serde_yml`: RUSTSEC-2025-0068（unsound/segfault）により使用禁止
- `yaml-front-matter`: `serde_yaml` 0.8（非推奨）に依存。raw YAML 文字列を返さない
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
}

pub struct ParsedNote {
    pub frontmatter_raw: String,           // gray_matter の matter フィールド（書き戻し時の原本）
    pub sprout: SproutFrontmatter,         // gray_matter の data フィールド（serde でデシリアライズ済み）
    pub body: String,                      // gray_matter の content フィールド
}
```

## パーシングアルゴリズム

`gray_matter` にフロントマターの分離・デシリアライズを委ねる:

1. ファイルを文字列として読み込み
2. `\r\n` を `\n` に正規化する（書き戻し時も `\n` のみで出力する）
3. `Matter::<YAML>::new().parse::<SproutFrontmatter>(input)` を呼び出す
4. 返却された `ParsedEntity` から `matter`（raw YAML）、`data`（パース済み）、`content`（本文）を取得
5. `data` が `None` の場合、フロントマターなしとして扱う（`sprout init` のケースA）
6. 未知キーは `#[serde(deny_unknown_fields)]` なしで無視される

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

### `sprout init` のケース分類

`sprout init` が管理するフィールド: `maturity`, `created`, `last_review`, `review_interval`, `next_review`, `ease`（計6フィールド）。

init はフィールド単位で存在を確認し、欠けているフィールドのみデフォルト値で追加する。既存フィールドの値は変更しない。

ファイル自体が存在しない場合は `file_not_found` エラー（exit 1）。`init` はファイルの新規作成を行わない。

| ケース | 状態 | init の挙動 |
|---|---|---|
| A | フロントマターなし | `---` ブロックを新規作成し、全6フィールドを挿入 |
| B | フロントマターあり、sproutフィールドなし | 既存ブロック末尾（閉じ `---` の直前）に全6フィールドを追加 |
| C | フロントマターあり、sproutフィールドが一部存在 | 欠けているフィールドのみデフォルト値で追加。stderr に補完した旨を警告する |
| D | 全6フィールドが存在 | `already_initialized` エラー |

「初期化済み」の判定基準: 6フィールド **全て** が存在すること。

ケースCの警告メッセージ例:

```
warning: missing fields added with defaults: ease, next_review
```

### 書き戻し時の注意事項

- **クォートスタイル**: sprout はクォートなしで値を書く。元のクォートスタイル（`"seedling"` vs `seedling`）は保持しない
- **複数フィールド同時更新**: `replace_field` を順次適用する。各フィールドは独立に「置換 or 追加」にフォールバックする
