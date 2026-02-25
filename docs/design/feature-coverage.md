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

## v0.1 カバー率

- **org-roam-review**: コア機能の約80%をカバー（タグフィルタ・バルクレビュー除く）
- **obsidian-sr**: ノートレビュー機能の約70%をカバー（フラッシュカード・モバイル除く）

## 優先度

### v0.1 (MVP)

1. フロントマターのパース・ラウンドトリップ
2. SRS基本計算（ease, interval, delay）
3. 全6コマンド（review, done, promote, stats, init, list）
4. JSON出力
5. Kakouneプラグイン
6. Nixパッケージング

### v0.2 (検討)

- タグフィルタリング (`sprout review --tag`)
- フォルダ無視設定 (`.sproutignore` or config)
- バルクレビューモード
- カスタムmaturityレベル
- レビュー履歴ログ

### v0.3+ (将来)

- `sprout graph`: リンクグラフの視覚化
- Kakoune inline maturity表示 (highlighter)
- エクスポート機能 (CSV, Anki)
