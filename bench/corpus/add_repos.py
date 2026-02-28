#!/usr/bin/env python3
"""Discover popular Ruby repos from GitHub and add them to the corpus manifest.

Uses the GitHub GraphQL API to search efficiently — a single query fetches
repos, their default branch SHA, and checks for a Gemfile, eliminating
the N+1 REST API calls of the previous implementation.

Usage:
    # Add top Ruby repos by stars (that have a Gemfile)
    python3 bench/corpus/add_repos.py --stars --count 50

    # Add a specific repo
    python3 bench/corpus/add_repos.py --repo https://github.com/rails/rails

    # Dry run (show what would be added)
    python3 bench/corpus/add_repos.py --stars --count 50 --dry-run
"""

from __future__ import annotations

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
    "instructure/canvas-lms",
    "googleapis/google-cloud-ruby",
}


def graphql(query: str, variables: dict | None = None) -> dict:
    """Run a GraphQL query via `gh api graphql` with retry on transient errors."""
    cmd = ["gh", "api", "graphql", "-f", f"query={query}"]
    if variables:
        for k, v in variables.items():
            cmd += ["-f", f"{k}={v}"]
    import time
    for attempt in range(3):
        result = subprocess.run(cmd, capture_output=True, text=True)
        if result.returncode == 0:
            return json.loads(result.stdout)
        if "502" in result.stderr or "503" in result.stderr:
            wait = 2 ** attempt
            print(f"  Retrying in {wait}s (HTTP error)...", file=sys.stderr)
            time.sleep(wait)
            continue
        print(f"GraphQL query failed: {result.stderr.strip()}", file=sys.stderr)
        sys.exit(1)
    print(f"GraphQL query failed after 3 retries: {result.stderr.strip()}", file=sys.stderr)
    sys.exit(1)


def search_repos_graphql(count: int, existing_urls: set[str]) -> list[dict]:
    """Search GitHub for Ruby repos with Gemfiles using GraphQL.

    Each query fetches up to 100 repos with their default branch SHA and
    Gemfile presence in a single network round trip.
    """
    seen = set(existing_urls)
    results = []

    star_queries = [
        "stars:>2000",
        "stars:1000..2000",
        "stars:500..1000",
        "stars:200..500",
        "stars:100..200",
    ]

    for star_range in star_queries:
        if len(results) >= count:
            break
        cursor = None
        while len(results) < count:
            after_clause = f', after: "{cursor}"' if cursor else ""
            query = f"""{{
  search(query: "language:ruby {star_range} sort:stars-desc", type: REPOSITORY, first: 50{after_clause}) {{
    pageInfo {{ hasNextPage endCursor }}
    edges {{
      node {{
        ... on Repository {{
          nameWithOwner
          url
          isArchived
          stargazerCount
          defaultBranchRef {{
            target {{ oid }}
          }}
          gemfile: object(expression: "HEAD:Gemfile") {{
            ... on Blob {{ byteSize }}
          }}
        }}
      }}
    }}
  }}
}}"""
            data = graphql(query)
            search = data.get("data", {}).get("search", {})
            edges = search.get("edges", [])
            if not edges:
                break

            for edge in edges:
                node = edge.get("node", {})
                if not node:
                    continue
                slug = node.get("nameWithOwner", "")
                url = node.get("url", "")
                archived = node.get("isArchived", False)
                stars = node.get("stargazerCount", 0)
                branch_ref = node.get("defaultBranchRef")
                gemfile = node.get("gemfile")

                if archived:
                    continue
                if slug in DENYLIST:
                    print(f"  Skipping {slug} (denylisted)", file=sys.stderr)
                    continue
                normalized = url.rstrip("/").lower()
                if normalized in seen:
                    continue
                if not gemfile:
                    continue
                if not branch_ref:
                    continue

                sha = branch_ref["target"]["oid"]
                owner, repo = slug.split("/", 1)
                results.append({
                    "id": make_id(owner, repo, sha),
                    "repo_url": f"https://github.com/{slug}",
                    "sha": sha,
                    "source": "github_stars",
                    "set": "frozen",
                    "notes": f"auto-discovered, {stars} stars",
                })
                seen.add(normalized)
                if len(results) >= count:
                    break

            page_info = search.get("pageInfo", {})
            if not page_info.get("hasNextPage"):
                break
            cursor = page_info["endCursor"]

    return results[:count]


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
    if url.endswith(".git"):
        url = url[:-4]
    return url


def make_id(owner: str, repo: str, sha: str) -> str:
    return f"{owner}__{repo}__{sha[:7]}"


def add_specific_repo(url: str) -> dict | None:
    """Create a manifest entry for a specific repo URL using GraphQL."""
    url = normalize_repo_url(url)
    parts = url.rstrip("/").split("/")
    if len(parts) < 2:
        print(f"Cannot parse repo URL: {url}", file=sys.stderr)
        return None
    owner, repo = parts[-2], parts[-1]

    query = """{
  repository(owner: "%s", name: "%s") {
    isArchived
    defaultBranchRef {
      target { oid }
    }
    gemfile: object(expression: "HEAD:Gemfile") {
      ... on Blob { byteSize }
    }
  }
}""" % (owner, repo)

    data = graphql(query)
    repo_data = data.get("data", {}).get("repository")
    if not repo_data:
        print(f"  Repository not found: {owner}/{repo}", file=sys.stderr)
        return None

    branch_ref = repo_data.get("defaultBranchRef")
    if not branch_ref:
        print(f"  No default branch for {owner}/{repo}", file=sys.stderr)
        return None

    sha = branch_ref["target"]["oid"]
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
    new_entries: list[dict] = []

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
        print(f"Searching for top {args.count} new Ruby repos by stars...", file=sys.stderr)
        new_entries.extend(search_repos_graphql(args.count, seen_urls))
        print(f"Found {len(new_entries)} new repos with Gemfiles", file=sys.stderr)

    if not new_entries:
        print("\nNo new repos to add.", file=sys.stderr)
        return

    if args.dry_run:
        print(f"\nDry run: would add {len(new_entries)} repos:", file=sys.stderr)
        for e in new_entries:
            print(f"  {e['repo_url']} ({e['sha'][:7]}, {e['notes']})", file=sys.stderr)
    else:
        with open(MANIFEST_PATH, "a") as f:
            for e in new_entries:
                f.write(json.dumps(e) + "\n")
        total = len(existing) + len(new_entries)
        print(f"\nAdded {len(new_entries)} repos. Manifest now has {total} entries.", file=sys.stderr)


if __name__ == "__main__":
    main()
