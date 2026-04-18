# safe-chains

A command safety checker that auto-allows safe bash commands without prompting.

When an agentic tool wants to run a bash command, safe-chains checks if every segment of the command is safe. If so, the command runs without asking for permission. If any segment is unsafe, the normal permission flow takes over.

safe-chains works as a Claude Code pre-hook, a CLI tool, or an OpenCode plugin.
