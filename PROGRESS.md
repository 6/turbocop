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

### M6 Cops — Rails: Method Call Detection (22)

- [x] Rails/Exit — flag `exit`/`exit!` calls
- [x] Rails/Output — flag `puts`/`print`/`p`/`pp` calls
- [x] Rails/OutputSafety — flag `html_safe`/`raw` calls
- [x] Rails/Blank — `!present?` → `blank?`
- [x] Rails/Present — `!blank?` → `present?`
- [x] Rails/NegateInclude — `!include?` → `exclude?`
- [x] Rails/SafeNavigation — `try`/`try!` → safe navigation `&.`
- [x] Rails/Delegate — manual delegation → `delegate`
- [x] Rails/DelegateAllowBlank — flag invalid `allow_blank` option
- [x] Rails/RequestReferer — `referer` → `referrer`
- [x] Rails/RefuteMethods — `refute_*` → `assert_not_*`
- [x] Rails/ToFormattedS — `to_formatted_s` → `to_fs`
- [x] Rails/ToSWithArgument — `to_s(:format)` → `to_fs`
- [x] Rails/StripHeredoc — `strip_heredoc` → squiggly heredoc
- [x] Rails/Inquiry — flag `.inquiry` calls
- [x] Rails/PluckId — `pluck(:id)` → `ids`
- [x] Rails/Pluck — `map { |x| x[:key] }` → `pluck`
- [x] Rails/IndexBy — `map.to_h` → `index_by`
- [x] Rails/WhereNot — flag manual negated SQL → `where.not`
- [x] Rails/WhereExists — `where.exists?` → `exists?`
- [x] Rails/PluckInWhere — flag `pluck` inside `where`
- [x] Rails/ActiveSupportAliases — `starts_with?`/`ends_with?` → standard Ruby

### M6 Cops — Rails: Method Chain Detection (18)

- [x] Rails/FindBy — `where.first` → `find_by`
- [x] Rails/FindEach — `all.each` → `find_each`
- [x] Rails/CompactBlank — `reject(&:blank?)` → `compact_blank`
- [x] Rails/Pick — `pluck.first` → `pick`
- [x] Rails/ContentTag — `content_tag` → `tag.*`
- [x] Rails/SelectMap — `select.map` → `filter_map`
- [x] Rails/RootJoinChain — chained `.join` → single `join`
- [x] Rails/RootPathnameMethods — `File.read(Rails.root.join)` → pathname methods
- [x] Rails/RootPublicPath — `Rails.root.join('public')` → `Rails.public_path`
- [x] Rails/FilePath — multi-arg `join` → single path string
- [x] Rails/ResponseParsedBody — `JSON.parse(response.body)` → `parsed_body`
- [x] Rails/RedundantActiveRecordAllMethod — remove redundant `.all`
- [x] Rails/HelperInstanceVariable — flag instance variables in helpers
- [x] Rails/UnusedRenderContent — flag content with head-only status
- [x] Rails/RedundantTravelBack — redundant `travel_back`
- [x] Rails/DurationArithmetic — `Time.now + 1.day` → `1.day.from_now`
- [x] Rails/ExpandedDateRange — explicit date range → `all_day`
- [x] Rails/WhereRange — SQL comparison → range in `where`

### M6 Cops — Rails: Time/Date (7)

- [x] Rails/Date — `Date.today` → `Date.current`
- [x] Rails/TimeZone — `Time.now` → `Time.zone.now`
- [x] Rails/RelativeDateConstant — flag relative date in constants
- [x] Rails/FreezeTime — `travel_to(Time.now)` → `freeze_time`
- [x] Rails/TimeZoneAssignment — flag `Time.zone =`
- [x] Rails/DotSeparatedKeys — dot-separated i18n keys
- [x] Rails/ShortI18n — `translate` → `t`, `localize` → `l`

### M6 Cops — Rails: Class Structure (15)

- [x] Rails/ApplicationController — `ActionController::Base` → `ApplicationController`
- [x] Rails/ApplicationRecord — `ActiveRecord::Base` → `ApplicationRecord`
- [x] Rails/ApplicationJob — `ActiveJob::Base` → `ApplicationJob`
- [x] Rails/ApplicationMailer — `ActionMailer::Base` → `ApplicationMailer`
- [x] Rails/HasManyOrHasOneDependent — flag missing `:dependent`
- [x] Rails/InverseOf — flag missing `:inverse_of` with `:foreign_key`
- [x] Rails/HasAndBelongsToMany — flag HABTM → `has_many :through`
- [x] Rails/DuplicateAssociation — flag duplicate association names
- [x] Rails/DuplicateScope — flag duplicate scope names
- [x] Rails/ReadWriteAttribute — `read_attribute`/`write_attribute` → `self[]`
- [x] Rails/ActionOrder — non-standard controller action ordering
- [x] Rails/ActiveRecordCallbacksOrder — callback lifecycle ordering
- [x] Rails/ActionControllerTestCase — flag `ActionController::TestCase`
- [x] Rails/LexicallyScopedActionFilter — filter action name checks
- [x] Rails/ActionControllerFlashBeforeRender — `flash` before `render`

### M6 Cops — Rails: Validation (4)

- [x] Rails/Validation — `validates_*_of` → `validates`
- [x] Rails/RedundantAllowNil — redundant `allow_nil` with `presence`
- [x] Rails/RedundantPresenceValidationOnBelongsTo — redundant presence on `belongs_to`
- [x] Rails/AttributeDefaultBlockValue — mutable `default:` → block form

### M6 Cops — Rails: Environment/Config (8)

- [x] Rails/EnvironmentComparison — `Rails.env ==` → predicate method
- [x] Rails/EnvironmentVariableAccess — `ENV[]` → `ENV.fetch`
- [x] Rails/Env — `ENV['RAILS_ENV']` → `Rails.env`
- [x] Rails/UnknownEnv — flag non-standard environment names
- [x] Rails/RakeEnvironment — flag missing `:environment` dependency
- [x] Rails/EnvLocal — `development? || test?` → `local?`
- [x] Rails/I18nLocaleAssignment — `I18n.locale =` → `with_locale`
- [x] Rails/I18nLazyLookup — flag full i18n keys → lazy lookup

### M6 Cops — Rails: Migration (7)

- [x] Rails/CreateTableWithTimestamps — flag missing `t.timestamps`
- [x] Rails/NotNullColumn — flag NOT NULL without default
- [x] Rails/ThreeStateBooleanColumn — flag boolean without `null: false`
- [x] Rails/DangerousColumnNames — flag reserved column names
- [x] Rails/AddColumnIndex — flag `_id` columns without index
- [x] Rails/MigrationClassName — migration class name conventions
- [x] Rails/SchemaComment — flag missing table/column comments

### M6 Cops — Rails: Enum (3)

- [x] Rails/EnumHash — array enum → hash enum
- [x] Rails/EnumSyntax — old enum syntax → Rails 7+ syntax
- [x] Rails/EnumUniqueness — flag duplicate enum values

### M6 Cops — Rails: Remaining (14)

- [x] Rails/HttpPositionalArguments — positional → keyword args in HTTP test methods
- [x] Rails/HttpStatus — numeric → symbolic HTTP status
- [x] Rails/HttpStatusNameConsistency — consistent status code style
- [x] Rails/DynamicFindBy — `find_by_name` → `find_by(name:)`
- [x] Rails/SkipsModelValidations — flag validation-skipping methods
- [x] Rails/ScopeArgs — scope without lambda
- [x] Rails/ReflectionClassName — string `class_name:` → constant
- [x] Rails/RedundantForeignKey — flag conventional foreign key
- [x] Rails/RenderInline — flag `render inline:`
- [x] Rails/RenderPlainText — `render text:` → `render plain:`
- [x] Rails/ReversibleMigrationMethodDefinition — `up`/`down` or `change`
- [x] Rails/TableNameAssignment — flag `self.table_name =`
- [x] Rails/AfterCommitOverride — flag multiple `after_commit`
- [x] Rails/TransactionExitStatement — flag `return`/`break`/`throw` in transaction

### M6 Summary

- [x] **src/cop/registry.rs** — `default_registry()` registers all 251 cops
- [x] **src/cop/rails/** — 98 new cop source files
- [x] **testdata/cops/rails/** — 196 new fixture files (offense + no_offense for each new cop)
- [x] 714 tests passing (691 unit + 23 integration)
- [x] Include/Exclude path pattern enforcement infrastructure
- [x] Test coverage guard: `all_cops_have_fixture_files` covers all 251 cops

## Completed: M8 — rubocop-rspec (113 cops)

113 new RSpec department cops (364 total), vendor fixture extraction from rubocop-rspec specs, RSpec DSL utility helpers.

### M8 Infrastructure

- [x] **src/cop/rspec/mod.rs** — New RSpec department with `register_all()` for 113 cops
- [x] **src/cop/util.rs** — RSpec-specific helpers: `RSPEC_EXAMPLE_GROUPS`, `RSPEC_EXAMPLES`, `RSPEC_HOOKS`, `RSPEC_LETS`, `RSPEC_SUBJECTS`, `RSPEC_FOCUSED_METHODS`, `RSPEC_DEFAULT_INCLUDE`, `is_rspec_example_group()`, `is_rspec_example()`, `is_rspec_hook()`, `is_rspec_focused()`, `has_rspec_focus_metadata()`, `first_positional_arg()`, `block_body_line_count()`
- [x] **CLAUDE.md** — Documented vendor fixture extraction process

### M8 Cops — RSpec: Simple Matchers (16)

- [x] RSpec/Focus — focused specs (`:focus`, `focus: true`, f-prefixed methods)
- [x] RSpec/Be — bare `be` matcher without arguments
- [x] RSpec/BeEmpty — `be_empty` matcher usage
- [x] RSpec/BeEq — `eq(true/false/nil)` → `be` matchers
- [x] RSpec/BeEql — `eql(true/false/nil)` → `be` matchers
- [x] RSpec/BeNil — `be_nil` vs `be(nil)` style
- [x] RSpec/ContainExactly — `contain_exactly` vs `match_array`
- [x] RSpec/MatchArray — `match_array` with array literal → `contain_exactly`
- [x] RSpec/NotToNot — `to_not` → `not_to`
- [x] RSpec/EmptyMetadata — empty metadata hash `{}`
- [x] RSpec/Pending — flag `pending` examples
- [x] RSpec/PendingWithoutReason — `pending`/`skip` without reason
- [x] RSpec/AnyInstance — `allow_any_instance_of`/`expect_any_instance_of`
- [x] RSpec/InstanceSpy — `instance_double` + `as_null_object` → `instance_spy`
- [x] RSpec/MessageChain — `receive_message_chain` usage
- [x] RSpec/VoidExpect — `expect()` without matcher

### M8 Cops — RSpec: Call Pattern Detection (16)

- [x] RSpec/ExpectActual — literal values as `expect` argument
- [x] RSpec/ExpectOutput — `$stdout`/`$stderr` in expect
- [x] RSpec/RemoveConst — `remove_const` in specs
- [x] RSpec/DescribeMethod — describe with method description convention
- [x] RSpec/DescribeSymbol — `describe :symbol` usage
- [x] RSpec/ContextMethod — `context` with method-like description
- [x] RSpec/ContextWording — context description wording
- [x] RSpec/EmptyHook — empty before/after/around blocks
- [x] RSpec/EmptyOutput — empty output matcher
- [x] RSpec/InstanceVariable — `@ivar` usage in examples
- [x] RSpec/Eq — `eq` matcher style
- [x] RSpec/Output — `output` matcher patterns
- [x] RSpec/ClassCheck — `is_a?` vs `instance_of?` style
- [x] RSpec/SingleArgumentMessageChain — single-arg `receive_message_chain`
- [x] RSpec/ReceiveNever — `receive.never` → `not_to receive`
- [x] RSpec/ReceiveCounts — `receive.once`/`.twice` patterns

### M8 Cops — RSpec: Block Structure (16)

- [x] RSpec/ExampleLength — example block line count
- [x] RSpec/MultipleExpectations — too many `expect` calls
- [x] RSpec/NoExpectationExample — example without expectations
- [x] RSpec/NestedGroups — deeply nested groups
- [x] RSpec/MultipleMemoizedHelpers — too many `let` declarations
- [x] RSpec/HooksBeforeExamples — hooks after examples
- [x] RSpec/LetBeforeExamples — `let` after examples
- [x] RSpec/EmptyLineAfterExample — missing blank line after examples
- [x] RSpec/EmptyLineAfterExampleGroup — missing blank line after groups
- [x] RSpec/EmptyLineAfterFinalLet — missing blank line after last `let`
- [x] RSpec/EmptyLineAfterHook — missing blank line after hooks
- [x] RSpec/EmptyLineAfterSubject — missing blank line after subject
- [x] RSpec/LeadingSubject — subject should come first
- [x] RSpec/ScatteredLet — `let` should be grouped
- [x] RSpec/ScatteredSetup — `before` should be grouped
- [x] RSpec/EmptyExampleGroup — example group with no examples

### M8 Cops — RSpec: Duplication/Ordering (16)

- [x] RSpec/MultipleDescribes — multiple top-level describes
- [x] RSpec/MultipleSubjects — multiple subject declarations
- [x] RSpec/DescribeClass — first arg to describe should be a class
- [x] RSpec/DescribedClass — use `described_class` instead of explicit name
- [x] RSpec/DescribedClassModuleWrapping — `described_class` in module wrapper
- [x] RSpec/OverwritingSetup — duplicate `let` names
- [x] RSpec/RepeatedDescription — duplicate example descriptions
- [x] RSpec/RepeatedExample — duplicate example bodies
- [x] RSpec/RepeatedExampleGroupBody — duplicate group bodies
- [x] RSpec/RepeatedExampleGroupDescription — duplicate group descriptions
- [x] RSpec/RepeatedIncludeExample — duplicate `include_examples`
- [x] RSpec/RepeatedSubjectCall — repeated `subject` calls
- [x] RSpec/LetSetup — `let` only used in hooks
- [x] RSpec/IncludeExamples — `include_examples` vs `it_behaves_like`
- [x] RSpec/ItBehavesLike — `it_behaves_like` vs `include_examples`
- [x] RSpec/SharedContext — shared context naming

### M8 Cops — RSpec: Naming/Expectation (16)

- [x] RSpec/SharedExamples — shared examples naming
- [x] RSpec/ExampleWithoutDescription — example without description string
- [x] RSpec/ExampleWording — example wording style ("should" → imperative)
- [x] RSpec/IndexedLet — `let(:item_1)` indexed names
- [x] RSpec/SubjectDeclaration — subject declaration style
- [x] RSpec/VariableDefinition — `let` vs `let!` style
- [x] RSpec/VariableName — variable naming convention
- [x] RSpec/IsExpectedSpecify — `is_expected` with `specify`
- [x] RSpec/ExpectChange — `expect { }.to change` style
- [x] RSpec/ExpectInHook — `expect` in hooks
- [x] RSpec/ExpectInLet — `expect` in `let`
- [x] RSpec/HookArgument — hook scope argument style
- [x] RSpec/ImplicitBlockExpectation — implicit block expectation
- [x] RSpec/ImplicitExpect — `is_expected` vs `expect(subject)`
- [x] RSpec/ImplicitSubject — implicit vs explicit subject
- [x] RSpec/IteratedExpectation — `expect` inside `.each`

### M8 Cops — RSpec: Mocking/Complex (16)

- [x] RSpec/MessageExpectation — message expectation style
- [x] RSpec/MessageSpies — spy vs mock style
- [x] RSpec/ReceiveMessages — multiple `receive` → `receive_messages`
- [x] RSpec/ReturnFromStub — stub return style
- [x] RSpec/StubbedMock — stubbed mock pattern
- [x] RSpec/SubjectStub — stubbing on subject
- [x] RSpec/VerifiedDoubles — `double` → verified doubles
- [x] RSpec/VerifiedDoubleReference — string vs constant reference
- [x] RSpec/PredicateMatcher — predicate matcher style
- [x] RSpec/RedundantPredicateMatcher — redundant predicate matcher
- [x] RSpec/RedundantAround — redundant `around` hook
- [x] RSpec/SkipBlockInsideExample — `skip` with block
- [x] RSpec/AroundBlock — `around` without yield/run
- [x] RSpec/BeforeAfterAll — `before(:all)`/`after(:all)`
- [x] RSpec/LeakyConstantDeclaration — constants in example groups
- [x] RSpec/LeakyLocalVariable — local variable leaking

### M8 Cops — RSpec: Remaining (17)

- [x] RSpec/UnspecifiedException — `raise_error` without class
- [x] RSpec/Dialect — non-standard RSpec aliases
- [x] RSpec/MissingExampleGroupArgument — missing describe/context args
- [x] RSpec/IdenticalEqualityAssertion — `expect(x).to eq(x)`
- [x] RSpec/ChangeByZero — `change { }.by(0)`
- [x] RSpec/UndescriptiveLiteralsDescription — literal descriptions
- [x] RSpec/ExcessiveDocstringSpacing — excessive whitespace in descriptions
- [x] RSpec/SpecFilePathFormat — spec path matches described class
- [x] RSpec/SpecFilePathSuffix — spec files end in `_spec.rb`
- [x] RSpec/AlignLeftLetBrace — left-align let braces
- [x] RSpec/AlignRightLetBrace — right-align let braces
- [x] RSpec/DuplicatedMetadata — duplicate metadata keys
- [x] RSpec/MetadataStyle — metadata hash vs symbol style
- [x] RSpec/SortMetadata — sorted metadata
- [x] RSpec/MissingExpectationTargetMethod — missing `.to`/`.not_to`
- [x] RSpec/NamedSubject — named subject style
- [x] RSpec/Yield — yield matcher issues

### M8 Summary

- [x] **src/cop/registry.rs** — `default_registry()` registers all 364 cops
- [x] **src/cop/rspec/** — 113 new cop source files
- [x] **testdata/cops/rspec/** — 226+ new fixture files (offense + no_offense for each cop)
- [x] 971 tests passing (944 unit + 27 integration)
- [x] RSPEC_DEFAULT_INCLUDE: all RSpec cops default to `**/*_spec.rb` and `**/spec/**/*`
- [x] Vendor fixture extraction documented in CLAUDE.md
- [x] Test coverage guard: `all_cops_have_fixture_files` covers all 364 cops

## Completed: M10 — Production Readiness (Config Inheritance + CLI)

Config inheritance (`inherit_from` + `inherit_gem`), `--rubocop-only` flag, `--stdin` support. Enables deployment to real projects.

### M10 Infrastructure

- [x] **src/config/mod.rs** — Full config inheritance: `inherit_from` (local YAML, recursive), `inherit_gem` (via `bundle info --path`), circular detection, merge logic
- [x] **src/config/gem_path.rs** — Gem path resolution via `bundle info --path` with Gemfile.lock mtime cache
- [x] **src/config/mod.rs** — `enabled_cop_names()` method on ResolvedConfig for `--rubocop-only`
- [x] **src/config/mod.rs** — RuboCop-compatible merge rules: Exclude appends, Include replaces, scalars last-writer-wins
- [x] **src/config/mod.rs** — Silent handling of `require:` and `plugins:` keys

### M10 CLI Features

- [x] **src/cli.rs** — `--rubocop-only` flag: prints comma-separated list of cops not covered by rblint
- [x] **src/cli.rs** — `--stdin PATH` flag: reads source from stdin, uses PATH for display and config matching
- [x] **src/lib.rs** — Early exit for `--rubocop-only`, stdin lint path
- [x] **src/parse/source.rs** — `SourceFile::from_string()` public constructor (non-test)
- [x] **src/linter.rs** — `lint_source()` public function for linting a single SourceFile directly

### M10 Tests

- [x] **src/config/mod.rs** — 12 new unit tests: inherit_from (single, array, override, exclude-appends, include-replaces, missing, circular), require/plugins ignored, deep merge, enabled_cop_names, merge logic
- [x] **src/config/gem_path.rs** — 2 unit tests: output parsing, cache key behavior
- [x] **tests/integration.rs** — 8 new integration tests: inherit_from merging, circular detection, --rubocop-only CLI, --stdin (trailing whitespace, clean exit, display path Include matching), inherited config pipeline, lint_source API
- [x] **testdata/config/** — Fixture files for inherit_from, circular, rubocop_only configs
- [x] 994 tests passing (959 unit + 35 integration)

### M10 Summary

- [x] Config inheritance: `inherit_from` (local files, recursive) + `inherit_gem` (bundle info)
- [x] `--rubocop-only`: enables hybrid `bin/lint` workflow
- [x] `--stdin PATH`: enables editor integration (VSCode, vim, Emacs)
- [x] All 364 cops preserved, zero regressions

## Completed: M11 — Drop-in RuboCop Config Compatibility

Full `.rubocop.yml` compatibility: auto-discovery, `require:` plugin default loading, department-level configs, `Enabled: pending` / `AllCops.NewCops`, `AllCops.DisabledByDefault`, and `inherit_mode`.

### M11 Infrastructure

- [x] **src/cop/mod.rs** — `EnabledState` enum (True, False, Pending, Unset) replacing `bool` for `CopConfig.enabled`
- [x] **src/config/mod.rs** — Config auto-discovery (walk up from target dir), `require:` plugin default config loading, department-level configs (`RSpec:`, `Rails:` Include/Exclude/Enabled), `AllCops.NewCops` / `AllCops.DisabledByDefault` support, `inherit_mode` (merge/override), `Enabled: pending` tri-state
- [x] **src/config/gem_path.rs** — `working_dir` parameter for `resolve_gem_path` (bundle runs from repo dir)
- [x] **src/lib.rs** — Pass target dir to `load_config` for auto-discovery, debug output for config dir

### M11 Tests

- [x] **src/config/mod.rs** — 11 new unit tests: auto-discovery (target dir, walk up parent, no config), `Enabled: pending` (default, with NewCops), `DisabledByDefault`, department Include, department Enabled, cop overrides department, `inherit_mode` merge/override, `enabled_cop_names` with pending/disabled_by_default
- [x] All existing tests updated for new `load_config` signature and `EnabledState`

### M11 Summary

- [x] Config auto-discovery from target path (no more false positives from missing config)
- [x] `require: rubocop-rspec` loads plugin `config/default.yml` (department Include/Exclude patterns)
- [x] Department-level configs (`RSpec:`, `Rails:`) for Include/Exclude/Enabled
- [x] `Enabled: pending` + `AllCops.NewCops: enable` tri-state
- [x] `AllCops.DisabledByDefault: true` support
- [x] `inherit_mode: merge/override` for Include/Exclude arrays

## In Progress: Conformance FP Reduction

Systematic false-positive reduction against Mastodon and Discourse benchmark repos. All rblint offenses on these repos are FPs (rubocop reports 0 offenses on both repos).

### FP Tracking

| Benchmark | Starting FPs | Current FPs | Reduction |
|-----------|-------------|-------------|-----------|
| Mastodon | ~3,357 | **856** | -74% |
| Discourse | ~375 | **60** | -84% |

### Cop Logic Fixes (completed across multiple sessions)

**Session 1 — 6 cops fixed:**
- [x] Layout/MultilineMethodCallIndentation — skip when inside rescue/ensure/else
- [x] Rails/DotSeparatedKeys — skip interpolated strings
- [x] Rails/TimeZone — implement `flexible` default style (allow `Time.current`, `Time.zone.*`)
- [x] RSpec/ContextWording — skip context blocks with method-style descriptions (`.method`, `#method`)
- [x] RSpec/ExampleWithoutDescription — skip `it {}` without block (pending examples)
- [x] Layout/EndAlignment — handle when/in alignment to case

**Session 2 — 3 cops fixed:**
- [x] Layout/ArgumentAlignment — skip single-line calls
- [x] RSpec/LetSetup — handle let used only in hooks (more conservative detection)

**Session 3 — 7 cops fixed:**
- [x] Layout/DefEndAlignment — handle `end` aligned to `def` for multi-line defs
- [x] Lint/SuppressedException — AllowComments support
- [x] Rails/Delegate — skip calls with blocks
- [x] RSpec/ExampleLength — respect configurable Max (default 5)
- [x] RSpec/InstanceVariable — skip `@` in strings/comments
- [x] Style/IfUnlessModifier — skip heredoc bodies, parenthesized assignment conditions, comments before body, include indentation in modifier line length
- [x] Layout/EmptyLineBetweenDefs — AllowAdjacentOneLineDefs support

**Session 4 — 3 cops fixed:**
- [x] Lint/NestedMethodDefinition — skip def inside Class.new/Module.new blocks
- [x] Performance/RedundantMatch — only flag when return value is unused

**Session 5 — 9 cops fixed, 1 infrastructure fix:**
- [x] Rails/ReflectionClassName — REVERSED logic (was flagging strings, should flag non-strings)
- [x] Style/Lambda — implement `line_count_dependent` default style
- [x] Style/ParenthesesAroundCondition — AllowSafeAssignment support
- [x] Style/EmptyMethod — skip methods with comment-only bodies
- [x] Style/TrailingCommaInArguments — skip &block args, validate comma context with heredoc protection
- [x] Performance/RedundantSplitRegexpArgument — only flag simple literal regexes
- [x] Style/Semicolon — add while/until to single-line body detection
- [x] Style/IfUnlessModifier — multiple fixes (comments, heredocs, indentation)
- [x] CodeMap — mark heredoc content as non-code (fixed Semicolon/TrailingComma FPs from heredoc CSS/SQL)

**Session 6 (current) — 4 cops fixed:**
- [x] Lint/EmptyConditionalBody — AllowComments: true (check parse_result comments between keyword and end)
- [x] RSpec/AroundBlock — handle `yield` as valid way to run example; handle BeginNode (rescue/ensure) and local variable assignment in body recursion
- [x] RSpec/ReturnFromStub — remove constants/constant paths from `is_static_value` (RuboCop doesn't consider them static)

### Remaining FPs — Mastodon (856)

Most are **config issues** requiring the target repo's `.rubocop.yml` to be loaded correctly:
- RSpec/IncludeExamples (118) — version-specific behavior
- Layout/MultilineMethodCallIndentation (66) — complex edge cases
- Rails/TimeZone (63) — remaining cases beyond flexible style
- RSpec/SpecFilePathFormat (52) — needs plugin config
- Rails/HttpStatus (52) — needs EnforcedStyle: numeric from config
- Rails/FilePath (42) — needs EnforcedStyle: arguments from config
- Rails/ContentTag (34) — needs config
- Layout/LineLength (20) — needs Max: 300 from config
- Style/WordArray (20) — needs MinSize: 3 from config
- RSpec/NamedSubject (10) — needs EnforcedStyle: named_only from config

### Remaining FPs — Discourse (60)

Mostly **config issues** (Discourse's rubocop-discourse gem disables many cops):
- RSpec/NamedSubject (15) — likely disabled by rubocop-discourse
- Lint/BooleanSymbol (12) — likely disabled by rubocop-discourse
- Style/FrozenStringLiteralComment (12) — likely disabled or set to `never`
- RSpec/EmptyExampleGroup (10) — likely disabled by rubocop-discourse
- RSpec/UnspecifiedException (3), Lint/Debugger (2), RSpec/ExpectActual (2), RSpec/ExpectOutput (2), Lint/EmptyConditionalBody (1), RSpec/ChangeByZero (1)

### Next Steps

The remaining FPs are dominated by config loading issues. The M11 config infrastructure handles `inherit_from` and `inherit_gem` but the bench tool runs rblint from the rblint project directory, so config auto-discovery may not find the target repo's config. Key areas:
1. Verify config auto-discovery works when target path is provided
2. Ensure `require:` plugin configs load correctly for rubocop-rspec, rubocop-rails, rubocop-performance
3. Handle rubocop-discourse gem's config for Discourse
4. Fix remaining logic bugs in cops with <10 FPs

## Completed: Config Audit + Prism Pitfalls — Zero Gaps

Eliminated all config audit (126 → 0) and prism pitfalls (68 → 0) gaps. Every YAML config key read by RuboCop cops is now implemented in the Rust source, and every cop correctly handles both `ConstantPathNode` and `KeywordHashNode` variants.

### Infrastructure Changes

- [x] **tests/integration.rs** — Removed baseline mechanism; both tests now assert zero gaps directly
- [x] **tests/baselines/** — Deleted entirely (no longer needed)
- [x] **src/cop/mod.rs** — Added `get_string_hash()` to CopConfig for hash-type YAML configs (CustomTransform, IgnoreMetadata)
- [x] **tests/integration.rs** — Expanded `infrastructure_keys` filter: `Supported*` prefix, `References`, `Severity`

### Prism Pitfalls Fixed (68 → 0)

- [x] **ConstantPathNode** (58 entries, 49 cops) — All cops now handle qualified constants (`Foo::Bar`) via `as_constant_path_node()` or `util::constant_name()`
- [x] **KeywordHashNode** (10 entries, 9 cops) — All cops now handle keyword args alongside hash literals

### Config Audit Fixed (126 → 0)

All config keys across all 364 cops now have real implementations with behavioral tests:

**Layout (25 cops):** EmptyLineBetweenDefs (DefLikeMacros), IndentationWidth (EnforcedStyleAlignWith), LineLength (AllowRBSInlineAnnotation), HashAlignment (7 keys), and 21 others

**Lint (8 cops):** EmptyConditionalBody (AllowComments), EmptyWhen (AllowComments), SuppressedException (AllowNil), NestedMethodDefinition (AllowedMethods/Patterns), Debugger (DebuggerMethods/Requires), and 3 others

**Metrics (6 cops):** MethodLength/BlockLength/ClassLength/ModuleLength (AllowedMethods, AllowedPatterns, CountAsOne), CyclomaticComplexity/PerceivedComplexity (AllowedMethods/Patterns)

**Naming (4 cops):** FileName (ExpectMatchingDefinition, Regex, IgnoreExecutableScripts), AsciiIdentifiers (AsciiConstants), and 2 others

**Performance (5 cops):** FlatMap (EnabledForFlattenWithoutParams), Sum (OnlySumOrWithInitialValue), DoubleStartEndWith (IncludeActiveSupportAliases), and 2 others

**Rails (17 cops):** Delegate (EnforceForPrefixed), FindBy (IgnoreWhereFirst), InverseOf (IgnoreScopes), RenderPlainText (ContentTypeCompatibility), SafeNavigation (ConvertTry), and 12 others

**RSpec (24 cops):** DescribedClass (OnlyStaticConstants, SkipBlocks), ExampleWording (CustomTransform), PredicateMatcher (AllowedExplicitMatchers), SpecFilePathFormat (CustomTransform, IgnoreMetadata, IgnoreMethods), and 20 others

**Style (18 cops):** FrozenStringLiteralComment (EnforcedStyle), EmptyMethod (EnforcedStyle), HashSyntax (EnforcedShorthandSyntax), TrailingCommaIn* (EnforcedStyleForMultiline), and 14 others

### Summary

- [x] 1,302 tests passing (1,210 lib + 50 codegen + 42 integration)
- [x] `cargo test config_audit` — 0 gaps (was 126)
- [x] `cargo test prism_pitfalls` — 0 gaps (was 68)
- [x] Zero compiler warnings
- [x] Only 2 intentional no-ops: `SplitStrings` (autocorrection-only), `InflectorPath` (Ruby-specific Zeitwerk)

## Upcoming Milestones

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
| **M10**: Production Readiness | 364 + config/CLI | **Done** |
| **M11**: Config Compatibility | Drop-in .rubocop.yml | **Done** |
