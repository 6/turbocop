#!/usr/bin/env python3
"""Discover popular Ruby repos from GitHub and add them to the corpus manifest.

Usage:
    # Add top Ruby repos by stars (that have a Gemfile)
    python3 bench/corpus/add_repos.py --stars --count 50

    # Add a specific repo
    python3 bench/corpus/add_repos.py --repo https://github.com/rails/rails

    # Dry run (show what would be added)
    python3 bench/corpus/add_repos.py --stars --count 50 --dry-run
"""

import argparse
import json
import subprocess
import sys
from pathlib import Path

MANIFEST_PATH = Path(__file__).parent / "manifest.jsonl"

# Repos too large for the 30-minute CI job timeout (RuboCop step alone exceeds it).
DENYLIST = {
    "rapid7/metasploit-framework",
    "gitlabhq/gitlabhq",
    "aws/aws-sdk-ruby",
    "googleapis/google-api-ruby-client",
}


def run_gh(args: list[str]) -> dict | list:
    """Run a gh api command and return parsed JSON."""
    cmd = ["gh", "api"] + args
    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.returncode != 0:
        print(f"gh api failed: {result.stderr.strip()}", file=sys.stderr)
        sys.exit(1)
    return json.loads(result.stdout)


def load_manifest() -> list[dict]:
    """Load existing manifest entries."""
    entries = []
    if MANIFEST_PATH.exists():
        for line in MANIFEST_PATH.read_text().splitlines():
            line = line.strip()
            if line:
                entries.append(json.loads(line))
    return entries


def existing_repo_urls(entries: list[dict]) -> set[str]:
    """Get set of repo URLs already in the manifest (for dedup)."""
    return {e["repo_url"].rstrip("/").lower() for e in entries}


def normalize_repo_url(url: str) -> str:
    """Normalize a GitHub repo URL to https://github.com/owner/repo form."""
    url = url.rstrip("/")
    if not url.startswith("http"):
        url = f"https://github.com/{url}"
    # Strip .git suffix
    if url.endswith(".git"):
        url = url[:-4]
    return url


def make_id(owner: str, repo: str, sha: str) -> str:
    return f"{owner}__{repo}__{sha[:7]}"


def search_stars(count: int, existing_urls: set[str] | None = None) -> list[dict]:
    """Search GitHub for top Ruby repos by stars.

    Searches multiple star ranges to find enough new repos beyond what's
    already in the manifest. The GitHub search API caps at 1000 results per
    query, so we use descending star-range buckets to reach deeper.
    """
    seen = set(existing_urls or ())
    repos = []

    # Start with highest stars, then go lower to find more repos
    star_queries = [
        "stars:>2000",
        "stars:1000..2000",
        "stars:500..1000",
        "stars:200..500",
        "stars:100..200",
    ]

    for query_stars in star_queries:
        if len(repos) >= count:
            break
        page = 1
        while len(repos) < count and page <= 10:
            data = run_gh([
                f"search/repositories?q=language:ruby+{query_stars}&sort=stars&order=desc&per_page=100&page={page}",
            ])
            items = data.get("items", [])
            if not items:
                break
            for item in items:
                url = item.get("html_url", "").rstrip("/").lower()
                if url not in seen:
                    repos.append(item)
                    seen.add(url)
            page += 1

    return repos[:count]


def get_default_branch_sha(owner: str, repo: str) -> str | None:
    """Get the HEAD SHA of the default branch."""
    try:
        data = run_gh([f"repos/{owner}/{repo}/commits?per_page=1"])
        if data and isinstance(data, list):
            return data[0]["sha"]
    except (KeyError, IndexError):
        pass
    return None


def has_gemfile(owner: str, repo: str) -> bool:
    """Check if the repo has a Gemfile at the root."""
    try:
        result = subprocess.run(
            ["gh", "api", f"repos/{owner}/{repo}/contents/Gemfile", "--silent"],
            capture_output=True, text=True,
        )
        return result.returncode == 0
    except Exception:
        return False


def add_specific_repo(url: str) -> dict | None:
    """Create a manifest entry for a specific repo URL."""
    url = normalize_repo_url(url)
    # Extract owner/repo from URL
    parts = url.rstrip("/").split("/")
    if len(parts) < 2:
        print(f"Cannot parse repo URL: {url}", file=sys.stderr)
        return None
    owner, repo = parts[-2], parts[-1]

    sha = get_default_branch_sha(owner, repo)
    if not sha:
        print(f"  Could not get HEAD SHA for {owner}/{repo}", file=sys.stderr)
        return None

    return {
        "id": make_id(owner, repo, sha),
        "repo_url": url,
        "sha": sha,
        "source": "manual",
        "set": "frozen",
        "notes": "manually added",
    }


def main():
    parser = argparse.ArgumentParser(description="Add repos to corpus manifest")
    parser.add_argument("--stars", action="store_true", help="Discover top Ruby repos by stars")
    parser.add_argument("--count", type=int, default=50, help="Number of repos to discover (with --stars)")
    parser.add_argument("--repo", type=str, help="Add a specific repo by URL")
    parser.add_argument("--dry-run", action="store_true", help="Show what would be added without writing")
    args = parser.parse_args()

    if not args.stars and not args.repo:
        parser.error("Specify --stars or --repo")

    existing = load_manifest()
    seen_urls = existing_repo_urls(existing)
    new_entries = []

    if args.repo:
        url = normalize_repo_url(args.repo)
        parts = url.rstrip("/").split("/")
        slug = f"{parts[-2]}/{parts[-1]}" if len(parts) >= 2 else ""
        if slug in DENYLIST:
            print(f"Repo is denylisted (too large for CI): {slug}", file=sys.stderr)
        elif url.lower() in seen_urls:
            print(f"Already in manifest: {url}", file=sys.stderr)
        else:
            print(f"Adding {url}...", file=sys.stderr)
            entry = add_specific_repo(url)
            if entry:
                new_entries.append(entry)

    if args.stars:
        print(f"Searching for top {args.count} Ruby repos by stars...", file=sys.stderr)
        candidates = search_stars(args.count, existing_urls=seen_urls)
        print(f"Found {len(candidates)} candidates", file=sys.stderr)

        for item in candidates:
            url = item.get("html_url", "")
            owner = item.get("owner", {}).get("login", "")
            repo_name = item.get("name", "")
            archived = item.get("archived", False)

            if archived:
                continue
            if f"{owner}/{repo_name}" in DENYLIST:
                print(f"  Skipping {owner}/{repo_name} (denylisted)", file=sys.stderr)
                continue
            if normalize_repo_url(url).lower() in seen_urls:
                continue

            # Check for Gemfile
            print(f"  Checking {owner}/{repo_name}...", file=sys.stderr, end="")
            if not has_gemfile(owner, repo_name):
                print(" no Gemfile, skipping", file=sys.stderr)
                continue

            sha = get_default_branch_sha(owner, repo_name)
            if not sha:
                print(" no SHA, skipping", file=sys.stderr)
                continue

            entry = {
                "id": make_id(owner, repo_name, sha),
                "repo_url": normalize_repo_url(url),
                "sha": sha,
                "source": "github_stars",
                "set": "frozen",
                "notes": f"auto-discovered, {item.get('stargazers_count', 0)} stars",
            }
            new_entries.append(entry)
            seen_urls.add(normalize_repo_url(url).lower())
            print(f" added ({sha[:7]})", file=sys.stderr)

    if not new_entries:
        print("\nNo new repos to add.", file=sys.stderr)
        return

    if args.dry_run:
        print(f"\nDry run: would add {len(new_entries)} repos:", file=sys.stderr)
        for e in new_entries:
            print(f"  {e['repo_url']} ({e['sha'][:7]})", file=sys.stderr)
    else:
        with open(MANIFEST_PATH, "a") as f:
            for e in new_entries:
                f.write(json.dumps(e) + "\n")
        total = len(existing) + len(new_entries)
        print(f"\nAdded {len(new_entries)} repos. Manifest now has {total} entries.", file=sys.stderr)


if __name__ == "__main__":
    main()
