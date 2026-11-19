# Judgement

**Store simple, recall simple.** A macOS menu-bar app plus a small CLI that
stores human-approved engineering guidance and recalls it for coding and review
agents at the point of decision.

- **Store**: learnings arrive under **Learnings** (from Learn, from agents, or from
  inception mode). Nothing becomes guidance until you confirm it.
- **Recall**: any agent that can run a shell command calls `judgement list`,
  picks what's relevant by meaning, and calls `judgement get`.
- **Learn**: manually through the Learn skill (default), or automatically
  through inception mode with an Off / Cool / Warm / Hot temperature.

Model-agnostic: nothing in the store, CLI, or skill depends on a provider. Every
learning records the harness and model that produced it, for your audit.

## Install

The full, AI-readable setup guide is [`docs/setup-guide.md`](docs/setup-guide.md). It's
also under **Help → Judgement Setup Guide** in the app and printed by `judgement guide`.

1. Install `Judgement.app` into `/Applications` (DMG or `brew install --cask`
   from `Casks/judgement.rb`) and open it. It lives in the menu bar; first
   launch is **paused** until you choose **Resume Judgement**.
2. In **Settings → Command-line tool**, choose **Link judgement into
   ~/.local/bin**. This creates one symlink into the app bundle and never edits
   your shell profile. Make sure `~/.local/bin` is on your `PATH`.
3. Install the agent skill by hand, like your other skills:

   ```sh
   mkdir -p ~/.agents/skills/judgement
   cp skill/judgement/SKILL.md ~/.agents/skills/judgement/SKILL.md
   # review the diff in ~/.agents, run link.sh, start a new session
   ```

## CLI

```sh
judgement status --json
judgement list --phase <planning|coding|review> [--repo <path>] --json
judgement get <id> [<id>...] --json
judgement propose --stdin [--source learn|manual|auto] --harness <name> --model <id> \
  [--session-model <id>] [--session <id>] --json
```

Output is always `{"apiVersion": 1, "result": {"kind": ...}}` with kinds `ok`,
`paused`, `invalid`, `unsafe`, `unavailable`, `error`.

| Exit | Meaning |
|---|---|
| 0 | ok, or a paused read (`list`/`get`) |
| 2 | invalid input or unsafe content |
| 3 | store unavailable or schema mismatch ("open Judgement to upgrade the store") |
| 4 | paused `propose` |
| 1 | anything else |

`--harness` / `--model` fall back to `JUDGEMENT_HARNESS` / `JUDGEMENT_MODEL`,
then `unknown`. Without `--repo`, the repo is the current directory's canonical
Git common dir, so linked worktrees share one identity; outside Git only
personal items are listed. In a proposal, `"repo_root": "."` scopes a new item to
the current repo.

The CLI works with the app closed, never migrates the store, and has no
confirm/edit/delete commands.

## Inception mode (auto-learn)

Off by default. When on, a Claude Code `SessionEnd` hook starts the configured
analyzer **detached** (it never blocks session exit). The analyzer is any
model's headless command, set in Settings and run as an argument array (never
through a shell), for example `claude -p --allowedTools Read` or
`codex exec -`. It receives a provider-neutral prompt with the transcript path,
the temperature, the items the session pulled, and your 20 most recent denials.
It prints a JSON array of learnings, and each one goes through the same validation,
unsafe check, pause, and temperature gate as any other write. "Nothing
learned" is the expected outcome.

Add the hook to `~/.claude/settings.json` yourself:

```json
"hooks": {
  "SessionEnd": [
    { "hooks": [{ "type": "command", "command": "/Applications/Judgement.app/Contents/MacOS/judgement-cli hook session-end" }] }
  ]
}
```

The hook exits immediately while paused, while Off, or inside the analyzer's
own session (`JUDGEMENT_ANALYZER=1`). Its log is `inception.log` next to the
store.

## Development

See [CONTRIBUTING.md](CONTRIBUTING.md). Releases follow Helm's flow:
`./release.sh patch|minor|major` signs, notarizes, verifies, and tags.

## Spike status (TDD §15)

| Gate | Status |
|---|---|
| G1 tray + on-demand window, no Dock icon when closed, clean quit | Built (`LSUIElement` + accessory policy, regular only while the window is open). **Needs a manual smoke test**: tray menus, window show/hide, Dock behavior. |
| G2 CLI links core without Tauri, bundled, runs with no Node/Rust, shares the store under concurrent writes | Verified: `cargo tree` shows no Tauri in the CLI; `Judgement.app/Contents/MacOS/judgement-cli` runs with `PATH=/usr/bin:/bin`; concurrent CLI proposes landed while the packaged app was running. Signed/notarized run still pending a real `release.sh`. |
| G3 SessionEnd input, detached analyzer survives exit, OpenAI harness runs the prompt | Hook + detached runner built and tested with a stand-in analyzer. **Needs real-session observation** (A11). |
| A10 real Claude + OpenAI sessions | Manual. |

### Deviations from TDD 0.2, for review

1. **Analyzer output path.** The analyzer prints JSON and the detached
   `judgement hook analyze` process proposes each learning through core,
   instead of the analyzer calling `propose` itself. The analyzer then needs
   no shell permission, and any model's headless mode works unchanged.
2. **Hidden `judgement hook` subcommand** (`session-end`, `analyze`) holds the
   Claude Code trigger. The four public commands are unchanged.
3. **Evidence cap on reinforce/correct.** Appending past 4 KB rolls off the
   oldest entries instead of rejecting the confirm.
4. **`JUDGEMENT_HOME`** overrides the store directory for the app and CLI
   (tests, development).
5. **Default analyzer** is `claude -p --allowedTools Read` with model label
   `unknown`; set the real label in Settings.
