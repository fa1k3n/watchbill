# Bash completion of wb and watchbill (#220). The RPM installs it as
# /usr/share/bash-completion/completions/wb, and bash-completion loads it the first time wb is typed.
# zsh can load it with bashcompinit. It runs no wb and no pattern, asks nothing of the network, and
# writes nothing: it reads file names and asks git for the root of the checkout. The lists below are
# held to the commands, the usage and the roles of wb by tests/completion.rs.

_wb_commands="init check doctor upgrade plan accept start status view answer revert clean prompt revision compare"
_wb_roles="build fix planner review tester"
_wb_stages=("Not planned" Planned Accepted Tested Built Reviewed Fixed "Questions open" "Needs a person" "Not read")

_wb_flags_init="--yes --survey --runner --quiet"
_wb_flags_check="--quiet"
_wb_flags_doctor="--quiet"
_wb_flags_upgrade="--dry-run --quiet"
_wb_flags_plan="--file --message --from --source --level --hash-only --no-commit --quiet"
_wb_flags_accept="--no-commit --quiet"
_wb_flags_start="--runner --prompt --ignore-local --ignore-kept --ignore-branch --no-push --take-back --issue --change --requirements --round --checkout --branch --issue_address --change_address --classed --classed_touched --description --brought --questions --change_comment --needs_person --parts --stamp --show-origin --list --verbose --quiet"
_wb_flags_status="--stage --session --quiet"
_wb_flags_view="--in --out --kept --verbose --quiet"
_wb_flags_answer="--change --show --suggested --quiet"
_wb_flags_revert="--change --round --quiet"
_wb_flags_clean="--dry-run --quiet"
_wb_flags_compare="--from --quiet"
_wb_flags_prompt="--issue --change --requirements --round --checkout --branch --issue_address --change_address --runner --classed --classed_touched --description --brought --questions --change_comment --needs_person --parts --stamp --show-origin --quiet"
_wb_flags_revision="--quiet"

# The short forms of the flags, as the usage names them (#285): offered after a single - (the long ones after --).
_wb_short_init="-q -y"
_wb_short_check="-q"
_wb_short_doctor="-q"
_wb_short_upgrade="-n -q"
_wb_short_plan="-F -m -n -q"
_wb_short_accept="-n -q"
_wb_short_start="-q -v"
_wb_short_status="-q"
_wb_short_view="-q -v"
_wb_short_answer="-q"
_wb_short_revert="-q"
_wb_short_clean="-n -q"
_wb_short_prompt="-q"
_wb_short_revision="-q"
_wb_short_compare="-q"

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

    # The values of wb plan add: a file after -F, and nothing for the others.
    if [[ $command == plan ]]; then
        case $prev in
        -F | --file)
            mapfile -t files < <(compgen -f -- "$cur")
            COMPREPLY=("${files[@]}")
            compopt -o filenames 2>/dev/null
            return 0
            ;;
        -m | --message | --from | --source | --level)
            return 0
            ;;
        esac
    fi

    # The value of a flag that takes one: the headings of wb status, and nothing for the others.
    case $prev in
    --stage)
        stage=${cur//\\/}
        for word in "${_wb_stages[@]}"; do
            [[ ${word,,} == "${stage,,}"* ]] && COMPREPLY+=("$(printf '%q' "$word")")
        done
        return 0
        ;;
    --from)
        # The root of a repository: a folder.
        mapfile -t files < <(compgen -d -- "$cur")
        COMPREPLY=("${files[@]}")
        compopt -o filenames 2>/dev/null
        return 0
        ;;
    --session | --runner | --issue | --change | --requirements | --round | --checkout | --branch | --issue_address | --change_address)
        return 0
        ;;
    esac

    if [[ $cur == --* ]]; then
        var=_wb_flags_$command
        COMPREPLY=($(compgen -W "${!var-}" -- "$cur"))
        return 0
    fi

    if [[ $cur == -* && $cur != --* ]]; then
        var=_wb_short_$command
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
        --stage | --session | --runner | --issue | --requirements | --round | --checkout | --branch | --issue_address | --change_address)
            ((i++))
            ;;
        -F | -m | --file | --message | --source | --level)
            [[ $command == plan ]] && ((i++))
            ;;
        --from)
            # --from of wb plan add takes an adapter and a number; that of wb compare a folder.
            if [[ $command == plan ]]; then ((i += 2)); else ((i++)); fi
            ;;
        --* | -q | -n | -v | -y) ;;
        *) position=$((position + 1)) ;;
        esac
    done

    # --change <n> takes the place of the issue in revert and answer.
    case $command in
    revert | answer) position=$((position + changed)) ;;
    esac

    case $command in
    plan)
        # The subcommands, then the issues for those that take the name of a plan (#290).
        case $position in
        0) COMPREPLY=($(compgen -W "add list show edit accept remove" -- "$cur")) ;;
        1)
            case ${COMP_WORDS[2]} in
            show | edit | accept | remove) _wb_add_issues ;;
            esac
            ;;
        esac
        ;;
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
