# Agent Dispatch for Corpus Conformance

Automated system for fixing corpus conformance gaps by dispatching Codex agents to fix one cop at a time. Each cop runs in a GitHub Actions runner with Codex CLI, which edits the code, validates with `cargo test`, and opens a PR.

## Architecture

```
You (any machine with gh CLI)
  │
  │  gh workflow run agent-cop-fix.yml -f cop="Style/NegatedWhile"
  ▼
GitHub Actions (agent-cop-fix.yml)
  │  1. Checkout repo + build Rust (cached, ~1 min)
  │  2. generate-cop-task.py → self-contained task prompt
  │  3. codex exec --full-auto → Codex edits files in the GHA runner
  │  4. cargo test --lib → validate the fix compiles + tests pass
  │  5. Commit, push branch, open PR
  ▼
GitHub CI (on the PR)
  │  checks.yml: clippy, full test suite, corpus smoke test
  │  agent-cop-check.yml: per-cop count check vs extended corpus baseline
  ▼
You review + merge
```

No external cloud service needed. Everything runs in GHA with Codex CLI as the AI agent.

## One-Time Setup

### Step 1: Codex Auth Credentials

The workflow uses your ChatGPT Pro plan (flat rate, no per-token billing).

1. Install Codex CLI locally: `npm install -g @openai/codex@latest`
2. Run `codex login` — this opens a browser for ChatGPT OAuth
3. Copy the auth file: `cat ~/.codex/auth.json`
4. The content of this file becomes your `CODEX_AUTH_JSON` secret

Codex automatically refreshes tokens, but if they expire between runs, re-run `codex login` locally and update the secret.

**Important:** Do not use Codex CLI locally with the same account while GHA jobs are running — token refreshes will conflict. This shouldn't be an issue if you use Claude Code locally (which uses a separate Anthropic account).

**Usage limits:** ChatGPT Pro allows 300-1500 messages per 5-hour window. Each cop fix uses ~10-30 internal messages. Dispatch in small batches (5-10 cops at a time) and monitor usage at [chatgpt.com/codex/settings/usage](https://chatgpt.com/codex/settings/usage).

**This is a supported use case.** OpenAI explicitly documents running Codex in CI/CD:
- [Non-interactive mode](https://developers.openai.com/codex/noninteractive) — `codex exec` is "designed for CI/CD jobs"
- [CI/CD auth guide](https://developers.openai.com/codex/auth/ci-cd-auth) — shows how to use ChatGPT-managed auth on CI runners
- [Auth docs](https://developers.openai.com/codex/auth) — documents both API key and ChatGPT OAuth auth methods

### Step 2: GitHub Repository Secrets

Go to **Settings > Secrets and variables > Actions** and add:

| Secret | Value |
|--------|-------|
| `CODEX_AUTH_JSON` | Contents of `~/.codex/auth.json` from Step 1 |

### Step 3: Branch Protection (optional)

To prevent accidental merges of bad fixes:

Go to **Settings > Rules > Rulesets > New ruleset**:

- Name: `agent-pr-required`
- Enforcement: Active
- Targets: `main` branch
- Bypass list: Add your GitHub username
- Rules:
  - [x] Require status checks to pass (add `build-and-test`)

## Operator Workflow

### Phase 1: Triage (5 min)

```bash
python3 scripts/agent/tier_cops.py --extended
```

### Phase 2: Pilot (10 cops)

```bash
# Inspect a task packet first
python3 scripts/agent/generate-cop-task.py Style/VariableInterpolation --extended

# Dispatch one cop
gh workflow run agent-cop-fix.yml -f cop="Style/VariableInterpolation"
```

Wait ~10-15 min (build + Codex agent + validation). Check the PR:

```bash
gh pr list --search "Fix in:title" --state open
```

**What to verify:**
- Does CI pass?
- Is the fix correct (read the diff)?
- Did it add a test case + doc comment?

If ≥7/10 pilot cops produce usable PRs, scale to Phase 3.

### Phase 3: Batch Dispatch

```bash
python3 scripts/agent/tier_cops.py --extended --tier 1 --names | while read cop; do
  gh workflow run agent-cop-fix.yml -f cop="$cop"
  sleep 5
done
```

GHA runs these in parallel (up to your concurrency limit, typically 20 for free/pro plans).

### Phase 4: Retry Failures

```bash
gh workflow run agent-cop-retry.yml -f cop="Style/VariableInterpolation"
gh workflow run agent-cop-retry.yml -f cop="Style/VariableInterpolation" \
  -f extra_context="The FN is a global variable interpolation"
```

The retry workflow auto-discovers all prior failed PRs, includes their diffs and CI failure logs in the prompt, and closes stale PRs before dispatching.

### Phase 5: Validate

After merging ~20-50 PRs:

```bash
gh workflow run corpus-oracle.yml -f corpus_size=extended
```

## How It Works

### Task Packet

`generate-cop-task.py` produces a markdown prompt containing:
- Focused instructions (TDD workflow, fixture format)
- The cop's Rust source
- RuboCop's Ruby implementation (ground truth)
- RuboCop spec excerpts
- Current test fixtures
- Corpus FP/FN examples with source context
- Prism pitfall notes (if relevant)

### Built-in Validation

The dispatch workflow validates BEFORE creating a PR:
1. `cargo test --lib --no-run` — compiles
2. `cargo test --lib -- cop::<dept>::<name>` — cop tests pass

If validation fails, no PR is created and the workflow fails visibly.

### CI Validation

On the PR, two additional workflows run:
- **checks.yml**: clippy, full test suite, corpus smoke test
- **agent-cop-check.yml**: per-cop count check against extended corpus baseline

### Cop Tiers

| Tier | FP+FN | Count | Est. Cost/Cop |
|------|-------|-------|---------------|
| 1 | 1-50 | ~319 | ~$0.30-1.00 |
| 2 | 51-1000 | ~88 | ~$1-3 |
| 3 | 1001+ | ~61 | ~$3-10 (or use `/fix-department` locally) |

## Scripts

| Script | Purpose |
|--------|---------|
| `scripts/agent/generate-cop-task.py` | Produces self-contained task prompt per cop |
| `scripts/agent/tier_cops.py` | Classifies cops by difficulty tier |
| `scripts/agent/detect_changed_cops.py` | Maps git diff to cop names (CI) |
| `scripts/agent/collect_prior_attempts.py` | Gathers diffs + logs from prior failed PRs |

## Workflows

| Workflow | Trigger | Purpose |
|----------|---------|---------|
| `agent-cop-fix.yml` | `workflow_dispatch` | Generate prompt → Codex fixes → validate → PR |
| `agent-cop-retry.yml` | `workflow_dispatch` | Retry with prior attempt context |
| `agent-cop-check.yml` | PR (cop file changes) | Validate changed cops against corpus |
| `agent-build-cache.yml` | `workflow_dispatch` | Pre-build Rust cache (optional optimization) |

## Security

- `workflow_dispatch` requires **write access** — public users can see but not trigger
- `OPENAI_API_KEY` is never exposed to forks or external PRs
- Branch protection prevents pushing directly to `main`

## Advantages over Kilo/external cloud agents

- **No external service dependency** — just GHA + OpenAI API
- **Full Rust build environment** — GHA has 14GB+ disk, Rust cache, no disk issues
- **Built-in validation** — Codex edits are tested before PR creation
- **Parallel execution** — GHA runs up to 20 jobs concurrently
- **No container bugs** — GHA runners are reliable and well-documented
- **Consistent branches** — we control the branch name, not a random cloud agent
