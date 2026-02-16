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

## Completed: M6 — rubocop-rails (98 cops) + Include/Exclude Infrastructure

98 new Rails department cops (251 total), Include/Exclude path pattern enforcement, Cop trait default_include/default_exclude.

### M6 Infrastructure

- [x] **src/config/mod.rs** — `is_cop_enabled()` now enforces Include/Exclude glob patterns using `globset`
- [x] **src/config/mod.rs** — `glob_matches()` helper for RuboCop-style pattern matching
- [x] **src/config/mod.rs** — Global excludes (AllCops.Exclude) now applied in path filtering
- [x] **src/cop/mod.rs** — Added `default_include()` and `default_exclude()` methods to Cop trait
- [x] **src/linter.rs** — Passes cop's default include/exclude to `is_cop_enabled()`
- [x] **src/cop/rails/mod.rs** — New Rails department with `register_all()`
- [x] **src/cop/util.rs** — Added Rails-specific helpers: `parent_class_name`, `is_dsl_call`, `class_body_calls`, `has_keyword_arg`, `keyword_arg_value`, `constant_name`, `full_constant_path`, `MethodChain3`, `as_method_chain3`

### M6 Cops — Rails (98 cops)

<details><summary>Full list (98 cops)</summary>

Rails/Exit, Rails/Output, Rails/OutputSafety, Rails/Blank, Rails/Present, Rails/NegateInclude, Rails/SafeNavigation, Rails/Delegate, Rails/DelegateAllowBlank, Rails/RequestReferer, Rails/RefuteMethods, Rails/ToFormattedS, Rails/ToSWithArgument, Rails/StripHeredoc, Rails/Inquiry, Rails/PluckId, Rails/Pluck, Rails/IndexBy, Rails/WhereNot, Rails/WhereExists, Rails/PluckInWhere, Rails/ActiveSupportAliases, Rails/FindBy, Rails/FindEach, Rails/CompactBlank, Rails/Pick, Rails/ContentTag, Rails/SelectMap, Rails/RootJoinChain, Rails/RootPathnameMethods, Rails/RootPublicPath, Rails/FilePath, Rails/ResponseParsedBody, Rails/RedundantActiveRecordAllMethod, Rails/HelperInstanceVariable, Rails/UnusedRenderContent, Rails/RedundantTravelBack, Rails/DurationArithmetic, Rails/ExpandedDateRange, Rails/WhereRange, Rails/Date, Rails/TimeZone, Rails/RelativeDateConstant, Rails/FreezeTime, Rails/TimeZoneAssignment, Rails/DotSeparatedKeys, Rails/ShortI18n, Rails/ApplicationController, Rails/ApplicationRecord, Rails/ApplicationJob, Rails/ApplicationMailer, Rails/HasManyOrHasOneDependent, Rails/InverseOf, Rails/HasAndBelongsToMany, Rails/DuplicateAssociation, Rails/DuplicateScope, Rails/ReadWriteAttribute, Rails/ActionOrder, Rails/ActiveRecordCallbacksOrder, Rails/ActionControllerTestCase, Rails/LexicallyScopedActionFilter, Rails/ActionControllerFlashBeforeRender, Rails/Validation, Rails/RedundantAllowNil, Rails/RedundantPresenceValidationOnBelongsTo, Rails/AttributeDefaultBlockValue, Rails/EnvironmentComparison, Rails/EnvironmentVariableAccess, Rails/Env, Rails/UnknownEnv, Rails/RakeEnvironment, Rails/EnvLocal, Rails/I18nLocaleAssignment, Rails/I18nLazyLookup, Rails/CreateTableWithTimestamps, Rails/NotNullColumn, Rails/ThreeStateBooleanColumn, Rails/DangerousColumnNames, Rails/AddColumnIndex, Rails/MigrationClassName, Rails/SchemaComment, Rails/EnumHash, Rails/EnumSyntax, Rails/EnumUniqueness, Rails/HttpPositionalArguments, Rails/HttpStatus, Rails/HttpStatusNameConsistency, Rails/DynamicFindBy, Rails/SkipsModelValidations, Rails/ScopeArgs, Rails/ReflectionClassName, Rails/RedundantForeignKey, Rails/RenderInline, Rails/RenderPlainText, Rails/ReversibleMigrationMethodDefinition, Rails/TableNameAssignment, Rails/AfterCommitOverride, Rails/TransactionExitStatement

</details>

### M6 Summary

- [x] **src/cop/registry.rs** — `default_registry()` registers all 251 cops
- [x] 714 tests passing (691 unit + 23 integration)

## Completed: M8 — rubocop-rspec (113 cops)

113 new RSpec department cops (364 total), vendor fixture extraction from rubocop-rspec specs, RSpec DSL utility helpers.

### M8 Summary

- [x] **src/cop/rspec/** — 113 new cop source files
- [x] 971 tests passing (944 unit + 27 integration)

## Completed: M10 — Production Readiness (Config Inheritance + CLI)

Config inheritance (`inherit_from` + `inherit_gem`), `--rubocop-only` flag, `--stdin` support.

### M10 Summary

- [x] Config inheritance: `inherit_from` (local files, recursive) + `inherit_gem` (bundle info)
- [x] `--rubocop-only`: enables hybrid `bin/lint` workflow
- [x] `--stdin PATH`: enables editor integration (VSCode, vim, Emacs)

## Completed: M11 — Drop-in RuboCop Config Compatibility

Full `.rubocop.yml` compatibility: auto-discovery, `require:` plugin default loading, department-level configs, `Enabled: pending` / `AllCops.NewCops`, `AllCops.DisabledByDefault`, and `inherit_mode`.

## Completed: Config Audit + Prism Pitfalls — Zero Gaps

Eliminated all config audit (126 → 0) and prism pitfalls (68 → 0) gaps.

## Completed: Conformance FP Reduction — 469 cops

Systematic false-positive reduction across all bench repos. Fixed 11 cops across Layout, Style, and Security departments. Reached 100% conformance on Discourse and Rails. Mastodon at 60.4% (51 FPs, 57 FNs remaining).

Key fixes:
- CommentIndentation: Rewrote to use Prism comments() for proper string/regex/heredoc detection
- RedundantCondition: Skip multi-line else branches
- RedundantPercentQ: Skip %q with interpolation patterns, accept %Q with double quotes
- Security/Open: Only flag dynamic args, exclude URI.open
- Security/MarshalLoad: Exclude Marshal.load(Marshal.dump(...)) pattern
- HashTransformValues: Skip when value references key variable
- SpaceAroundOperators: Skip safe navigation operators
- SpaceBeforeFirstArg: Skip operator and setter methods
- EmptyLinesAroundAccessModifier: Handle block body boundaries
- EmptyLineAfterMagicComment: Recognize # coding: as magic comment

## Completed: M9 — Core Cop Coverage Expansion (469 cops)

Added 105 new cops across 6 departments. Three new departments created from scratch (Security, Bundler, Gemspec, Migration). Six departments now at 100% coverage.

### M9 New Departments

- [x] **Security** (7/7 = 100%): Eval, JSONLoad, YAMLLoad, MarshalLoad, Open, IoMethods, CompoundHash
- [x] **Bundler** (7/7 = 100%): DuplicatedGem, DuplicatedGroup, GemComment, GemFilename, GemVersion, InsecureProtocolSource, OrderedGems
- [x] **Gemspec** (10/10 = 100%): AddRuntimeDependency, AttributeAssignment, DependencyVersion, DeprecatedAttributeAssignment, DevelopmentDependencies, DuplicatedAssignment, OrderedDependencies, RequireMFA, RequiredRubyVersion, RubyVersionGlobalsUsage
- [x] **Migration** (1/1 = 100%): DepartmentName

### M9 Department Expansions

- [x] **Metrics** — 10/10 = 100% (+2: BlockNesting, CollectionLiteralLength)
- [x] **Naming** — 19/19 = 100% (+11: BinaryOperatorParameterName, BlockForwarding, BlockParameterName, HeredocDelimiterCase, HeredocDelimiterNaming, InclusiveLanguage, MemoizedInstanceVariableName, MethodParameterName, PredicateMethod, RescuedExceptionsVariableName, VariableNumber)
- [x] **Lint** — 53/152 = 35% (+34 new cops)
- [x] **Style** — 56/287 = 20% (+25 new cops)
- [x] **Layout** — 48/100 = 48% (+8 new cops)

### M9 FP Reduction (22 cops fixed)

Systematic FP elimination on the 105 new cops, reducing total FPs from 7,190 to 207:

| Repo | Before | After | Match Rate |
|------|-------:|------:|---------:|
| **Mastodon** | 1,229 | 76 | 55.4% |
| **Discourse** | 2,046 | 8 | **98.1%** |
| **Rails** | 3,915 | 123 | 2.4% |

Key fixes:
- Lint/RequireParentheses: Full AST rewrite (eliminated ~2,972 FPs)
- Layout cops: AllowForAlignment, `<=>` operator, `def ==` method names, `yield()`, RDoc comments
- Style cops: frozen_string_literal awareness, EnforcedStyle conditionals, single-line def handling
- Lint cops: NilClass method expansion, superclass skip, singleton method distinction, super forwarding
- Metrics/BlockNesting: elsif handling

### M9 Summary

- [x] 469 cops registered (was 364)
- [x] 1,601 tests passing
- [x] 6 departments at 100%: Metrics, Naming, Security, Bundler, Gemspec, Migration
- [x] 3 plugin departments at 100%: Rails, RSpec, Performance

## Completed: M12 — Core Cop Expansion + 100% Conformance (514 cops)

Added 45 new cops across Lint (25), Style (4), and Layout (16). Fixed all remaining false positives to achieve 100% conformance across all 3 bench repos.

### M12 Infrastructure

- [x] **src/config/mod.rs** — Per-directory `.rubocop.yml` resolution: `sub_config_dirs`, `nearest_config_dir()`, `discover_sub_config_dirs()`
- [x] **src/config/mod.rs** — `TargetRubyVersion` propagation from `AllCops` into cop configs

### M12 Cops — Lint (25 new → 78 total)

- [x] Lint/EmptyEnsure — Empty ensure block
- [x] Lint/EmptyBlock — Empty `{}` / `do..end` block
- [x] Lint/EmptyClass — Empty class body
- [x] Lint/EmptyExpression — Empty expression `()`
- [x] Lint/EmptyInterpolation — Empty `#{}`
- [x] Lint/LiteralInInterpolation — Literal inside `#{}`
- [x] Lint/InterpolationCheck — `#{}` inside single-quoted string
- [x] Lint/DuplicateHashKey — Duplicate keys in hash literal
- [x] Lint/DuplicateRequire — Duplicate `require` statements
- [x] Lint/DuplicateMagicComment — Duplicate magic comments
- [x] Lint/SelfAssignment — `x = x`
- [x] Lint/IdentityComparison — `x.equal?(x)`
- [x] Lint/OrAssignmentToConstant — `CONST ||= value`
- [x] Lint/SymbolConversion — Unnecessary symbol conversion
- [x] Lint/TripleQuotes — Triple-quoted strings
- [x] Lint/RedundantWithIndex — Unused `.with_index`
- [x] Lint/RedundantWithObject — Unused `.with_object`
- [x] Lint/Void — Void value used/ignored
- [x] Lint/RescueException — Rescuing `Exception`
- [x] Lint/ReturnInVoidContext — Return in `initialize`
- [x] Lint/ConstantDefinitionInBlock — Constant defined inside block
- [x] Lint/DisjunctiveAssignmentInConstructor — `||=` in initialize
- [x] Lint/RedundantRequireStatement — Redundant `require` (version-aware)
- [x] Lint/ParenthesesAsGroupedExpression — `foo (bar)` misread
- [x] Lint/NonLocalExitFromIterator — `return` inside `.each` block

### M12 Cops — Style (4 new → 60 total)

- [x] Style/RedundantBegin — Unnecessary `begin..end`
- [x] Style/RedundantSelf — Explicit `self.` when implicit
- [x] Style/AndOr — `and`/`or` → `&&`/`||`
- [x] Style/MethodDefParentheses — Parens in `def foo()`

### M12 Cops — Layout (16 new → 64 total)

- [x] Layout/SpaceAroundOperators — `x+1` → `x + 1`
- [x] Layout/SpaceAroundKeyword — `if(x)` → `if (x)`
- [x] Layout/SpaceBeforeComment — `x#comment` → `x #comment`
- [x] Layout/LeadingCommentSpace — `#comment` → `# comment`
- [x] Layout/EmptyLineAfterMagicComment — Blank line after magic comment
- [x] Layout/EmptyLinesAroundAccessModifier — Spacing around private/protected
- [x] Layout/SpaceBeforeBrackets — Space before `[]`
- [x] Layout/SpaceInLambdaLiteral — Space in `-> (x)`
- [x] Layout/SpaceInsideStringInterpolation — Space in `#{ x }`
- [x] Layout/SpaceInsideReferenceBrackets — Space in `foo[ 0 ]`
- [x] Layout/MultilineBlockLayout — Newline after `do`
- [x] Layout/ClosingParenthesisIndentation — Closing `)` alignment
- [x] Layout/IndentationStyle — Tabs vs spaces
- [x] Layout/EmptyLineAfterGuardClause — Blank line after guard clause
- [x] Layout/CommentIndentation — Misaligned comments
- [x] Layout/BlockEndNewline — Newline before block `end`/`}`

### M12 FP Fixes (8 cops fixed)

- SpaceBeforeBrackets: Rewrote from text-based to AST-based (eliminated 225 FPs)
- MultilineBlockLayout: Unwrap BeginNode for rescue/ensure blocks (55 FPs)
- ThreeStateBooleanColumn: Per-directory config resolution (21 FPs)
- IndentationStyle: Skip heredoc content via CodeMap (8 FPs)
- DisjunctiveAssignmentInConstructor: Break on first non-`||=` (5 FPs)
- EmptyLineAfterGuardClause: Extended guard line detection for embedded returns (1 FP)
- ConstantDefinitionInBlock: Only flag ConstantWriteNode, not ConstantPathWriteNode (1 FP)
- RedundantRequireStatement: Version-tiered redundancy with TargetRubyVersion (1 FP)

### M12 Summary

- [x] 514 cops registered (was 469)
- [x] 1,607 unit + 42 integration tests passing
- [x] **100% conformance on all 3 bench repos** (Mastodon, Discourse, Rails)

## Cop Coverage Summary

### Core RuboCop departments

| Department | RuboCop | rblint | Coverage |
|------------|--------:|-------:|---------:|
| Layout | 100 | 64 | 64% |
| Lint | 152 | 78 | 51% |
| Style | 287 | 60 | 21% |
| Metrics | 10 | 10 | **100%** |
| Naming | 19 | 19 | **100%** |
| Security | 7 | 7 | **100%** |
| Bundler | 7 | 7 | **100%** |
| Gemspec | 10 | 10 | **100%** |
| Migration | 1 | 1 | **100%** |
| **Total Core** | **593** | **256** | **43.2%** |

### Plugin departments

| Department | RuboCop | rblint | Coverage |
|------------|--------:|-------:|---------:|
| rubocop-rails | 98 | 98 | **100%** |
| rubocop-rspec | 113 | 113 | **100%** |
| rubocop-performance | 47 | 47 | **100%** |
| **Total Plugins** | **258** | **258** | **100%** |

### Grand total

| | RuboCop | rblint | Coverage |
|--|--------:|-------:|---------:|
| **All departments** | **851** | **514** | **60.4%** |

Remaining gaps: Style (227 missing), Lint (74 missing), Layout (36 missing).

1,649 tests passing (1,607 unit + 42 integration).

### Bench Conformance

| Repo | Category | FPs | FNs | Match Rate |
|------|----------|----:|----:|---------:|
| Mastodon | Large Rails app | 0 | 0 | **100.0%** |
| Discourse | Large Rails app | 0 | 0 | **100.0%** |
| Rails | Framework | 0 | 0 | **100.0%** |
| rubocop | Ruby tool (RSpec-heavy) | 102 | 0 | 1.9% |
| chatwoot | Large Rails app | ~11 | 0 | ~92% |
| errbit | Small Rails app | 50 | 228 | 82.0% |

6 bench repos total. 3 at 100% conformance on 514 covered cops.

### Bench Repo Candidates (Evaluated)

See [docs/bench_repo_candidates.md](docs/bench_repo_candidates.md) for detailed evaluation. Top 5 recommended additions:
- **rubygems.org** — First minitest-based repo, `ParserEngine: parser_prism`
- **activeadmin** — First gem/engine, `DisabledByDefault: true`, `plugins:` config syntax
- **good_job** — `plugins:` syntax, 5 plugins including split gems
- **docuseal** — Large modern Rails app, Ruby 4.0 target
- **doorkeeper** — OAuth gem, gem-with-rails-plugin validation

## Completed: M12b — FP Reduction Pass 2 (16 cops fixed)

Systematic false-positive reduction across expanded bench set (6 repos). Fixed 16 cops, reducing rubocop FPs from 688→193, chatwoot FPs from 469→90.

### M12b FP Fixes

- FirstHashElementIndentation: Visitor-based `special_inside_parentheses` (205 FP eliminated)
- IfUnlessModifier: MaxLineLength injection from Layout/LineLength (71 FP)
- EmptyLineAfterGuardClause: Comment-skipping, multi-line guards, rubocop directives (42 FP)
- ArrayAlignment: `last_checked_line` tracking for multi-element lines (40 FP)
- FirstArrayElementIndentation: `special_inside_parentheses` with parent paren detection (35 FP)
- PredicateMethod: Ruby regex normalization, rescue/block return types (32 FP)
- EmptyLineAfterExample: Block requirement, scope-closing keywords (28 FP)
- ClassAndModuleChildren: Rewritten compact check from outer node (23 FP)
- RepeatedExampleGroupBody: MaxExtentFinder visitor for heredoc signatures (8 FP)
- LeakyLocalVariable: Removed duplicate recursion (11 FP)
- ReflectionClassName: Accept symbol class_name (10 FP)
- ParameterAlignment: Skip multiple params on same line (9 FP)
- MultilineOperationIndentation: Fixed default style to `aligned` (7 FP)
- InstanceSpy: Require `have_received` in file (6 FP)
- StringLiterals: Fixed heredoc content detection (2 FP)
- InterpolationCheck: Improved pattern matching (1 FP)

### M12b Summary

- [x] 1,649 tests passing
- [x] 3 repos at 100%: Mastodon, Discourse, Rails

## In Progress: M12c — Chatwoot FP/FN Elimination

Focused on getting chatwoot to 100% conformance. Fixed FPs across 15+ cops and 5 FNs across 3 cops.

### M12c FP Fixes (15+ cops)

Major fixes:
- FirstArgumentIndentation: Rewritten with visitor + special inner call logic (10 FP on chatwoot, 1 on rubocop)
- ClassAndModuleChildren: Superclass exemption for compact style (23 FP)
- EmptyLineAfterExample: Block requirement + terminator detection (20 FP on chatwoot, 8 on rubocop)
- Rails/Pluck: Better `select` call detection, not_a_rails_pluck guard (5 FP)
- Metrics/MethodLength: Struct.new block exemption (4 FP)
- EmptyLinesAroundAccessModifier: Module body boundary fix (3 FP)
- Metrics/AbcSize: Struct.new block exemption (3 FP)
- RSpec/InstanceVariable: Improved detection (3 FP)
- Rails/RelativeDateConstant: Rewrote to use correct RELATIVE_DATE_METHODS (1 FP)
- Security/YAMLLoad: TargetRubyVersion 3.0 maximum check (1 FP)
- Rails/SkipsModelValidations: good_insert?/good_touch? guards (1 FP)
- RSpec/EmptyLineAfterHook: Comment-line skipping (1 FP)
- RSpec/ExcessiveDocstringSpacing: Multi-line whitespace regex fix (1 FP)
- RSpec/IteratedExpectation: Require exactly 1 block param (1 FP)
- Style/TrailingCommaInHashLiteral: Skip comment content in comma scan (1 FP)

### M12c FN Fixes (3 cops, 5 FNs)

- Naming/PredicateMethod: Assignment nodes classified as Opaque (2 FN)
- Rails/HttpStatusNameConsistency: Recurse into ternary/if branches (2 FN)
- RSpec/LeakyLocalVariable: Handle assignment RHS + conditional branches (1 FN)

### M12c Infrastructure

- Config: `.ruby-version` fallback when `AllCops.TargetRubyVersion` not set
- cop/util.rs: New helpers for access modifier detection, indentation helpers

### M12c Summary

- [x] All tests passing
- [x] 3 repos at 100%: Mastodon, Discourse, Rails
- [x] chatwoot: 87 FP + 5 FN → ~11 FP + 0 FN

## Milestones

| Milestone | Cops | Status |
|-----------|------|--------|
| **M0**: Skeleton | 0 | **Done** |
| **M1**: Line-based cops | 8 | **Done** |
| **M2**: Token/simple-pattern cops | 25 | **Done** |
| **M3**: AST single-node | 70 | **Done** |
| **M4**: Performance cops | 117 | **Done** |
| **M5**: Complex core cops | 153 | **Done** |
| **M6**: rubocop-rails + Include/Exclude | 251 | **Done** |
| **M7**: Autocorrect | +30 fixes | Pending |
| **M8**: rubocop-rspec | 364 | **Done** |
| **M9**: Core Cop Coverage Expansion | 469 | **Done** |
| **M10**: Production Readiness | 364 + config/CLI | **Done** |
| **M11**: Config Compatibility | Drop-in .rubocop.yml | **Done** |
| **M12**: Core Cop Expansion + 100% Conformance | 514 | **Done** |
| **M12c**: Chatwoot FP/FN Elimination | 514 | **Done** |
