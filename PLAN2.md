## turbocop Direction Plan

### Goals

1. **Fast path is always Rust-only.** No runtime Ruby fallback. Predictable speed and behavior.
2. **Correctness is per-cop, scoped.** turbocop guarantees RuboCop-equivalent behavior **only for cops it implements**, given the same *resolved* configuration.
3. **Full `.rubocop.yml` ecosystem compatibility is handled by a resolver step**, not by the linter runtime:

   * `inherit_from`, `inherit_gem`, `require`, plugin gems, custom cops are resolved by Ruby.
4. **Plugins/custom cops never block turbocop**: they become “external cops” that are reported, not executed.

Non-goals:

* “100% drop-in RuboCop replacement” (including arbitrary plugins/custom cops execution).
* Implementing bundler/gem resolution in Rust.
* Executing Ruby code in the hot lint loop.

---

## Architecture Overview

### Components

1. **Rust linter (existing)**

   * Reads either `.rubocop.yml` (limited mode) or a **resolved lockfile** (preferred).
   * Runs only implemented cops.
   * Emits RuboCop-style text or JSON.

2. **Ruby resolver helper (new)**

   * Runs RuboCop’s config machinery to compute the effective config.
   * Loads plugins and `require:` custom cops *only to resolve config and enabled cops*.
   * Produces a flattened “resolved config” + metadata in a lockfile.

3. **Lockfile format**

   * Single source of truth for:

     * resolved config (fully flattened)
     * which cops are enabled/disabled
     * which cops are external (plugin/custom)
     * versions (Ruby, RuboCop, plugin gems)
     * any warnings (unsupported features, missing gems)

---

## CLI Plan

### 1) `turbocop resolve` (new)

Purpose: produce `.turbocop/lock.json` and `.turbocop/resolved.yml` (or embed resolved config inside lock).

Example:

```bash
turbocop resolve                # uses .rubocop.yml discovery
turbocop resolve -c path/to/.rubocop.yml
turbocop resolve --ruby ruby3.3 # optional override (or just use current ruby)
```

Output artifacts:

* `.turbocop/lock.json`
* `.turbocop/resolved.yml` (or `.json`)

### 2) `turbocop` default behavior

Priority:

1. If `.turbocop/lock.json` exists and is fresh → use it.
2. Else:

   * either error with helpful hint (`run turbocop resolve`)
   * or run in “limited config mode” (see below)

### 3) Modes / strictness

Add `--mode` (or separate flags):

* `--mode=fast` (default)

  * runs supported cops only
  * external cops → warning summary (does not fail)

* `--mode=strict`

  * if any enabled cops are external → exit non-zero
  * intended for teams that require full coverage discipline

* `--mode=limited`

  * reads `.rubocop.yml` but only supports a subset (no inherit_gem/require)
  * meant as convenience for small repos; clearly documented as limited

### 4) Coverage / helper commands

* `turbocop external-cops` prints enabled-but-external cops (from lockfile), e.g.

  * `Minitest/AssertTruthy,RSpec/Focus,...`
* `turbocop doctor` prints:

  * unsupported config keys if in limited mode
  * missing lockfile / stale lockfile
  * external cops breakdown by source (plugin vs require)

---

## Lockfile Design

### Suggested `.turbocop/lock.json` structure

```json
{
  "schema_version": 1,
  "generated_at": "2026-02-15T12:34:56Z",
  "project_root": "/abs/path",
  "ruby": { "version": "3.3.1", "engine": "ruby" },
  "rubocop": { "version": "1.63.0" },
  "plugins": [
    { "name": "rubocop-rails", "version": "2.24.0" },
    { "name": "rubocop-rspec", "version": "2.27.0" },
    { "name": "rubocop-minitest", "version": "0.35.0" }
  ],
  "config_sources": {
    "primary": ".rubocop.yml",
    "resolved_path": ".turbocop/resolved.yml",
    "resolution_hash": "sha256:..."
  },
  "enabled_cops": ["Layout/LineLength", "..."],
  "external_cops": [
    {
      "cop": "Minitest/AssertTruthy",
      "source": { "type": "plugin", "name": "rubocop-minitest" }
    },
    {
      "cop": "Custom/FooBar",
      "source": { "type": "require", "path": "lib/rubocop/cop/custom/foo_bar.rb" }
    }
  ],
  "warnings": [
    { "kind": "unsupported_runtime", "message": "limited mode ignores inherit_gem; run turbocop resolve" }
  ]
}
```

### Resolved config file `.turbocop/resolved.yml`

This should be fully flattened:

* no `inherit_from`
* no `inherit_gem`
* no `require`
* all effective `Enabled`, `Exclude/Include`, `AllCops` merged
* cop options final

---

## Ruby Resolver Implementation

### Approach

Ship a small Ruby script (e.g. `tools/resolve.rb`) invoked by `turbocop resolve`.

Responsibilities:

1. Discover `.rubocop.yml` using RuboCop’s own discovery (or mimic your Rust discovery).

2. Ask RuboCop to load and resolve configuration exactly as it would when running.

3. Capture:

   * resolved config hash
   * list of enabled cops
   * list of all cops RuboCop knows about in this environment
   * (optional) which cops originate from which plugin (heuristic; see below)

4. Write:

   * `.turbocop/resolved.yml`
   * `.turbocop/lock.json`

### Handling plugins like `rubocop-minitest`

Example repo config:

```yml
require:
  - rubocop-minitest

Minitest/AssertTruthy:
  Enabled: true
```

Resolver behavior:

* RuboCop loads `rubocop-minitest` so the cop class exists.
* Enabled cops list includes `Minitest/*`.
* turbocop compares enabled cops to its implemented set → all `Minitest/*` become **external**.

### Handling handwritten custom cops

Example:

```yml
require:
  - ./lib/rubocop/cop/custom/foo_bar

Custom/FooBar:
  Enabled: true
```

Resolver behavior:

* RuboCop loads local Ruby file
* Cop appears in enabled list
* turbocop marks it external with `source.type=require` and path.

### “Which plugin provided this cop?” (nice-to-have)

If RuboCop exposes cop class source locations, you can map:

* plugin gem path → plugin name (via `Gem::Specification.find_by_name`)
* local path → require path

If mapping is hard, you can still record external cops without source attribution; source attribution is UX sugar.

---

## Rust Runtime Changes

### 1) Implement “lockfile-driven” config path

* If lockfile exists, load `.turbocop/resolved.yml` and enabled cops list.
* Skip all inheritance logic at runtime.

### 2) Implement external cop reporting

* Determine enabled cops from lockfile.
* Partition into:

  * implemented cops
  * external cops

Then:

* `fast` mode: warn summary
* `strict` mode: error and exit non-zero

### 3) Keep existing `.rubocop.yml` loader, but reposition it

Treat it as:

* used only for `limited` mode, small repos, or bootstrapping
* not the canonical “works everywhere” path

**Important:** runtime must not attempt `inherit_gem` or `require`.

### 4) Make correctness claims testable

Update internal tests to use the lockfile/resolved config, not raw `.rubocop.yml`.

---

## Testing Plan for Correctness (Per Cop)

### Conformance tests per cop

For each cop (or cop group):

1. Fixture repo / files
2. Resolved config (generated by resolver; committed to repo fixtures)
3. Run RuboCop with JSON formatter to produce expected offenses
4. Run turbocop with same resolved config → diff results

Store golden JSON output:

* `tests/fixtures/<cop>/expected_rubocop.json`
* `tests/fixtures/<cop>/source.rb`
* `tests/fixtures/<cop>/resolved.yml`

### Tests involving external cops

Create fixtures that enable plugin/custom cops and validate:

* turbocop runs its implemented cops
* external cops are listed correctly

Example fixture A (plugin):

* config requires `rubocop-minitest`
* enables `Minitest/AssertTruthy`
  Expected:
* `external_cops` includes `Minitest/AssertTruthy`
* no attempt to execute it

Example fixture B (handwritten):

* config requires `./lib/rubocop/cop/custom/foo_bar`
* enables `Custom/FooBar`
  Expected:
* external list includes `Custom/FooBar` with require-path attribution

---

## User-Facing Behavior

### Expected user workflow

* For real Rails repos with inheritance/plugins:

  1. `turbocop resolve`
  2. `turbocop` (fast)

* For minimal repos:

  * `turbocop --mode=limited` (optional convenience)

### Error/warn messaging (important)

When external cops exist:

* print counts and names
* show how to list them and how to enforce strictness

Example:

```
turbocop: 312 enabled cops, 287 supported, 25 external
External cops (not run): Minitest/AssertTruthy, Custom/FooBar, ...
Tip: `turbocop external-cops` to list all. Use `--mode=strict` to fail CI.
```

---

## Migration of Current README Claims

Update positioning so it can’t be misconstrued as “drop-in replacement”:

* Replace “targeting drop-in RuboCop compatibility” with:

  * “RuboCop-compatible rules and config (via resolved lockfile), fast Rust-only execution.”
* Explicitly state:

  * “Plugins/custom cops are resolved but not executed.”

---

## Implementation Steps Order

1. **Lockfile reader + resolved-config runner in Rust**
2. **Ruby resolver script** that writes lockfile + resolved config
3. **External cop partitioning + modes** (fast/strict)
4. **Doctor / external-cops CLI sugar**
5. **Convert conformance tests** to the resolved-config harness
6. Optional: support `inherit_from` locally in limited mode (nice but not required if resolve is smooth)
