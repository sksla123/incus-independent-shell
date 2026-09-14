#!/bin/bash

set -u

PATH='/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin'
export PATH

DENY_MSG='This command is not permitted on the host.
Run workloads inside an Incus container.'

HISTFILE="${HOME:-/tmp}/.incus-only-shell_history"
HISTSIZE=1000
HISTFILESIZE=2000
export HISTFILE HISTSIZE HISTFILESIZE

HOST_COMMANDS=(
    ls pwd clear grep
    whoami id groups who w uptime uname
    free vmstat mpstat lscpu ps
    ss ping traceroute tracepath
    df du lsblk findmnt mountpoint blkid
    gpu-stat
)

deny() {
    printf '%s\n' "$DENY_MSG" >&2
}

banner() {
    cat <<'BANNER'
This host shell is restricted.

Use this shell primarily for Incus commands.
Do not run workloads or perform system administration on the host.

Run applications and arbitrary commands inside your Incus containers.

The "bash" command is available when you explicitly need a normal host shell.
Commands executed inside bash are not filtered by this wrapper.

Type "help" to see available commands.
BANNER
}

help_text() {
    echo '===== Incus help ====='
    echo
    /usr/bin/incus --help
    echo
    echo '===== Host commands allowed by this shell ====='

    cat <<'HELP'

Navigation:
  ls
      List files and directories.

  cd
      Change the current directory.

  pwd
      Show the current directory.

  clear
      Clear the terminal.

  history
      Show incus-only-shell command history.

  grep
      Search text.

      grep is also allowed as the final stage of one
      read-only pipeline.

      Examples:
        ps aux | grep incus
        incus exec c1 -- sshd -T | grep passwordauthentication

  help
      Show Incus help and this allowed-command list.

  exit
  logout
      Exit this shell.


Host shell:
  bash
      Start a normal Bash shell on the host.

      WARNING:
      Commands inside Bash are NOT filtered by incus-only-shell.

      Exit Bash to return to incus-only-shell.


Identity / session:
  whoami
      Show your current username.

  id
      Show UID, GID and group membership.

  groups
      Show group membership.

  who
  w
      Show logged-in users and sessions.

  uptime
      Show host uptime and load averages.

  hostname
      Show the host name.
      Hostname modification is not permitted.

  uname
      Show kernel and system information.


CPU / memory / process monitoring:
  free
      Show memory usage.

  vmstat
      Show virtual memory and system statistics.

  mpstat
      Show CPU statistics.

  lscpu
      Show CPU information.

  ps
      Show process information.

  top
      Run top in secure mode.

  htop
      Run htop in read-only mode.

  btop
      Not permitted because process-control functions cannot
      be reliably disabled by this wrapper.


Network monitoring:
  ss
      Show sockets and network connections.

  ping
      Test network reachability.

  traceroute
  tracepath
      Trace network paths.

  resolvectl
      Inspect resolver and DNS state.
      Resolver configuration changes are not permitted.

  ip
      Read-only network inspection.

      Allowed examples:
        ip addr
        ip address
        ip link
        ip route
        ip neigh
        ip -br addr
        ip -br link

      Network modification is not permitted.


Disk / filesystem monitoring:
  df
      Show filesystem usage.

  du
      Show directory/file disk usage.

  lsblk
      Show block devices.

  findmnt
      Show mounted filesystems.

  mountpoint
      Check whether a path is a mount point.

  blkid
      Show block-device identifiers.


GPU monitoring:
  gpu-stat
      Show GPU usage when installed.

  nvidia-smi
      Show NVIDIA GPU status when available.
      GPU configuration changes are not permitted.


Incus:
  incus ...
      All Incus subcommands and options are passed directly to Incus.

      Authorization is enforced by your Incus project.

Examples:
  incus list
  incus launch images:debian/13/cloud c1
  incus exec c1 -- bash
  incus exec c1 -- bash -c 'echo hello && id'

Multiline example:
  incus exec c1 -- sh -c '
  echo hello
  id
  ip route
  '

Pipeline example:
  incus exec c1 -- sshd -T | grep passwordauthentication


Note:
  Pasting multiple independent commands as several separate lines
  is not supported by this wrapper.

  Run independent commands one at a time.

Commands not listed above are not permitted directly by this wrapper.

If you explicitly start "bash", this wrapper is no longer filtering
commands until you exit from Bash.
HELP
}

is_host_command() {
    local cmd="$1"
    local x

    for x in "${HOST_COMMANDS[@]}"; do
        [[ "$cmd" == "$x" ]] && return 0
    done

    return 1
}

quote_state() {
    local input="$1"
    local len=${#input}
    local i=0
    local ch
    local state="plain"

    while (( i < len )); do
        ch="${input:i:1}"

        case "$state" in
            plain)
                case "$ch" in
                    "'")
                        state="single"
                        ;;

                    '"')
                        state="double"
                        ;;

                    "\\")
                        if (( i + 1 >= len )); then
                            printf '%s' "escape"
                            return 0
                        fi

                        ((i++))
                        ;;
                esac
                ;;

            single)
                if [[ "$ch" == "'" ]]; then
                    state="plain"
                fi
                ;;

            double)
                case "$ch" in
                    '"')
                        state="plain"
                        ;;

                    "\\")
                        if (( i + 1 >= len )); then
                            printf '%s' "escape"
                            return 0
                        fi

                        ((i++))
                        ;;
                esac
                ;;
        esac

        ((i++))
    done

    printf '%s' "$state"
}

split_top_level_commands() {
    local input="$1"
    local len=${#input}
    local i=0
    local ch
    local state="plain"
    local current=""

    COMMANDS=()

    while (( i < len )); do
        ch="${input:i:1}"

        case "$state" in
            plain)
                case "$ch" in
                    "'")
                        state="single"
                        current+="$ch"
                        ;;

                    '"')
                        state="double"
                        current+="$ch"
                        ;;

                    "\\")
                        current+="$ch"

                        if (( i + 1 >= len )); then
                            return 1
                        fi

                        ((i++))
                        current+="${input:i:1}"
                        ;;

                    $'\n')
                        if [[ -n "${current//[[:space:]]/}" ]]; then
                            COMMANDS+=("$current")
                        fi

                        current=""
                        ;;

                    *)
                        current+="$ch"
                        ;;
                esac
                ;;

            single)
                current+="$ch"

                if [[ "$ch" == "'" ]]; then
                    state="plain"
                fi
                ;;

            double)
                current+="$ch"

                case "$ch" in
                    '"')
                        state="plain"
                        ;;

                    "\\")
                        if (( i + 1 >= len )); then
                            return 1
                        fi

                        ((i++))
                        current+="${input:i:1}"
                        ;;
                esac
                ;;
        esac

        ((i++))
    done

    [[ "$state" == "plain" ]] || return 1

    if [[ -n "${current//[[:space:]]/}" ]]; then
        COMMANDS+=("$current")
    fi

    return 0
}

split_safe_grep_pipeline() {
    local input="$1"
    local len=${#input}
    local i=0
    local ch
    local state="plain"
    local side="left"
    local found=0

    PIPE_LEFT=""
    PIPE_RIGHT=""

    while (( i < len )); do
        ch="${input:i:1}"

        case "$state" in
            plain)
                case "$ch" in
                    "'")
                        state="single"
                        ;;

                    '"')
                        state="double"
                        ;;

                    "\\")
                        if [[ "$side" == "left" ]]; then
                            PIPE_LEFT+="$ch"
                        else
                            PIPE_RIGHT+="$ch"
                        fi

                        if (( i + 1 >= len )); then
                            return 1
                        fi

                        ((i++))
                        ch="${input:i:1}"

                        if [[ "$side" == "left" ]]; then
                            PIPE_LEFT+="$ch"
                        else
                            PIPE_RIGHT+="$ch"
                        fi

                        ((i++))
                        continue
                        ;;

                    '|')
                        if (( found != 0 )); then
                            return 2
                        fi

                        found=1
                        side="right"
                        ((i++))
                        continue
                        ;;

                    ';'|'&'|'<'|'>'|'`')
                        return 2
                        ;;
                esac
                ;;

            single)
                if [[ "$ch" == "'" ]]; then
                    state="plain"
                fi
                ;;

            double)
                case "$ch" in
                    '"')
                        state="plain"
                        ;;

                    "\\")
                        if [[ "$side" == "left" ]]; then
                            PIPE_LEFT+="$ch"
                        else
                            PIPE_RIGHT+="$ch"
                        fi

                        if (( i + 1 >= len )); then
                            return 1
                        fi

                        ((i++))
                        ch="${input:i:1}"

                        if [[ "$side" == "left" ]]; then
                            PIPE_LEFT+="$ch"
                        else
                            PIPE_RIGHT+="$ch"
                        fi

                        ((i++))
                        continue
                        ;;
                esac
                ;;
        esac

        if [[ "$side" == "left" ]]; then
            PIPE_LEFT+="$ch"
        else
            PIPE_RIGHT+="$ch"
        fi

        ((i++))
    done

    [[ "$state" == "plain" ]] || return 1

    if (( found == 0 )); then
        return 3
    fi

    [[ -n "${PIPE_LEFT//[[:space:]]/}" ]] || return 2
    [[ -n "${PIPE_RIGHT//[[:space:]]/}" ]] || return 2

    return 0
}

tokenize() {
    local input="$1"
    local len=${#input}
    local i=0
    local ch
    local state="plain"
    local token=""
    local have_token=0

    ARGV=()

    while (( i < len )); do
        ch="${input:i:1}"

        case "$state" in
            plain)
                case "$ch" in
                    "'")
                        state="single"
                        have_token=1
                        ;;

                    '"')
                        state="double"
                        have_token=1
                        ;;

                    "\\")
                        ((i++))

                        if (( i >= len )); then
                            return 1
                        fi

                        token+="${input:i:1}"
                        have_token=1
                        ;;

                    ' '|$'\t'|$'\n')
                        if (( have_token )); then
                            ARGV+=("$token")
                            token=""
                            have_token=0
                        fi
                        ;;

                    ';'|'&'|'|'|'<'|'>'|'`')
                        return 2
                        ;;

                    *)
                        token+="$ch"
                        have_token=1
                        ;;
                esac
                ;;

            single)
                if [[ "$ch" == "'" ]]; then
                    state="plain"
                else
                    token+="$ch"
                fi
                ;;

            double)
                case "$ch" in
                    '"')
                        state="plain"
                        ;;

                    "\\")
                        ((i++))

                        if (( i >= len )); then
                            return 1
                        fi

                        token+="${input:i:1}"
                        ;;

                    *)
                        token+="$ch"
                        ;;
                esac
                ;;
        esac

        ((i++))
    done

    [[ "$state" == "plain" ]] || return 1

    if (( have_token )); then
        ARGV+=("$token")
    fi

    return 0
}

ip_allowed() {
    local -a args=("$@")
    local arg
    local object=""

    for arg in "${args[@]}"; do
        case "$arg" in
            add|del|delete|change|replace|set|flush|append|prepend|\
            batch|-batch|--batch|netns|exec)
                return 1
                ;;
        esac
    done

    for arg in "${args[@]}"; do
        case "$arg" in
            addr|address|link|route|neigh|neighbor)
                object="$arg"
                break
                ;;
        esac
    done

    [[ -n "$object" ]] || return 1
    return 0
}

hostname_allowed() {
    case "$#" in
        0)
            return 0
            ;;

        1)
            case "$1" in
                -s|--short|\
                -f|--fqdn|\
                -d|--domain|\
                -i|--ip-address|\
                -I|--all-ip-addresses)
                    return 0
                    ;;
            esac
            ;;
    esac

    return 1
}

resolvectl_allowed() {
    case "${1-}" in
        "")
            return 0
            ;;

        status|query|statistics|show-server-state)
            return 0
            ;;
    esac

    return 1
}

nvidia_smi_allowed() {
    local arg

    for arg in "$@"; do
        case "$arg" in
            -pm|--persistence-mode*|\
            -pl|--power-limit*|\
            -lgc|--lock-gpu-clocks*|\
            -lmc|--lock-memory-clocks*|\
            -ac|--applications-clocks*|\
            -rac|--reset-applications-clocks*|\
            -rgc|--reset-gpu-clocks*|\
            -rmc|--reset-memory-clocks*|\
            -gom|--gpu-operation-mode*|\
            -mig|--mig-mode*|\
            -r|--gpu-reset*)
                return 1
                ;;
        esac
    done

    return 0
}

history_allowed() {
    case "$#" in
        0)
            return 0
            ;;

        1)
            [[ "$1" =~ ^[0-9]+$ ]]
            return $?
            ;;
    esac

    return 1
}

run_command() {
    local cmd="${ARGV[0]}"
    local -a args=("${ARGV[@]:1}")

    case "$cmd" in
        exit|logout)
            history -a 2>/dev/null || true
            exit 0
            ;;

        help)
            help_text
            return 0
            ;;

        history)
            if ! history_allowed "${args[@]}"; then
                printf 'history: only "history" or "history N" is permitted.\n' >&2
                return 126
            fi

            if (( ${#args[@]} == 0 )); then
                history
            else
                history "${args[0]}"
            fi

            return $?
            ;;

        cd)
            if (( ${#args[@]} > 1 )); then
                printf 'cd: too many arguments\n' >&2
                return 1
            fi

            if (( ${#args[@]} == 0 )); then
                cd "$HOME" || return 1
            else
                cd -- "${args[0]}" || return 1
            fi

            return 0
            ;;

        bash)
            cat >&2 <<'WARNING'

WARNING: Entering an unrestricted Bash shell on the host.
Commands in this Bash session are NOT filtered by incus-only-shell.
Type "exit" to return to incus-only-shell.

WARNING

            command /bin/bash "${args[@]}"
            return $?
            ;;

        incus)
            command /usr/bin/incus "${args[@]}"
            return $?
            ;;

        ip)
            if ! ip_allowed "${args[@]}"; then
                deny
                return 126
            fi

            command /usr/sbin/ip "${args[@]}"
            return $?
            ;;

        hostname)
            if ! hostname_allowed "${args[@]}"; then
                deny
                return 126
            fi

            command /usr/bin/hostname "${args[@]}"
            return $?
            ;;

        resolvectl)
            if ! resolvectl_allowed "${args[@]}"; then
                deny
                return 126
            fi

            command /usr/bin/resolvectl "${args[@]}"
            return $?
            ;;

        nvidia-smi)
            if ! nvidia_smi_allowed "${args[@]}"; then
                deny
                return 126
            fi

            local nvidia_path

            nvidia_path="$(command -v nvidia-smi 2>/dev/null)" || {
                printf 'nvidia-smi: command not installed\n' >&2
                return 127
            }

            command "$nvidia_path" "${args[@]}"
            return $?
            ;;

        top)
            local top_path

            top_path="$(command -v top 2>/dev/null)" || {
                printf 'top: command not installed\n' >&2
                return 127
            }

            command "$top_path" -s "${args[@]}"
            return $?
            ;;

        htop)
            local htop_path

            htop_path="$(command -v htop 2>/dev/null)" || {
                printf 'htop: command not installed\n' >&2
                return 127
            }

            command "$htop_path" --readonly "${args[@]}"
            return $?
            ;;

        btop)
            deny
            return 126
            ;;
    esac

    if is_host_command "$cmd"; then
        local path

        path="$(command -v -- "$cmd" 2>/dev/null)" || {
            printf '%s: command not installed\n' "$cmd" >&2
            return 127
        }

        command "$path" "${args[@]}"
        return $?
    fi

    deny
    return 126
}

run_simple_line() {
    local line="$1"
    local rc

    [[ -n "${line//[[:space:]]/}" ]] || return 0

    tokenize "$line"
    rc=$?

    case "$rc" in
        0)
            ;;

        1)
            printf 'Invalid quoting or escape sequence.\n' >&2
            return 2
            ;;

        2)
            deny
            return 126
            ;;

        *)
            deny
            return 126
            ;;
    esac

    (( ${#ARGV[@]} > 0 )) || return 0

    run_command
}

run_line() {
    local line="$1"
    local rc

    split_safe_grep_pipeline "$line"
    rc=$?

    case "$rc" in
        3)
            run_simple_line "$line"
            return $?
            ;;

        0)
            ;;

        1)
            printf 'Invalid quoting or escape sequence.\n' >&2
            return 2
            ;;

        *)
            deny
            return 126
            ;;
    esac

    tokenize "$PIPE_RIGHT"
    rc=$?

    if (( rc != 0 )); then
        printf 'Invalid grep pipeline.\n' >&2
        return 2
    fi

    (( ${#ARGV[@]} > 0 )) || {
        printf 'Invalid grep pipeline.\n' >&2
        return 2
    }

    [[ "${ARGV[0]}" == "grep" ]] || {
        deny
        return 126
    }

    local -a grep_args=("${ARGV[@]:1}")

    (
        run_simple_line "$PIPE_LEFT"
    ) | command /usr/bin/grep "${grep_args[@]}"

    return "${PIPESTATUS[1]}"
}

run_input() {
    local input="$1"
    local cmd
    local rc=0

    if ! split_top_level_commands "$input"; then
        printf 'Invalid quoting or escape sequence.\n' >&2
        return 2
    fi

    for cmd in "${COMMANDS[@]}"; do
        run_line "$cmd"
        rc=$?
    done

    return "$rc"
}

read_interactive_block() {
    local prompt="$1"
    local line
    local input
    local state

    if ! IFS= read -e -r -p "$prompt" line; then
        printf '\n'
        return 1
    fi

    input="${line//$'\r'/}"

    while true; do
        state="$(quote_state "$input")"

        case "$state" in
            plain)
                break
                ;;

            single|double|escape)
                if ! IFS= read -e -r -p '> ' line; then
                    printf '\n'
                    return 2
                fi

                line="${line//$'\r'/}"
                input+=$'\n'"$line"
                ;;

            *)
                printf 'Internal parser error.\n' >&2
                return 2
                ;;
        esac
    done

    READ_BLOCK="$input"
    return 0
}

#
# Non-interactive SSH command:
#
#   ssh host 'incus list'
#
if [[ "${1-}" == "-c" ]]; then
    [[ $# -eq 2 ]] || {
        deny
        exit 126
    }

    run_input "$2"
    exit $?
fi

#
# Interactive shell setup.
#
if [[ -t 0 ]]; then
    #
    # Keep bracketed paste enabled.
    #
    bind 'set enable-bracketed-paste on' 2>/dev/null || true

    #
    # Disable Readline numeric argument bindings.
    #
    # This avoids "(arg: N)" appearing with some IME/key sequences.
    #
    bind -r '\e0' 2>/dev/null || true
    bind -r '\e1' 2>/dev/null || true
    bind -r '\e2' 2>/dev/null || true
    bind -r '\e3' 2>/dev/null || true
    bind -r '\e4' 2>/dev/null || true
    bind -r '\e5' 2>/dev/null || true
    bind -r '\e6' 2>/dev/null || true
    bind -r '\e7' 2>/dev/null || true
    bind -r '\e8' 2>/dev/null || true
    bind -r '\e9' 2>/dev/null || true
    bind -r '\e-' 2>/dev/null || true

    set -o history

    shopt -s cmdhist
    shopt -s lithist

    touch "$HISTFILE" 2>/dev/null || true
    chmod 600 "$HISTFILE" 2>/dev/null || true

    history -r "$HISTFILE" 2>/dev/null || true

    banner
    echo
fi

while true; do
    prompt="[incus-shell] ${USER:-user}@$(hostname):${PWD}\$ "

    if [[ -t 0 ]]; then
        READ_BLOCK=""

        read_interactive_block "$prompt"
        rc=$?

        case "$rc" in
            0)
                ;;

            1)
                history -a 2>/dev/null || true
                exit 0
                ;;

            *)
                continue
                ;;
        esac

        input="$READ_BLOCK"

        if [[ -n "${input//[[:space:]]/}" ]]; then
            history -s "$input"
            history -a 2>/dev/null || true
        fi
    else
        if ! input="$(cat)"; then
            exit 1
        fi

        [[ -n "$input" ]] || exit 0
    fi

    run_input "$input"
done
