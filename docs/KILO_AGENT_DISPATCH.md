# Agent Dispatch for Corpus Conformance

Automated system for fixing corpus conformance gaps by dispatching remote agents (via Kilo Cloud Agent + MiniMax BYOK) to fix one cop at a time. Each agent gets a self-contained task prompt, works on a prepared branch, and pushes changes validated by CI.

## Architecture

```
You (any machine with gh CLI)
  │
  │  gh workflow run agent-cop-fix.yml -f cop="Style/NegatedWhile"
  ▼
GitHub Actions (agent-cop-fix.yml)
  │  1. Checkout repo with vendor submodules
  │  2. generate-cop-task.py → self-contained task prompt
  │  3. Create + push branch: fix/style-negated_while
  │  4. kilo_dispatch.py → POST task to Kilo webhook
  ▼
Kilo Cloud Agent (MiniMax M2.7-highspeed via BYOK)
  │  1. Clones repo, runs startup commands (install Rust, cargo build,
  │     remove CLAUDE.md/AGENTS.md so agent reads only .kilocode/rules/)
  │  2. Reads task prompt + .kilocode/rules/cop-fix.md
  │  3. TDD: add test → verify fails → fix cop → verify passes
  │  4. Auto-commits and pushes to branch after each message
  ▼
GitHub CI (on push to the branch)
  │  checks.yml: clippy, full test suite, corpus smoke test
  │  agent-cop-check.yml: per-cop count check vs extended corpus baseline
  ▼
You review + merge (or retry with stronger model)
```

## One-Time Setup

### Step 1: MiniMax Token Plan

1. Go to [platform.minimax.io](https://platform.minimax.io/subscribe/token-plan)
2. Subscribe to **Plus-Highspeed** ($40/mo) — 4,500 requests per 5-hour window on M2.7-highspeed
3. Note your MiniMax API key

This is enough for all 319 Tier 1 cops. Upgrade to Max-Highspeed ($80/mo, 15K req/5hr) if you hit rate limits.

### Step 2: Kilo Cloud Agent

1. Sign up at [kilo.ai](https://kilo.ai)
2. Connect your GitHub repo under **Integrations**
3. Create an **environment profile** named `minimax-highspeed`:
   - BYOK provider: MiniMax
   - API key: your MiniMax API key
   - Model: MiniMax M2.7-highspeed
   - Startup commands: `bash .kilocode/setup-script`
4. Create a **webhook trigger** linked to the `minimax-highspeed` profile:
   - Prompt template: `{{bodyJson}}` (the dispatch sends JSON with a `message` field containing the full task)
   - Inbound auth: set a shared secret (this becomes `KILO_WEBHOOK_SECRET` in GitHub secrets)
5. Note the webhook URL

The setup script (`.kilocode/setup-script`, committed to the repo) installs build tools + Rust, builds the project, and removes files that would confuse the agent (CLAUDE.md, AGENTS.md, .claude/skills/, etc). These deletions are ephemeral — they only happen in the container and are never committed. The agent gets its instructions from `.kilocode/rules/cop-fix.md` and the task prompt.

**Optional: additional profiles for harder cops.** Create more environment profiles (`claude-sonnet`, `claude-opus`) with Anthropic API keys instead of MiniMax. Create a webhook trigger for each. You only need `minimax-highspeed` to start.

### Step 3: GitHub Repository Secrets

Go to **Settings > Secrets and variables > Actions** and add:

| Secret | Value | Required? |
|--------|-------|-----------|
| `KILO_WEBHOOK_SECRET` | Shared secret you set on the Kilo webhook trigger (any random string) | Yes |
| `KILO_WEBHOOK_MINIMAX` | Webhook URL for minimax-highspeed profile | Yes |
| `KILO_WEBHOOK_CLAUDE_SONNET` | Webhook URL for claude-sonnet profile | For Tier 2 cops |
| `KILO_WEBHOOK_CLAUDE_OPUS` | Webhook URL for claude-opus profile | For Tier 3 cops |

### Step 4: Branch Protection

Prevent agents from pushing directly to `main`.

**Recommended: GitHub Ruleset (restricts agents, not you)**

Go to **Settings > Rules > Rulesets > New ruleset**:

- Name: `agent-pr-required`
- Enforcement: Active
- Targets: `main` branch
- Bypass list: Add your GitHub username
- Rules:
  - [x] Require a pull request before merging
  - [x] Require status checks to pass (add `build-and-test`, `cop-check`)

### Step 5: Verify kilo_dispatch.py

The dispatch script sends a JSON POST to the Kilo webhook. The current payload format is a best-guess:

```json
{"message": "<task prompt>", "metadata": {"cop": "Style/NegatedWhile", "branch": "fix/style-negated_while"}}
```

You may need to adjust `scripts/agent/kilo_dispatch.py` (~125 lines) once you test with Kilo's actual webhook API. The Kilo webhook docs say `{{body}}` and `{{bodyJson}}` template variables are available in the trigger's prompt template.

## Operator Workflow

This is what YOU do locally (or ask Claude to do). The agents handle the coding.

### Phase 1: Triage (5 min)

See what needs fixing and how hard it is:

```bash
# Overall picture
python3 scripts/agent/tier_cops.py --extended

# Output:
#   Tier 1 (1-50 FP+FN): 319 cops, ~2,854 total FP+FN
#   Tier 2 (51-1000 FP+FN): 88 cops, ~26,877 total FP+FN
#   Tier 3 (1001+ FP+FN): 61 cops, ~441,907 total FP+FN

# Browse Tier 1 cops (sorted by difficulty)
python3 scripts/agent/tier_cops.py --extended --tier 1
```

**No per-cop investigation needed for Tier 1.** These are 1-50 FP+FN cops — the task packet already includes corpus examples with source context. Let the agent figure it out.

For **Tier 2+**, you may want to investigate first:

```bash
# Investigate a specific cop before dispatching
python3 scripts/investigate-cop.py Style/GuardClause --extended --context

# See which repos are affected
python3 scripts/investigate-cop.py Style/GuardClause --extended --repos-only
```

### Phase 2: Pilot (10 cops, ~1 hour)

Test the pipeline with the 10 easiest cops before scaling:

```bash
# Inspect a task packet first to see what the agent gets
python3 scripts/agent/generate-cop-task.py Style/VariableInterpolation --extended

# Dispatch 10 cops one at a time
for cop in \
  "Layout/ConditionPosition" \
  "Layout/SpaceInsideRangeLiteral" \
  "Layout/SpaceBeforeBrackets" \
  "Lint/DuplicateRegexpCharacterClassElement" \
  "Lint/ElseLayout" \
  "Lint/RescueException" \
  "Performance/ChainArrayAllocation" \
  "Style/NegatedWhile" \
  "Style/KeywordParametersOrder" \
  "Style/VariableInterpolation"; do
  gh workflow run agent-cop-fix.yml -f cop="$cop"
  sleep 5
done
```

**Check the results** (wait ~15-30 min for agents to finish):

```bash
# List recent PRs from agents
gh pr list --label agent-fix  # if you label them
gh pr list --search "Fix in:title"

# Check if CI passed on each
gh pr checks <pr-number>
```

**What to verify:**
- Did the agent push changes to the branch?
- Does CI pass (`checks.yml` + `agent-cop-check.yml`)?
- Did it follow TDD (test added before fix)?
- Did it stay within its cop's files?
- Did it add a `///` doc comment?

If ≥7/10 are usable → proceed to Phase 3. If <5/10 → tweak `.kilocode/rules/cop-fix.md` or the task template and re-pilot.

### Phase 3: Batch Dispatch (Tier 1)

```bash
# Dispatch all Tier 1 cops
python3 scripts/agent/tier_cops.py --extended --tier 1 --names | while read cop; do
  gh workflow run agent-cop-fix.yml -f cop="$cop"
  sleep 5  # avoid GHA rate limiting
done
```

This dispatches ~319 cops. At Kilo's 20 in-flight cap per webhook, they'll queue and execute over several hours.

**Monitor progress:**

```bash
# How many PRs are open?
gh pr list --state open | wc -l

# How many passed CI?
gh pr list --state open --search "status:success" | wc -l
```

**Merge the good ones:**

```bash
# Review and merge a PR
gh pr view <number>
gh pr merge <number> --squash
```

### Phase 4: Retry Failures

After the first pass, some cops will have failed PRs (CI failure, wrong fix, etc.):

```bash
# Retry with a stronger model — auto-includes prior attempt context
gh workflow run agent-cop-retry.yml -f cop="Style/VariableInterpolation"

# Retry with extra guidance
gh workflow run agent-cop-retry.yml \
  -f cop="Style/VariableInterpolation" \
  -f model="claude-sonnet" \
  -f extra_context="The FN is a global variable interpolation (#\$0) not an instance variable"
```

The retry workflow automatically:
- Finds all prior failed PRs for that cop
- Extracts their diffs and CI failure logs
- Closes stale open PRs
- Sends everything to the agent so it doesn't repeat mistakes
- Defaults to `claude-sonnet` (stronger than MiniMax)

### Phase 5: Validate

After merging a batch (~20-50 PRs), run the full corpus oracle:

```bash
gh workflow run corpus-oracle.yml -f corpus_size=extended
```

This takes ~90 min and validates against all 5,593 repos with exact location matching.

### Phase 6: Tier 2 (Optional — harder cops)

For Tier 2 cops (51-1000 FP+FN), investigate before dispatching:

```bash
# Investigate the cop
python3 scripts/investigate-cop.py Style/GuardClause --extended --context

# If the pattern is clear, dispatch with claude-sonnet
gh workflow run agent-cop-fix.yml -f cop="Style/GuardClause" -f model="claude-sonnet"

# If it's really hard, use opus
gh workflow run agent-cop-fix.yml -f cop="Layout/IndentationWidth" -f model="claude-opus"
```

For Tier 3 cops (1001+ FP+FN), use Claude Code's `/fix-department` workflow locally — these have fundamental implementation gaps that need deep investigation.

## How It Works

### Task Packet

`generate-cop-task.py` produces a single markdown document per cop containing:

- **Focused instructions**: TDD workflow, fixture format, validation commands
- **The cop's Rust source** (`src/cop/<dept>/<name>.rs`)
- **RuboCop's Ruby implementation** (ground truth from `vendor/`)
- **RuboCop spec excerpts** (`expect_offense` / `expect_no_offenses` blocks)
- **Current test fixtures** (`offense.rb`, `no_offense.rb`)
- **Corpus FP/FN examples** with source context
- **Prism pitfall notes** (only if relevant to this cop)

### Agent Configuration

The agent's behavior is controlled by two things:

1. **`.kilocode/rules/cop-fix.md`** (committed to repo) — focused rules for cop-fix tasks: what to modify, how to test, fixture format, Prism notes
2. **Startup commands** (in Kilo environment profile) — remove CLAUDE.md, AGENTS.md, and .claude/skills/ so the agent doesn't read irrelevant project context

### CI Validation

**`checks.yml`** (existing, runs on all PRs):
- `cargo fmt --check`, `cargo clippy`, full `cargo test`, corpus smoke test

**`agent-cop-check.yml`** (new, runs when cop files change):
- Auto-detects which cop(s) changed via git diff
- Downloads `corpus-results.json` from the latest CI corpus oracle run
- Runs `check-cop.py --extended --verbose` — count check against cached baseline
- Advisory only (cached data may be stale)

Neither workflow runs against the full 5,593-repo corpus. Use `corpus-oracle.yml` for definitive validation after merging batches.

### Cop Tiers

| Tier | FP+FN | Count | Model | Est. Cost |
|------|-------|-------|-------|-----------|
| 1 | 1-50 | ~319 | MiniMax M2.7-highspeed | $40/mo plan |
| 2 | 51-1000 | ~88 | Claude Sonnet (BYOK) | ~$0.60/cop |
| 3 | 1001+ | ~61 | Claude Opus (BYOK) | ~$2-5/cop |

All tiers use the same Kilo Cloud Agent — just different BYOK model profiles.

## Scripts

| Script | Where it runs | Purpose |
|--------|--------------|---------|
| `scripts/agent/generate-cop-task.py` | GitHub Actions | Produces self-contained task prompt |
| `scripts/agent/tier_cops.py` | Your laptop | Classifies cops by difficulty |
| `scripts/agent/detect_changed_cops.py` | GitHub Actions (CI) | Maps git diff to cop names |
| `scripts/agent/kilo_dispatch.py` | GitHub Actions | Sends task to Kilo webhook |
| `scripts/agent/collect_prior_attempts.py` | GitHub Actions (retry) | Gathers diffs + logs from prior failed PRs |

## Workflows

| Workflow | Trigger | Purpose |
|----------|---------|---------|
| `agent-cop-fix.yml` | `workflow_dispatch` | Generate task + create branch + dispatch to Kilo |
| `agent-cop-retry.yml` | `workflow_dispatch` | Retry with prior attempt context, close stale PRs |
| `agent-cop-check.yml` | PR (cop file changes) | Validate changed cops against corpus baseline |

## Security

- `workflow_dispatch` requires **write access** — public users can see the YAML but cannot trigger it
- Secrets are never exposed to forks or external PRs
- Branch protection / rulesets prevent pushing directly to `main`

## Fallback: Hetzner + MiniMax

If Kilo doesn't work out (15-min limit too tight, quality too low, etc.), the same task packets work with a self-hosted Hetzner box running Claude Code in headless mode with MiniMax via `ANTHROPIC_BASE_URL`.
