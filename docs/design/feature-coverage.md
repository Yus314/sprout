# 機能カバー率: org-roam-review / obsidian-sr 比較

## org-roam-review との比較

| 機能 | org-roam-review | sprout v0.1 | 備考 |
|------|----------------|-------------|------|
| SRSスケジューリング | ✅ | ✅ | SM-2ベースの独自実装 |
| 成熟度レベル (maturity) | ✅ | ✅ | seedling/budding/evergreen |
| レビューキュー | ✅ | ✅ | `sprout review` |
| 評価 (hard/good/easy) | ✅ (1-5) | ✅ (3段階) | シンプルな3段階に |
| 遅延日数考慮 | ❌ | ✅ | sprout独自 |
| ノート間リンク考慮 | ❌ | ✅ | リンクファクター |
| 負荷分散 | ❌ | ✅ | ファジングによる日付分散 |
| レビュー後フック | ✅ (Emacs hook) | ✅ (Kakoune User hook) | エディタ層で対応。`trigger-user-hook` |
| レビューセッション自動進行 | ✅ (デフォルト有効) | 🔮 | v0.2: sprout.kak拡張 |
| タグフィルタリング | ✅ | 🔮 | v0.2検討 |
| バルクレビュー | ✅ | 🔮 | v0.2検討 |
| Emacsインライン表示 | ✅ | N/A | Kakoune対応に置換 |
| org-roam統合 | ✅ | N/A | Markdown + wiki-link |

## obsidian-sr (Spaced Repetition) との比較

| 機能 | obsidian-sr | sprout v0.1 | 備考 |
|------|------------|-------------|------|
| SRSスケジューリング | ✅ | ✅ | |
| フラッシュカード | ✅ | ❌ | sproutはノート単位 |
| ノートレビュー | ✅ | ✅ | |
| リンク先ノートのease参照 | ✅ | ✅ (簡易版) | リンク数のみ使用 |
| フォルダ無視設定 | ✅ | 🔮 | v0.2検討 |
| モバイル対応 | ✅ (Obsidian経由) | ❌ | CLIのみ |
| Obsidianプラグイン | ✅ | N/A | Kakoune + CLI |
| YAML frontmatter | ✅ | ✅ | 互換フォーマット |
| Dataview統合 | ✅ | N/A | CLIネイティブ |

## Hook設計方針

org-roam-reviewのフック（`node-accepted-hook`等5種）はEmacs Lispの`run-hooks`で実行されるEmacsネイティブの機構である。sproutではCLIにフック機構を持たせず、Kakouneの`trigger-user-hook`でエディタ層に実装する。

**根拠:**

- org-roam-reviewのフックと同じ抽象レイヤー（エディタ）に配置される
- CLIは純粋なデータ変換ツール（入力→フロントマター更新+JSON出力）として保たれる
- `trigger-user-hook`の1行追加で実装でき、設定項目やエラーハンドリングが不要
- 他エディタ（Neovim: `User` autocmd、Emacs: `run-hooks`等）も各自の流儀で実装可能

**org-roam-reviewとのフック対応:**

| org-roam-review | Kakoune User hook | トリガー |
|----------------|-------------------|---------|
| `node-accepted-hook` | `SproutDoneGood`, `SproutDoneEasy` | `sprout-done good/easy` 成功後 |
| `node-forgotten-hook` | `SproutDoneHard` | `sprout-done hard` 成功後 |
| `node-buried-hook` | （v0.2でbury実装時） | — |
| `node-processed-hook` | `SproutDone` | `sprout-done` 成功後（評価問わず） |
| `next-node-selected-hook` | `SproutReviewNext` | v0.2: レビューセッション拡張 |

## v0.1 カバー率

- **org-roam-review**: コア機能の約85%をカバー（タグフィルタ・バルクレビュー・セッション自動進行除く）
- **obsidian-sr**: ノートレビュー機能の約70%をカバー（フラッシュカード・モバイル除く）

## 優先度

### v0.1 (MVP)

1. フロントマターのパース・ラウンドトリップ
2. SRS基本計算（ease, interval, delay）
3. 全7コマンド（review, done, promote, stats, init, list, show）
4. JSON出力
5. Kakouneプラグイン
6. Nixパッケージング

### v0.2 (検討)

- タグフィルタリング (`sprout review --tag`)
- フォルダ無視設定 (`.sproutignore` or config)
- バルクレビューモード
- カスタムmaturityレベル
- レビュー履歴ログ
- レビューセッション自動進行 (sprout.kak: `SproutReviewNext` hook)

### v0.3+ (将来)

- `sprout graph`: リンクグラフの視覚化
- Kakoune inline maturity表示 (highlighter)
- エクスポート機能 (CSV, Anki)
