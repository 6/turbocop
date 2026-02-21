# Autocorrect Validation Report

Validates that `turbocop -A` corrections are recognized as clean by `rubocop`.

## Autocorrect Validation

No offenses were corrected by turbocop across all repos. These repos are already clean for the 54 autocorrectable cops.

## Detection Gaps

Offenses rubocop finds that turbocop did not detect (not an autocorrect issue).

| Cop | Rubocop Offenses |
|-----|-----------------|
| Layout/EmptyLineAfterGuardClause | 283 |
| Layout/EmptyLineAfterMagicComment | 370 |
| Layout/EmptyLinesAfterModuleInclusion | 14 |
| Layout/EmptyLinesAroundAccessModifier | 3 |
| Layout/EmptyLinesAroundAttributeAccessor | 4 |
| Layout/EmptyLinesAroundClassBody | 99 |
| Layout/EmptyLinesAroundExceptionHandlingKeywords | 1 |
| Layout/EmptyLinesAroundMethodBody | 5 |
| Layout/EmptyLinesAroundModuleBody | 159 |
| Layout/LeadingCommentSpace | 5 |
| Layout/SpaceAfterNot | 1 |
| Layout/SpaceInsideArrayLiteralBrackets | 24 |
| Layout/SpaceInsideBlockBraces | 1 |
| Layout/SpaceInsideHashLiteralBraces | 1614 |
| Layout/SpaceInsideStringInterpolation | 8 |
| Layout/TrailingWhitespace | 2 |
| Style/FrozenStringLiteralComment | 1594 |

## Per-repo Details

### activeadmin

**Detection gaps:**

| Cop | Rubocop Offenses |
|-----|-----------------|
| Layout/EmptyLineAfterGuardClause | 17 |
| Layout/EmptyLineAfterMagicComment | 370 |
| Layout/EmptyLinesAfterModuleInclusion | 6 |
| Layout/EmptyLinesAroundAccessModifier | 3 |
| Layout/EmptyLinesAroundAttributeAccessor | 4 |
| Layout/EmptyLinesAroundClassBody | 99 |
| Layout/EmptyLinesAroundExceptionHandlingKeywords | 1 |
| Layout/EmptyLinesAroundMethodBody | 5 |
| Layout/EmptyLinesAroundModuleBody | 159 |
| Layout/LeadingCommentSpace | 5 |
| Layout/SpaceAfterNot | 1 |
| Layout/SpaceInsideArrayLiteralBrackets | 24 |
| Layout/SpaceInsideStringInterpolation | 8 |

### doorkeeper

**Detection gaps:**

| Cop | Rubocop Offenses |
|-----|-----------------|
| Layout/EmptyLineAfterGuardClause | 1 |
| Layout/EmptyLinesAfterModuleInclusion | 8 |
| Layout/SpaceInsideBlockBraces | 1 |

### errbit

**Detection gaps:**

| Cop | Rubocop Offenses |
|-----|-----------------|
| Layout/SpaceInsideHashLiteralBraces | 652 |

### lobsters

**Detection gaps:**

| Cop | Rubocop Offenses |
|-----|-----------------|
| Layout/EmptyLineAfterGuardClause | 47 |
| Layout/SpaceInsideHashLiteralBraces | 962 |
| Layout/TrailingWhitespace | 2 |
| Style/FrozenStringLiteralComment | 463 |

### rubygems.org

**Detection gaps:**

| Cop | Rubocop Offenses |
|-----|-----------------|
| Layout/EmptyLineAfterGuardClause | 218 |
| Style/FrozenStringLiteralComment | 1131 |

