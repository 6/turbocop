# rblint — Progress Tracker

See [PLAN.md](PLAN.md) for the full roadmap and technical design.

## Completed: M0 — Skeleton

Parse Ruby files with Prism, basic config loading, parallel cop execution framework.
All files compile, binary runs, produces "0 offenses detected."

### M0 Tasks

- [x] **Cargo.toml** — Dependencies, edition 2024, Prism C FFI build validated
- [x] **src/diagnostic.rs** — Diagnostic, Location, Severity types
- [x] **src/parse/source.rs** — SourceFile with byte-offset-to-line:col translation
- [x] **src/parse/mod.rs** — Prism parser wrapper (`parse_source()`)
- [x] **src/cop/mod.rs** — Cop trait + CopConfig
- [x] **src/cop/registry.rs** — CopRegistry (empty at M0, registration mechanism works)
- [x] **src/config/mod.rs** — Stub config loader (reads .rubocop.yml, no inheritance)
- [x] **src/fs/mod.rs** — File discovery with `ignore` crate, .gitignore + AllCops.Exclude
- [x] **src/linter.rs** — Linter orchestration, parallel cop execution via rayon
- [x] **src/formatter/text.rs** — RuboCop-compatible text output
- [x] **src/formatter/json.rs** — JSON formatter
- [x] **src/formatter/mod.rs** — Formatter trait + factory
- [x] **src/cli.rs** — Clap CLI args
- [x] **src/lib.rs** + **src/main.rs** — Entry point wiring
- [x] **Verify**: `cargo run -- .` parses .rb files, reports 0 offenses
- [x] Unit tests for SourceFile, config loading, file discovery, formatters

## Completed: M1 — Line-based cops + Test harness

8 line-based cops, annotation-based test harness, RuboCop git submodule for reference.

### M1 Tasks

- [x] **vendor/rubocop** — Git submodule (shallow clone) for reference specs
- [x] **src/testutil.rs** — Annotation-based fixture parser + assertion helpers
- [x] **src/cop/layout/** — 6 layout cops:
  - [x] Layout/TrailingWhitespace
  - [x] Layout/LineLength (configurable Max, default 120)
  - [x] Layout/TrailingEmptyLines
  - [x] Layout/LeadingEmptyLines
  - [x] Layout/EndOfLine
  - [x] Layout/InitialIndentation
- [x] **src/cop/style/** — 2 style cops:
  - [x] Style/FrozenStringLiteralComment (shebang/encoding-aware)
  - [x] Style/Tab
- [x] **src/cop/registry.rs** — `default_registry()` registers all 8 cops
- [x] **testdata/cops/** — Fixture files with offense/no_offense cases
- [x] **.gitattributes** — Whitespace preservation for test fixtures
- [x] 96 tests passing (43 from M0 + 53 new)

## Completed: M2 — Token / simple-pattern cops

17 new cops (25 total), CodeMap infrastructure, `check_source` cop method, CopWalker extraction.

### M2 Infrastructure

- [x] **src/cop/walker.rs** — Extracted CopWalker from linter.rs (shared by linter + test harness)
- [x] **src/parse/codemap.rs** — CodeMap: sorted non-code byte ranges for O(log n) `is_code()` lookups
- [x] **src/cop/mod.rs** — Added `check_source` method to Cop trait (default no-op)
- [x] **src/linter.rs** — Builds CodeMap per file, calls `check_source` between check_lines and check_node
- [x] **src/testutil.rs** — Full-pipeline test helpers: `run_cop_full`, `assert_cop_offenses_full`, etc.
- [x] **src/cop/lint/mod.rs** — New Lint department with `register_all()`

### M2 Cops — Layout (11 new)

- [x] Layout/EmptyLines — consecutive blank lines > Max (line-based)
- [x] Layout/SpaceAfterComma — check_source + CodeMap
- [x] Layout/SpaceAfterSemicolon — check_source + CodeMap
- [x] Layout/SpaceBeforeComma — check_source + CodeMap
- [x] Layout/SpaceAroundEqualsInParameterDefault — AST (OptionalParameterNode)
- [x] Layout/SpaceAfterColon — AST (AssocNode shorthand hash)
- [x] Layout/SpaceInsideParens — AST (ParenthesesNode)
- [x] Layout/SpaceInsideHashLiteralBraces — AST (HashNode, configurable)
- [x] Layout/SpaceInsideBlockBraces — AST (BlockNode)
- [x] Layout/SpaceInsideArrayLiteralBrackets — AST (ArrayNode)
- [x] Layout/SpaceBeforeBlockBraces — AST (BlockNode)

### M2 Cops — Lint (2 new)

- [x] Lint/Debugger — AST (CallNode: binding.pry, debugger, byebug, binding.irb)
- [x] Lint/LiteralAsCondition — AST (IfNode/WhileNode/UntilNode with literal predicate)

### M2 Cops — Style (4 new)

- [x] Style/StringLiterals — AST (StringNode, configurable EnforcedStyle)
- [x] Style/RedundantReturn — AST (DefNode last statement is ReturnNode)
- [x] Style/NumericLiterals — AST (IntegerNode, configurable MinDigits)
- [x] Style/Semicolon — check_source + CodeMap

### M2 Summary

- [x] **src/cop/registry.rs** — `default_registry()` registers all 25 cops
- [x] **testdata/cops/** — 34 new fixture files (offense + no_offense for each new cop)
- [x] 175 tests passing (159 unit + 16 integration)

## Completed: M3 — AST single-node cops

45 new AST-based cops (70 total), two new departments (Metrics, Naming), shared utilities module.

### M3 Infrastructure

- [x] **src/cop/util.rs** — Shared utilities: line counting, name case checks, trailing comma detection
- [x] **src/cop/metrics/mod.rs** — New Metrics department with `register_all()`
- [x] **src/cop/naming/mod.rs** — New Naming department with `register_all()`
- [x] **src/cop/mod.rs** — Added `pub mod metrics; pub mod naming; pub mod util;`
- [x] **src/cop/registry.rs** — Added `metrics::register_all` and `naming::register_all` to `default_registry()`

### M3 Cops — Metrics (8 new)

- [x] Metrics/MethodLength — DefNode, configurable Max (10), CountComments
- [x] Metrics/ClassLength — ClassNode, configurable Max (100), CountComments
- [x] Metrics/ModuleLength — ModuleNode, configurable Max (100), CountComments
- [x] Metrics/BlockLength — BlockNode, configurable Max (25), CountComments
- [x] Metrics/ParameterLists — DefNode, configurable Max (5), CountKeywordArgs
- [x] Metrics/AbcSize — DefNode, ABC score via subtree Visit, configurable Max (17)
- [x] Metrics/CyclomaticComplexity — DefNode, cyclomatic score via subtree Visit, configurable Max (7)
- [x] Metrics/PerceivedComplexity — DefNode, perceived score via subtree Visit, configurable Max (8)

### M3 Cops — Naming (8 new)

- [x] Naming/MethodName — DefNode, snake_case check
- [x] Naming/VariableName — LocalVariableWriteNode, snake_case check
- [x] Naming/ConstantName — ConstantWriteNode/ConstantPathWriteNode, SCREAMING_SNAKE_CASE check
- [x] Naming/ClassAndModuleCamelCase — ClassNode/ModuleNode, CamelCase check
- [x] Naming/AccessorMethodName — DefNode, flag get_/set_ prefix
- [x] Naming/PredicateName — DefNode, flag has_/is_ prefix
- [x] Naming/AsciiIdentifiers — DefNode/LocalVariableWriteNode, ASCII check
- [x] Naming/FileName — check_lines (path check), snake_case file path

### M3 Cops — Style (12 new)

- [x] Style/EmptyMethod — DefNode, empty body detection
- [x] Style/NegatedIf — IfNode, `!` predicate without else → use `unless`
- [x] Style/NegatedWhile — WhileNode, `!` predicate → use `until`
- [x] Style/ParenthesesAroundCondition — IfNode/WhileNode/UntilNode, parenthesized predicate
- [x] Style/IfUnlessModifier — IfNode, single-stmt body → modifier form
- [x] Style/WordArray — ArrayNode, all StringNodes → `%w[]`
- [x] Style/SymbolArray — ArrayNode, all SymbolNodes → `%i[]`
- [x] Style/TrailingCommaInArguments — CallNode, trailing comma in args
- [x] Style/TrailingCommaInArrayLiteral — ArrayNode, trailing comma
- [x] Style/TrailingCommaInHashLiteral — HashNode, trailing comma
- [x] Style/ClassAndModuleChildren — ClassNode/ModuleNode, compact vs nested style
- [x] Style/TernaryParentheses — IfNode (ternary), unnecessary parens

### M3 Cops — Lint (17 new)

- [x] Lint/EmptyConditionalBody — IfNode, empty body
- [x] Lint/EmptyWhen — WhenNode, empty body
- [x] Lint/BooleanSymbol — SymbolNode, `:true`/`:false`
- [x] Lint/DeprecatedClassMethods — CallNode, `File.exists?`/`Dir.exists?`
- [x] Lint/EnsureReturn — EnsureNode, ReturnNode in ensure body via Visit
- [x] Lint/FloatOutOfRange — FloatNode, parse source → `.is_infinite()`
- [x] Lint/Loop — WhileNode/UntilNode, `begin..end while` form
- [x] Lint/NestedMethodDefinition — DefNode, nested DefNode via Visit (skip class/module)
- [x] Lint/RaiseException — CallNode, `raise Exception` detection
- [x] Lint/SuppressedException — RescueNode, empty rescue body
- [x] Lint/UnifiedInteger — ConstantReadNode, `Fixnum`/`Bignum`
- [x] Lint/UriEscapeUnescape — CallNode, `URI.escape`/`URI.unescape`
- [x] Lint/UriRegexp — CallNode, `URI.regexp`
- [x] Lint/DuplicateCaseCondition — CaseNode, duplicate when condition source text
- [x] Lint/ElseLayout — IfNode, code on same line as `else`
- [x] Lint/RedundantStringCoercion — InterpolatedStringNode, `#{x.to_s}`
- [x] Lint/EachWithObjectArgument — CallNode, immutable literal argument

### M3 Summary

- [x] **src/cop/registry.rs** — `default_registry()` registers all 70 cops
- [x] **testdata/cops/** — ~90 new fixture files (offense + no_offense for each new cop)
- [x] 275 tests passing (259 unit + 16 integration)

## Completed: M4 — Performance cops + Test Hardening

47 new Performance department cops (117 total), rubocop-performance submodule, method chain utility, config variation tests, M3 integration tests.

### M4 Infrastructure

- [x] **vendor/rubocop-performance** — Git submodule (shallow clone) for reference specs
- [x] **src/cop/performance/mod.rs** — New Performance department with `register_all()`
- [x] **src/cop/mod.rs** — Added `pub mod performance;`
- [x] **src/cop/registry.rs** — Added `performance::register_all` to `default_registry()`
- [x] **src/cop/util.rs** — Added `MethodChain` struct and `as_method_chain()` helper for detecting `x.inner().outer()` patterns

### M4 Cops — Performance (39 new)

- [x] Performance/AncestorsInclude — ancestors.include?(X) → is_a?(X)
- [x] Performance/ArraySemiInfiniteRangeSlice — arr[n..] → arr.drop(n)
- [x] Performance/BigDecimalWithNumericArgument — BigDecimal(2) → BigDecimal('2')
- [x] Performance/BindCall — method(:bar).bind(obj).call → bind_call
- [x] Performance/BlockGivenWithExplicitBlock — block_given? with explicit &block param
- [x] Performance/Caller — caller[n] → caller(n..n).first
- [x] Performance/CaseWhenSplat — splat in when → move to end
- [x] Performance/Casecmp — downcase == → casecmp
- [x] Performance/ChainArrayAllocation — intermediate array allocation detection
- [x] Performance/CompareWithBlock — sort{|a,b| a.x <=> b.x} → sort_by
- [x] Performance/ConcurrentMonotonicTime — Concurrent.monotonic_time → Process.clock_gettime
- [x] Performance/Count — select{}.count → count{}
- [x] Performance/DeletePrefix — gsub(/\Aprefix/,'') → delete_prefix
- [x] Performance/DeleteSuffix — gsub(/suffix\z/,'') → delete_suffix
- [x] Performance/Detect — select{}.first → detect{}
- [x] Performance/DoubleStartEndWith — chained || start_with? → multi-arg
- [x] Performance/EndWith — match?(/foo\z/) → end_with?
- [x] Performance/FlatMap — map{}.flatten → flat_map{}
- [x] Performance/InefficientHashSearch — hash.keys.include? → hash.key?
- [x] Performance/IoReadlines — IO.readlines.each → IO.foreach
- [x] Performance/MapCompact — map{}.compact → filter_map{}
- [x] Performance/MapMethodChain — chained map calls
- [x] Performance/MethodObjectAsBlock — &method(:foo) → block
- [x] Performance/OpenStruct — flag OpenStruct usage
- [x] Performance/RangeInclude — Range#include? → Range#cover?
- [x] Performance/RedundantBlockCall — block.call → yield
- [x] Performance/RedundantEqualityComparisonBlock — select{|x| x == val}
- [x] Performance/RedundantMatch — match → match?
- [x] Performance/RedundantMerge — merge! single pair → []=
- [x] Performance/RedundantSortBlock — sort{|a,b| a <=> b} → sort
- [x] Performance/RedundantSplitRegexpArgument — split(/,/) → split(',')
- [x] Performance/RedundantStringChars — chars[n] → [n]
- [x] Performance/RegexpMatch — =~ → match?
- [x] Performance/ReverseEach — reverse.each → reverse_each
- [x] Performance/ReverseFirst — reverse.first → last.reverse
- [x] Performance/SelectMap — select{}.map{} → filter_map{}
- [x] Performance/Size — count → size
- [x] Performance/SortReverse — sort.reverse → sort with reversed block
- [x] Performance/Squeeze — gsub(/a+/,'a') → squeeze('a')
- [x] Performance/StartWith — match?(/\Afoo/) → start_with?
- [x] Performance/StringIdentifierArgument — send('foo') → send(:foo)
- [x] Performance/StringInclude — match?(/literal/) → include?
- [x] Performance/StringReplacement — gsub single char → tr
- [x] Performance/Sum — inject(0,:+) → sum
- [x] Performance/TimesMap — n.times.map{} → Array.new(n){}
- [x] Performance/UnfreezeString — String.new → unary plus
- [x] Performance/UriDefaultParser — URI.decode/encode → DEFAULT_PARSER

### M4 Test Hardening

- [x] Config variation tests for configurable cops (Metrics/MethodLength, ClassLength, CyclomaticComplexity, etc.)
- [x] M3 integration tests: metrics_cops_fire_on_complex_code, naming_cops_fire_on_bad_names, config_overrides_new_departments
- [x] SpaceInsideHashLiteralBraces no_space config test
- [x] Style/WordArray MinSize config test

### M4 Summary

- [x] **src/cop/registry.rs** — `default_registry()` registers all 117 cops
- [x] **testdata/cops/performance/** — ~78 new fixture files (offense + no_offense for each new cop)
- [x] 389 tests passing (370 unit + 19 integration)

## Completed: M5 — Complex core cops + Test Hardening

36 new cops (153 total), bug fix, integration tests, test coverage guard.

### M5 Bug Fixes

- [x] **Performance/Casecmp** — Fixed message to interpolate actual method (`upcase`/`downcase`) instead of hardcoding `downcase ==`

### M5 Infrastructure

- [x] **src/cop/util.rs** — Added `preceding_comment_line()`, `node_on_single_line()`, `expected_indent_for_body()`, `line_at()`, `indentation_of()` utilities
- [x] **tests/integration.rs** — Added `all_cops_have_fixture_files` test coverage guard, Performance/Lint/multi-department integration tests

### M5 Cops — Style (13 new)

- [x] Style/Documentation — ClassNode/ModuleNode, missing preceding comment
- [x] Style/Lambda — CallNode `lambda`, prefer stabby `->` syntax
- [x] Style/Proc — CallNode `Proc.new`, prefer `proc` keyword
- [x] Style/RaiseArgs — CallNode `raise`, separate args vs `.new()`
- [x] Style/RescueModifier — RescueModifierNode, flag inline `rescue`
- [x] Style/RescueStandardError — RescueNode, omit explicit `StandardError`
- [x] Style/SignalException — CallNode `raise`/`fail`, configurable preference
- [x] Style/SingleLineMethods — DefNode, flag single-line `def foo; bar; end`
- [x] Style/SpecialGlobalVars — GlobalVariableReadNode, Perl vars → English names
- [x] Style/StabbyLambdaParentheses — LambdaNode, parentheses around params
- [x] Style/YodaCondition — CallNode `==`/`!=`, flag literal-on-left
- [x] Style/HashSyntax — AssocNode, Ruby 1.9 hash syntax (configurable)
- [x] Style/MethodCallWithArgsParentheses — CallNode, require parens for args

### M5 Cops — Layout: EmptyLines family (6 new)

- [x] Layout/EmptyLineBetweenDefs — DefNode sequence, blank line between defs
- [x] Layout/EmptyLinesAroundClassBody — ClassNode, no blank after `class`/before `end`
- [x] Layout/EmptyLinesAroundModuleBody — ModuleNode, no blank after `module`/before `end`
- [x] Layout/EmptyLinesAroundMethodBody — DefNode, no blank after `def`/before `end`
- [x] Layout/EmptyLinesAroundBlockBody — BlockNode, no blank after `do`/`{`/before `end`/`}`
- [x] Layout/CaseIndentation — CaseNode, `when` aligned to `case`

### M5 Cops — Layout: Alignment (9 new)

- [x] Layout/ArgumentAlignment — CallNode, multi-line args alignment
- [x] Layout/ArrayAlignment — ArrayNode, multi-line elements alignment
- [x] Layout/HashAlignment — HashNode, multi-line pairs alignment
- [x] Layout/BlockAlignment — BlockNode, `end` aligned to block start
- [x] Layout/ConditionPosition — IfNode/WhileNode/UntilNode, condition on same line
- [x] Layout/DefEndAlignment — DefNode, `end` aligned to `def`
- [x] Layout/ElseAlignment — IfNode, `else`/`elsif` aligned to `if`
- [x] Layout/EndAlignment — ClassNode/ModuleNode/IfNode/etc., `end` aligned to keyword
- [x] Layout/RescueEnsureAlignment — BeginNode/DefNode, `rescue`/`ensure` alignment

### M5 Cops — Layout: Indentation (8 new)

- [x] Layout/IndentationWidth — Body indented by Width (default 2), configurable
- [x] Layout/IndentationConsistency — All statements at same depth use same indentation
- [x] Layout/FirstArgumentIndentation — First arg indentation relative to call
- [x] Layout/FirstArrayElementIndentation — First element indentation in arrays
- [x] Layout/FirstHashElementIndentation — First pair indentation in hashes
- [x] Layout/AssignmentIndentation — RHS indentation after `=`
- [x] Layout/MultilineMethodCallIndentation — Chained method calls indentation
- [x] Layout/MultilineOperationIndentation — Binary ops continuation line indent

### M5 Summary

- [x] **src/cop/registry.rs** — `default_registry()` registers all 153 cops
- [x] **testdata/cops/** — ~72 new fixture files (offense + no_offense for each new cop)
- [x] 513 tests passing (490 unit + 23 integration)
- [x] Test coverage guard: `all_cops_have_fixture_files` integration test

## Next: M6 — bin/lint + --rubocop-only

## Upcoming Milestones

| Milestone | Cops | Status |
|-----------|------|--------|
| **M0**: Skeleton | 0 | **Done** |
| **M1**: Line-based cops | 8 | **Done** |
| **M2**: Token/simple-pattern cops | 25 | **Done** |
| **M3**: AST single-node | 70 | **Done** |
| **M4**: Performance cops | 117 | **Done** |
| **M5**: Complex core cops | 153 | **Done** |
| **M6**: bin/lint + --rubocop-only | 0 new | Pending |
| **M7**: Autocorrect | +30 fixes | Pending |
| **M8**: rubocop-rspec | 80 | Pending |
| **M9**: rubocop-rails | 70 | Pending |
