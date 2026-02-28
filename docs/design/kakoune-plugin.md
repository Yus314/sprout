# Kakoune プラグイン設計

## ユーザーモードマッピング

- `user s` でsproutモードに入る（既存の `user l` (LSPモード) と同じパターン）
- 注: ノーマルモードの `s` はDvorak設定で `j` にリマップされているが、user-modeマッピングは独立

## コマンド

| コマンド | 動作 |
|---------|------|
| `sprout-review` | `fzf` でレビュー予定ノート一覧表示、選択で `edit`（fzf未検出時は `menu` フォールバック） |
| `sprout-done <rating>` | 現在のバッファを評価、`edit!` でリロード |
| `sprout-promote <maturity>` | 成熟度を変更 |
| `sprout-init` | 現在のバッファにフロントマター追加 |
| `sprout-stats` | `info` ボックスに統計表示 |
| `sprout-list` | `fzf` で全ノート一覧表示、選択で `edit`（fzf未検出時は `menu` フォールバック） |
| `sprout-show` | 現在のバッファのノート詳細を `info` ボックスに表示 |
| `sprout-note` | fzf で全ノートから選択、未選択ならクエリからノート新規作成 |

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
| `n` | note | ノート検索/新規作成 |
| `?` | show | ノート詳細表示 |

## 実装 (sprout.kak)

```kak
# sprout.kak -- Kakoune integration for sprout evergreen note CLI
# Requires: sprout binary in PATH

# ─── User mode ───────────────────────────────────────────────────────

declare-option str sprout_vault
declare-option str sprout_fzf_opts ''
declare-user-mode sprout

map global user s ':enter-user-mode sprout<ret>' -docstring 'sprout mode'

# ─── Commands ────────────────────────────────────────────────────────

define-command _sprout-fzf-select -hidden -params 1 \
    -docstring 'internal: launch fzf for sprout review/list' %{
    evaluate-commands %sh{
        subcmd="$1"
        session="$kak_session"
        client="$kak_client"
        fzf_opts="$kak_opt_sprout_fzf_opts"

        # Resolve vault
        if [ -n "$kak_opt_sprout_vault" ]; then
            vault="$kak_opt_sprout_vault"
        elif [ -n "$SPROUT_VAULT" ]; then
            vault="$SPROUT_VAULT"
        elif [ -n "$kak_buffile" ] && [ -f "$kak_buffile" ]; then
            vault=$(dirname "$kak_buffile")
        else
            vault="$PWD"
        fi

        # Run sprout subcommand
        err=$(mktemp)
        output=$(sprout "$subcmd" --vault "$vault" --format json 2>"$err")
        rc=$?
        if [ $rc -ne 0 ]; then
            msg=$(jq -r '.message // "unknown error"' < "$err")
            rm -f "$err"
            printf 'fail "sprout %s: %s"\n' "$subcmd" "$msg"
            exit
        fi
        rm -f "$err"

        count=$(printf '%s' "$output" | jq 'length')
        if [ "$count" = "0" ]; then
            case "$subcmd" in
                review) printf 'info "No notes due for review today"\n' ;;
                list)   printf 'info "No tracked notes"\n' ;;
            esac
            exit
        fi

        # fzf unavailable → fallback to menu
        if ! command -v fzf >/dev/null 2>&1; then
            items=$(printf '%s' "$output" | jq -rj '.[] | " %{" + .relative_path + " (" + .maturity + ")} %{edit %{" + .path + "}}"')
            printf 'menu%s\n' "$items"
            exit
        fi

        # Write fzf input to temp file: path<TAB>label per line
        candidates_file=$(mktemp "${TMPDIR:-/tmp}/sprout-fzf-cands-XXXXXX")
        printf '%s' "$output" | jq -r '.[] | .path + "\t" + .relative_path + " (" + .maturity + ")"' > "$candidates_file"

        # Generate preview script (frontmatter skip + bat highlight)
        preview_script=$(mktemp "${TMPDIR:-/tmp}/sprout-fzf-preview-XXXXXX.sh")
        if command -v bat >/dev/null 2>&1; then
            bat --paging=never --style=plain --color=always /dev/null >/dev/null 2>&1 &
            cat > "$preview_script" << 'PREVIEW_OUTER'
#!/bin/sh
cache_dir="$2"
if [ -d "$cache_dir" ]; then
    hash=$(printf '%s' "$1" | cksum | cut -d' ' -f1)
    cached="$cache_dir/$hash.md"
    if [ ! -f "$cached" ]; then
        tmp="$cached.tmp.$$"
        cp -- "$1" "$tmp" 2>/dev/null && mv -f "$tmp" "$cached" || rm -f "$tmp"
    fi
    [ -f "$cached" ] && src="$cached" || src="$1"
else
    src="$1"
fi
end=$(awk 'NR==1 && !/^---/ { print 1; exit } /^---/ && NR>1 { print NR+1; exit } NR>200 { print 1; exit }' "$src")
bat --line-range="${end:-1}:+49" --style=plain --color=always --paging=never -- "$src"
PREVIEW_OUTER
            preview_cache=$(mktemp -d "${TMPDIR:-/tmp}/sprout-preview-cache-XXXXXX")
        else
            cat > "$preview_script" << 'PREVIEW_OUTER'
#!/bin/sh
awk 'NR==1&&/^---/{f=1;next} f&&/^---/{f=0;next} f{next} {if(++n>50)exit;print}' "$1"
PREVIEW_OUTER
            preview_cache=""
        fi
        chmod +x "$preview_script"

        # Generate temporary script
        script=$(mktemp "${TMPDIR:-/tmp}/sprout-fzf-XXXXXX.sh")
        cat > "$script" << OUTER
#!/bin/sh
${preview_cache:+trap 'rm -rf "$preview_cache"' EXIT INT TERM}
candidates_file="\$1"
session="\$2"
client="\$3"
fzf_opts="\$4"
script="\$5"
selected=\$(fzf \\
    --delimiter='\\t' --with-nth=2.. \\
    --preview='$preview_script {1} $preview_cache' \\
    --preview-window=right:50%:wrap \\
    \$fzf_opts < "\$candidates_file")
if [ -n "\$selected" ]; then
    file=\$(printf '%s' "\$selected" | cut -f1)
    printf 'evaluate-commands -client %s edit %%{%s}\\n' "\$client" "\$file" | kak -p "\$session"
fi
rm -f "\$candidates_file" "\$script" "$preview_script"
OUTER
        chmod +x "$script"

        if [ -n "$TMUX" ]; then
            printf "nop %%sh{ tmux popup -E -w 80%% -h 80%% sh '%s' '%s' '%s' '%s' '%s' '%s' & }\n" \
                "$script" "$candidates_file" "$session" "$client" "$fzf_opts" "$script"
        else
            printf "terminal sh '%s' '%s' '%s' '%s' '%s' '%s'\n" \
                "$script" "$candidates_file" "$session" "$client" "$fzf_opts" "$script"
        fi
    }
}

define-command sprout-review -docstring 'List notes due for review' %{
    _sprout-fzf-select review
}

define-command sprout-done -params 1 -docstring 'sprout-done <hard|good|easy>: rate the current note' %{
    write
    evaluate-commands %sh{
        file="$kak_buffile"
        rating="$1"
        err=$(mktemp)
        output=$(sprout done "$file" "$rating" --format json 2>"$err")
        rc=$?
        if [ $rc -ne 0 ]; then
            msg=$(jq -r '.message // "unknown error"' < "$err")
            rm -f "$err"
            printf 'fail "sprout done: %s"\n' "$msg"
            exit
        fi
        rm -f "$err"
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
        err=$(mktemp)
        output=$(sprout promote "$file" "$maturity" --format json 2>"$err")
        rc=$?
        if [ $rc -ne 0 ]; then
            msg=$(jq -r '.message // "unknown error"' < "$err")
            rm -f "$err"
            printf 'fail "sprout promote: %s"\n' "$msg"
            exit
        fi
        rm -f "$err"
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
        err=$(mktemp)
        output=$(sprout init "$file" --format json 2>"$err")
        rc=$?
        if [ $rc -ne 0 ]; then
            msg=$(jq -r '.message // "unknown error"' < "$err")
            rm -f "$err"
            printf 'fail "sprout init: %s"\n' "$msg"
            exit
        fi
        rm -f "$err"
        maturity=$(printf '%s' "$output" | jq -r '.maturity')
        next_review=$(printf '%s' "$output" | jq -r '.next_review')
        printf 'info "Initialized: %s, next review: %s"\n' "$maturity" "$next_review"
        printf 'edit!\n'
        printf 'trigger-user-hook SproutInit\n'
    }
}

define-command sprout-stats -docstring 'Show vault statistics in info box' %{
    evaluate-commands %sh{
        # Resolve vault: kak option > SPROUT_VAULT env > dirname of buffile
        if [ -n "$kak_opt_sprout_vault" ]; then
            vault="$kak_opt_sprout_vault"
        elif [ -n "$SPROUT_VAULT" ]; then
            vault="$SPROUT_VAULT"
        elif [ -n "$kak_buffile" ] && [ -f "$kak_buffile" ]; then
            vault=$(dirname "$kak_buffile")
        else
            vault="$PWD"
        fi
        err=$(mktemp)
        output=$(sprout stats --vault "$vault" --format json 2>"$err")
        rc=$?
        if [ $rc -ne 0 ]; then
            msg=$(jq -r '.message // "unknown error"' < "$err")
            rm -f "$err"
            printf 'fail "sprout stats: %s"\n' "$msg"
            exit
        fi
        rm -f "$err"
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
    _sprout-fzf-select list
}

define-command sprout-note -docstring 'Open or create a note via fzf' %{
    evaluate-commands %sh{
        session="$kak_session"
        client="$kak_client"
        fzf_opts="$kak_opt_sprout_fzf_opts"

        # Resolve vault
        if [ -n "$kak_opt_sprout_vault" ]; then
            vault="$kak_opt_sprout_vault"
        elif [ -n "$SPROUT_VAULT" ]; then
            vault="$SPROUT_VAULT"
        elif [ -n "$kak_buffile" ] && [ -f "$kak_buffile" ]; then
            vault=$(dirname "$kak_buffile")
        else
            vault="$PWD"
        fi

        # Run sprout note (list all .md files)
        err=$(mktemp)
        output=$(sprout note --vault "$vault" --format json 2>"$err")
        rc=$?
        if [ $rc -ne 0 ]; then
            msg=$(jq -r '.message // "unknown error"' < "$err")
            rm -f "$err"
            printf 'fail "sprout note: %s"\n' "$msg"
            exit
        fi
        rm -f "$err"

        # fzf required for this command
        if ! command -v fzf >/dev/null 2>&1; then
            printf 'fail "sprout-note requires fzf"\n'
            exit
        fi

        # Write fzf input: path<TAB>relative_path per line
        candidates_file=$(mktemp "${TMPDIR:-/tmp}/sprout-note-cands-XXXXXX")
        printf '%s' "$output" | jq -r '.[] | .path + "\t" + .relative_path' > "$candidates_file"

        # Preview script (frontmatter skip + bat highlight)
        preview_script=$(mktemp "${TMPDIR:-/tmp}/sprout-note-preview-XXXXXX.sh")
        if command -v bat >/dev/null 2>&1; then
            bat --paging=never --style=plain --color=always /dev/null >/dev/null 2>&1 &
            cat > "$preview_script" << 'PREVIEW_OUTER'
#!/bin/sh
cache_dir="$2"
if [ -d "$cache_dir" ]; then
    hash=$(printf '%s' "$1" | cksum | cut -d' ' -f1)
    cached="$cache_dir/$hash.md"
    if [ ! -f "$cached" ]; then
        tmp="$cached.tmp.$$"
        cp -- "$1" "$tmp" 2>/dev/null && mv -f "$tmp" "$cached" || rm -f "$tmp"
    fi
    [ -f "$cached" ] && src="$cached" || src="$1"
else
    src="$1"
fi
end=$(awk 'NR==1 && !/^---/ { print 1; exit } /^---/ && NR>1 { print NR+1; exit } NR>200 { print 1; exit }' "$src")
bat --line-range="${end:-1}:+49" --style=plain --color=always --paging=never -- "$src"
PREVIEW_OUTER
            preview_cache=$(mktemp -d "${TMPDIR:-/tmp}/sprout-preview-cache-XXXXXX")
        else
            cat > "$preview_script" << 'PREVIEW_OUTER'
#!/bin/sh
awk 'NR==1&&/^---/{f=1;next} f&&/^---/{f=0;next} f{next} {if(++n>50)exit;print}' "$1"
PREVIEW_OUTER
            preview_cache=""
        fi
        chmod +x "$preview_script"

        # Main fzf script: --print-query to capture typed query when nothing is selected
        script=$(mktemp "${TMPDIR:-/tmp}/sprout-note-XXXXXX.sh")
        cat > "$script" << OUTER
#!/bin/sh
${preview_cache:+trap 'rm -rf "$preview_cache"' EXIT INT TERM}
candidates_file="\$1"
session="\$2"
client="\$3"
fzf_opts="\$4"
script="\$5"
vault="\$6"

result=\$(fzf \\
    --delimiter='\\t' --with-nth=2.. \\
    --print-query \\
    --preview='$preview_script {1} $preview_cache' \\
    --preview-window=right:50%:wrap \\
    \$fzf_opts < "\$candidates_file")

query=\$(printf '%s' "\$result" | sed -n '1p')
selected=\$(printf '%s' "\$result" | sed -n '2p')

if [ -n "\$selected" ]; then
    # User selected an existing note
    file=\$(printf '%s' "\$selected" | cut -f1)
    printf 'evaluate-commands -client %s edit %%{%s}\\n' "\$client" "\$file" | kak -p "\$session"
elif [ -n "\$query" ]; then
    # No selection but query typed → create new note
    err=\$(mktemp)
    create_output=\$(sprout note "\$query" --vault "\$vault" --format json 2>"\$err")
    rc=\$?
    if [ \$rc -ne 0 ]; then
        msg=\$(jq -r '.message // "unknown error"' < "\$err")
        rm -f "\$err"
        printf 'evaluate-commands -client %s fail "sprout note: %s"\\n' "\$client" "\$msg" | kak -p "\$session"
    else
        rm -f "\$err"
        file=\$(printf '%s' "\$create_output" | jq -r '.path')
        printf 'evaluate-commands -client %s "edit %%{%s}; trigger-user-hook SproutNote"\\n' "\$client" "\$file" | kak -p "\$session"
    fi
fi
rm -f "\$candidates_file" "\$script" "$preview_script"
OUTER
        chmod +x "$script"

        if [ -n "$TMUX" ]; then
            printf "nop %%sh{ tmux popup -E -w 80%% -h 80%% sh '%s' '%s' '%s' '%s' '%s' '%s' '%s' & }\n" \
                "$script" "$candidates_file" "$session" "$client" "$fzf_opts" "$script" "$vault"
        else
            printf "terminal sh '%s' '%s' '%s' '%s' '%s' '%s' '%s'\n" \
                "$script" "$candidates_file" "$session" "$client" "$fzf_opts" "$script" "$vault"
        fi
    }
}

define-command sprout-show -docstring 'Show detailed info about current note' %{
    evaluate-commands %sh{
        err=$(mktemp)
        output=$(sprout show "$kak_buffile" --format json 2>"$err")
        rc=$?
        if [ $rc -ne 0 ]; then
            msg=$(jq -r '.message // "unknown error"' < "$err")
            rm -f "$err"
            printf 'fail "sprout show: %s"\n' "$msg"
            exit
        fi
        rm -f "$err"
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
map global sprout n ':sprout-note<ret>'                      -docstring 'open/create note'
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
| `SproutNote` | `sprout-note` で新規ノート作成後 | — |

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
- **fzf統合**: fzf でノート選択、bat プレビュー（frontmatter スキップ）、tmux popup 対応、プレビューキャッシュ（cksum ベース）、bat ページキャッシュプレウォーム。fzf 未検出時は `menu` フォールバック（`sprout-note` は fzf 必須で `fail`）
- **エラーハンドリング**: stderr を一時ファイルにキャプチャし `jq -r '.message'` でエラーメッセージを抽出。終了コードとJSON両方で判定
- **JSON一貫性**: 全コマンドで `--format json` を使用し、human出力形式への依存を排除。プラグイン側で表示文字列を構築する
- **フックのエディタ層実装**: CLIは純粋なデータ変換に徹し、ワークフロー拡張はKakouneの`trigger-user-hook`で実現。他エディタも各自のイベント機構（Neovim: `User` autocmd、Emacs: `run-hooks`）で同等の実装が可能
- **sprout_vault オプション**: vault パスの Kakoune 側上書き（`set-option global sprout_vault /path/to/vault`）。未設定時は `SPROUT_VAULT` → buffile の親 → CWD の順で解決。Kakoune 側が常に `--vault` を渡すため、`config.toml` の `vault_path` は使われない
- **sprout_fzf_opts オプション**: fzf に渡す追加オプション（例: `--height 40%`）
- **bat依存（オプション）**: プレビューの syntax highlight に bat 使用。未検出時は awk 代替
- **tmux依存（オプション）**: tmux 検出時に `tmux popup -E -w 80% -h 80%` でポップアップ UI
