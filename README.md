# sprout

Evergreen Notes の育成を支援する、間隔反復（SRS）ベースの CLI ツール。

## Evergreen Notes とは

[Evergreen Notes](https://notes.andymatuschak.org/Evergreen_notes) は Andy Matuschak が提唱するノートテイキングの方法論で、時間をかけて繰り返し見直し・改善することで、ノートを永続的な知識資産へと育てていく考え方である。Zettelkasten の原子的なノート構造と、間隔反復による定期的な再訪を組み合わせることで、知識の定着とノートの質の向上を同時に実現する。

sprout は、このプロセスを 3 段階の成熟度モデルで支援する:

- **seedling** (種) — 書いたばかりのノート。頻繁にレビューし、内容を練り上げる
- **budding** (芽) — ある程度育ったノート。リンクが増え、文脈が明確になりつつある
- **evergreen** (常緑) — 十分に成熟したノート。長いインターバルで定期的に再訪する

## 主な機能

- **SRS スケジューリング** — SM-2 ベースのアルゴリズムで最適なレビュータイミングを計算
- **成熟度管理** — seedling → budding → evergreen の 3 段階でノートの成長を追跡
- **リンクファクター** — `[[wiki-link]]` の数に応じて ease を調整し、接続の多いノートの間隔を適切に延長
- **負荷分散** — ファジングにより特定の日にレビューが集中するのを防止
- **遅延日数の考慮** — 予定日を過ぎたレビューでも、理解度に応じて遅延分をインターバルに反映

## コマンド一覧

| コマンド | 説明 |
|---------|------|
| `sprout review` | 今日レビュー予定のノートを一覧表示 |
| `sprout done <file> <hard\|good\|easy>` | レビュー完了をマーク、フロントマター更新 |
| `sprout promote <file> <seedling\|budding\|evergreen>` | 成熟度レベルを変更 |
| `sprout stats` | 成熟度別の統計を表示 |
| `sprout init <file>` | フロントマターを追加（seedling, interval=1） |
| `sprout list [--maturity <m>]` | トラッキング中の全ノートを一覧表示 |
| `sprout show <file>` | 単一ノートの詳細情報を表示 |

全コマンドで `--vault <path>` と `--format human|json` オプションが使用可能。詳細は [CLI コマンド仕様](docs/design/cli.md) を参照。

## インストール

Nix Flake でビルドする:

```bash
# ビルド
nix build github:Yus314/sprout
./result/bin/sprout --help

# 開発シェル
nix develop github:Yus314/sprout
cargo build && cargo test
```

## 設定

設定ファイル: `~/.config/sprout/config.toml`

```toml
vault_path = "/home/user/notes"
max_interval = 90
default_ease = 2.5
link_weight = 0.1
load_balance = true
```

全フィールドはオプション。設定ファイルなしでもカレントディレクトリをデフォルトとして動作する。詳細は [設定ファイル仕様](docs/design/config.md) を参照。

## 設計ドキュメント

| ドキュメント | 内容 |
|-------------|------|
| [overview.md](docs/design/overview.md) | 背景・動機・アーキテクチャ概観 |
| [cli.md](docs/design/cli.md) | CLI コマンド仕様・Clap 構造・JSON 出力形式 |
| [algorithm.md](docs/design/algorithm.md) | SRS アルゴリズム・リンクファクター・負荷分散 |
| [frontmatter.md](docs/design/frontmatter.md) | YAML フロントマター形式・パース・書き戻し方針 |
| [config.md](docs/design/config.md) | 設定ファイル仕様・vault パス解決順序 |
| [kakoune-plugin.md](docs/design/kakoune-plugin.md) | Kakoune プラグイン・User hook 設計 |
| [nix-packaging.md](docs/design/nix-packaging.md) | Nix Flake パッケージング・dotfiles 統合 |
| [feature-coverage.md](docs/design/feature-coverage.md) | org-roam-review / obsidian-sr との機能比較 |

## ライセンス

MIT
