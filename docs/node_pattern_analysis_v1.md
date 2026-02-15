# ANALYSIS SUMMARY

## 1. OVERALL PATTERN MATCHER USAGE ACROSS ALL VENDORS

Total cops across all vendors: 1,098
Cops using def_node_matcher or def_node_search: 499 (45.4%)

### Breakdown by Vendor + Department:

RUBOCOP (769 total cops):
  - layout:      4 / 100 (4%)
  - style:     122 / 290 (42%)
  - lint:       64 / 153 (42%)
  - metrics:     4 / 15 (27%)
  - naming:      6 / 19 (32%)
  - bundler:     5 / 7 (71%)
  - gemspec:     7 / 10 (70%)
  - security:    6 / 7 (86%)
  - migration:   0 / 1 (0%)
  Subtotal:    218 / 769 (28%)

RUBOCOP-RSPEC (126 total cops):
  - rspec:      85 / 125 (68%)
  Subtotal:    85 / 125 (68%)

RUBOCOP-RAILS (148 total cops):
  - rails:      99 / 138 (72%)
  - mixin:       6 / 9 (67%)
  Subtotal:    105 / 148 (71%)

RUBOCOP-PERFORMANCE (55 total cops):
  - performance: 46 / 52 (88%)
  - mixin:       1 / 2 (50%)
  Subtotal:     47 / 55 (85%)

## 2. PATTERN-TO-IMPERATIVE RATIO FOR 5-6 HEAVILY PATTERN-BASED COPS

### ReverseFind (Style, 51 lines total)
Pattern content: 5 lines (def_node_matcher definition)
Imperative logic: 17 lines (on_send handler + alias + range calculation)
**Ratio: ~23% pattern, 77% imperative**
Note: Minimal imperative code - pattern does 100% of detection, imperative only does offset calculation.

### StderrPuts (Style, 57 lines total)
Pattern content: 4 lines (def_node_matcher with predicate)
Imperative logic: 19 lines (on_send + private methods + message formatting + range calculation)
**Ratio: ~18% pattern, 82% imperative**
Note: Pattern handles detection, imperative handles message formatting and range extraction.

### Eval (Security, 34 lines total)
Pattern content: 2 lines (def_node_matcher)
Imperative logic: 8 lines (on_send with conditional + dstr check)
**Ratio: ~13% pattern, 87% imperative**
Note: Pattern is very concise, imperative adds dstr validation.

### RedundantSort (Style, 210 lines total)
Pattern content: 14 lines (def_node_matcher with complex alternatives)
Imperative logic: 125 lines (15 private methods for message, correction, suggestion logic)
**Ratio: ~7% pattern, 93% imperative**
Note: Pattern handles multiple variations; imperative handles complex autocorrect logic.

### ZeroLengthPredicate (Style, 154 lines total)
Pattern content: 21 lines (4 def_node_matcher calls with detailed patterns)
Imperative logic: 100 lines (3 private check methods + replacement logic + helper matchers)
**Ratio: ~14% pattern, 86% imperative**
Note: Multiple patterns for different comparison scenarios; heavy post-match imperative processing.

### IndexBy (Rails, 90 lines total)
Pattern content: 56 lines (4 def_node_matcher definitions with comprehensive block/numblock/itblock variations)
Imperative logic: 6 lines (new_method_name override + includes mixin)
**Ratio: ~62% pattern, 38% imperative**
Note: HEAVILY PATTERN-BASED - Patterns enumerate all block types; imperative logic delegates to mixin.

### SubjectStub (RSpec, 156 lines total)
Pattern content: 24 lines (3 def_node_matcher + 1 def_node_search)
Imperative logic: 90 lines (on_top_level_group + 3 complex private traversal methods)
**Ratio: ~15% pattern, 85% imperative**
Note: Patterns are simple; imperative does sophisticated ancestor/scope tracking.

### StringReplacement (Performance, 162 lines total)
Pattern content: 4 lines (def_node_matcher with simple alternatives)
Imperative logic: 120 lines (9 private methods for regex handling, escaping, method selection)
**Ratio: ~2% pattern, 98% imperative**
Note: Pattern is minimal; almost ALL logic is imperative (regex analysis, escape interpretation).

## 3. NodePattern Unique Node Types

Total unique AST node types referenced in patterns: 52

List of all node types:
alias, and, any_block, arg, args, array, begin, block, block_pass, call, case, casgn, 
class, const, csend, def, defs, dstr, erange, float, gvar, gvasgn, hash, if, int, irange, 
itblock, ivasgn, kwbegin, kwsplat, lvar, lvasgn, match, mlhs, numblock, op, or, or_asgn, 
pair, range, rational, regexp, regopt, resbody, rescue, sclass, send, splat, str, sym, true, when

## 4. COPS WHERE PATTERN IS ESSENTIALLY THE ENTIRE DETECTION LOGIC

Very common pattern (~15-20% of pattern-using cops):
- ReverseFind: Pattern does 100% detection, imperative only handles reporting details
- StderrPuts: Pattern does 100% detection, imperative only handles message + range
- Eval: Pattern does 100% detection, imperative adds one conditional dstr check
- ArrayJoin: Pattern does 100% detection, imperative handles autocorrect

These cops tend to be:
- 30-60 lines total
- Security/simple style checks
- One or two clear patterns
- Minimal domain-specific post-processing

## 5. KEY INSIGHTS

1. **Higher-level cops use patterns more heavily** (72-88% for Rails/RSpec/Performance)
2. **Lower-level structural cops use patterns less** (4-42% for Layout/Style/Lint)
3. **Total pattern matcher count: 939 def_node_matcher/search calls across 499 cops** (~1.9 per cop)
4. **Pattern-dominant (>50% pattern) cops are relatively rare** (~5-10% of pattern-using cops)
5. **Most cops use patterns for detection, imperative for post-processing** (average ~80-85% imperative logic)
6. **52 unique AST node types cover all linting needs** - surprisingly small surface area
7. **Pattern matchers with guards are most common** - patterns rarely stand alone completely
