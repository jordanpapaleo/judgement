---
name: judgement
description: Recall the user's approved engineering guidance at planning, coding, and review boundaries, and propose new evidence-backed learnings for the user to confirm. Use at the start of planning, before writing code, and before reviewing.
---

# Judgement

Judgement stores engineering guidance the user has approved. It tells you
*what to watch for*; your skills and the repository's instructions still tell
you *how to work*. Items are advisory and always subordinate to the task and to
repository instructions.

Every command prints JSON: `{"apiVersion": 1, "result": {"kind": ...}}`. Read
`result.kind`.

## Recall at each boundary

At the start of **planning**, before **coding**, and before **review**:

1. List the index for that phase:

   ```sh
   judgement list --phase planning --json   # or coding, review
   ```

2. Read the titles and summaries. Pick the items that are relevant **by
   meaning** to the work in front of you — not by keyword overlap. Often none
   are.

3. Fetch the ones you picked (up to 10 IDs):

   ```sh
   judgement get <id> <id> --json
   ```

4. Apply them. Respect each item's `applies_when` and `exceptions`. If an item
   conflicts with the task or repository instructions, the task and repository
   win — say so to the user rather than silently choosing.

## When Judgement isn't there

If `result.kind` is `paused` or `unavailable`, or the `judgement` command is not
found (for example inside a container where only the skill is mounted):
continue normally without it and mention it to the user **once**. Do not retry
in a loop and do not look for the store yourself.

## Propose a learning

Propose only when the user taught something durable during this work: an
explicit rule, a correction, or clear reinforcement of an item you used. Never
propose task-specific instructions.

Pipe one JSON object to `propose`, and **always** pass your harness and model:

```sh
judgement propose --stdin --source learn --harness <harness> --model <model-id> --json <<'JSON'
{
  "kind": "new",
  "payload": {
    "title": "Short title",
    "summary": "One line agents will see in the list.",
    "guidance": "The advice itself.",
    "applies_when": "When to use it.",
    "exceptions": "When not to.",
    "tags": ["testing"],
    "phases": ["coding", "review"],
    "repo_root": "."
  },
  "evidence": "One or two sentences describing what the user said or did."
}
JSON
```

- `kind` is `new`, `reinforce` (the user held up an item you used; give
  `target_id`, no payload), or `correct` (the user overrode an item; give
  `target_id` and only the changed payload fields).
- `phases`: any of `planning`, `coding`, `review`; empty means all.
- `repo_root`: `"."` scopes the item to the current repository; omit it for
  guidance that applies everywhere.
- `--harness`: the tool you are running in, e.g. `claude-code`, `codex`, `pi`,
  `cursor`, `openhands`. `--model`: your model ID. If the environment sets
  `JUDGEMENT_HARNESS` / `JUDGEMENT_MODEL`, those are used when the flags are
  omitted.

**Describe, don't paste.** Evidence is prose about what happened. Never paste
code, logs, config, file contents, customer or tenant data, credentials, or
secrets. Writes that look like secrets are rejected (`result.kind` is `unsafe`);
rewrite the evidence in prose.

A proposal is only a candidate. Nothing becomes guidance until the user confirms
it under Learnings in Judgement, so tell the user what you proposed.

Pausing Judgement does not disable rules already written into skills.
