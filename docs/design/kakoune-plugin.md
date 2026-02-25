# Kakoune プラグイン設計

## ユーザーモードマッピング

- `user s` でsproutモードに入る（既存の `user l` (LSPモード) と同じパターン）
- 注: ノーマルモードの `s` はDvorak設定で `j` にリマップされているが、user-modeマッピングは独立

## コマンド

| コマンド | 動作 |
|---------|------|
| `sprout-review` | `menu` でレビュー予定ノート一覧表示、選択で `edit` |
| `sprout-done <rating>` | 現在のバッファを評価、`edit!` でリロード |
| `sprout-promote <maturity>` | 成熟度を変更 |
| `sprout-init` | 現在のバッファにフロントマター追加 |
| `sprout-stats` | `info` ボックスに統計表示 |

## キーマッピング

| キー | 動作 | 説明 |
|------|------|------|
| `r` | review | レビュー予定ノート一覧 |
| `h` | hard | 評価: hard |
| `g` | good | 評価: good |
| `e` | easy | 評価: easy |
| `i` | init | フロントマター初期化 |
| `s` | stats | 統計表示 |
| `p` | promote seedling | seedlingに変更 |
| `b` | promote budding | buddingに変更 |
| `v` | promote evergreen | evergreenに変更 |

## 実装 (sprout.kak)

```kak
# sprout.kak -- Kakoune integration for sprout evergreen note CLI
# Requires: sprout binary in PATH

# ─── User mode ───────────────────────────────────────────────────────

declare-user-mode sprout

map global user s ':enter-user-mode sprout<ret>' -docstring 'sprout mode'

# ─── Commands ────────────────────────────────────────────────────────

define-command sprout-review -docstring 'List notes due for review' %{
    evaluate-commands %sh{
        output=$(sprout review --format json 2>&1)
        if [ $? -ne 0 ]; then
            printf 'fail "sprout review failed: %s"\n' "$(printf '%s' "$output" | head -1)"
            exit
        fi
        count=$(printf '%s' "$output" | jq 'length')
        if [ "$count" = "0" ]; then
            printf 'info "No notes due for review today"\n'
            exit
        fi
        # Build menu entries: each note becomes a menu item that opens the file
        printf 'menu'
        printf '%s' "$output" | jq -r '.[] | " %{" + .relative_path + " (" + .maturity + ", interval:" + (.review_interval|tostring) + "d)} %{edit " + .path + "}"'
        printf '\n'
    }
}

define-command sprout-done -params 1 -docstring 'sprout-done <hard|good|easy>: rate the current note' %{
    evaluate-commands %sh{
        file="$kak_buffile"
        rating="$1"
        output=$(sprout done "$file" "$rating" --format json 2>&1)
        if [ $? -ne 0 ]; then
            printf 'fail "sprout done failed: %s"\n' "$(printf '%s' "$output" | head -1)"
            exit
        fi
        new_interval=$(printf '%s' "$output" | jq -r '.new_interval')
        next_review=$(printf '%s' "$output" | jq -r '.next_review')
        maturity=$(printf '%s' "$output" | jq -r '.maturity')
        printf 'info "Reviewed: %s → interval %sd, next: %s"\n' "$maturity" "$new_interval" "$next_review"
        # Reload buffer to reflect frontmatter changes
        printf 'edit!\n'
    }
}

define-command sprout-promote -params 1 -docstring 'sprout-promote <seedling|budding|evergreen>: set maturity' %{
    evaluate-commands %sh{
        file="$kak_buffile"
        maturity="$1"
        output=$(sprout promote "$file" "$maturity" 2>&1)
        if [ $? -ne 0 ]; then
            printf 'fail "sprout promote failed: %s"\n' "$(printf '%s' "$output" | head -1)"
            exit
        fi
        printf 'info "Promoted to: %s"\n' "$maturity"
        printf 'edit!\n'
    }
}

define-command sprout-init -docstring 'Initialize sprout frontmatter for current buffer' %{
    evaluate-commands %sh{
        file="$kak_buffile"
        output=$(sprout init "$file" 2>&1)
        if [ $? -ne 0 ]; then
            printf 'fail "sprout init failed: %s"\n' "$(printf '%s' "$output" | head -1)"
            exit
        fi
        printf 'info "Sprout frontmatter added"\n'
        printf 'edit!\n'
    }
}

define-command sprout-stats -docstring 'Show vault statistics in info box' %{
    evaluate-commands %sh{
        output=$(sprout stats --format human 2>&1)
        if [ $? -ne 0 ]; then
            printf 'fail "sprout stats failed: %s"\n' "$(printf '%s' "$output" | head -1)"
            exit
        fi
        # Escape for Kakoune info
        escaped=$(printf '%s' "$output" | sed "s/'/''/g")
        printf "info '%s'\n" "$escaped"
    }
}

# ─── Sprout mode key mappings ────────────────────────────────────────

map global sprout r ':sprout-review<ret>'                    -docstring 'review due notes'
map global sprout h ':sprout-done hard<ret>'                 -docstring 'rate: hard'
map global sprout g ':sprout-done good<ret>'                 -docstring 'rate: good'
map global sprout e ':sprout-done easy<ret>'                 -docstring 'rate: easy'
map global sprout s ':sprout-stats<ret>'                     -docstring 'show stats'
map global sprout i ':sprout-init<ret>'                      -docstring 'init frontmatter'
map global sprout p ':sprout-promote seedling<ret>'          -docstring 'promote: seedling'
map global sprout b ':sprout-promote budding<ret>'           -docstring 'promote: budding'
map global sprout v ':sprout-promote evergreen<ret>'         -docstring 'promote: evergreen'
```

## 設計上の考慮点

- **jq依存**: JSONパースに `jq` を使用。ユーザー環境で既に利用可能
- **バッファリロード**: `sprout done` と `sprout promote` 後に `edit!` を発行し、更新されたフロントマターを反映
- **menuコマンド**: Kakouneビルトインの `menu` を使用して選択可能なレビュー予定ノート一覧を表示
