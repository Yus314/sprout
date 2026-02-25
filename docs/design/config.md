# 設定ファイル仕様

## ファイル場所

`~/.config/sprout/config.toml`

## 形式

```toml
# vault_path = "/home/kaki/notes"  # デフォルト: カレントディレクトリ
# max_interval = 90                # 最大インターバル（日数）
# default_ease = 2.5               # 初期ease
# link_weight = 0.1                # リンク考慮の重み（0で無効化）
# load_balance = true              # 負荷分散の有効/無効
```

## 設定パラメータ

| パラメータ | 型 | デフォルト | 説明 |
|-----------|-----|-----------|------|
| `vault_path` | string | カレントディレクトリ | ノートvaultのパス |
| `max_interval` | u32 | `90` | レビューインターバルの上限（日数） |
| `default_ease` | f64 | `2.5` | 新規ノートの初期ease factor |
| `link_weight` | f64 | `0.1` | リンクファクターの重み（0で無効化） |
| `load_balance` | bool | `true` | 負荷分散の有効化 |

## Vault パス解決順序

1. `--vault` CLIフラグ（最優先）
2. `SPROUT_VAULT` 環境変数
3. `~/.config/sprout/config.toml` の `vault_path`
4. カレントワーキングディレクトリ（フォールバック）

## Rust実装

```rust
#[derive(Deserialize, Default)]
pub struct Config {
    pub vault_path: Option<PathBuf>,
    pub max_interval: Option<u32>,        // default 90
    pub default_ease: Option<f64>,        // default 2.5
    pub link_weight: Option<f64>,         // default 0.1
    pub load_balance: Option<bool>,       // default true
}

pub fn load_config() -> Result<Config>;
```

全フィールドはオプション。設定ファイルなしでもカレントディレクトリをデフォルトとして動作する。
