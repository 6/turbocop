#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
RESULTS_DIR="$SCRIPT_DIR/results"
mkdir -p "$RESULTS_DIR"

echo "Building rblint (release)..."
cargo build --release --manifest-path "$ROOT/Cargo.toml" --quiet
RBLINT="$ROOT/target/release/rblint"

# Get covered cop list for filtering rubocop output
"$RBLINT" --list-cops > "$RESULTS_DIR/covered-cops.txt"
COP_COUNT=$(wc -l < "$RESULTS_DIR/covered-cops.txt" | tr -d ' ')
echo "$COP_COUNT cops covered by rblint"

for REPO in mastodon discourse; do
  REPO_DIR="$SCRIPT_DIR/repos/$REPO"
  if [ ! -d "$REPO_DIR" ]; then
    echo "Repo $REPO not found. Run bench/setup.sh first."
    continue
  fi
  echo ""
  echo "=== Conformance: $REPO ==="

  # 1. Run rblint in JSON mode
  echo "  Running rblint..."
  "$RBLINT" "$REPO_DIR" --format json --no-color \
    > "$RESULTS_DIR/${REPO}-rblint.json" 2>/dev/null || true

  # 2. Run rubocop in JSON mode (all cops — compare.rb filters to covered)
  echo "  Running rubocop..."
  (cd "$REPO_DIR" && bundle exec rubocop --format json --no-color \
    > "$RESULTS_DIR/${REPO}-rubocop.json" 2>/dev/null || true)

  # 3. Compare (console output + JSON for report)
  echo ""
  ruby "$SCRIPT_DIR/compare.rb" \
    --json "$RESULTS_DIR/${REPO}-conformance.json" \
    "$RESULTS_DIR/${REPO}-rblint.json" \
    "$RESULTS_DIR/${REPO}-rubocop.json" \
    "$RESULTS_DIR/covered-cops.txt" \
    "$REPO_DIR"
done

echo ""
echo "Conformance JSON written to $RESULTS_DIR/"
echo "Run bench/report.rb to generate results.md"
