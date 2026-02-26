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
        cat > "$preview_script" << 'PREVIEW_OUTER'
#!/bin/sh
awk 'NR==1&&/^---/{f=1;next} f&&/^---/{f=0;next} !f{if(++n>50)exit}1' "$1" | \
    if command -v bat >/dev/null 2>&1; then
        bat -l md --style=plain --color=always --paging=never
    else
        cat
    fi
PREVIEW_OUTER
        chmod +x "$preview_script"

        # Generate temporary script
        script=$(mktemp "${TMPDIR:-/tmp}/sprout-fzf-XXXXXX.sh")
        cat > "$script" << OUTER
#!/bin/sh
candidates_file="\$1"
session="\$2"
client="\$3"
fzf_opts="\$4"
script="\$5"
selected=\$(fzf \\
    --delimiter='\\t' --with-nth=2.. \\
    --preview='$preview_script {1}' \\
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
map global sprout ? ':sprout-show<ret>'                      -docstring 'show note info'
