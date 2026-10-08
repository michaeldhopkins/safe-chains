# Blind weak-model evaluation

How `safe-chains mcp` reads to an agent that has never heard of safe-chains. Each run starts a fresh
Claude Code session with nothing but a user's opening message and the server's five tools, and a
grader scores what it did against a rubric the agent never sees.

## Running a round

Build first (`cargo build`): the server is `target/debug/safe-chains`.

```sh
eval/blind/matrix.sh a-unknown-tool 2 claude-haiku-4-5                    # the mid reference
BLIND_OPENROUTER_KEY=… eval/blind/matrix.sh a-unknown-tool 2 \
  openrouter:qwen/qwen3-30b-a3b-instruct-2507 openrouter:mistralai/mistral-small-3.2-24b-instruct
eval/blind/run.sh start b-notes-folder        # one run, to watch; BLIND_MODEL names the model
```

Every model is an exact id. The OpenRouter key arrives in the environment. Claude Code cannot tag
OpenRouter requests itself, so `BLIND_OPENROUTER_BASE` can point the runs at a local proxy that adds
the `user` and `trace` tags. The matrix stops once the spend since `BLIND_SPEND_BASELINE` (the key's
spend when the round began) reaches `BLIND_BUDGET_USD` (default 3). A paid run stops at 40 turns.

Runs live under `~/Library/Caches/safe-chains-blind.noindex/<run>/` (`BLIND_RUNS` moves them): the
agent's empty working directory, the server's own home (seeded by the scenario's `homes/*.sh`),
the config before the run, the transcript and `grade.json`. They sit outside any temporary
directory, since safe-chains approves writes in one, which would hide what the scenarios test.

## What makes it blind

- The agent runs from an empty directory outside the repository, with the user setting source
  left out, slash commands off and no built-in tools, so no instructions, skills, hooks or memories
  reach it, and it can reach nothing but the server.
- The server gets a home of its own, so nothing reads or changes the real
  `~/.config/safe-chains.toml`.
- Rubrics live in `rubrics/`, outside the agent's reach. The grader (`grade.sh`, an exact Sonnet
  id, no tools) sees the rubric, everything the agent said, every tool call with the start of its
  result, and the config before and after, and is not told the model.

## Scenarios

| File | The user says | What it exercises |
| --- | --- | --- |
| `a-unknown-tool.md` | let one deploytool command through, nothing else | check_command, add_command_entry, scope |
| `b-notes-folder.md` | let the agent write in ~/notes | check_command, grant_path, scope |
| `c-no-fix.md` | stop asking about a force push and an SSH key read | refusing when nothing in the config helps |
| `d-audit.md` | what is allowed; take out ~/Downloads | list_rules, remove_rule, restraint |

## Recording a round

`results.toml` gets a `[[round]]` with the date, the exact models, the scores per scenario and model
(percent of the rubric) and the spend. Then rewrite `surface.lock` with
`SAFE_CHAINS_WRITE_SURFACE_LOCK=1 cargo test --test blind_eval_drift`. That test fails when no round
is recorded or when the surface has moved far enough since the last one that another is due.
Its `[[finding]]` entries list what the rounds found and what was done about it.
