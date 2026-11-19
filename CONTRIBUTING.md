# Contributing to Judgement

## Prerequisites

- macOS 11+ (Apple Silicon builds are what release.sh ships)
- Rust 1.94+ (`rustup`), Node 22.13.1+
- Xcode Command Line Tools

```sh
npm install
npm start            # tauri dev
```

`npm start` builds the debug CLI and stages it at
`src-tauri/binaries/judgement-cli-<triple>` (Tauri's `externalBin`) before the
app compiles. Plain `cargo` commands that build the app crate need it staged
too: run `npm run cli` once.

## Store location

`~/Library/Application Support/com.jordanpapaleo.judgement/judgement.sqlite`
(directory 0700, files 0600). Set `JUDGEMENT_HOME=/some/dir` to point both the
app and the CLI at a different store, e.g. while developing.

Backups live in `backups/` next to the store: `auto-*` before each migration,
`manual-*` from **Back Up Now**, five of each kept. To restore: quit Judgement,
replace `judgement.sqlite` with a backup copy, relaunch.

## Tests

| Command | What |
|---|---|
| `npm run test:rust` | Core ops on temp DBs, the real CLI binary against temp stores, icons |
| `npm test` | Vitest: Learnings / Knowledge / Settings / App behavior with a fake backend |
| `npm run test:types` | TypeScript |
| `npm run lint` / `npm run lint:rust` | Biome; rustfmt + Clippy |

No test needs paid inference. The inception-mode test uses a stand-in analyzer.

## Releasing

**Always use `release.sh` to cut a release.** Do not bump versions or build manually.

```sh
./release.sh patch   # 0.1.0 → 0.1.1
./release.sh minor   # 0.1.0 → 0.2.0
./release.sh major   # 0.1.0 → 1.0.0
```

The script:
1. Sources `sign.sh` and validates the Apple signing credentials: all four of
   `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD`, and `APPLE_TEAM_ID`
   must be exported, and the identity must exist in the keychain.
2. Bumps the version in `package.json`, `src-tauri/tauri.conf.json`, and the
   `[workspace.package]` version in `src-tauri/Cargo.toml` (shared by the app and CLI).
3. Generates a `CHANGELOG.md` entry from commits since the last tag.
4. Commits the version bump.
5. Runs `npm run tauri build` (builds the release CLI, bundles it into
   `Judgement.app/Contents/MacOS/judgement-cli`, signs, and notarizes).
6. **Verifies**: `codesign --verify` on the app and the bundled CLI, `xcrun
   stapler validate`, a Gatekeeper assessment that must report *Notarized
   Developer ID*, and that the bundled CLI runs with a bare `PATH` (no Node/Rust).
7. Copies the DMG to `releases/vX.Y.Z/`, commits it, and tags `vX.Y.Z`.
8. Syncs `Casks/judgement.rb` to the new version and the DMG's sha256.

`sign.sh` lives at the repo root, is git-ignored, and exports the four Apple
variables. Start from `sign.sh.example` (the Helm `sign.sh` uses the same
Developer ID).

When prompted, answer **y** to push and create a draft GitHub Release with the
`.dmg`. The release stays a draft; publish it yourself:

```sh
gh release edit vX.Y.Z --draft=false
```

Without a terminal attached (scripts, agents), the push prompt is skipped and
the release is only built, committed, and tagged locally.
