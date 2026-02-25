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
}

#[derive(ValueEnum, Clone)]
pub enum Rating { Hard, Good, Easy }

#[derive(ValueEnum, Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Maturity { Seedling, Budding, Evergreen }

#[derive(ValueEnum, Clone)]
pub enum OutputFormat { Human, Json }
```

## JSON出力形式

`--format json` フラグはKakouneプラグイン統合に不可欠。KakouneのシェルブロックがJSON出力をパースし、メニューやinfoボックスに表示する。

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

### `sprout done --format json` 出力例

```json
{
  "path": "/home/kaki/notes/zettelkasten/note1.md",
  "maturity": "seedling",
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

### `sprout promote --format json` 出力例

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

## エラー出力規約

- **成功**: exit 0, stdout に出力
- **エラー**: exit 1, stderr にメッセージ
  - `--format json` 時は `{"error": "<code>", "message": "..."}` 形式で stderr に出力
- **エラー時は stdout は空** — プラグイン側で `2>&1` を使っても安全にパースできる

### エラー種別

| エラーコード | 説明 |
|-------------|------|
| `file_not_found` | 指定されたファイルが存在しない |
| `no_frontmatter` | sprout フロントマターが見つからない |
| `vault_not_found` | vault パスが解決できない |
| `already_initialized` | 既にフロントマターが存在する（`init` 時） |
| `parse_error` | フロントマターのパースに失敗 |

## ソースファイル構成

```
src/
├── main.rs          # エントリポイント
├── cli.rs           # clap derive定義
├── config.rs        # 設定読み込み (~/.config/sprout/config.toml)
├── frontmatter.rs   # YAMLフロントマターのパース（serde_yaml_ng）と文字列書き戻し
├── note.rs          # ノート検出、読み書き
├── links.rs         # [[wiki-link]] パースとリンクカウント
├── srs.rs           # SRSアルゴリズム（遅延・リンク・負荷分散）
├── output.rs        # human / JSON 出力フォーマット
└── commands/
    ├── mod.rs
    ├── review.rs    # sprout review
    ├── done.rs      # sprout done <file> <rating>
    ├── promote.rs   # sprout promote <file> <maturity>
    ├── stats.rs     # sprout stats
    ├── init.rs      # sprout init <file>
    ├── list.rs      # sprout list [--maturity <m>]
    └── show.rs      # sprout show <file>
```
