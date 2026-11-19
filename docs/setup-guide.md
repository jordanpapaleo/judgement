# Judgement Setup Guide

This guide is complete on its own. You can read it yourself, or hand the whole
thing to any AI agent (Claude Code, Codex, Cursor, Pi, or another tool that can
run shell commands) and ask it to set Judgement up for you. Get the same text in
a terminal with `judgement guide`.

## 1. What Judgement is

Judgement stores engineering guidance you have approved and hands it to coding
agents when they plan, code, and review. Agents propose what they learn from
working with you; nothing becomes guidance until you confirm it.

It has two parts that share one local SQLite file. There is no server and no
network access.

- **Judgement.app**: a menu-bar app where you review and curate.
- **`judgement` CLI**: what agents call. It works whether or not the app is open.

## 2. Words used in the app

| Word | Meaning |
|---|---|
| **Learning** | A candidate an agent (or you) proposed. It waits under **Learnings** until you confirm, edit-then-confirm, or deny it. |
| **Knowledge** | Your approved items. Agents only ever see these. |
| **Item** | One piece of guidance: title, one-line summary, guidance, when it applies, exceptions, tags, phases, scope. |
| **Omit** | Hide one item from agents without deleting it. Reversible; the date is kept. |
| **Pause** | Stop all agent participation: agents get no guidance and can't propose. You can still curate. |
| **Phase** | `planning`, `coding`, or `review`. An item with no phases applies to all three. |
| **Scope** | **Personal** (every repository) or one repository. Worktrees of a repository share its scope. |
| **Source** | Where a learning came from: `learn` (an agent or the Learn skill), `manual`, or `auto` (inception mode). |
| **Inception mode** | Optional automatic learning at the end of each Claude Code session. Off by default. |

## 3. Install the app

1. Install `Judgement.app` into `/Applications` (open the DMG and drag it, or
   `brew install --cask` with the Judgement cask). Keep it in `/Applications`;
   the CLI link points inside the app.
2. Open it. A dot appears in the menu bar. There is no Dock icon unless the
   window is open.
3. Judgement starts **paused**. Click the menu-bar icon and choose **Resume
   Judgement** when you are ready for agents to use it.

The menu-bar icon shows the state: a filled dot is **Active**, two bars are
**Paused**, and an orange triangle means the store can't be opened (choose
**Show Details…**).

Optional: turn on **Launch at Login** from the menu-bar menu or **Settings →
General**.

## 4. Install the command-line tool

Agents reach Judgement through `judgement`, which ships inside the app.

1. Open Judgement, then **Settings** (gear at the bottom of the sidebar) →
   **General** → **Link judgement into ~/.local/bin**. This creates one symlink:
   `~/.local/bin/judgement` → `/Applications/Judgement.app/Contents/MacOS/judgement-cli`.
   It never edits your shell profile and never replaces an existing file.
2. Make sure `~/.local/bin` is on your `PATH`. Check with:

   ```sh
   command -v judgement
   ```

   If nothing prints, add this line to your shell profile (`~/.zshrc` for zsh)
   yourself, then open a new terminal:

   ```sh
   export PATH="$HOME/.local/bin:$PATH"
   ```

3. Verify:

   ```sh
   judgement status --json
   ```

   Expect `"kind": "ok"` with `"participation": "active"`. If you see
   `"participation": "paused"`, resume Judgement from the menu bar.

Without the link you can always call the CLI by its full path:
`/Applications/Judgement.app/Contents/MacOS/judgement-cli`.

## 5. Give your agents the skill

The skill is a Markdown file that tells an agent when to recall guidance and how
to propose learnings. Print it with:

```sh
judgement skill
```

Install it where your agent tools look for skills or instructions. Each harness
is different:

| Harness | Where it goes |
|---|---|
| Claude Code | `~/.claude/skills/judgement/SKILL.md` |
| Shared `~/.agents` setup (Pi, Cursor, and Claude Code through a symlink) | `~/.agents/skills/judgement/SKILL.md`, then link it for Claude Code: `ln -s ~/.agents/skills/judgement ~/.claude/skills/judgement` (or run your own `link.sh`) |
| A tool without skills folders (e.g. Codex) | Append the skill's body to the tool's global instructions file, such as `~/.codex/AGENTS.md` |
| Containers (e.g. OpenHands) | Mount the skill folder. Without the binary inside the container, the skill tells the agent to continue without Judgement. |

For example, for Claude Code:

```sh
mkdir -p ~/.claude/skills/judgement
judgement skill > ~/.claude/skills/judgement/SKILL.md
```

Start a **new** agent session afterwards; tools load skills when a session
starts. Reinstall the skill after updating Judgement so it matches the CLI.

## 6. Check that it works

In a new agent session inside any Git repository, ask the agent:

> Use the judgement skill. List the coding guidance for this repo, then propose a
> test learning: "Prefer small pull requests", evidence "Setup check".

Then open Judgement → **Learnings**. The test learning should be there with the
harness and model that wrote it. Deny or delete it.

You can also check from a terminal:

```sh
judgement list --phase coding --json
```

## 7. Everyday use

- **Learnings**: review what agents proposed, oldest first. Each one shows its
  evidence, who proposed it (harness and model), and, for a correction, the
  current and proposed text side by side. Use the actions at the top right:
  **Confirm**, **Edit Then Confirm**, **Deny**, **Delete**. Denied learnings are
  kept so inception mode stops proposing similar ones.
- **Knowledge**: everything approved. Sort by least recently returned to find
  guidance agents never use. Flip **Omit** to hide an item from agents, select
  two or more to **Combine** them into one, or delete them. The sidebar's
  **Scopes** and **Tags** filter this list.
- **Pause**: the Active/Paused switch in the window's top right, or the
  menu-bar menu.
- **Settings**: **Agents** (participation, inception mode, analyzer, gauge),
  **Typography**, **Themes**, **General** (launch at login, backups, CLI link).

## 8. Optional: inception mode (learn automatically)

Inception mode reads each finished Claude Code session and proposes what you
taught the agent. Everything still waits under Learnings for you.

1. **Add the hook** to `~/.claude/settings.json`. Merge it into the existing
   `"hooks"` object; don't replace the file. Use the full path to the CLI,
   because hooks may not see your shell `PATH`:

   ```json
   {
     "hooks": {
       "SessionEnd": [
         {
           "hooks": [
             {
               "type": "command",
               "command": "/Applications/Judgement.app/Contents/MacOS/judgement-cli hook session-end"
             }
           ]
         }
       ]
     }
   }
   ```

2. **Choose a temperature** in **Settings → Agents**:

   | Temperature | Captures |
   |---|---|
   | Off (default) | Nothing; the hook exits immediately |
   | Cool | Only explicit rules ("never do X", "we always Y") |
   | Warm | Explicit rules, corrections and pushback, clear reinforcement ("yes, exactly") |
   | Hot | Any reaction, including implied preferences. Expect noise. |

3. **Set the analyzer**: the headless command that reads the session and
   reports learnings. It runs without a shell, receives its prompt on standard
   input, and must print a JSON array. Examples:
   - Claude Code: `claude -p --allowedTools Read`
   - Codex: `codex exec -`

   Set **Analyzer model label** to the model that command uses (for example
   `claude-opus-5-5`); it is recorded on every learning.

4. **Check it**: end a Claude Code session in which you corrected the agent.
   Within a minute or two, look under Learnings. Each run is logged in
   `~/Library/Application Support/com.jordanpapaleo.judgement/inception.log`.
   "Nothing learned" is the normal result for most sessions.

Tune with the **Temperature Gauge**: mostly denied → turn it cooler; quiet and
mostly confirmed → turn it warmer.

## 9. Reference for AI agents

**Recall** at the start of planning, before coding, and before review:

```sh
judgement list --phase <planning|coding|review> --json   # index: id, title, summary, tags, phases, scope
judgement get <id> [<id>...] --json                       # full items, up to 10
```

Pick items by meaning, not keywords. Items are advisory: the task and the
repository's own instructions win, and conflicts should be raised with the user.

**Propose** a learning (JSON on standard input):

```sh
judgement propose --stdin --source learn --harness <harness> --model <model-id> --json <<'JSON'
{
  "kind": "new",
  "payload": {
    "title": "Short title",
    "summary": "One line agents see in the list.",
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

- `kind`: `new`; `reinforce` (needs `target_id`, no payload); or `correct`
  (needs `target_id` and only the changed payload fields).
- `repo_root: "."` scopes a new item to the current repository; omit it for
  personal guidance.
- Limits: title 120 characters; summary 200, one line; guidance 4000;
  applies_when and exceptions 1000 each; up to 20 one-word tags; evidence 1 KB.
- `--harness` / `--model` fall back to `JUDGEMENT_HARNESS` / `JUDGEMENT_MODEL`,
  then `unknown`.
- Describe, don't paste: never include code, logs, config, customer data, or
  secrets. Secret-shaped text is rejected.

**Every response** is `{"apiVersion": 1, "result": {"kind": ...}}`:

| `result.kind` | Exit code | What to do |
|---|---|---|
| `ok` | 0 | Use the result |
| `paused` | 0 for list/get, 4 for propose | Continue without Judgement; mention it once |
| `invalid` | 2 | Fix the input (the message names the field) |
| `unsafe` | 2 | Rewrite the named field in prose |
| `unavailable` | 3 | The app has never run, or needs to upgrade the store; continue without Judgement and tell the user |
| `error` | 1 | Continue without Judgement and tell the user |

If `judgement` is not found, continue without it and mention it once.

## 10. Troubleshooting

| Symptom | Fix |
|---|---|
| `judgement: command not found` | Link the CLI (section 4) and check `PATH`, or use the full path. |
| `unavailable` … "no Judgement store" | Open Judgement.app once; it creates the store. |
| `unavailable` … "open Judgement to upgrade the store" | Open the updated app; it backs up and upgrades the store. |
| Every call says `paused` | Resume from the menu bar or the window's top-right switch. |
| Proposal rejected as `unsafe` | The named field looked like a secret. Describe it in prose. |
| Agents never call Judgement | Start a new session after installing the skill. Ask the agent "use the judgement skill" to confirm it loaded. |
| Inception mode never proposes anything | Check the hook path, temperature (not Off), participation (Active), and `inception.log`. Nothing learned is normal for many sessions. |
| Menu-bar icon is an orange triangle | The store couldn't be opened. It was not modified. Choose **Show Details…**, or restore a backup (section 11). |
| Can't find the window | Click the menu-bar icon → **Open Judgement…**. Closing the window doesn't quit the app. |

## 11. Data, backups, and removal

- **Store**: `~/Library/Application Support/com.jordanpapaleo.judgement/judgement.sqlite`
  (folder readable only by you).
- **Backups**: `backups/` in that folder. Judgement backs up automatically
  before upgrading the store and keeps five; **Settings → General → Back Up
  Now** keeps five more. To restore: quit Judgement, replace `judgement.sqlite`
  with a backup copy, and reopen.
- **Privacy**: the app makes no network calls. Only the analyzer command you
  configure for inception mode talks to a model provider.
- **Remove Judgement**: quit it, delete `Judgement.app`, delete
  `~/.local/bin/judgement`, the skill folder you installed, the `SessionEnd`
  hook, and (to erase your data) the store folder above.

## 12. Setup checklist for an AI agent

If you are an AI agent asked to set Judgement up, work through these steps in
order and report the result of each. **Ask the user before changing any file
outside a temporary directory** (`~/.claude`, `~/.agents`, `~/.codex`, shell
profiles), and show them what you will write.

1. Confirm `/Applications/Judgement.app` exists. If not, ask the user to install it.
2. Ask the user to open Judgement and resume it from the menu bar.
3. Find the CLI: `command -v judgement`, falling back to
   `/Applications/Judgement.app/Contents/MacOS/judgement-cli`. If it isn't
   linked, ask the user to use **Settings → General → Link judgement into
   ~/.local/bin**.
4. Run `judgement status --json`. Continue only when `result.kind` is `ok` and
   `participation` is `active`.
5. Ask which tools the user runs (Claude Code, Codex, Cursor, Pi, others), then
   install the skill for each (section 5) with `judgement skill`, after showing
   the user the destination.
6. Ask whether they want inception mode. If yes, show the hook JSON (section 8),
   merge it into `~/.claude/settings.json` only with their approval, and remind
   them to pick a temperature in **Settings → Agents**.
7. Run the check in section 6, or ask the user to start a new session and run it.
8. Summarize what was installed and where, and how to undo each step
   (section 11).
