# CLI コマンド仕様

## コマンド一覧

| コマンド | 説明 |
|---------|------|
| `sprout review` | 今日レビュー予定のノートを一覧表示 (next_review <= today) |
| `sprout done <file> <hard\|good\|easy>` | レビュー完了をマーク、フロントマター更新 |
| `sprout promote <file> <seedling\|budding\|evergreen>` | 成熟度レベルを変更 |
| `sprout stats` | 成熟度別の統計を表示 |
| `sprout init <file>` | フロントマター追加 (seedling, interval=1) |
| `sprout list [--maturity <m>]` | トラッキング中の全ノートを一覧表示 |
| `sprout show <file>` | 単一ノートの詳細情報を表示 |
| `sprout note` | vault内の全.mdファイルを一覧表示 |
| `sprout note <title>` | 新規ノートを作成（既存なら冪等にパスを返す） |

## グローバルオプション

全コマンドで使用可能:

- `--vault <path>`: vault パスを上書き
- `--format human|json`: 出力形式（デフォルト: `human`）

## Clap Derive 構造

```rust
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "sprout", version, about = "Evergreen note cultivation with spaced repetition")]
pub struct Cli {
    /// Path to notes vault (overrides config and current directory)
    #[arg(long, global = true)]
    pub vault: Option<PathBuf>,

    /// Output format
    #[arg(long, global = true, default_value = "human")]
    pub format: OutputFormat,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List notes due for review today
    Review,
    /// Mark a note as reviewed with a difficulty rating
    Done {
        /// Path to the reviewed note file
        file: PathBuf,
            /// Difficulty rating
        rating: Rating,
    },
    /// Change the maturity level of a note
    Promote {
        /// Path to the note file
        file: PathBuf,
        /// Target maturity level
        maturity: Maturity,
    },
    /// Show statistics about your note collection
    Stats,
    /// Add sprout frontmatter to a new or existing note
    Init {
        /// Path to the note file
        file: PathBuf,
    },
    /// List all tracked notes
    List {
        /// Filter by maturity level
        #[arg(long)]
        maturity: Option<Maturity>,
    },
    /// Show detailed information about a single note
    Show {
        /// Path to the note file
        file: PathBuf,
    },
    /// Open an existing note or create a new one
    Note {
        /// Title for a new note (omit to list all notes)
        title: Option<String>,
        /// Template name to use
        #[arg(long)]
        template: Option<String>,
    },
}

#[derive(ValueEnum, Clone)]
pub enum Rating { Hard, Good, Easy }

#[derive(ValueEnum, Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Maturity { Seedling, Budding, Evergreen }

#[derive(ValueEnum, Clone)]
pub enum OutputFormat { Human, Json }
```

## Vault スキャン

`review`, `list`, `stats`, `done`（負荷分散）の4コマンドは vault 全体をスキャンする。

### スキャン対象

- **拡張子**: `.md` ファイルのみ
- **再帰**: vault パス以下のサブディレクトリを再帰的に走査する
- **シンボリックリンク**: 追跡する（Obsidian 互換）。`walkdir` の循環検出に依存し、循環リンクはスキップする。重複防止のためパスを正規化（`canonicalize`）し、同一実体が複数回出現しないようにする
- **除外ディレクトリ**: `config.exclude_dirs`（デフォルト: `[".git", ".obsidian", ".trash"]`）に一致するディレクトリ名はスキャンから除外する

### トラッキング判定

ノートが sprout でトラッキングされているかの判定基準は **`maturity` フィールドの存在** とする。

- `maturity` が存在する → トラッキング中（`tracked: true`）
- `maturity` が存在しない → 未トラッキング（`tracked: false`）

`maturity` があっても他の必須フィールド（`ease`, `review_interval`, `next_review` 等）が欠けている場合、各コマンドは以下のように振る舞う:

| コマンド | `next_review` なし | `ease`/`review_interval` なし |
|----------|--------------------|-------------------------------|
| `list`   | 表示する（`maturity` のみで動作可能） | 表示する |
| `stats`  | `total`/maturity 別には計上、`due_today`/`overdue` からは除外 | maturity 別に計上 |
| `review` | スキップ（due 判定不能） | スキップ |
| `done`   | `no_frontmatter` エラー。`sprout init` による補完を促す | `no_frontmatter` エラー |

### `relative_path` の基準

JSON 出力の `relative_path` は vault ルートからの相対パスとする。

## JSON出力形式

`--format json` フラグはKakouneプラグイン統合に不可欠。KakouneのシェルブロックがJSON出力をパースし、メニューやinfoボックスに表示する。

### `sprout review` ソート順

`next_review` 昇順（overdue が長いノートが先頭）。

### `sprout review --format json` 出力例

```json
[
  {
    "path": "/home/kaki/notes/zettelkasten/note1.md",
    "relative_path": "zettelkasten/note1.md",
    "maturity": "seedling",
    "review_interval": 3,
    "next_review": "2026-02-25",
    "ease": 2.5
  }
]
```

`done` は due でないノート（`next_review` が未来）にも実行可能。`delayed = max(0, today - next_review)` により `delayed = 0` で通常の SRS 計算が走る。

### `sprout done --format json` 出力例

```json
{
  "path": "/home/kaki/notes/zettelkasten/note1.md",
  "maturity": "seedling",
  "last_review": "2026-02-26",
  "new_interval": 5,
  "next_review": "2026-03-02",
  "ease": 2.5
}
```

### `sprout stats --format json` 出力例

```json
{
  "total": 150,
  "seedling": 80,
  "budding": 50,
  "evergreen": 20,
  "due_today": 12,
  "overdue": 5
}
```

`due_today` と `overdue` は排他的:
- `overdue`: `next_review < today`（過去に予定日を過ぎたノート）
- `due_today`: `next_review == today`（今日が予定日のノート）
- `review` コマンドが返すノート数は `due_today + overdue` と一致する

### `sprout promote --format json` 出力例

`promote` は `maturity` フィールドのみを変更する。`ease`, `review_interval`, `next_review` 等の SRS 値は一切変更しない。SRS 値の調整は `done` コマンドの責務とする。

- **任意方向の変更を許可**: evergreen → seedling のような降格も可能。コマンド名は `promote` だが、実質的には maturity ラベルの書き換え操作
- **同一 maturity への promote は no-op 成功**: `previous_maturity == new_maturity` として exit 0 を返す（冪等性）

```json
{
  "path": "/home/kaki/notes/zettelkasten/note1.md",
  "relative_path": "zettelkasten/note1.md",
  "previous_maturity": "seedling",
  "new_maturity": "budding",
  "review_interval": 3,
  "next_review": "2026-02-28",
  "ease": 2.5
}
```

### `sprout init --format json` 出力例

新規初期化（ケースA・B）:

```json
{
  "path": "/home/kaki/notes/zettelkasten/note1.md",
  "relative_path": "zettelkasten/note1.md",
  "maturity": "seedling",
  "review_interval": 1,
  "next_review": "2026-02-26",
  "ease": 2.5,
  "created": "2026-02-25"
}
```

部分補完（ケースC — 一部フィールドが欠けていた場合）:

```json
{
  "path": "/home/kaki/notes/zettelkasten/note1.md",
  "relative_path": "zettelkasten/note1.md",
  "maturity": "seedling",
  "review_interval": 1,
  "next_review": "2026-02-26",
  "ease": 2.5,
  "created": "2026-02-25",
  "fields_added": ["ease", "next_review"]
}
```

`fields_added` は補完されたフィールド名のリスト。全フィールドが新規追加された場合（ケースA・B）はこのキーを含まない。

### `sprout list` ソート順

`relative_path` のアルファベット昇順。

### `sprout list --format json` 出力例

`review` と同じ配列形式（同一フィールド）。`list` が全ノートを返し、`review` が due のみを返す点が異なる。

```json
[
  {
    "path": "/home/kaki/notes/zettelkasten/note1.md",
    "relative_path": "zettelkasten/note1.md",
    "maturity": "seedling",
    "review_interval": 3,
    "next_review": "2026-02-25",
    "ease": 2.5
  }
]
```

### `sprout show --format json` 出力例

`days_until_review` は `next_review - today` の符号付き日数。overdue なら負数（例: 3日超過で `-3`）。

トラッキング中のノート:

```json
{
  "path": "/home/kaki/notes/zettelkasten/note1.md",
  "relative_path": "zettelkasten/note1.md",
  "tracked": true,
  "maturity": "seedling",
  "created": "2026-02-25",
  "last_review": "2026-02-25",
  "review_interval": 3,
  "next_review": "2026-02-28",
  "ease": 2.5,
  "is_due": true,
  "days_until_review": 0,
  "link_count": 5
}
```

未トラッキングのファイル（exit 0）:

```json
{"path": "/home/kaki/notes/zettelkasten/note1.md", "relative_path": "zettelkasten/note1.md", "tracked": false}
```

ファイル自体が存在しない場合は exit 1。

### `sprout note --format json` 出力例（List モード）

vault 内の全 `.md` ファイル（SRS トラッキング有無を問わない）を `relative_path` 昇順で返す。

```json
[
  {
    "path": "/home/kaki/notes/zettelkasten/note1.md",
    "relative_path": "zettelkasten/note1.md"
  }
]
```

### `sprout note <title> --format json` 出力例（Create モード）

新規作成時:

```json
{
  "path": "/home/kaki/notes/テストノート.md",
  "relative_path": "テストノート.md",
  "is_new": true,
  "initialized": true
}
```

既存ファイルがある場合（冪等動作）:

```json
{
  "path": "/home/kaki/notes/テストノート.md",
  "relative_path": "テストノート.md",
  "is_new": false,
  "initialized": false
}
```

**タイトルバリデーション**: `/`, `\`, `\0`, `..` を含むタイトルは `invalid_title` エラー。`.md` サフィックスが既にあれば除去して使用（二重拡張子回避）。

**`--template <name>`**: テンプレート名を指定。`{template_dir}/{name}.md` を読み込む。デフォルト: `default`。

## エラー出力規約

- **成功**: exit 0, stdout に出力
- **エラー**: exit 1, stderr にメッセージ
  - `--format json` 時は `{"error": "<code>", "message": "..."}` 形式で stderr に出力
- **エラー時は stdout は空** — プラグイン側で `2>&1` を使っても安全にパースできる

### エラー種別

| エラーコード | 説明 |
|-------------|------|
| `file_not_found` | 指定されたファイルが存在しない |
| `outside_vault` | 指定されたファイルが vault ディレクトリ外にある |
| `no_frontmatter` | sprout フロントマターが見つからない |
| `vault_not_found` | vault パスが解決できない |
| `already_initialized` | 全sproutフィールドが既に存在する（`init` 時） |
| `parse_error` | フロントマターのパースに失敗 |
| `invalid_title` | ノートタイトルに不正な文字が含まれている |

## ソースファイル構成

```
src/
├── main.rs          # エントリポイント
├── cli.rs           # clap derive定義
├── config.rs        # 設定読み込み (~/.config/sprout/config.toml)
├── frontmatter.rs   # YAMLフロントマターのパース（gray_matter）と文字列書き戻し
├── note.rs          # ノート検出、読み書き
├── links.rs         # [[wiki-link]] パースとリンクカウント
├── srs.rs           # SRSアルゴリズム（遅延・リンク・負荷分散）
├── output.rs        # human / JSON 出力フォーマット
├── template.rs      # テンプレート読み込みと変数展開
└── commands/
    ├── mod.rs
    ├── review.rs    # sprout review
    ├── done.rs      # sprout done <file> <rating>
    ├── promote.rs   # sprout promote <file> <maturity>
    ├── stats.rs     # sprout stats
    ├── init.rs      # sprout init <file>
    ├── list.rs      # sprout list [--maturity <m>]
    ├── note.rs      # sprout note [<title>] [--template <name>]
    └── show.rs      # sprout show <file>
```
