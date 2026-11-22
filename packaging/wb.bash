# Bash completion of wb and watchbill (#220). The RPM installs it as
# /usr/share/bash-completion/completions/wb, and bash-completion loads it the first time wb is typed.
# zsh can load it with bashcompinit. It runs no wb and no pattern, asks nothing of the network, and
# writes nothing: it reads file names and asks git for the root of the checkout. The lists below are
# held to the commands, the usage and the roles of wb by tests/completion.rs.

_wb_commands="init check upgrade accept start status view answer revert prompt revision"
_wb_roles="breakdown build fix review tester"
_wb_stages=("Not broken down" "Broken down" Accepted Tested Built Reviewed Fixed "Questions open" "Not read")

_wb_flags_init="--yes"
_wb_flags_upgrade="--dry-run"
_wb_flags_accept="--no-commit"
_wb_flags_start="--runner --prompt --ignore-local --ignore-kept --issue --change --requirements --round --checkout --branch --issue_address --change_address --classed --classed_touched --parts --stamp"
_wb_flags_status="--stage"
_wb_flags_view="--in --out"
_wb_flags_answer="--change --show --suggested"
_wb_flags_revert="--change --round"
_wb_flags_prompt="--issue --change --requirements --round --checkout --branch --issue_address --change_address --runner --classed --classed_touched --parts --stamp"

# Adds to COMPREPLY the issues that start with $cur: the names of the files .watchbill/issues/<name>.yml of the
# checkout that wb takes as an issue (letters, digits, . _ -, at most 64, not starting with a dot). A name read
# from disk is only matched here, never expanded: it does not go through compgen -W, eval or a command.
_wb_add_issues() {
    local top file name
    top=$(git rev-parse --show-toplevel 2>/dev/null) || return 0
    for file in "$top"/.watchbill/issues/*.yml; do
        [[ -e $file ]] || continue
        name=${file##*/}
        name=${name%.yml}
        [[ $name =~ ^[A-Za-z0-9_-][A-Za-z0-9._-]{0,63}$ ]] || continue
        [[ $name == "$cur"* ]] && COMPREPLY+=("$name")
    done
    return 0
}

_wb() {
    local cur=${COMP_WORDS[COMP_CWORD]} prev="" command word i position=0 flags var stage files changed=0
    COMPREPLY=()
    ((COMP_CWORD > 0)) && prev=${COMP_WORDS[COMP_CWORD - 1]}

    if ((COMP_CWORD == 1)); then
        COMPREPLY=($(compgen -W "$_wb_commands help --version --help" -- "$cur"))
        return 0
    fi
    command=${COMP_WORDS[1]}
    if [[ $command == help ]]; then
        ((COMP_CWORD == 2)) && COMPREPLY=($(compgen -W "$_wb_commands" -- "$cur"))
        return 0
    fi
    case " $_wb_commands " in
    *" $command "*) ;;
    *) return 0 ;;
    esac

    # The value of a flag that takes one: the headings of wb status, and nothing for the others.
    case $prev in
    --stage)
        stage=${cur//\\/}
        for word in "${_wb_stages[@]}"; do
            [[ ${word,,} == "${stage,,}"* ]] && COMPREPLY+=("$(printf '%q' "$word")")
        done
        return 0
        ;;
    --runner | --issue | --change | --requirements | --round | --checkout | --branch | --issue_address | --change_address)
        return 0
        ;;
    esac

    if [[ $cur == --* ]]; then
        var=_wb_flags_$command
        COMPREPLY=($(compgen -W "${!var-}" -- "$cur"))
        return 0
    fi

    # The words that are neither flags nor the values of flags, between the command and the cursor.
    for ((i = 2; i < COMP_CWORD; i++)); do
        word=${COMP_WORDS[i]}
        case $word in
        --change)
            changed=1
            ((i++))
            ;;
        --stage | --runner | --issue | --requirements | --round | --checkout | --branch | --issue_address | --change_address)
            ((i++))
            ;;
        --*) ;;
        *) position=$((position + 1)) ;;
        esac
    done

    # --change <n> takes the place of the issue in revert and answer.
    case $command in
    revert | answer) position=$((position + changed)) ;;
    esac

    case $command in
    prompt | start)
        case $position in
        0) COMPREPLY=($(compgen -W "$_wb_roles" -- "$cur")) ;;
        1) _wb_add_issues ;;
        esac
        ;;
    revert)
        case $position in
        0) _wb_add_issues ;;
        1) COMPREPLY=($(compgen -W "$_wb_roles" -- "$cur")) ;;
        esac
        ;;
    accept | status | answer)
        ((position == 0)) && _wb_add_issues
        ;;
    view)
        case $position in
        0) _wb_add_issues ;;
        1)
            COMPREPLY=($(compgen -W "$_wb_roles" -- "$cur"))
            mapfile -t files < <(compgen -f -- "$cur")
            COMPREPLY+=("${files[@]}")
            compopt -o filenames 2>/dev/null
            ;;
        esac
        ;;
    esac
    return 0
}

complete -F _wb wb watchbill
