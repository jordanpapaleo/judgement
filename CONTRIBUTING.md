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

## Creating a DMG (macOS)

The DMG is built automatically as part of `npm run tauri build`. You'll find it at:

```
src-tauri/target/release/bundle/dmg/Judgement_<version>_aarch64.dmg
```

To distribute the DMG, you'll need to **sign and notarize** it. Set these environment variables before building:

```sh
export APPLE_CERTIFICATE="Developer ID Application: Your Name (TEAMID)"
export APPLE_CERTIFICATE_PASSWORD="keychain-password"
export APPLE_SIGNING_IDENTITY="Developer ID Application: Your Name (TEAMID)"
export APPLE_ID="you@example.com"
export APPLE_PASSWORD="app-specific-password"   # from appleid.apple.com
export APPLE_TEAM_ID="YOURTEAMID"
```

Then build normally:
```sh
npm run tauri build
```

Tauri handles code signing, notarization, and stapling automatically when those variables are present.

For an **unsigned** local build (no Apple Developer account needed), just run `npm run tauri build` without setting those variables. The app will work on your own machine but macOS Gatekeeper will block it on other machines by default.

---

## Releasing

**Always use `release.sh` to cut a release.** Do not bump versions or build manually.

```sh
./release.sh patch   # 0.3.0 → 0.3.1
./release.sh minor   # 0.3.0 → 0.4.0
./release.sh major   # 0.3.0 → 1.0.0
```

The script:
1. Sources `sign.sh` and validates the Apple signing credentials: all four of `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD`, and `APPLE_TEAM_ID` must be exported, and the signing identity must exist in the keychain. It aborts otherwise — a partial `sign.sh` produces a build that Gatekeeper rejects on other machines.
2. Bumps the version in `package.json`, `src-tauri/tauri.conf.json`, and `src-tauri/Cargo.toml`
3. Generates a changelog entry in `CHANGELOG.md` from commits since the last tag
4. Commits the version bump
5. Runs `npm run tauri build` to produce the `.dmg` (Tauri signs and notarizes using the credentials from step 1)
6. **Verifies the artifacts**: `codesign --verify` and `xcrun stapler validate` on `Judgement.app`, plus a Gatekeeper assessment (`spctl`) that must report *Notarized Developer ID*. If any check fails, the script aborts without committing artifacts.
7. Copies the DMG to `releases/vX.Y.Z/`, commits it, and tags `vX.Y.Z`
8. Syncs `Casks/judgement.rb` to the new version and the DMG's sha256

`sign.sh` lives at the repo root, is git-ignored, and must export the four Apple signing environment variables (see the Creating a DMG section below).

When prompted, answer **y** to push and create a draft GitHub Release (attaches the `.dmg`).
CI then builds the Linux packages and uploads them to that draft.

**The Release stays a draft — CI does not publish it.** Review the notes, then publish yourself:

```sh
gh release edit vX.Y.Z --draft=false
```

Order matters: the tag push is what starts CI, and the job quits early if no Release exists yet for the tag (`gh release view`). Create the draft immediately after pushing the tag, which is what answering **y** does for you.

If you run `release.sh` without a terminal attached (a script, an agent), the push prompt is skipped entirely — the release is built, committed, and tagged locally only, and you push and create the draft by hand.

**File names carry the version** (the one difference from Helm's flow): the
release has `Judgement_X.Y.Z_aarch64.dmg` and CI adds
`Judgement_X.Y.Z_amd64.deb`, `.rpm`, and `.AppImage`. The cask downloads the
versioned DMG.

---

