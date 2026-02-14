#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
RESULTS_DIR="$SCRIPT_DIR/results"
mkdir -p "$RESULTS_DIR"

# Preflight
if ! command -v hyperfine &>/dev/null; then
  echo "Error: hyperfine not found. Install via: mise install" >&2
  exit 1
fi

RUNS="${BENCH_RUNS:-5}"
WARMUP="${BENCH_WARMUP:-2}"

echo "Building rblint (release)..."
cargo build --release --manifest-path "$ROOT/Cargo.toml" --quiet
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
  echo "  $RUNS runs, $WARMUP warmup"
  echo ""

  hyperfine \
    --warmup "$WARMUP" \
    --runs "$RUNS" \
    --ignore-failure \
    --export-json "$RESULTS_DIR/${REPO}-bench.json" \
    --command-name "rblint" "$RBLINT $REPO_DIR --no-color 2>/dev/null" \
    --command-name "rubocop" "cd $REPO_DIR && bundle exec rubocop --no-color 2>/dev/null"
done

echo ""
echo "Benchmark JSON written to $RESULTS_DIR/"
echo "Run bench/report.rb to generate results.md"
