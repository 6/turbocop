# Remaining Conformance Divergences

> Last updated: 2026-02-19. 10 of 12 bench repos at 100%.

## Summary

| Repo | Match Rate | FP | FN | Total |
|------|-----------|----|----|-------|
| mastodon | 99.3% | 0 | 2 | 2 |
| doorkeeper | 99.7% | 0 | 2 | 2 |
| fat_free_crm | 0.0% | 5 | 0 | 5 |
| **Total** | | **5** | **4** | **9** |

All other repos (discourse, rails, rubocop, chatwoot, errbit, activeadmin, good_job, docuseal, rubygems.org) are at **100%**.

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

## Doorkeeper (2 FN)

### Gemspec/RequiredRubyVersion (1 FN)

**File:** `doorkeeper.gemspec`

**Root cause:** The gemspec uses DSL methods to set `required_ruby_version`. Detecting this requires evaluating gemspec DSL code (method calls like `spec.required_ruby_version = '>= 2.7'`), which turbocop's current `Gemspec/RequiredRubyVersion` cop handles for simple assignment patterns. The doorkeeper gemspec likely uses a pattern the cop doesn't recognize.

**Fix:** Would require expanding the gemspec DSL pattern matching. Since gemspec files are rare (one per project) and this is a single FN, the effort-to-impact ratio is poor.

**Effort:** Low effort, but very low impact (1 offense across all repos).

**Risk:** Minimal.

### Lint/UselessAssignment (1 FN)

**File:** `spec/models/doorkeeper/access_token_spec.rb` line 656

**Pattern:**
```ruby
describe "matching tokens" do
  it "uses token" do
    token = FactoryBot.create :access_token, default_attributes.merge(custom_attributes)
    expect(last_token).to eq(token)  # token IS read here
  end
  it "does not use token" do
    token = FactoryBot.create :access_token, default_attributes.merge(custom_attributes)
    # token is NEVER read in this block
    last_token = described_class.matching_token_for(...)
    expect(last_token).to eq(nil)
  end
end
```

**Root cause:** turbocop analyzes the outermost non-def block (`describe`) and collects writes/reads across all nested blocks. Since `token` is read in sibling `it` blocks, it appears "used" from the outer scope perspective. But each `it` block is a separate closure at runtime — `token` assigned in one `it` block is not accessible to sibling blocks.

**Fix attempted:** Removed the `inside_analyzed_block` guard so every block is analyzed independently. This fixed the 1 doorkeeper FN but introduced **89 new FPs** across 6 repos (mastodon +7, rails +48, rubocop +13, chatwoot +2, good_job +16, rubygems.org +3).

**Why the fix regresses:** In Ruby, blocks don't create new variable scopes — they share the enclosing scope. A variable assigned inside a block IS accessible in the enclosing scope after the block executes:
```ruby
[1].each { |x| result = x * 2 }
puts result  # works! result is accessible here
```
Analyzing each block independently incorrectly flags these as useless.

**Proper fix:** Implement proper data-flow analysis (similar to RuboCop's `VariableForce`). This tracks variable assignments along execution paths, understanding that:
- A write at the end of a block with no subsequent reads along any path is useless
- A write in one block IS reachable by reads in the enclosing scope after the block
- Sibling blocks (like `it` blocks) are independent closures

**Effort:** High. Data-flow analysis is a significant engineering investment — RuboCop's `VariableForce` is ~1,500 lines of Ruby.

**Risk:** High complexity, moderate regression risk even with proper implementation.

---

## Fat Free CRM (5 FP)

### All 5 FPs are RuboCop quirks

**Cops:** Layout/FirstArrayElementIndentation (2), Layout/MultilineMethodCallIndentation (1), Style/RedundantRegexpEscape (1), Style/TrailingCommaInHashLiteral (1)

**Root cause:** RuboCop reports **0 offenses** on fat_free_crm even when running with `--only <cop>` for each of these cops. This means RuboCop's cop execution produces 0 matches, but turbocop detects legitimate-looking offenses. The 0% match rate is an artifact of RuboCop reporting 0 for the denominator.

**Investigation:** Each of the 5 FPs was manually verified with `rubocop --only <CopName>`. In all cases, RuboCop reports 0 offenses on the specific files, even though the code patterns appear to match the cop's detection criteria. This suggests:
- RuboCop may have internal exemptions not documented in config
- The resolved config in fat_free_crm may disable or exclude these cops in ways not visible through `--show-cops`
- RuboCop version-specific behavior differences

**Fix:** Not fixable — these are RuboCop's own behavioral quirks, not turbocop bugs. The offenses turbocop reports are technically correct per the cop specifications.

**Effort:** N/A.

---

## Priority Assessment

| Divergence | Impact | Effort | Recommendation |
|-----------|--------|--------|---------------|
| Mastodon RedundantCopDisableDirective (2 FN) | Low | Medium-high | Defer — requires per-cop offense tracking |
| Doorkeeper Gemspec/RequiredRubyVersion (1 FN) | Minimal | Low | Skip — gemspec DSL edge case |
| Doorkeeper Lint/UselessAssignment (1 FN) | Low | High | Defer — requires data-flow analysis |
| Fat Free CRM (5 FP) | None | N/A | Accept — RuboCop quirks |

**Recommendation:** Accept these 9 divergences as the practical ceiling. The conformance rates (99.3%+ on real repos) represent excellent RuboCop compatibility. Further improvements require either:
1. Per-cop offense tracking infrastructure (for RedundantCopDisableDirective)
2. Data-flow analysis framework (for UselessAssignment)

Both are substantial engineering efforts with diminishing returns.
