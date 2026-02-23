# Corpus Oracle

CI-based conformance testing against 100 real-world Ruby repositories. Runs turbocop and RuboCop on the same code with a shared baseline config, then diffs the results to find false positives and false negatives.

Triggered weekly and on PRs that change cop implementations. See `.github/workflows/corpus-oracle.yml`.

## Files

- `manifest.jsonl` — curated list of repos (id, URL, pinned SHA, notes)
- `baseline_rubocop.yml` — shared RuboCop config used for both tools (all plugins, all defaults)
- `Gemfile` — baseline gem versions matching vendor submodule tags
- `add_repos.py` — discover and add repos from GitHub by stars
- `diff_results.py` — diff turbocop vs RuboCop JSON output into a report

## Adding repos

```
python3 corpus/add_repos.py --stars --count 50        # top Ruby repos by stars
python3 corpus/add_repos.py --repo https://github.com/org/repo
python3 corpus/add_repos.py --stars --count 50 --dry-run
```
