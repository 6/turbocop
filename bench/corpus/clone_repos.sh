#!/usr/bin/env bash
#
# Clone all corpus repos into vendor/corpus/<repo_id>/ for local investigation.
#
# Usage:
#   bench/corpus/clone_repos.sh              # clone all corpus repos
#   bench/corpus/clone_repos.sh --jobs 8     # parallel clones (default: 4)
#   bench/corpus/clone_repos.sh --dry-run    # show what would be cloned
#
# Repos are shallow-cloned (--depth 50) at the exact SHA from the manifest,
# with no submodules. Already-cloned repos are skipped (safe to re-run).
#
# Estimated disk usage: ~5GB for all repos.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
MANIFEST="$REPO_ROOT/bench/corpus/manifest.jsonl"
DEST_DIR="$REPO_ROOT/vendor/corpus"
JOBS=4
DRY_RUN=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --jobs|-j) JOBS="$2"; shift 2 ;;
        --dry-run) DRY_RUN=true; shift ;;
        --help|-h)
            echo "Usage: $0 [--jobs N] [--dry-run]"
            exit 0
            ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

if [[ ! -f "$MANIFEST" ]]; then
    echo "ERROR: manifest not found: $MANIFEST" >&2
    exit 1
fi

TOTAL=$(wc -l < "$MANIFEST" | tr -d ' ')
echo "Corpus: $TOTAL repos → $DEST_DIR"
echo "Parallelism: $JOBS"
echo ""

mkdir -p "$DEST_DIR"

# Parse manifest into arrays
mapfile -t IDS < <(python3 -c "
import json, sys
for line in open('$MANIFEST'):
    r = json.loads(line.strip())
    print(r['id'])
")
mapfile -t URLS < <(python3 -c "
import json, sys
for line in open('$MANIFEST'):
    r = json.loads(line.strip())
    print(r['repo_url'])
")
mapfile -t SHAS < <(python3 -c "
import json, sys
for line in open('$MANIFEST'):
    r = json.loads(line.strip())
    print(r['sha'])
")

RUNNING=0
CLONED=0
SKIPPED=0
FAILED=0

clone_one() {
    local id="$1" repo_url="$2" sha="$3"
    local dest="$DEST_DIR/$id"

    # Skip if already cloned at the right SHA
    if [[ -d "$dest/.git" ]]; then
        local current_sha
        current_sha=$(git -C "$dest" rev-parse HEAD 2>/dev/null || echo "")
        if [[ "$current_sha" == "$sha"* ]]; then
            echo "SKIP  $id (already at ${sha:0:7})"
            return 0
        else
            echo "STALE $id (at ${current_sha:0:7}, want ${sha:0:7}) — removing"
            rm -rf "$dest"
        fi
    fi

    if $DRY_RUN; then
        echo "WOULD $id ← $repo_url @ ${sha:0:7}"
        return 0
    fi

    # Shallow clone + checkout exact SHA
    if ! git clone --depth 50 --no-recurse-submodules --single-branch -q "$repo_url" "$dest" 2>/dev/null; then
        echo "FAIL  $id (clone failed)" >&2
        return 1
    fi

    if ! git -C "$dest" checkout -q "$sha" 2>/dev/null; then
        git -C "$dest" fetch --depth 200 -q 2>/dev/null || true
        if ! git -C "$dest" checkout -q "$sha" 2>/dev/null; then
            echo "WARN  $id (checkout ${sha:0:7} failed, using HEAD)" >&2
        fi
    fi

    echo "OK    $id (${sha:0:7})"
}

for i in "${!IDS[@]}"; do
    clone_one "${IDS[$i]}" "${URLS[$i]}" "${SHAS[$i]}" &

    RUNNING=$(( RUNNING + 1 ))
    if (( RUNNING >= JOBS )); then
        wait -n 2>/dev/null || true
        RUNNING=$(( RUNNING - 1 ))
    fi
done

# Wait for remaining jobs
wait

echo ""
CLONED=$(find "$DEST_DIR" -maxdepth 1 -mindepth 1 -type d 2>/dev/null | wc -l | tr -d ' ')
echo "Done. $CLONED/$TOTAL repos in $DEST_DIR"
du -sh "$DEST_DIR" 2>/dev/null || true
