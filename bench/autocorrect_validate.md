# Autocorrect Validation Report

Validates that `turbocop -A` corrections are recognized as clean by `rubocop`.

## Autocorrect Validation

| Cop | Corrected | Remaining | Status |
|-----|-----------|-----------|--------|
| Layout/EmptyLineAfterGuardClause | 11 | 1 | FAIL |
| Layout/EmptyLinesAfterModuleInclusion | 64 | 0 | PASS |

**1/2 cops passing** (0 remaining offenses after correction)

## Detection Gaps

Offenses rubocop finds that turbocop did not detect (not an autocorrect issue).

| Cop | Rubocop Offenses |
|-----|-----------------|
| Layout/SpaceInsideBlockBraces | 1 |

## Per-repo Details

### chatwoot

**Autocorrect validation:**

| Cop | Corrected | Remaining | Status |
|-----|-----------|-----------|--------|
| Layout/EmptyLinesAfterModuleInclusion | 55 | 0 | PASS |

### doorkeeper

**Detection gaps:**

| Cop | Rubocop Offenses |
|-----|-----------------|
| Layout/EmptyLineAfterGuardClause | 1 |
| Layout/SpaceInsideBlockBraces | 1 |

### errbit

**Autocorrect validation:**

| Cop | Corrected | Remaining | Status |
|-----|-----------|-----------|--------|
| Layout/EmptyLineAfterGuardClause | 11 | 0 | PASS |
| Layout/EmptyLinesAfterModuleInclusion | 1 | 0 | PASS |

### mastodon

**Autocorrect validation:**

| Cop | Corrected | Remaining | Status |
|-----|-----------|-----------|--------|
| Layout/EmptyLinesAfterModuleInclusion | 8 | 0 | PASS |

