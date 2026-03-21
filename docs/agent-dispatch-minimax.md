# Agent Dispatch: Claude Code + MiniMax on GHA

Alternative to the Codex-based dispatch. Runs Claude Code CLI with MiniMax M2.7 as the backend directly in GitHub Actions. Cheaper than Codex ($40/mo MiniMax plan vs $200/mo ChatGPT Pro), and uses the same Claude Code agent behavior you're familiar with.

## Architecture

```
You (any machine with gh CLI)
  │
  │  gh workflow run agent-cop-fix-minimax.yml -f cop="Style/NegatedWhile"
  ▼
GitHub Actions
  │  1. Checkout repo + build Rust (cached)
  │  2. generate-cop-task.py → task prompt
  │  3. Install Claude Code CLI
  │  4. claude -p --dangerously-skip-permissions "task prompt"
  │     (backed by MiniMax M2.7 via ANTHROPIC_BASE_URL)
  │  5. cargo test --lib → validate
  │  6. Commit, push, open PR
  ▼
GitHub CI validates the PR
```

## Cost

MiniMax M2.7 pricing: $0.30/1M input tokens, $1.20/1M output tokens.

Estimated cost per cop fix: ~$0.03-0.10 (much cheaper than Codex).

With the MiniMax Plus-Highspeed plan ($40/mo, 4,500 requests/5hr), all 319 Tier 1 cops cost well under the monthly plan. Or use pay-as-you-go for even less.

## One-Time Setup

### Step 1: MiniMax API Key

1. Sign up at [platform.minimax.io](https://platform.minimax.io)
2. Get an API key from the developer dashboard
3. Optionally subscribe to Plus-Highspeed ($40/mo) for higher rate limits

### Step 2: GitHub Repository Secret

Go to **Settings > Secrets and variables > Actions** and add:

| Secret | Value |
|--------|-------|
| `MINIMAX_API_KEY` | Your MiniMax API key |

### Step 3: Branch Protection

Same as the Codex setup — see [agent-dispatch.md](agent-dispatch.md#step-3-branch-protection-optional).

## Usage

Same commands as the Codex workflow, just a different workflow name:

```bash
# Single cop
gh workflow run agent-cop-fix-minimax.yml -f cop="Style/VariableInterpolation"

# Batch (Tier 1)
python3 scripts/agent/tier_cops.py --extended --tier 1 --names | while read cop; do
  gh workflow run agent-cop-fix-minimax.yml -f cop="$cop"
  sleep 5
done

# Retry
gh workflow run agent-cop-retry-minimax.yml -f cop="Style/VariableInterpolation"
```

## Tradeoffs vs Codex

| | Claude Code + MiniMax | Codex (ChatGPT Pro) |
|---|---|---|
| **Cost** | ~$0.03-0.10/cop ($40/mo plan) | Flat $200/mo |
| **Model quality** | MiniMax M2.7 (good, not top-tier) | GPT-5-Codex (strong) |
| **Blast radius** | Pay-per-token, predictable | Capped at $200/mo |
| **Token conflict** | None (API key, no OAuth) | Must not use Codex locally |
| **Agent behavior** | Claude Code (you know it well) | Codex (different conventions) |

**Recommendation:** Try MiniMax for Tier 1 cops (easy, 1-50 FP+FN). If quality is too low, use Codex for Tier 2+.

## Supported use case

Claude Code CLI supports headless/CI execution via `-p` (prompt mode) and `--dangerously-skip-permissions`. MiniMax documents using their API as a Claude Code backend via `ANTHROPIC_BASE_URL`. This is the intended BYOK usage pattern.
