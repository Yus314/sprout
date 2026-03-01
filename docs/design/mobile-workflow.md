# モバイルワークフロー設計

## 背景と問題

sprout は CLI 前提のアーキテクチャで設計されている（overview.md: 「CLI は純粋なデータ変換ツール」）。モバイルは CLI 層・エディタプラグイン層に続く第3の「入力経路」として位置付けられる。

feature-coverage.md では obsidian-sr との比較で「モバイル対応 ❌ CLIのみ」と明記されている。しかし Obsidian の vault は同期されるため、モバイルで作成したノートも sprout 管理下に置きたいという要求がある。

**問題の本質:** 「ノート作成」と「SRS 初期化」は本来別の操作であり、デスクトップでは `sprout note` + `auto_init` で同時に行われているが、モバイルでは CLI が使えないためこの結合が断たれる。

**前提:** レビュー（`sprout done`）はデスクトップ限定。モバイルで解決すべきは「作成時のフロントマター挿入」のみである。

## 設計原則

- CLI の哲学（純粋なデータ変換ツール）を維持する
- sprout の Option 設計・Case C 補完を安全ネットとして活用する
- Obsidian エコシステムのプラグインでモバイル入力経路を補う

## 採用: Obsidian Templater 連携

### 方式

Templater テンプレートで全6フィールドを挿入する。Folder Template で自動適用することで、モバイルでの新規ノート作成時にユーザー操作なしで sprout 互換フロントマターが挿入される。

### Templater テンプレート

`_templates/sprout-note.md`:

```markdown
---
maturity: seedling
created: <% tp.date.now("YYYY-MM-DD") %>
last_review: <% tp.date.now("YYYY-MM-DD") %>
review_interval: 1
next_review: <% tp.date.now("YYYY-MM-DD", 1) %>
ease: 2.50
---

# <% tp.file.title %>
```

- `tp.date.now("YYYY-MM-DD")` — 作成日をローカルタイムゾーンで挿入
- `tp.date.now("YYYY-MM-DD", 1)` — 翌日を next_review として挿入（moment.js ベース）
- `tp.file.title` — ノートのファイル名をタイトルとして挿入
- `ease: 2.50` — f64 の小数2桁フォーマット（frontmatter.md の `{:.2}` 仕様に準拠）

### Obsidian 設定

1. **Templater > Template folder location**: `_templates`
2. **Templater > Folder Templates**: `/` → `sprout-note`（vault 全体に適用）
3. **Templater > Trigger Templater on new file creation**: ON

### sprout 側設定

config.toml の `exclude_dirs` に `_templates` を追加する:

```toml
exclude_dirs = [".git", ".obsidian", ".trash", "_templates"]
```

**理由:** テンプレートファイルは未展開の `<% %>` 構文を含み、sprout がパースすると `created` 等が NaiveDate として不正な値になる。`exclude_dirs` は名前ベースのマッチング（note.rs の walkdir `filter_entry`）で動作する。

### モバイル固有の動作

- Templater プラグインは iOS / Android の Obsidian Mobile で動作する
- Folder Template の自動適用もモバイルで動作する
- `tp.date.now()` は端末のローカルタイムゾーンを使用する
- 翌日計算（`tp.date.now("YYYY-MM-DD", 1)`）は moment.js ベースで問題ない

### sprout との相互作用

- Templater で全6フィールド挿入 → `sprout review` / `done` / `list` すべて即時動作
- 後から `sprout init` を実行した場合: Case D（already_initialized）→ 害なし
- Templater が部分適用された場合: Case C 補完が安全ネットとして機能
- `sprout done` 実行時: Templater 挿入値は正しい型（date, u32, f64）でパース可能
- Obsidian のプロパティビュー: YAML フィールドがプロパティとして正しく認識される

### 二重管理の問題と対策

- **ease 値**: テンプレートに `2.50` をハードコード。config.toml の `default_ease`（デフォルト 2.5）と二重管理になるが、`default_ease` を変更するケースは稀であり、変更時にテンプレートも合わせて更新すれば済む
- **フィールド仕様変更**: sprout が将来フィールドを追加した場合、テンプレート更新が必要。ただし Case C 補完が安全ネットとして機能するため、更新忘れは致命的ではない

### 利点

- 即座に sprout 管理対象（review, done, list すべて動作）
- next_review の翌日計算が Templater で可能
- created がモバイル作成日を正確に記録
- Folder Template で自動適用（テンプレート選択操作すら不要）

### 懸念と緩和策

- **Templater 依存** → sprout init の Case C / A が安全ネット
- **ease 等のハードコード** → default_ease 変更は稀、変更時にテンプレートも更新
- **フィールド仕様変更** → Case C 補完で安全

## 不採用の代替案

### B. 最小フロントマター + sprout init 補完

maturity + created のみ挿入し、PC で `sprout init` により Case C 補完する方式。

- **利点**: 外部依存なし、sprout 設計に最も忠実
- **不採用理由**: PC 操作が必須、init 忘れリスク、即時レビュー対象にならない

### C. sprout 独自テンプレートの拡張

sprout テンプレートに SRS 変数を追加する方式。

- **不採用理由**: CLI 経由でのみ使えるためモバイル問題の直接解決にならない。`auto_init` が既にこの役割を果たしている

### D. Obsidian Linter / Metadata Menu

保存時に自動的にフロントマターフィールドを追加する方式。

- **不採用理由**: next_review の翌日計算が困難、maturity の定型文字列挿入の制約、ease の `2.5` vs `2.50` のフォーマットノイズ

### E. sprout 側 sync / auto-init 機能

`sprout sync` コマンドで未初期化ノートを一括 init する方式。

- **不採用理由**: sprout 側のコード改修が必要、created 日付が不正確（sync 実行日になる）、意図しない初期化リスク。将来検討として言及にとどめる

## データ整合性

### Templater 適用後の状態

| コマンド | 動作 |
|---------|------|
| `sprout list` | ✅ 表示される |
| `sprout review` | ✅ next_review <= today ならレビュー対象 |
| `sprout done` | ✅ 全フィールド揃っているため正常動作 |
| `sprout init` | ⚠️ already_initialized（害なし） |
| `sprout show` | ✅ 全情報表示 |

### Templater 未適用時（安全ネット）

| 状態 | `sprout list` | `sprout review` | 対処 |
|------|-----------|-------------|------|
| フロントマターなし | ❌ | ❌ | `sprout init`（Case A） |
| maturity のみ | ✅ | ❌ | `sprout init`（Case C） |

## 同期に関する考慮事項

- Obsidian Sync / iCloud / Syncthing による vault 同期を前提とする
- **同期タイミング**: Templater 展開後のファイルが同期されるため、デスクトップ側は完成したフロントマターを受け取る
- **競合**: フロントマターの同一フィールドをモバイルとデスクトップで同時編集した場合、同期ツールの競合解決に依存する。sprout 側の書き戻しは文字列操作のため、同期ツールのマージとの相性は良い（行単位の差分）
- **キャッシュ**: sprout の mtime + size キャッシュ（cache.rs）は同期によるファイル変更を mtime の変化で検出し、キャッシュを無効化する

## 将来の拡張可能性

- `sprout sync` コマンド（init 忘れのバッチ補完）
- Obsidian プラグイン（sprout コマンドを Obsidian 内から直接実行）
