#!/usr/bin/env bash
# Usage: ./release.sh [patch|minor|major]
# Bumps version, generates changelog, commits, tags, and builds a signed,
# notarized .dmg. Mirrors Helm's release flow (macOS only; no Linux CI).

set -e

BUMP=${1:-patch}

if [[ ! "$BUMP" =~ ^(patch|minor|major)$ ]]; then
  echo "Usage: $0 [patch|minor|major]"
  exit 1
fi

# Load signing credentials — abort if missing to prevent unsigned builds
if [[ -f ./sign.sh ]]; then
  source ./sign.sh
else
  echo "✗ sign.sh not found — aborting. Create sign.sh with Apple signing credentials."
  echo "  See CONTRIBUTING.md § Releasing for details."
  exit 1
fi

# Tauri only signs AND notarizes when every one of these is set; a partial
# sign.sh produces a build Gatekeeper rejects on other machines. Fail loudly.
MISSING=()
for VAR in APPLE_SIGNING_IDENTITY APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID; do
  [[ -n "${!VAR:-}" ]] || MISSING+=("$VAR")
done
if (( ${#MISSING[@]} > 0 )); then
  echo "✗ sign.sh did not export: ${MISSING[*]} — aborting."
  echo "  All four are required for signing + notarization. See CONTRIBUTING.md § Releasing."
  exit 1
fi

if ! security find-identity -v -p codesigning | grep -qF "$APPLE_SIGNING_IDENTITY"; then
  echo "✗ Signing identity not found in keychain: $APPLE_SIGNING_IDENTITY"
  echo "  Install the Developer ID Application certificate in your login keychain."
  exit 1
fi

echo "→ Signing credentials OK ($APPLE_SIGNING_IDENTITY)"

# Bump package.json and capture the new version
NEW_VERSION=$(npm version "$BUMP" --no-git-tag-version | sed 's/^v//')
TODAY=$(date +%Y-%m-%d)

echo "→ Bumping to $NEW_VERSION"

# Sync tauri.conf.json
sed -i '' "s/\"version\": \".*\"/\"version\": \"$NEW_VERSION\"/" src-tauri/tauri.conf.json

# Sync Cargo.toml (first occurrence — [workspace.package] version, inherited by
# the app and the CLI so `judgement --version` matches the app)
awk -v ver="$NEW_VERSION" 'done { print; next } /^version = "/ { sub(/^version = ".*"/, "version = \"" ver "\""); done=1 } { print }' src-tauri/Cargo.toml > src-tauri/Cargo.toml.tmp && mv src-tauri/Cargo.toml.tmp src-tauri/Cargo.toml

# Generate changelog from commits since last tag
LAST_TAG=$(git describe --tags --abbrev=0 2>/dev/null || echo "")
if [[ -n "$LAST_TAG" ]]; then
  LOG=$(git log "$LAST_TAG"..HEAD --pretty=format:"- %s" --no-merges)
else
  LOG=$(git log --pretty=format:"- %s" --no-merges)
fi

CHANGELOG_ENTRY="## v$NEW_VERSION — $TODAY

$LOG"

if [[ -f CHANGELOG.md ]]; then
  EXISTING=$(cat CHANGELOG.md)
  printf '%s\n\n%s\n' "$CHANGELOG_ENTRY" "$EXISTING" > CHANGELOG.md
else
  printf '# Changelog\n\n%s\n' "$CHANGELOG_ENTRY" > CHANGELOG.md
fi

echo "→ Changelog updated"

# Refresh Cargo.lock for the new workspace version before committing
(cd src-tauri && cargo metadata --format-version 1 >/dev/null)

# Commit version bump (tag after the DMG commit so the tag includes release artifacts)
git add package.json package-lock.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock CHANGELOG.md
git commit -m "chore: release v$NEW_VERSION"

echo "→ Building (signing + notarization can take a few minutes)..."

npm run tauri build

APP_PATH="src-tauri/target/release/bundle/macos/Judgement.app"
DMG_PATH="src-tauri/target/release/bundle/dmg/Judgement_${NEW_VERSION}_aarch64.dmg"
CLI_PATH="$APP_PATH/Contents/MacOS/judgement-cli"

# Verify the artifacts are signed AND notarized before committing anything.
echo "→ Verifying signature and notarization..."
codesign --verify --deep --strict "$APP_PATH"
codesign --verify --strict "$CLI_PATH"
xcrun stapler validate "$APP_PATH"
if ! spctl -a -t exec -vv "$APP_PATH" 2>&1 | grep -q "Notarized Developer ID"; then
  echo "✗ Gatekeeper does not accept the app as notarized — not committing artifacts."
  echo "  Check the notarization log: xcrun notarytool history --apple-id \$APPLE_ID --team-id \$APPLE_TEAM_ID"
  exit 1
fi
codesign --verify "$DMG_PATH"
echo "✓ App is signed and notarized"

# The bundled CLI must run with no Node/Rust on PATH (A9).
if ! env -i HOME="$HOME" PATH=/usr/bin:/bin "$CLI_PATH" --version | grep -q "$NEW_VERSION"; then
  echo "✗ Bundled CLI did not report v$NEW_VERSION with a bare PATH — not committing artifacts."
  exit 1
fi
echo "✓ Bundled CLI runs without dev runtimes"

# Copy DMG to releases/ and commit
RELEASE_DIR="releases/v$NEW_VERSION"
mkdir -p "$RELEASE_DIR"
cp "$DMG_PATH" "$RELEASE_DIR/"
cp -r "$APP_PATH" "$RELEASE_DIR/" 2>/dev/null || true

DMG_ASSET="$RELEASE_DIR/Judgement_${NEW_VERSION}_aarch64.dmg"
DMG_LATEST="$RELEASE_DIR/Judgement_aarch64.dmg"
cp "$DMG_ASSET" "$DMG_LATEST"

# Sync Homebrew cask version and sha256 (sha256 only available after the DMG is built)
sed -i '' "s/^  version \".*\"/  version \"$NEW_VERSION\"/" Casks/judgement.rb
DMG_SHA=$(shasum -a 256 "$DMG_LATEST" | awk '{ print $1 }')
sed -i '' "s/^  sha256 \".*\"/  sha256 \"$DMG_SHA\"/" Casks/judgement.rb

git add "$DMG_ASSET" Casks/judgement.rb
git commit -m "release: add Judgement_${NEW_VERSION}_aarch64.dmg"
git tag "v$NEW_VERSION"

echo ""
echo "✓ v$NEW_VERSION released"
echo "  Artifacts: $RELEASE_DIR"

if [[ -t 0 ]]; then
  read -r -p "Push and create a draft GitHub Release now? [y/N] " CREATE_DRAFT
  if [[ "$CREATE_DRAFT" =~ ^[Yy]$ ]]; then
    if ! command -v gh >/dev/null 2>&1; then
      echo "✗ gh not found — install GitHub CLI, then create the draft manually:"
      echo "  gh release create \"v$NEW_VERSION\" --draft --title \"Judgement v$NEW_VERSION\" --notes \"\" \"$DMG_LATEST\""
      exit 1
    fi
    if ! gh auth status >/dev/null 2>&1; then
      echo "✗ gh is not authenticated — run 'gh auth login', then create the draft manually."
      exit 1
    fi

    echo "→ Pushing commits and tags..."
    git push && git push --tags

    echo "→ Creating draft GitHub Release..."
    gh release create "v$NEW_VERSION" --draft \
      --title "Judgement v$NEW_VERSION" \
      --notes "" \
      "$DMG_LATEST"

    echo "✓ Draft v$NEW_VERSION created — review the notes, then publish with:"
    echo "  gh release edit v$NEW_VERSION --draft=false"
  fi
fi
