#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "=== Step 1: Setup ==="
bash "$SCRIPT_DIR/setup.sh"

echo ""
echo "=== Step 2: Benchmark ==="
bash "$SCRIPT_DIR/bench.sh"

echo ""
echo "=== Step 3: Conformance ==="
bash "$SCRIPT_DIR/conform.sh"

echo ""
echo "=== Step 4: Generate report ==="
ruby "$SCRIPT_DIR/report.rb"

echo ""
echo "Done! See bench/results.md"
