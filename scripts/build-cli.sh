#!/usr/bin/env bash
# Builds the judgement CLI and stages it as Tauri's externalBin
# (src-tauri/binaries/judgement-cli-<target-triple>), which the bundle copies
# to Judgement.app/Contents/MacOS/judgement-cli.
#
# Usage: scripts/build-cli.sh [debug|release]

set -euo pipefail

PROFILE=${1:-debug}
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TRIPLE=$(rustc -vV | sed -n 's/^host: //p')

cd "$ROOT/src-tauri"
if [[ "$PROFILE" == "release" ]]; then
  cargo build --release -p judgement-core --bin judgement
else
  cargo build -p judgement-core --bin judgement
fi

mkdir -p binaries
cp "target/$PROFILE/judgement" "binaries/judgement-cli-$TRIPLE"
echo "→ staged binaries/judgement-cli-$TRIPLE ($PROFILE)"
