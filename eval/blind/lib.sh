# shellcheck shell=bash
# Shared by run.sh and matrix.sh. Sourced; defines functions only.

# A round names its models by exact id. Claude Code's aliases move when Claude Code does, so a
# round run with `haiku` would not say which model it measured.
refuse_alias() { # model; empty is Claude Code's default model, which moves too
  case "$1" in
    '' | haiku | sonnet | opus | default | best | opusplan | haiku\[1m\] | sonnet\[1m\] | opus\[1m\])
      echo "'$1' is a Claude Code alias, which moves with Claude Code; name an exact model id" >&2
      return 2 ;;
  esac
}
