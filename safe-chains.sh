#!/usr/bin/env python3
import sys, json, re, shlex

input_data = json.load(sys.stdin)
command = input_data.get("tool_input", {}).get("command", "")

SAFE_CMDS = {
    "find", "grep", "rg", "fd", "head", "tail", "cat", "ls", "wc", "sort",
    "uniq", "tr", "cut", "awk", "sed", "echo", "xargs", "tee", "dirname",
    "basename", "realpath", "file", "stat", "du", "df", "env", "printenv",
    "which", "whoami", "date", "pwd", "tree", "lsof", "jq", "base64",
    "xxd", "pgrep", "getconf",
}

XARGS_FLAGS_WITH_ARG = {"-I", "-L", "-n", "-P", "-s", "-E", "-d"}
XARGS_FLAGS_NO_ARG = {"-0", "-r", "-t", "-p", "-x"}

GH_READ_ONLY_SUBCOMMANDS = {"pr", "issue", "repo", "release", "run", "workflow"}
GH_READ_ONLY_ACTIONS = {"view", "list", "status", "diff", "checks"}
GH_API_BODY_FLAGS = {"-f", "-F", "--field", "--raw-field", "--input"}

GIT_READ_ONLY_SUBCOMMANDS = {
    "log", "diff", "show", "status", "ls-tree", "grep", "rev-parse",
    "merge-base", "merge-tree", "fetch", "help", "--version", "shortlog",
    "describe", "blame", "reflog",
}
GIT_REMOTE_MUTATING_ACTIONS = {"add", "remove", "rename", "set-url", "set-branches", "prune"}

JJ_READ_ONLY_SUBCOMMANDS = {"log", "diff", "show", "status", "st", "help", "--version"}
JJ_READ_ONLY_MULTI = {
    "op": {"log"},
    "file": {"show"},
    "config": {"get"},
}

YARN_READ_ONLY = {"list", "info", "why", "--version"}
NPM_READ_ONLY = {"view", "info"}
BUNDLE_READ_ONLY = {"list", "info", "show", "check"}
BUNDLE_EXEC_SAFE = {"rspec", "standardrb", "cucumber", "brakeman", "erb_lint", "herb"}

MISE_READ_ONLY = {"ls", "list", "current", "which", "doctor", "--version"}
MISE_READ_ONLY_MULTI = {"settings": {"get"}}
ASDF_READ_ONLY = {"current", "which", "help", "list", "--version"}
GEM_READ_ONLY = {"list", "info", "environment", "which", "pristine"}
BREW_READ_ONLY = {"list", "info", "--version"}
CARGO_SAFE = {"clippy", "test", "build", "check", "doc", "search", "--version", "bench"}
NPX_SAFE = {"eslint", "@herb-tools/linter", "karma"}

TIMEOUT_FLAGS_WITH_ARG = {"-s", "--signal", "-k", "--kill-after"}


def split_outside_quotes(cmd):
    segments = []
    current = []
    in_single = False
    in_double = False
    escaped = False
    i = 0
    while i < len(cmd):
        c = cmd[i]
        if escaped:
            current.append(c)
            escaped = False
            i += 1
            continue
        if c == "\\" and not in_single:
            escaped = True
            current.append(c)
            i += 1
            continue
        if c == "'" and not in_double:
            in_single = not in_single
            current.append(c)
            i += 1
            continue
        if c == '"' and not in_single:
            in_double = not in_double
            current.append(c)
            i += 1
            continue
        if not in_single and not in_double:
            if c == "|":
                segments.append("".join(current))
                current = []
                i += 1
                continue
            if c == "&" and i + 1 < len(cmd) and cmd[i + 1] == "&":
                segments.append("".join(current))
                current = []
                i += 2
                continue
            if c == ";":
                segments.append("".join(current))
                current = []
                i += 1
                continue
        current.append(c)
        i += 1
    segments.append("".join(current))
    return [s.strip() for s in segments if s.strip()]


def tokenize(segment):
    try:
        return shlex.split(segment)
    except ValueError:
        return None


def is_safe(segment):
    segment = re.sub(r"^([A-Z_][A-Z_0-9]*=[^ ]* )*", "", segment).strip()
    if not segment:
        return True

    tokens = tokenize(segment)
    if tokens is None:
        return False
    if not tokens:
        return True

    cmd = tokens[0].split("/")[-1]

    if cmd in ("sh", "bash"):
        if "-c" in tokens:
            idx = tokens.index("-c")
            if idx + 1 < len(tokens):
                return all(is_safe(s) for s in split_outside_quotes(tokens[idx + 1]))
        return False

    if cmd == "xargs":
        i = 1
        while i < len(tokens):
            if tokens[i] in XARGS_FLAGS_WITH_ARG:
                i += 2
                continue
            if tokens[i] in XARGS_FLAGS_NO_ARG:
                i += 1
                continue
            if tokens[i].startswith("-"):
                i += 1
                continue
            return is_safe(" ".join(tokens[i:]))
        return True

    if cmd == "gh":
        if len(tokens) < 2:
            return False
        subcmd = tokens[1]

        if subcmd in GH_READ_ONLY_SUBCOMMANDS:
            if len(tokens) < 3:
                return False
            return tokens[2] in GH_READ_ONLY_ACTIONS

        if subcmd == "api":
            for i, token in enumerate(tokens[2:], start=2):
                if token in ("-X", "--method"):
                    if i + 1 >= len(tokens):
                        return False
                    return tokens[i + 1].upper() == "GET"
                if token.startswith("-X=") or token.startswith("--method="):
                    return token.split("=", 1)[1].upper() == "GET"
                for flag in GH_API_BODY_FLAGS:
                    if token == flag:
                        return False
                    if len(flag) == 2 and len(token) > 2 and token.startswith(flag):
                        return False
                    if flag.startswith("--") and token.startswith(flag + "="):
                        return False
            return True

        return False

    if cmd == "git":
        args = tokens[1:]
        while args and args[0] == "-C" and len(args) >= 2:
            args = args[2:]
        if not args:
            return False
        subcmd = args[0]
        if subcmd in GIT_READ_ONLY_SUBCOMMANDS:
            return True
        if subcmd == "remote":
            if len(args) < 2:
                return True
            return args[1] not in GIT_REMOTE_MUTATING_ACTIONS
        return False

    if cmd == "jj":
        if len(tokens) < 2:
            return False
        subcmd = tokens[1]
        if subcmd in JJ_READ_ONLY_SUBCOMMANDS:
            return True
        if subcmd in JJ_READ_ONLY_MULTI:
            if len(tokens) < 3:
                return False
            return tokens[2] in JJ_READ_ONLY_MULTI[subcmd]
        return False

    if cmd == "yarn":
        if len(tokens) < 2:
            return False
        if tokens[1] in YARN_READ_ONLY:
            return True
        if tokens[1] == "test" or tokens[1].startswith("test:"):
            return True
        return False

    if cmd == "npm":
        if len(tokens) < 2:
            return False
        return tokens[1] in NPM_READ_ONLY

    if cmd == "bundle":
        if len(tokens) < 2:
            return False
        if tokens[1] in BUNDLE_READ_ONLY:
            return True
        if tokens[1] == "exec" and len(tokens) >= 3:
            return tokens[2] in BUNDLE_EXEC_SAFE
        return False

    if cmd == "mise":
        if len(tokens) < 2:
            return False
        if tokens[1] in MISE_READ_ONLY:
            return True
        if tokens[1] in MISE_READ_ONLY_MULTI:
            if len(tokens) < 3:
                return False
            return tokens[2] in MISE_READ_ONLY_MULTI[tokens[1]]
        return False

    if cmd == "asdf":
        if len(tokens) < 2:
            return False
        return tokens[1] in ASDF_READ_ONLY

    if cmd == "gem":
        if len(tokens) < 2:
            return False
        return tokens[1] in GEM_READ_ONLY

    if cmd == "brew":
        if len(tokens) < 2:
            return False
        return tokens[1] in BREW_READ_ONLY

    if cmd == "cargo":
        if len(tokens) < 2:
            return False
        return tokens[1] in CARGO_SAFE

    if cmd == "npx":
        if len(tokens) < 2:
            return False
        i = 1
        while i < len(tokens):
            if tokens[i] in ("--package", "-p"):
                i += 2
                continue
            if tokens[i] in ("--yes", "-y", "--no", "--ignore-existing", "-q", "--quiet"):
                i += 1
                continue
            if tokens[i] == "--":
                i += 1
                break
            if tokens[i].startswith("-"):
                return False
            break
        if i >= len(tokens):
            return False
        return tokens[i] in NPX_SAFE

    if cmd in ("timeout", "time"):
        i = 1
        if cmd == "timeout":
            while i < len(tokens) and tokens[i].startswith("-"):
                if tokens[i] in TIMEOUT_FLAGS_WITH_ARG:
                    i += 2
                else:
                    i += 1
            i += 1
        else:
            if i < len(tokens) and tokens[i] == "-p":
                i += 1
        if i >= len(tokens):
            return False
        return is_safe(" ".join(tokens[i:]))

    return cmd in SAFE_CMDS


segments = split_outside_quotes(command)

for segment in segments:
    if not is_safe(segment):
        sys.exit(0)

json.dump({
    "hookSpecificOutput": {
        "hookEventName": "PreToolUse",
        "permissionDecision": "allow",
        "permissionDecisionReason": "All commands in chain are safe read-only utilities",
    }
}, sys.stdout)
