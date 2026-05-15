_anonsurf()
{
    local cur prev commands config_commands shells
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    commands="start stop restart status changeid new-identity myip tor-check repair logs doctor completions config help"
    config_commands="show-default apply-default apply-file help"
    shells="bash zsh fish"

    case "${prev}" in
        anonsurf)
            COMPREPLY=( $(compgen -W "${commands}" -- "${cur}") )
            return 0
            ;;
        config)
            COMPREPLY=( $(compgen -W "${config_commands}" -- "${cur}") )
            return 0
            ;;
        completions)
            COMPREPLY=( $(compgen -W "${shells}" -- "${cur}") )
            return 0
            ;;
    esac

    COMPREPLY=( $(compgen -W "${commands}" -- "${cur}") )
}
complete -F _anonsurf anonsurf
