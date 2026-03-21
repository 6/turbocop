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
import time
from pathlib import Path

MANIFEST_PATH = Path(__file__).parent / "manifest.jsonl"

# Repos excluded from the corpus.
# - Too large: RuboCop step alone exceeds the CI job timeout.
# - Not Ruby: repo is miscategorized or contains no meaningful Ruby source.
# - Broken: RuboCop crashes on the repo (e.g. malformed UTF-8).
DENYLIST = {
    "rapid7/metasploit-framework",
    "gitlabhq/gitlabhq",
    "aws/aws-sdk-ruby",
    "googleapis/google-api-ruby-client",
    "instructure/canvas-lms",
    "googleapis/google-cloud-ruby",
    "Shopify/shopify-api-ruby",
    "collabnix/kubelabs",  # Kubernetes tutorials, zero Ruby source (only vendored gems)
    "toretore/barby",  # RuboCop crashes on malformed UTF-8 in source files
    "illacceptanything/illacceptanything",  # Not a Ruby project
    "remote-jp/remote-in-japan",  # Markdown list, not Ruby source
    "syxanash/awesome-web-desktops",  # Markdown list, not Ruby source
    "lewagon/data-setup",  # Setup instructions, not Ruby source
    "raganwald-deprecated/homoiconic",  # Deprecated blog/essays, not Ruby source
    "fpsvogel/learn-ruby",  # Learning resource list, not Ruby source
    "hahwul/MobileHackersWeapons",  # Security tool list, not Ruby source
    "brunofacca/zen-rails-security-checklist",  # Markdown checklist, not Ruby source
    "hahwul/mad-metasploit",  # Bundler can't load RuboCop — zero usable oracle data
    "Shopify/ess",  # Layout/EndAlignment crash on vendored regression test file
    "rails-engine/script_core",  # Layout/EndAlignment crash on vendored regression test file
    "opencongress/opencongress",  # Invalid multibyte escape crashes RuboCop parser
    "airesis/airesis",  # RuboCop parser crash, 162 files dropped
    "smalruby/smalruby-editor",  # Ancient config format (no department given for LineLength)
    "pubnub/ruby",  # RuboCop parser crash, 46 files dropped
    "codeout/junoser",  # RuboCop parser crash, 32 files dropped
    "AliOsm/phlex-icons",  # RuboCop parser crash, 1580 files dropped
}


def graphql(query: str, variables: dict | None = None) -> dict:
    """Run a GraphQL query via `gh api graphql` with retry on transient errors."""
    cmd = ["gh", "api", "graphql", "-f", f"query={query}"]
    if variables:
        for k, v in variables.items():
            cmd += ["-f", f"{k}={v}"]
    for attempt in range(5):
        result = subprocess.run(cmd, capture_output=True, text=True)
        if result.returncode != 0:
            stderr_lower = result.stderr.lower()
            if "502" in result.stderr or "503" in result.stderr or "stream error" in stderr_lower or "cancel" in stderr_lower:
                wait = 2 ** attempt
                print(f"  Retrying in {wait}s (transient error)...", file=sys.stderr)
                time.sleep(wait)
                continue
            if result.returncode == 4 or "403" in result.stderr or "secondary rate limit" in stderr_lower:
                print(f"  Rate limited, sleeping 60s (attempt {attempt+1}/5)...", file=sys.stderr)
                time.sleep(60)
                continue
            print(f"GraphQL query failed: {result.stderr.strip()}", file=sys.stderr)
            sys.exit(1)
        data = json.loads(result.stdout)
        # Check for rate limit errors in the GraphQL response body
        errors = data.get("errors", [])
        if errors:
            error_msg = " ".join(e.get("message", "") for e in errors).lower()
            if "rate limit" in error_msg or "abuse" in error_msg:
                print(f"  Rate limited (GraphQL error), sleeping 60s (attempt {attempt+1}/5)...", file=sys.stderr)
                time.sleep(60)
                continue
        return data
    print(f"GraphQL query failed after 5 retries: {result.stderr.strip()}", file=sys.stderr)
    sys.exit(1)


def search_repos_graphql(count: int, existing_urls: set[str], min_stars: int = 50,
                         resume_path: Path | None = None) -> list[dict]:
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
        "stars:300..500",
        "stars:200..300",
        "stars:150..200",
        "stars:100..150",
        "stars:75..100",
        "stars:60..75",
        "stars:50..60",
    ]

    # Filter star ranges by min_stars (approximate: skip ranges whose lower bound < min_stars)
    def lower_bound(sq: str) -> int:
        """Extract the lower bound from a star range string."""
        if ">" in sq:
            return int(sq.split(">")[1])
        return int(sq.split(":")[1].split("..")[0])

    star_queries = [sq for sq in star_queries if lower_bound(sq) >= min_stars]

    for star_range in star_queries:
        if len(results) >= count:
            break
        print(f"  Searching {star_range}...", file=sys.stderr)
        cursor = None
        empty_pages = 0  # consecutive pages with no new repos
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
            time.sleep(2.0)
            search = data.get("data", {}).get("search", {})
            edges = search.get("edges", [])
            if not edges:
                break

            found_new = False
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
                entry = {
                    "id": make_id(owner, repo, sha),
                    "repo_url": f"https://github.com/{slug}",
                    "sha": sha,
                    "source": "github_stars",
                    "set": "frozen",
                    "notes": f"auto-discovered, {stars} stars",
                }
                results.append(entry)
                seen.add(normalized)
                found_new = True
                if resume_path is not None:
                    with open(resume_path, "a") as rf:
                        rf.write(json.dumps(entry) + "\n")
                if len(results) % 50 == 0:
                    print(f"  Progress: {len(results)} repos found", file=sys.stderr)
                if len(results) >= count:
                    break

            if found_new:
                empty_pages = 0
            else:
                empty_pages += 1
                if empty_pages >= 3:
                    print(f"  Skipping rest of {star_range} (3 consecutive pages with no new repos)", file=sys.stderr)
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
    parser.add_argument("--min-stars", type=int, default=50, help="Minimum star count for --stars mode")
    parser.add_argument("--output", type=str, default=None, help="Output manifest path (default: manifest.jsonl)")
    parser.add_argument("--resume", action="store_true", help="Write repos incrementally in --stars mode (survives Ctrl-C)")
    args = parser.parse_args()

    if not args.stars and not args.repo:
        parser.error("Specify --stars or --repo")

    output_path = Path(args.output) if args.output else MANIFEST_PATH

    # Load existing entries from the default manifest for dedup.
    # If --output points elsewhere, also load that file for dedup.
    existing = load_manifest()
    seen_urls = existing_repo_urls(existing)
    if args.output and output_path != MANIFEST_PATH and output_path.exists():
        output_entries = []
        for line in output_path.read_text().splitlines():
            line = line.strip()
            if line:
                output_entries.append(json.loads(line))
        seen_urls |= existing_repo_urls(output_entries)
        existing.extend(output_entries)

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
        print(f"Searching for top {args.count} new Ruby repos by stars (min {args.min_stars} stars)...", file=sys.stderr)
        resume_path = output_path if args.resume else None
        try:
            new_entries.extend(search_repos_graphql(args.count, seen_urls,
                                                    min_stars=args.min_stars,
                                                    resume_path=resume_path))
        except KeyboardInterrupt:
            print(f"\nInterrupted. {len(new_entries)} repos saved so far.", file=sys.stderr)
            if args.resume:
                print(f"Repos written to {output_path}", file=sys.stderr)
            return
        print(f"Found {len(new_entries)} new repos with Gemfiles", file=sys.stderr)

    if not new_entries:
        print("\nNo new repos to add.", file=sys.stderr)
        return

    if args.dry_run:
        print(f"\nDry run: would add {len(new_entries)} repos:", file=sys.stderr)
        for e in new_entries:
            print(f"  {e['repo_url']} ({e['sha'][:7]}, {e['notes']})", file=sys.stderr)
    elif not args.resume:
        # In resume mode, entries were already written incrementally
        with open(output_path, "a") as f:
            for e in new_entries:
                f.write(json.dumps(e) + "\n")
        total = len(existing) + len(new_entries)
        print(f"\nAdded {len(new_entries)} repos to {output_path}. Manifest now has {total} entries.", file=sys.stderr)
    else:
        total = len(existing) + len(new_entries)
        print(f"\nAdded {len(new_entries)} repos to {output_path} (resume mode). Manifest now has {total} entries.", file=sys.stderr)


if __name__ == "__main__":
    main()
