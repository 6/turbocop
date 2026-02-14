#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPOS_DIR="$SCRIPT_DIR/repos"
mkdir -p "$REPOS_DIR"

# Mastodon — pin to a stable release tag
MASTODON_REPO="https://github.com/mastodon/mastodon.git"
MASTODON_REF="v4.3.4"

if [ ! -d "$REPOS_DIR/mastodon" ]; then
  echo "Cloning mastodon at $MASTODON_REF..."
  git clone --depth 1 --branch "$MASTODON_REF" "$MASTODON_REPO" "$REPOS_DIR/mastodon"
else
  echo "mastodon already cloned, skipping."
fi
echo "Installing mastodon bundle..."
(cd "$REPOS_DIR/mastodon" && bundle install --jobs 4 --quiet)

# Discourse — pin to a stable tag
DISCOURSE_REPO="https://github.com/discourse/discourse.git"
DISCOURSE_REF="v3.4.3"

if [ ! -d "$REPOS_DIR/discourse" ]; then
  echo "Cloning discourse at $DISCOURSE_REF..."
  git clone --depth 1 --branch "$DISCOURSE_REF" "$DISCOURSE_REPO" "$REPOS_DIR/discourse"
else
  echo "discourse already cloned, skipping."
fi
echo "Installing discourse bundle..."
(cd "$REPOS_DIR/discourse" && bundle install --jobs 4 --quiet)

echo "Setup complete."
