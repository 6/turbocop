# GitHub App Setup for Agent Workflows

Agent workflows create PRs using `GITHUB_TOKEN`, which has a known limitation: PRs created by `GITHUB_TOKEN` **do not trigger `pull_request` workflows**. This means CI checks (e.g., `checks.yml`) won't run on agent PRs.

The fix: use a GitHub App token instead. PRs created by a GitHub App trigger all workflows normally.

## Setup

### 1. Create or reuse a GitHub App

If you already have a GitHub App (e.g., https://github.com/apps/6), you can reuse it. Otherwise, create one at https://github.com/settings/apps/new.

**Required repository permissions:**

| Permission | Access | Why |
|-----------|--------|-----|
| Contents | Read & write | Push branches |
| Pull requests | Read & write | Create/comment on PRs |
| Workflows | Read & write | Push branches that contain workflow file changes |
| Secrets | Read & write | Optional: allow `codex-auth-refresh.yml` to update `CODEX_AUTH_JSON` |
| Metadata | Read-only | Required (default) |

The `Workflows` permission is needed because the corpus oracle pushes PR branches based on `main`, which may include recent workflow file changes. Without it, GitHub rejects the push with "refusing to allow a GitHub App to create or update workflow without `workflows` permission".
The `Secrets` permission is only needed if you want the manual Codex auth refresh workflow to round-trip `~/.codex/auth.json` back into the repository secret automatically.

### 2. Install the app on the repo

Go to your app's page (e.g., https://github.com/apps/6) and click **Install**. Select the `6/nitrocop` repository (or "All repositories" if you want to reuse the app across repos).

### 3. Get the App ID and private key

- Go to https://github.com/settings/apps → your app → **General**
- Note the **App ID** (a number like `12345`)
- Under **Private keys**, click "Generate a private key" (if you don't have one)
- This downloads a `.pem` file — keep it safe

### 4. Add repo secrets

Go to https://github.com/6/nitrocop/settings/secrets/actions and create:

| Secret | Value |
|--------|-------|
| `GH_APP_ID` | The numeric App ID from step 3 |
| `GH_APP_PRIVATE_KEY` | The entire contents of the `.pem` file |

### 5. Verify

Trigger the `Agent Cop Fix` workflow. The "Create PR" step should:
1. Generate an app installation token via `actions/create-github-app-token`
2. Push the branch and create the PR using that token
3. The PR author will show as your app (e.g., "6[bot]")
4. `checks.yml` and other `pull_request` workflows will trigger automatically

## How it works

The workflow uses [`actions/create-github-app-token`](https://github.com/actions/create-github-app-token) to generate a short-lived installation token from the app's credentials:

```yaml
- uses: actions/create-github-app-token@v1
  id: app-token
  with:
    app-id: ${{ secrets.GH_APP_ID }}
    private-key: ${{ secrets.GH_APP_PRIVATE_KEY }}

- name: Create PR
  env:
    GH_TOKEN: ${{ steps.app-token.outputs.token }}
  run: |
    git remote set-url origin "https://x-access-token:${GH_TOKEN}@github.com/${{ github.repository }}.git"
    git push -u origin "$BRANCH"
    gh pr create --base main --head "$BRANCH" ...
```

The token is scoped to the repository, expires after 1 hour, and has only the permissions granted to the app. It is NOT stored anywhere — it's generated fresh each workflow run.

## Reusing across repos

Since the app is owned by your GitHub account, you can install it on any repo under `github.com/6/*`. Each repo just needs the `GH_APP_ID` and `GH_APP_PRIVATE_KEY` secrets (or use organization-level secrets to share them).

## Fallback

If the app secrets are not configured, the workflow falls back to `GITHUB_TOKEN`. PRs will still be created but CI won't trigger automatically — validation results will be in the workflow summary instead.
