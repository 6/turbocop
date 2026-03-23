#!/usr/bin/env bash
#
# Land patch-new commits from remote branches onto local main.
#
# Usage: land_branch_commits.sh <branch1> [branch2 ...]
#
# - Resolves branch names against origin (suffix match)
# - Cherry-picks only patch-new commits (git cherry)
# - Preserves original author date, resets author to local git user
# - Strips claude.ai URLs and Co-Authored-By lines from messages
# - Appends Co-Authored-By trailer for original author if different
# - Exits non-zero on conflict so the caller (LLM) can resolve
#
set -euo pipefail

die()  { printf 'error: %s\n' "$1" >&2; exit 1; }
info() { printf ':: %s\n' "$1"; }

[[ $# -ge 1 ]] || die "usage: land_branch_commits.sh <branch1> [branch2 ...]"

# Must be on main
current_branch=$(git symbolic-ref --short HEAD 2>/dev/null || true)
[[ "$current_branch" == "main" ]] || die "not on main (currently on '$current_branch')"

# Check for staged changes (unstaged/untracked are fine — they can't conflict with cherry-picks)
if ! git diff --cached --quiet; then
  die "staged changes exist — commit them first"
fi

# Local git identity
local_name=$(git config user.name)
local_email=$(git config user.email)

# Fetch remote refs once
info "fetching remote refs"
remote_refs=$(git ls-remote --heads origin)

resolve_branch() {
  local input="$1"
  # Try exact match first
  local match
  match=$(echo "$remote_refs" | awk -v b="refs/heads/$input" '$2 == b {print $2}')
  if [[ -n "$match" ]]; then
    echo "${match#refs/heads/}"
    return
  fi
  # Suffix match
  local matches
  matches=$(echo "$remote_refs" | awk -v suffix="$input" '$2 ~ suffix"$" {print $2}')
  local count
  count=$(echo "$matches" | grep -c . || true)
  if [[ "$count" -eq 0 ]]; then
    die "no remote branch matching '$input'"
  elif [[ "$count" -eq 1 ]]; then
    echo "${matches#refs/heads/}"
  else
    echo "ambiguous branch '$input' — matches:" >&2
    echo "$matches" | sed 's|refs/heads/|  |' >&2
    exit 1
  fi
}

# Resolve all branch names
resolved=()
for arg in "$@"; do
  resolved+=("$(resolve_branch "$arg")")
done

# Fetch main + all resolved branches
info "fetching main + ${#resolved[@]} branch(es)"
git fetch origin main "${resolved[@]}"

# Check local main vs origin/main
local_main=$(git rev-parse main)
origin_main=$(git rev-parse origin/main)
if [[ "$local_main" != "$origin_main" ]]; then
  ahead=$(git rev-list origin/main..main --count)
  behind=$(git rev-list main..origin/main --count)
  info "local main diverges from origin/main (ahead=$ahead, behind=$behind)"
fi

total_landed=0
total_skipped=0

clean_message() {
  # Strip claude.ai URLs and Co-Authored-By lines, trim trailing blank lines
  grep -v 'https://claude\.ai' \
    | grep -v '^Co-Authored-By:' \
    | awk '{lines[NR]=$0} END{while(NR>0 && lines[NR]=="") NR--; for(i=1;i<=NR;i++) print lines[i]}'
}

for branch in "${resolved[@]}"; do
  info "processing origin/$branch"

  # Get patch-new commits (oldest first)
  mapfile -t cherry_lines < <(git cherry -v main "origin/$branch")

  new_shas=()
  skipped=()
  for line in "${cherry_lines[@]}"; do
    marker="${line:0:1}"
    sha="${line:2:40}"
    subject="${line:43}"
    if [[ "$marker" == "+" ]]; then
      new_shas+=("$sha")
    else
      skipped+=("$sha")
      info "  skip (already on main): ${sha:0:8} $subject"
      total_skipped=$((total_skipped + 1))
    fi
  done

  if [[ ${#new_shas[@]} -eq 0 ]]; then
    info "  no patch-new commits"
    continue
  fi

  info "  ${#new_shas[@]} patch-new commit(s) to land"

  for sha in "${new_shas[@]}"; do
    subject=$(git log -1 --format='%s' "$sha")
    orig_name=$(git log -1 --format='%an' "$sha")
    orig_email=$(git log -1 --format='%ae' "$sha")
    orig_date=$(git log -1 --format='%aI' "$sha")

    info "  cherry-pick ${sha:0:8} $subject"

    # Cherry-pick — exit on conflict so LLM can resolve
    if ! git cherry-pick "$sha"; then
      echo ""
      echo "CONFLICT during cherry-pick of ${sha:0:8}"
      echo "  $subject"
      echo ""
      echo "Resolve the conflict, then run:"
      echo "  git cherry-pick --continue"
      echo ""
      echo "Remaining commits from this branch were not landed."
      exit 2
    fi

    # Build clean message
    msg_file=$(mktemp)
    git log -1 --format='%B' HEAD | clean_message > "$msg_file"

    # Append Co-Authored-By if original author differs from local user
    if [[ "$orig_name" != "$local_name" || "$orig_email" != "$local_email" ]]; then
      printf '\n\nCo-Authored-By: %s <%s>\n' "$orig_name" "$orig_email" >> "$msg_file"
    fi

    # Amend: reset author identity but preserve original author date
    GIT_AUTHOR_DATE="$orig_date" git commit --amend --reset-author -F "$msg_file" --quiet
    rm -f "$msg_file"

    new_sha=$(git rev-parse --short HEAD)
    info "    -> $new_sha"
    total_landed=$((total_landed + 1))
  done

  # Verify branch is fully landed
  remaining=$(git cherry -v main "origin/$branch" | grep -c '^+' || true)
  if [[ "$remaining" -gt 0 ]]; then
    info "  WARNING: $remaining patch-new commit(s) still remain on origin/$branch"
  else
    info "  all commits from origin/$branch landed"
  fi
done

echo ""
info "done: $total_landed landed, $total_skipped skipped (already on main)"

# Final state
ahead=$(git rev-list origin/main..main --count)
if [[ "$ahead" -gt 0 ]]; then
  info "main is ahead of origin/main by $ahead commit(s) — not pushed"
fi

echo ""
echo "Landed commits:"
git log --oneline --reverse origin/main..main | grep -v "$(git log --oneline origin/main -1 | cut -d' ' -f1)" || git log --oneline --reverse origin/main..main
