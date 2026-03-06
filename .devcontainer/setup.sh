#!/bin/bash
set -e

export DEBIAN_FRONTEND=noninteractive

# System dependencies
sudo apt-get update -qq
sudo apt-get install -y --no-install-recommends \
  build-essential \
  clang \
  curl \
  fd-find \
  git-lfs \
  jq \
  libffi-dev \
  libreadline-dev \
  libssl-dev \
  libyaml-dev \
  locales \
  python3 \
  ripgrep \
  zlib1g-dev
sudo rm -rf /var/lib/apt/lists/*

# Locale (prevents perl/bundler warnings)
sudo locale-gen en_US.UTF-8

# Install gh CLI
(type -p wget >/dev/null || sudo apt-get install -y wget) \
  && sudo mkdir -p -m 755 /etc/apt/keyrings \
  && out=$(mktemp) && wget -nv -O"$out" https://cli.github.com/packages/githubcli-archive-keyring.gpg \
  && cat "$out" | sudo tee /etc/apt/keyrings/githubcli-archive-keyring.gpg > /dev/null \
  && sudo chmod go+r /etc/apt/keyrings/githubcli-archive-keyring.gpg \
  && echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" | sudo tee /etc/apt/sources.list.d/github-cli.list > /dev/null \
  && sudo apt-get update -qq \
  && sudo apt-get install -y gh

# Install mise and add shims to PATH
curl https://mise.jdx.dev/install.sh | sh
export PATH="$HOME/.local/share/mise/shims:$HOME/.local/bin:$PATH"
eval "$(mise activate bash)"

# Toolchains via mise (+ node for codex, not in project mise.toml)
mise settings ruby.compile=false
mise trust
mise use -g node@lts
mise install
mise reshim

# Persist mise to zsh: PATH in .zshenv (all shells), activation in .zshrc (interactive)
echo 'export PATH="$HOME/.local/share/mise/shims:$HOME/.local/bin:$PATH"' >> ~/.zshenv
echo 'eval "$(~/.local/bin/mise activate zsh)"' >> ~/.zshrc

# AI coding assistants
curl -fsSL https://claude.ai/install.sh | bash
timeout 30 npm install -g @openai/codex || echo "Warning: codex install failed (network issue?), skipping"

# Claude Code config
mkdir -p ~/.claude
cp .devcontainer/.claude/settings.json ~/.claude/settings.json 2>/dev/null \
  || cp .devcontainer/.claude/settings.example.json ~/.claude/settings.json

# Override gpg.ssh.program for container (host mounts op-ssh-sign path which doesn't exist in Linux).
# ~/.gitconfig is bind-mounted read-only, so write overrides to a separate file and use
# GIT_CONFIG_GLOBAL to layer it on top via [include].
# Extract signing key from host gitconfig and write to a file (ssh-keygen needs a file path)
SIGNING_KEY=$(git config -f ~/.gitconfig user.signingkey 2>/dev/null || true)
if [ -n "$SIGNING_KEY" ]; then
  echo "$SIGNING_KEY" > ~/.ssh-signing-key.pub
fi
cat > ~/.gitconfig-local <<'GITCFG'
[include]
    path = ~/.gitconfig
[gpg "ssh"]
    program = /usr/bin/ssh-keygen
GITCFG
# Point signingkey to the file if we extracted one
if [ -f ~/.ssh-signing-key.pub ]; then
  git config -f ~/.gitconfig-local user.signingkey ~/.ssh-signing-key.pub
fi
echo '{"hasCompletedOnboarding":true}' > ~/.claude.json

# Shell aliases
echo 'alias ccd="claude --dangerously-skip-permissions"' >> ~/.zshrc
echo 'alias cxd="codex --full-auto"' >> ~/.zshrc

# Codex config
mkdir -p ~/.codex
cp .devcontainer/.codex/config.toml ~/.codex/config.toml 2>/dev/null \
  || cp .devcontainer/.codex/config.example.toml ~/.codex/config.toml

# Git submodules
git submodule update --init

echo ""
echo "Setup complete. Run:"
echo "  devcontainer exec --workspace-folder . zsh"
