# sprout.kak -- Kakoune integration for sprout evergreen note CLI
# Requires: sprout binary in PATH

# ─── User mode ───────────────────────────────────────────────────────

declare-option str sprout_vault
declare-user-mode sprout

map global user s ':enter-user-mode sprout<ret>' -docstring 'sprout mode'

# ─── Commands ────────────────────────────────────────────────────────

define-command sprout-review -docstring 'List notes due for review' %{
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
        output=$(sprout review --vault "$vault" --format json 2>"$err")
        rc=$?
        if [ $rc -ne 0 ]; then
            msg=$(jq -r '.message // "unknown error"' < "$err")
            rm -f "$err"
            printf 'fail "sprout review: %s"\n' "$msg"
            exit
        fi
        rm -f "$err"
        count=$(printf '%s' "$output" | jq 'length')
        if [ "$count" = "0" ]; then
            printf 'info "No notes due for review today"\n'
            exit
        fi
        # Build menu entries: each note becomes a menu item that opens the file
        items=$(printf '%s' "$output" | jq -rj '.[] | " %{" + .relative_path + " (" + .maturity + ", interval:" + (.review_interval|tostring) + "d)} %{edit %{" + .path + "}}"')
        printf 'menu%s\n' "$items"
    }
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
        output=$(sprout list --vault "$vault" --format json 2>"$err")
        rc=$?
        if [ $rc -ne 0 ]; then
            msg=$(jq -r '.message // "unknown error"' < "$err")
            rm -f "$err"
            printf 'fail "sprout list: %s"\n' "$msg"
            exit
        fi
        rm -f "$err"
        count=$(printf '%s' "$output" | jq 'length')
        if [ "$count" = "0" ]; then
            printf 'info "No tracked notes"\n'
            exit
        fi
        # Build menu entries: same pattern as sprout-review
        items=$(printf '%s' "$output" | jq -rj '.[] | " %{" + .relative_path + " (" + .maturity + ", interval:" + (.review_interval|tostring) + "d)} %{edit %{" + .path + "}}"')
        printf 'menu%s\n' "$items"
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
map global sprout ? ':sprout-show<ret>'                      -docstring 'show note info'
