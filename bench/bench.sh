#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "Building rblint (release)..."
cargo build --release --manifest-path "$ROOT/Cargo.toml"
RBLINT="$ROOT/target/release/rblint"

for REPO in mastodon discourse; do
  REPO_DIR="$SCRIPT_DIR/repos/$REPO"
  if [ ! -d "$REPO_DIR" ]; then
    echo "Repo $REPO not found. Run bench/setup.sh first."
    continue
  fi

  echo ""
  echo "=== Benchmarking $REPO ==="
  RB_COUNT=$(find "$REPO_DIR" -name '*.rb' -not -path '*/vendor/*' | wc -l | tr -d ' ')
  echo "  $RB_COUNT .rb files"

  if command -v hyperfine &>/dev/null; then
    echo ""
    echo "--- hyperfine (3 runs, 1 warmup) ---"
    hyperfine --warmup 1 --runs 3 \
      --command-name "rblint" "$RBLINT $REPO_DIR --no-color 2>/dev/null" \
      --command-name "rubocop" "cd $REPO_DIR && bundle exec rubocop --no-color 2>/dev/null"
  else
    echo ""
    echo "--- rblint (release) ---"
    time "$RBLINT" "$REPO_DIR" --no-color 2>/dev/null | tail -1

    echo ""
    echo "--- rubocop ---"
    (cd "$REPO_DIR" && time bundle exec rubocop --no-color 2>/dev/null | tail -1)
  fi

  echo ""
done
