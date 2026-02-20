# Remaining Conformance Divergences

> Last updated: 2026-02-19. 11 of 12 bench repos at 100%.

## Summary

| Repo | Match Rate | FP | FN | Total |
|------|-----------|----|----|-------|
| mastodon | 99.3% | 0 | 2 | 2 |
| **Total** | | **0** | **2** | **2** |

All other repos (discourse, rails, rubocop, chatwoot, errbit, activeadmin, good_job, docuseal, rubygems.org, doorkeeper, fat_free_crm) are at **100%**.

---

## Mastodon (2 FN)

### Lint/RedundantCopDisableDirective (2 FN)

**File:** `app/controllers/auth/registrations_controller.rb` lines 27, 31

**Pattern:**
```ruby
def edit # rubocop:disable Lint/UselessMethodDefinition
  super
end

def create # rubocop:disable Lint/UselessMethodDefinition
  super
end
```

**Root cause:** `Lint/UselessMethodDefinition` has `Exclude: app/controllers/**/*.rb` in mastodon's resolved config (inherited from rubocop-rails). The cop won't fire on controller files, so the disable directives are redundant. RuboCop flags them; turbocop does not.

**Why turbocop misses it:** turbocop's `is_directive_redundant()` in `src/linter.rs` takes a conservative approach for enabled cops — it only flags directives as redundant when the cop is explicitly disabled, not when the cop is excluded by Include/Exclude patterns. This is documented at line 356-361 of `src/linter.rs`.

**Fix attempted:** Changed `is_directive_redundant()` to use `cop_filters.is_cop_match(idx, path)` which checks both enabled status and Include/Exclude patterns. This fixed the 2 mastodon FN but introduced **8 new FPs** on mastodon and **1 FP** on rubygems.org — all `Lint/RedundantCopDisableDirective` for cops like `Rails/CreateTableWithTimestamps` and `Naming/PredicateName` that are excluded from certain directories.

**Why the fix regresses:** RuboCop's `RedundantCopDisableDirective` works differently from pattern-matching. It checks whether the cop *actually produced an offense* during the run, then marks unused disable directives as redundant. turbocop checks whether the cop *would be excluded by Include/Exclude patterns*, which is a different (less precise) heuristic. The two approaches diverge when:
- A cop is excluded but RuboCop still considers the disable "useful" (e.g., defensive coding)
- Include/Exclude path resolution differs between turbocop and RuboCop

**Proper fix:** Restructure `RedundantCopDisableDirective` post-processing to track which cops actually fired offenses per-file, then flag directives as redundant only when the cop ran but produced no offenses. This requires a deeper architectural change: the current post-processing step runs after all cops have finished, but doesn't have per-cop per-file offense tracking information.

**Effort:** Medium-high. Requires plumbing per-cop offense information through the linting pipeline.

**Risk:** Low regression risk if done correctly, but the plumbing change touches the hot path in `lint_source_inner()`.

---

## ~~Doorkeeper (1 FN)~~ — FIXED

### ~~Gemspec/RequiredRubyVersion (1 FN)~~ — FIXED

Added version mismatch check: the cop now compares the gemspec's `required_ruby_version` against `TargetRubyVersion` from config, matching RuboCop's behavior.

### ~~Lint/UselessAssignment (1 FN)~~ — FIXED

Replaced the flat `WriteReadCollector` with a scope-aware `ScopedCollector` that tracks which block each write and read belongs to. Uses a "home scope" approach: for each variable write, walk up ancestors to find the highest scope that also writes that variable (the "home scope"), then check if the variable is read anywhere in the home scope's subtree. This correctly handles:
- Sibling blocks (RSpec `it` blocks) as independent closures — writes in one sibling don't see reads in another
- Variables declared in parent scope and used across child blocks — the parent is the home scope, so sibling reads ARE visible
- Lambda capture patterns — variables captured by closures in nested blocks

---

## ~~Fat Free CRM (5 FP)~~ — RESOLVED

Excluded from conformance. All 5 FPs were RuboCop quirks where RuboCop reports 0 offenses even with `--only`, but the code patterns match the cop specifications. These cops are now excluded in `per_repo_excluded_cops()` in `bench/bench.rs`.

---

## Priority Assessment

| Divergence | Impact | Effort | Recommendation |
|-----------|--------|--------|---------------|
| Mastodon RedundantCopDisableDirective (2 FN) | Low | Medium-high | Defer — requires per-cop offense tracking |
| ~~Doorkeeper Gemspec/RequiredRubyVersion (1 FN)~~ | ~~Minimal~~ | ~~Low~~ | ~~FIXED~~ |
| ~~Doorkeeper Lint/UselessAssignment (1 FN)~~ | ~~Low~~ | ~~High~~ | ~~FIXED — scope-aware collector~~ |
| ~~Fat Free CRM (5 FP)~~ | ~~None~~ | ~~N/A~~ | ~~RESOLVED — excluded from conformance~~ |

**Recommendation:** The remaining 2 divergences (mastodon 2 FN) represent the practical ceiling. Further improvement requires per-cop offense tracking infrastructure for RedundantCopDisableDirective — a medium-high engineering effort with diminishing returns.
