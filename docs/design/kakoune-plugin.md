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
| `sprout-list` | `menu` で全ノート一覧表示、選択で `edit` |
| `sprout-show` | 現在のバッファのノート詳細を `info` ボックスに表示 |

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
| `l` | list | 全ノート一覧 |
| `?` | show | ノート詳細表示 |

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
    write
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
        # Hooks: generic then rating-specific
        printf 'trigger-user-hook SproutDone\n'
        case "$rating" in
            hard) printf 'trigger-user-hook SproutDoneHard\n' ;;
            good) printf 'trigger-user-hook SproutDoneGood\n' ;;
            easy) printf 'trigger-user-hook SproutDoneEasy\n' ;;
        esac
    }
}

define-command sprout-promote -params 1 -docstring 'sprout-promote <seedling|budding|evergreen>: set maturity' %{
    write
    evaluate-commands %sh{
        file="$kak_buffile"
        maturity="$1"
        output=$(sprout promote "$file" "$maturity" --format json 2>&1)
        if [ $? -ne 0 ]; then
            printf 'fail "sprout promote failed: %s"\n' "$(printf '%s' "$output" | head -1)"
            exit
        fi
        prev=$(printf '%s' "$output" | jq -r '.previous_maturity')
        new=$(printf '%s' "$output" | jq -r '.new_maturity')
        printf 'info "Promoted: %s → %s"\n' "$prev" "$new"
        printf 'edit!\n'
        # Hooks: generic then maturity-specific
        printf 'trigger-user-hook SproutPromote\n'
        case "$maturity" in
            seedling)  printf 'trigger-user-hook SproutPromoteSeedling\n' ;;
            budding)   printf 'trigger-user-hook SproutPromoteBudding\n' ;;
            evergreen) printf 'trigger-user-hook SproutPromoteEvergreen\n' ;;
        esac
    }
}

define-command sprout-init -docstring 'Initialize sprout frontmatter for current buffer' %{
    write
    evaluate-commands %sh{
        file="$kak_buffile"
        output=$(sprout init "$file" --format json 2>&1)
        if [ $? -ne 0 ]; then
            printf 'fail "sprout init failed: %s"\n' "$(printf '%s' "$output" | head -1)"
            exit
        fi
        maturity=$(printf '%s' "$output" | jq -r '.maturity')
        next_review=$(printf '%s' "$output" | jq -r '.next_review')
        printf 'info "Initialized: %s, next review: %s"\n' "$maturity" "$next_review"
        printf 'edit!\n'
        printf 'trigger-user-hook SproutInit\n'
    }
}

define-command sprout-stats -docstring 'Show vault statistics in info box' %{
    evaluate-commands %sh{
        output=$(sprout stats --format json 2>&1)
        if [ $? -ne 0 ]; then
            printf 'fail "sprout stats failed: %s"\n' "$(printf '%s' "$output" | head -1)"
            exit
        fi
        total=$(printf '%s' "$output" | jq -r '.total')
        seedling=$(printf '%s' "$output" | jq -r '.seedling')
        budding=$(printf '%s' "$output" | jq -r '.budding')
        evergreen=$(printf '%s' "$output" | jq -r '.evergreen')
        due=$(printf '%s' "$output" | jq -r '.due_today')
        overdue=$(printf '%s' "$output" | jq -r '.overdue')
        printf 'info "Total: %s (seedling: %s, budding: %s, evergreen: %s)\nDue today: %s, Overdue: %s"\n' \
            "$total" "$seedling" "$budding" "$evergreen" "$due" "$overdue"
    }
}

define-command sprout-list -docstring 'List all tracked notes' %{
    evaluate-commands %sh{
        output=$(sprout list --format json 2>&1)
        if [ $? -ne 0 ]; then
            printf 'fail "sprout list failed: %s"\n' "$(printf '%s' "$output" | head -1)"
            exit
        fi
        count=$(printf '%s' "$output" | jq 'length')
        if [ "$count" = "0" ]; then
            printf 'info "No tracked notes"\n'
            exit
        fi
        # Build menu entries: same pattern as sprout-review
        printf 'menu'
        printf '%s' "$output" | jq -r '.[] | " %{" + .relative_path + " (" + .maturity + ", interval:" + (.review_interval|tostring) + "d)} %{edit " + .path + "}"'
        printf '\n'
    }
}

define-command sprout-show -docstring 'Show detailed info about current note' %{
    evaluate-commands %sh{
        output=$(sprout show "$kak_buffile" --format json 2>&1)
        if [ $? -ne 0 ]; then
            printf 'fail "sprout show failed: %s"\n' "$(printf '%s' "$output" | head -1)"
            exit
        fi
        tracked=$(printf '%s' "$output" | jq -r '.tracked')
        if [ "$tracked" = "false" ]; then
            printf 'info "Not tracked by sprout"\n'
            exit
        fi
        maturity=$(printf '%s' "$output" | jq -r '.maturity')
        interval=$(printf '%s' "$output" | jq -r '.review_interval')
        next_review=$(printf '%s' "$output" | jq -r '.next_review')
        ease=$(printf '%s' "$output" | jq -r '.ease')
        is_due=$(printf '%s' "$output" | jq -r '.is_due')
        link_count=$(printf '%s' "$output" | jq -r '.link_count')
        due_label="no"
        if [ "$is_due" = "true" ]; then due_label="YES"; fi
        printf 'info "Maturity: %s\nInterval: %sd, Next: %s\nEase: %s, Links: %s\nDue: %s"\n' \
            "$maturity" "$interval" "$next_review" "$ease" "$link_count" "$due_label"
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
map global sprout l ':sprout-list<ret>'                      -docstring 'list all notes'
map global sprout ? ':sprout-show<ret>'                      -docstring 'show note info'
```

## User hook

各コマンドの成功後に `trigger-user-hook` でカスタムイベントを発火する。org-roam-reviewの`run-hooks`によるEmacs hookと同じ抽象レイヤー（エディタ）でフック機能を提供する。CLIにフック機構を持たせない設計とする。

### 発火するフック一覧

| User hook | トリガー | 発火順序 |
|-----------|---------|---------|
| `SproutDone` | `sprout-done` 成功後（評価問わず） | 1st |
| `SproutDoneHard` | `sprout-done hard` 成功後 | 2nd |
| `SproutDoneGood` | `sprout-done good` 成功後 | 2nd |
| `SproutDoneEasy` | `sprout-done easy` 成功後 | 2nd |
| `SproutPromote` | `sprout-promote` 成功後（レベル問わず） | 1st |
| `SproutPromoteSeedling` | `sprout-promote seedling` 成功後 | 2nd |
| `SproutPromoteBudding` | `sprout-promote budding` 成功後 | 2nd |
| `SproutPromoteEvergreen` | `sprout-promote evergreen` 成功後 | 2nd |
| `SproutInit` | `sprout-init` 成功後 | — |

汎用フック（`SproutDone`, `SproutPromote`）が先に発火し、次に個別フックが発火する。失敗時（`fail`）にはフックは発火しない。

`sprout-review`, `sprout-list`, `sprout-show`, `sprout-stats` は読み取り専用コマンドのためフックを発火しない。

### ユーザー設定例

```kak
# レビュー後に自動git commit
hook global User SproutDone %{
    nop %sh{
        cd "$(dirname "$kak_buffile")"
        git add "$kak_buffile"
        git commit -m "review: $(basename "$kak_buffile")" --quiet
    }
}

# hard評価のノートをログに記録
hook global User SproutDoneHard %{
    nop %sh{
        echo "$(date +%Y-%m-%d) HARD: $kak_buffile" >> ~/.local/share/sprout/review.log
    }
}

# レビュー完了後に次のノートを自動で開く
hook global User SproutDone %{
    sprout-review
}
```

## 設計上の考慮点

- **jq依存**: JSONパースに `jq` を使用。ユーザー環境で既に利用可能
- **バッファリロード**: `sprout done` と `sprout promote` 後に `edit!` を発行し、更新されたフロントマターを反映
- **バッファ保存の保証**: ファイルを変更する3コマンド（`sprout-done`, `sprout-promote`, `sprout-init`）は `evaluate-commands %sh{...}` の前に `write` を発行し、未保存の編集内容がCLIのファイル書き換えで消失するのを防ぐ
- **menuコマンド**: Kakouneビルトインの `menu` を使用して選択可能なノート一覧を表示（`sprout-review`, `sprout-list`）
- **エラー分離の安全性**: CLIはエラー時にstdoutを空にする規約のため、`2>&1` でstdoutとstderrを結合してもJSONパースが壊れない
- **JSON一貫性**: 全コマンドで `--format json` を使用し、human出力形式への依存を排除。プラグイン側で表示文字列を構築する
- **フックのエディタ層実装**: CLIは純粋なデータ変換に徹し、ワークフロー拡張はKakouneの`trigger-user-hook`で実現。他エディタも各自のイベント機構（Neovim: `User` autocmd、Emacs: `run-hooks`）で同等の実装が可能
