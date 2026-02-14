# rubylint — Project Plan

## Overview

A fast Ruby linter written in Rust, targeting **RuboCop core + rubocop-performance**
cop compatibility, with a path to full RuboCop replacement.

Designed to run alongside RuboCop initially via a `bin/lint` wrapper, handling
the ~70% of cops that are core + performance while RuboCop handles the
remaining plugin cops (rspec, rails, vendor, custom).

**Strategy**: rubylint checks ~390 cops in <1s. RuboCop checks the remaining
~168 plugin cops via `--only`. Porting rspec + rails cops is high priority
because benchmarking shows a **hard 12-second floor** for any RuboCop
invocation regardless of cop count (Ruby boot + Bundler + AST parsing).

### The 12-second floor

Benchmarking RuboCop with a single trivial cop on a 13,000-file codebase:

```
$ time bundle exec rubocop --only Style/FrozenStringLiteralComment
13027 files inspected, 6895 offenses detected
bundle exec rubocop --only Style/FrozenStringLiteralComment  46.43s user 14.69s system 499% cpu 12.231 total
```

12 seconds for ONE cop. This is the irreducible cost of: booting Ruby,
loading Bundler, requiring RuboCop + all plugins, and parsing 13,000 files
into Ruby ASTs. Adding more cops to `--only` only adds 1-3s on top.

This means every plugin ported to Rust doesn't just save cop-execution
time — it gets closer to eliminating the 12s Ruby boot tax entirely.

| State | CI lint time |
|-------|-------------|
| Current (RuboCop alone, 558 cops) | **42s** |
| Core + Performance in Rust, rest in RuboCop | **~13-16s** |
| + RSpec + Rails in Rust | **~4s** (boot + ~18 remaining cops) |
| Everything in Rust except custom cops | **~4s** (boot + 3 cops) |
| Everything in Rust, no RuboCop at all | **<1s** |

## Tech Stack

- **Parser**: `ruby-prism` crate (Rust bindings to Prism, Ruby's official parser since 3.3)
  - Prism is the official future of Ruby parsing and what RuboCop is migrating to
  - **Do NOT start with lib-ruby-parser** — migrating parsers later means rewriting
    every AST visitor for all cops. The AST node types, child structures, and naming
    are fundamentally different between parsers. Eat the FFI complexity now.
- **Parallelism**: `rayon` for parallel file processing
- **CLI**: `clap` (derive mode)
- **Config parsing**: `serde_yaml` + `serde`
- **Path matching**: `globset` (for Include/Exclude patterns)
- **Diagnostics output**: `serde_json` (for `--format json`), custom for text/emacs formats

## Project Structure

```
rubylint/
├── Cargo.toml
├── src/
│   ├── main.rs                  # CLI entry point
│   ├── lib.rs                   # Public API
│   ├── config/
│   │   ├── mod.rs               # Config loading orchestration
│   │   ├── resolve.rs           # inherit_gem / inherit_from resolution
│   │   ├── gem_path.rs          # .ruby-version + Gemfile.lock → gem install path
│   │   └── merge.rs             # YAML layer merging logic
│   ├── fs/
│   │   ├── mod.rs               # File discovery
│   │   ├── walker.rs            # Parallel directory walker
│   │   └── ignore.rs            # Exclude pattern matching
│   ├── parse/
│   │   ├── mod.rs               # Prism parser wrapper
│   │   └── source.rs            # Source file representation (content, lines, offsets)
│   ├── cops/
│   │   ├── mod.rs               # Cop registry + trait definition
│   │   ├── layout/
│   │   │   ├── mod.rs
│   │   │   ├── trailing_whitespace.rs
│   │   │   ├── line_length.rs
│   │   │   └── ...
│   │   ├── style/
│   │   │   ├── mod.rs
│   │   │   ├── frozen_string_literal_comment.rs
│   │   │   └── ...
│   │   ├── lint/
│   │   │   ├── mod.rs
│   │   │   ├── debugger.rs
│   │   │   └── ...
│   │   ├── metrics/
│   │   │   ├── mod.rs
│   │   │   ├── method_length.rs
│   │   │   └── ...
│   │   └── performance/
│   │       ├── mod.rs
│   │       ├── detect.rs
│   │       ├── flat_map.rs
│   │       └── ...
│   ├── diagnostic.rs            # Offense/diagnostic type
│   ├── formatter/
│   │   ├── mod.rs
│   │   ├── text.rs              # Default human-readable output
│   │   ├── json.rs              # --format json
│   │   └── emacs.rs             # --format emacs (for editor integration)
│   └── fix/
│       └── mod.rs               # Autocorrect infrastructure (later)
├── tests/
│   ├── cop_tests/
│   │   ├── layout/
│   │   │   ├── trailing_whitespace_test.rs
│   │   │   └── ...
│   │   ├── performance/
│   │   │   ├── detect_test.rs
│   │   │   └── ...
│   │   └── ...
│   └── integration/
│       ├── config_resolution_test.rs
│       └── full_run_test.rs
└── testdata/
    ├── cops/
    │   ├── layout/
    │   │   ├── trailing_whitespace/
    │   │   │   ├── offense.rb        # File that should trigger offense
    │   │   │   └── no_offense.rb     # File that should pass
    │   │   └── ...
    │   ├── performance/
    │   │   ├── detect/
    │   │   │   ├── offense.rb
    │   │   │   └── no_offense.rb
    │   │   └── ...
    │   └── ...
    └── config/
        ├── simple.yml
        └── inherit_gem.yml
```

## Core Data Structures

```rust
// --- Diagnostic ---

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub cop_name: &'static str,       // "Layout/TrailingWhitespace"
    pub severity: Severity,            // convention, warning, error, fatal
    pub message: String,               // "Trailing whitespace detected."
    pub file: PathBuf,
    pub location: Location,
    pub correctable: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct Location {
    pub line: usize,       // 1-indexed
    pub column: usize,     // 0-indexed
    pub end_line: usize,
    pub end_column: usize,
}

#[derive(Debug, Clone, Copy)]
pub enum Severity {
    Convention,  // C
    Warning,     // W
    Error,       // E
    Fatal,       // F
}

// --- Source File ---

pub struct SourceFile {
    pub path: PathBuf,
    pub content: String,
    pub lines: Vec<&str>,              // pre-split for line-based cops
    pub line_offsets: Vec<usize>,      // byte offset of each line start
}

// --- Cop Trait ---

pub trait Cop: Send + Sync {
    fn name(&self) -> &'static str;
    fn severity(&self) -> Severity { Severity::Convention }

    /// Line-based check. Called once with all lines.
    /// Default: no-op. Override for line-based cops.
    fn check_lines(&self, src: &SourceFile, cfg: &CopConfig) -> Vec<Diagnostic> {
        vec![]
    }

    /// AST-based check. Called once with the parsed Prism AST.
    /// Default: no-op. Override for AST-based cops.
    fn check_ast(&self, src: &SourceFile, ast: &Node, cfg: &CopConfig) -> Vec<Diagnostic> {
        vec![]
    }
}

// --- Cop Config ---

#[derive(Debug, Clone, Default)]
pub struct CopConfig {
    pub enabled: bool,
    pub severity: Option<Severity>,
    pub options: HashMap<String, serde_yaml::Value>,  // cop-specific: Max, EnforcedStyle, etc.
    pub exclude: Vec<GlobPattern>,
    pub include: Vec<GlobPattern>,
}

// --- Linter orchestration ---

pub struct Linter {
    cops: Vec<Box<dyn Cop>>,
    config: ResolvedConfig,
}

impl Linter {
    /// Main entry point. Processes files in parallel.
    pub fn run(&self, files: &[SourceFile]) -> Vec<Diagnostic> {
        files.par_iter()
            .flat_map(|src| self.check_file(src))
            .collect()
    }

    fn check_file(&self, src: &SourceFile) -> Vec<Diagnostic> {
        // Parse once per file using Prism
        let ast = prism_parse(&src.content);

        self.cops.iter()
            .filter(|cop| self.config.is_enabled(cop.name(), &src.path))
            .flat_map(|cop| {
                let cfg = self.config.cop_config(cop.name());
                let mut diags = cop.check_lines(src, &cfg);
                if let Ok(ref tree) = ast {
                    diags.extend(cop.check_ast(src, tree, &cfg));
                }
                diags
            })
            .collect()
    }
}
```

## Config Resolution (Pure Rust, No Shell-outs)

```rust
/// Resolves the full RuboCop config chain in pure Rust.
///
/// 1. Read .ruby-version → "3.4.2" → abi "3.4.0"
/// 2. Detect version manager: check which path exists
///    - ~/.local/share/mise/installs/ruby/   → mise
///    - ~/.asdf/installs/ruby/               → asdf
///    - ~/.rbenv/versions/                   → rbenv
/// 3. Read Gemfile.lock → extract gem name + version
/// 4. Construct gem path:
///    {vm_root}/{ruby_version}/lib/ruby/gems/{abi}/gems/{gem}-{version}/
/// 5. Read YAML at gem path, recurse for inherit_from/inherit_gem
/// 6. Merge layers bottom-up:
///    - RuboCop defaults (compiled into binary as static YAML)
///    - inherited gem config (from gem path)
///    - .rubocop.yml (local overrides)
///
/// Merge rules:
///    - Scalar values: last writer wins
///    - Enabled/Severity: last writer wins
///    - Exclude: arrays are APPENDED
///    - Include: arrays are REPLACED
///    - Everything else: last writer wins
///
/// Cache strategy: hash .rubocop.yml mtime + Gemfile.lock mtime,
/// store resolved config in .rubylint-cache/. Only re-resolve when
/// something changes.
///
/// Manual override for unusual environments (Docker, global Ruby, etc.):
///
///   # .rubylint.yml
///   gem_paths:
///     my-style-gem: /custom/path/to/my-style-gem-1.2.3
///
/// If auto-detection fails and no manual override exists, rubylint
/// prints a clear error with instructions for setting gem_paths.

pub struct ConfigResolver {
    project_root: PathBuf,
}

impl ConfigResolver {
    pub fn resolve(&self) -> Result<ResolvedConfig> {
        // Check for manual override first
        if let Some(overrides) = self.read_rubylint_yml()? {
            return self.resolve_with_overrides(overrides);
        }

        let ruby_version = self.read_ruby_version()?;        // .ruby-version
        let abi = format!("{}.{}.0", ruby_version.major, ruby_version.minor);
        let vm_root = self.detect_version_manager()?;         // mise/asdf/rbenv
        let lockfile = self.parse_gemfile_lock()?;            // Gemfile.lock

        let local_yml = self.project_root.join(".rubocop.yml");
        let local_config = self.read_yaml(&local_yml)?;

        // Recursively resolve inherit_gem and inherit_from
        let mut layers = vec![self.rubocop_defaults()];
        self.resolve_inheritance(&local_config, &vm_root, &ruby_version, &abi, &lockfile, &mut layers)?;
        layers.push(local_config);

        Ok(self.merge_layers(layers))
    }

    fn gem_path(&self, vm_root: &Path, ruby_ver: &str, abi: &str, gem: &str, ver: &str) -> PathBuf {
        vm_root
            .join(ruby_ver)
            .join("lib/ruby/gems")
            .join(abi)
            .join("gems")
            .join(format!("{}-{}", gem, ver))
    }

    fn detect_version_manager(&self) -> Result<PathBuf> {
        // Also check GEM_HOME env var first (set by some version managers)
        if let Ok(gem_home) = std::env::var("GEM_HOME") {
            return Ok(PathBuf::from(gem_home));
        }

        let home = dirs::home_dir().unwrap();
        let candidates = [
            home.join(".local/share/mise/installs/ruby"),
            home.join(".asdf/installs/ruby"),
            home.join(".rbenv/versions"),
        ];
        candidates.iter()
            .find(|p| p.exists())
            .cloned()
            .ok_or_else(|| anyhow!(
                "No Ruby version manager detected. \
                 Add gem_paths to .rubylint.yml or set GEM_HOME."
            ))
    }

    fn read_ruby_version(&self) -> Result<RubyVersion> {
        // Try .ruby-version first, then .tool-versions
        let rv_path = self.project_root.join(".ruby-version");
        if rv_path.exists() {
            let content = fs::read_to_string(&rv_path)?;
            return content.trim().parse();
        }
        let tv_path = self.project_root.join(".tool-versions");
        if tv_path.exists() {
            let content = fs::read_to_string(&tv_path)?;
            for line in content.lines() {
                if line.starts_with("ruby ") {
                    return line[5..].trim().parse();
                }
            }
        }
        Err(anyhow!("No .ruby-version or .tool-versions found"))
    }

    fn parse_gemfile_lock(&self) -> Result<HashMap<String, String>> {
        // Parse the GEM section of Gemfile.lock
        // Extract lines matching /^\s{4}(\S+) \((\S+)\)/
        // Return map of gem_name → version
        let content = fs::read_to_string(self.project_root.join("Gemfile.lock"))?;
        let mut gems = HashMap::new();
        let re = Regex::new(r"^\s{4}(\S+) \(([^)]+)\)")?;
        for line in content.lines() {
            if let Some(caps) = re.captures(line) {
                gems.insert(caps[1].to_string(), caps[2].to_string());
            }
        }
        Ok(gems)
    }
}
```

## Cop Implementation Batches

### Batch 0: Line-based cops (no AST needed)

These operate on raw text. Fastest to implement, fastest to run.

```
Layout/TrailingWhitespace          → line.ends_with(' ') || line.ends_with('\t')
Layout/LineLength                  → line.len() > config.max (default 120)
Layout/TrailingEmptyLines          → check last lines of file
Layout/LeadingEmptyLines           → check first lines of file
Layout/EmptyLines                  → consecutive blank lines
Layout/EndOfLine                   → check for \r\n
Layout/InitialIndentation          → first non-empty line indentation
Style/FrozenStringLiteralComment   → check first line(s) for magic comment
Style/MagicComment                 → encoding/frozen_string comment format
Lint/ScriptPermission              → check file permissions (fs metadata)
Layout/TrailingBlankLines          → blank lines at end of file
```

Target: 10-12 cops. ~1-2 days.

### Batch 1: Token/simple pattern cops (minimal AST)

These need tokens or very shallow AST checks.

```
Layout/SpaceAfterComma             → token: COMMA followed by non-SPACE
Layout/SpaceAfterColon             → token: COLON followed by non-SPACE
Layout/SpaceAfterSemicolon         → token: SEMI followed by non-SPACE
Layout/SpaceBeforeComma            → token: SPACE followed by COMMA
Layout/SpaceAroundOperators        → tokens around +-*/=<>
Layout/SpaceInsideBlockBraces      → { and } spacing
Layout/SpaceInsideHashLiteralBraces → { and } in hash literals
Layout/SpaceInsideParens           → ( and ) spacing
Layout/SpaceInsideArrayLiteralBrackets
Layout/SpaceBeforeBlockBraces
Layout/SpaceAroundEqualsInParameterDefault
Style/StringLiterals               → single vs double quote preference
Style/SymbolProc                   → { |x| x.foo } → &:foo
Style/RedundantReturn              → explicit return at end of method
Style/RedundantSelf                → self.foo when unnecessary
Lint/Debugger                      → AST match: binding.pry, debugger, byebug, console.log
Lint/UselessAssignment             → assigned but never read
```

Target: 15-18 cops. ~3-4 days.

### Batch 2: AST-walking cops (single node patterns)

These match specific node types in the AST.

```
Metrics/MethodLength               → count lines between def..end
Metrics/ClassLength                → count lines between class..end
Metrics/ModuleLength               → count lines between module..end
Metrics/BlockLength                → count lines in do..end / { }
Metrics/ParameterLists             → count params in def
Metrics/AbcSize                    → assignments + branches + conditions
Metrics/CyclomaticComplexity       → count branches
Metrics/PerceivedComplexity        → weighted branch count
Style/ClassAndModuleChildren       → compact vs nested style
Style/EmptyMethod                  → compact vs expanded empty methods
Style/GuardClause                  → if..return at start of method
Style/IfUnlessModifier             → single-line if/unless as modifier
Style/NegatedIf                    → if !x → unless x
Style/NegatedWhile                 → while !x → until x
Style/Next                         → next instead of wrapping in if
Style/NumericLiterals              → 1_000_000 vs 1000000
Style/ParenthesesAroundCondition   → if (x) → if x
Style/Semicolon                    → no semicolons between statements
Style/TernaryParentheses           → x ? y : z parens
Style/TrailingCommaInArguments     → trailing comma style
Style/TrailingCommaInArrayLiteral
Style/TrailingCommaInHashLiteral
Style/WordArray                    → %w[] vs ['a', 'b']
Style/SymbolArray                  → %i[] vs [:a, :b]
Lint/UnusedMethodArgument          → def foo(x); end (x unused)
Lint/UnusedBlockArgument           → { |x| 42 }
Lint/ShadowingOuterLocalVariable   → block param shadows outer var
Lint/DuplicateMethods              → two def foo in same class
Lint/EmptyConditionalBody          → if x; end
Lint/EmptyWhen                     → when x; (nothing)
Lint/LiteralAsCondition            → if true
Lint/UnreachableCode               → code after return/raise
Lint/Void                          → expression used in void context
Lint/AmbiguousOperator             → ambiguous unary operators
Lint/AmbiguousRegexpLiteral
Lint/BooleanSymbol                 → :true, :false
Lint/DeprecatedClassMethods        → File.exists? → File.exist?
Lint/DuplicateCaseCondition        → same condition in two when clauses
Lint/EachWithObjectArgument        → wrong arg type
Lint/ElseLayout                    → else on same line as last if-body expr
Lint/EnsureReturn                  → return inside ensure
Lint/FloatOutOfRange
Lint/ImplicitStringConcatenation   → "foo" "bar" (accidental)
Lint/IneffectiveAccessModifier     → private on class methods
Lint/Loop                          → begin..end while
Lint/NestedMethodDefinition        → def inside def
Lint/ParenthesesAsGroupedExpression
Lint/RaiseException                → raise Exception (too broad)
Lint/RedundantStringCoercion       → "#{x.to_s}"
Lint/RedundantWithIndex            → each_with_index without index
Lint/RedundantWithObject           → each_with_object not using object
Lint/SafeNavigationConsistency     → x&.foo&.bar consistency
Lint/SendWithMixinArgument
Lint/SuppressedException           → empty rescue
Lint/UnderscorePrefixedVariableName → _x used
Lint/UnifiedInteger                → Fixnum/Bignum → Integer
Lint/UriEscapeUnescape             → URI.escape deprecated
Lint/UriRegexp                     → URI.regexp deprecated
Naming/AccessorMethodName          → get_/set_ prefixes
Naming/AsciiIdentifiers            → non-ASCII in names
Naming/ClassAndModuleCamelCase     → class my_class
Naming/ConstantName                → SCREAMING_SNAKE for constants
Naming/FileName                    → file name matches class/module
Naming/MethodName                  → snake_case methods
Naming/PredicateName               → has_/is_ prefixes
Naming/VariableName                → snake_case variables
```

Target: 60-70 cops. ~2-3 weeks.

### Batch 3: rubocop-performance cops (ALL)

These are almost all simple method-chain AST pattern matches.
Each cop matches a specific call pattern and suggests an alternative.
Most are 20-40 lines of Rust.

```
Performance/AncestorsInclude       → ancestors.include?(X) → is_a?(X)
Performance/ArraySemiInfiniteRangeSlice → arr[n..] → arr.drop(n)
Performance/BigDecimalWithNumericArgument → BigDecimal(2) → BigDecimal('2')
Performance/BindCall               → foo.method(:bar).bind(obj).call → obj.method(:bar).call
Performance/BlockGivenWithExplicitBlock → block_given? when &block param exists
Performance/Caller                 → caller[n] → caller(n..n).first
Performance/CaseWhenSplat          → splat in when → move to end
Performance/Casecmp                → downcase == → casecmp (disabled by default)
Performance/ChainArrayAllocation   → array methods that allocate intermediate arrays
Performance/CollectionLiteralInLoop → literal array/hash in loop → extract to variable
Performance/CompareWithBlock       → sort { |a,b| a.x <=> b.x } → sort_by(&:x)
Performance/ConcurrentMonotonicTime → Concurrent.monotonic_time → Process.clock_gettime
Performance/ConstantRegexp         → /#{CONST}/ in loop → precompile
Performance/Count                  → select { }.count → count { }
Performance/DeletePrefix           → gsub(/\Afoo/, '') → delete_prefix('foo')
Performance/DeleteSuffix           → gsub(/foo\z/, '') → delete_suffix('foo')
Performance/Detect                 → select { }.first → detect { }
Performance/DoubleStartEndWith     → x.start_with?('a') || x.start_with?('b') → x.start_with?('a', 'b')
Performance/EndWith                → match?(/foo\z/) → end_with?('foo')
Performance/FixedSize              → [].length in loop
Performance/FlatMap                → map { }.flatten → flat_map { }
Performance/InefficientHashSearch  → hash.keys.include? → hash.key?
Performance/IoReadlines            → IO.readlines.each → IO.foreach
Performance/MapCompact             → map { }.compact → filter_map { } (Ruby 2.7+)
Performance/MapMethodChain         → map(&:foo).map(&:bar) → map { |x| x.foo.bar }
Performance/MethodObjectAsBlock    → method(:foo) as block → { |x| foo(x) }
Performance/OpenStruct             → OpenStruct is slow
Performance/RangeInclude           → (a..b).include?(x) → (a..b).cover?(x)
Performance/RedundantBlockCall     → block.call → yield
Performance/RedundantEqualityComparisonBlock → select { |x| x == val } → select(val)
Performance/RedundantMatch         → x.match(y) when bool needed → x.match?(y)
Performance/RedundantMerge         → hash.merge!(a: 1) → hash[:a] = 1
Performance/RedundantSortBlock     → sort { |a,b| a <=> b } → sort
Performance/RedundantSplitRegexpArgument → split(/,/) → split(',')
Performance/RedundantStringChars   → string.chars[0] → string[0]
Performance/RegexpMatch            → =~ when bool needed → match?
Performance/ReverseEach            → reverse.each → reverse_each
Performance/ReverseFirst           → reverse.first(n) → last(n).reverse
Performance/SelectMap              → select { }.map { } → filter_map { } (Ruby 2.7+)
Performance/Size                   → .length → .size (or vice versa, configurable)
Performance/SortReverse            → sort.reverse → sort { |a,b| b <=> a }
Performance/Squeeze                → gsub(/a+/, 'a') → squeeze('a')
Performance/StartWith              → match?(/\Afoo/) → start_with?('foo')
Performance/StringIdentifierArgument → send('foo') → send(:foo)
Performance/StringInclude          → match?(/foo/) → include?('foo')
Performance/StringReplacement      → gsub('a', 'b') → tr('a', 'b') (single char)
Performance/Sum                    → inject(0, :+) → sum
Performance/TimesMap               → n.times.map { } → Array.new(n) { }
Performance/UnfreezeString         → String.new('') → +''.dup
Performance/UriDefaultParser       → URI.decode → URI::DEFAULT_PARSER
```

These all follow the same pattern in Rust:

```rust
// Example: Performance/Detect
impl Cop for Detect {
    fn name(&self) -> &'static str { "Performance/Detect" }

    fn check_ast(&self, src: &SourceFile, ast: &Node, cfg: &CopConfig) -> Vec<Diagnostic> {
        // Match: receiver.select { ... }.first
        // Pattern: method_call(method_call(_, :select, block), :first)
        visit_method_chains(ast, |chain| {
            if chain.methods == ["select", "first"] || chain.methods == ["select", "last"] {
                let replacement = if chain.methods[1] == "first" { "detect" } else { "reverse_each.detect" };
                vec![diagnostic(src, chain.loc, format!(
                    "Use `{}` instead of `{}.{}`.",
                    replacement, chain.methods[0], chain.methods[1]
                ))]
            } else { vec![] }
        })
    }
}
```

Target: ~40 cops. ~2-3 days (they're formulaic).

### Batch 4: Complex core cops + remaining core

Things that need slightly more context: multi-node patterns, scope tracking,
cross-method analysis.

```
Style/Documentation                → class without top-level comment
Style/MethodCallWithArgsParentheses → foo(x) vs foo x
Style/HashSyntax                   → { a: 1 } vs { :a => 1 }
Style/Lambda                       → -> vs lambda
Style/Proc                         → Proc.new vs proc
Style/RaiseArgs                    → raise Foo, "msg" vs raise Foo.new("msg")
Style/RescueModifier               → foo rescue bar
Style/RescueStandardError          → rescue => e vs rescue StandardError => e
Style/SignalException              → raise vs fail
Style/SingleLineMethods            → def foo; bar; end
Style/SpecialGlobalVars            → $! vs $ERROR_INFO
Style/StabbyLambdaParentheses      → ->(x) vs -> (x)
Style/YodaCondition                → 5 == x
Layout/IndentationWidth            → needs scope tracking
Layout/IndentationConsistency      → needs scope tracking
Layout/EmptyLineBetweenDefs        → context-aware blank line rules
Layout/EmptyLinesAroundClassBody   → blank lines after class/before end
Layout/EmptyLinesAroundModuleBody
Layout/EmptyLinesAroundMethodBody
Layout/EmptyLinesAroundBlockBody
Layout/MultilineMethodCallIndentation  → chained method indentation
Layout/MultilineOperationIndentation
Layout/FirstArgumentIndentation
Layout/FirstArrayElementIndentation
Layout/FirstHashElementIndentation
Layout/AssignmentIndentation
Layout/CaseIndentation             → when indentation relative to case
Layout/HashAlignment               → key: value alignment
Layout/ArrayAlignment
Layout/ArgumentAlignment
Layout/BlockAlignment
Layout/ConditionPosition           → condition on same line as if?
Layout/DefEndAlignment
Layout/ElseAlignment
Layout/EndAlignment
Layout/RescueEnsureAlignment
```

Target: 40-50 cops. ~2-3 weeks.

## CLI Interface

```
USAGE:
    rubylint [OPTIONS] [PATHS]...

ARGS:
    <PATHS>...    Files or directories to lint [default: .]

OPTIONS:
    -c, --config <PATH>          Path to .rubocop.yml [default: .rubocop.yml]
    -f, --format <FORMAT>        Output format: text, json, emacs, github [default: text]
        --only <COPS>            Run only specified cops (comma-separated)
        --except <COPS>          Skip specified cops (comma-separated)
        --rubocop-only           Print comma-separated list of cops NOT covered by rubylint
        --autocorrect            Auto-fix correctable offenses (later milestone)
        --stdin <PATH>           Read source from stdin, use PATH for display
        --no-color               Disable colorized output
    -j, --jobs <N>               Number of parallel jobs [default: num_cpus]
        --cache-config           Cache resolved config to .rubylint-cache/
        --debug                  Print timing info and cop statistics
    -v, --version
    -h, --help

EXIT CODES:
    0    No offenses found
    1    Offenses found
    2    Error (config not found, parse failure, etc.)
```

## Output Format (matching RuboCop)

```
Inspecting 13027 files
.............C..........W..........

Offenses:

app/models/user.rb:14:5: C: Layout/TrailingWhitespace: Trailing whitespace detected.
    name = user.name   
                    ^^^

app/models/user.rb:45:1: W: Lint/Debugger: Remove debugger entry point `binding.pry`.
    binding.pry
    ^^^^^^^^^^^

13027 files inspected, 2 offenses detected

Finished in 0.43 seconds
```

## Integration: `bin/lint` Wrapper

rubylint is a pure, self-contained binary with zero subprocess calls.
Integration with RuboCop for uncovered cops is handled by a simple
wrapper script.

```bash
#!/usr/bin/env bash
set -euo pipefail

# Step 1: Rust linter handles core + performance cops (<1 second)
rubylint "$@"

# Step 2: RuboCop handles remaining plugin cops
# --rubocop-only outputs the cops rubylint doesn't cover
REMAINING=$(rubylint --rubocop-only)
if [ -n "$REMAINING" ]; then
  bundle exec rubocop --only "$REMAINING" "$@"
fi
```

As rubylint covers more cops (rspec, rails in later milestones),
the REMAINING list shrinks automatically. When rubylint covers
everything except custom cops, the rubocop invocation handles only
those 3 cops — but still pays the 12s Ruby boot tax.

### CI Integration

```yaml
# .github/workflows/lint.yml
- name: Lint
  run: bin/lint .
```

### Local dev (pre-commit hook)

```yaml
# .lefthook.yml
pre-commit:
  commands:
    rubylint:
      glob: "*.rb"
      run: rubylint {staged_files}
      # rubylint only — sub-second feedback on changed files
      # Full plugin cop coverage happens in CI via bin/lint
```

### How --rubocop-only works

rubylint reads the resolved .rubocop.yml, identifies all enabled cops,
subtracts the ones it natively implements, and outputs the remainder as
a comma-separated list.

```bash
$ rubylint --rubocop-only
RSpec/ExampleLength,RSpec/NestedGroups,RSpec/MultipleExpectations,
Rails/HttpPositionalArguments,Rails/FindEach,Rails/HasManyOrHasOneDependent,
Vendor/StrictDryStruct,Custom/NoRailsHelperRequire,...

$ rubylint --rubocop-only | tr ',' '\n' | wc -l
168
```

## Milestone Summary

| Milestone | Cops | Cumulative | Wall time | Target |
|-----------|------|------------|-----------|--------|
| M0: Skeleton | 0 | 0 | Week 1 | Parse 13k files with Prism, config resolution works |
| M1: Line-based | 12 | 12 | Week 2 | <500ms on full repo |
| M2: Token cops | 18 | 30 | Week 2-3 | Layout/Style basics covered |
| M3: AST single-node | 70 | 100 | Week 4-6 | Lint, Metrics, Naming, most Style |
| M4: Performance cops | 40 | 140 | Week 6 | ALL rubocop-performance cops (2-3 days) |
| M5: AST complex | 50 | 190 | Week 7-9 | Indentation, alignment, multiline |
| M6: bin/lint + --rubocop-only | 0 new | 190 | Week 9-10 | Hybrid CI mode works end-to-end |
| M7: Autocorrect | +30 fixes | 190 | Week 10-12 | --autocorrect for easy cops |
| M8: rubocop-rspec | 113 | 364 | **Done** | All 113 rubocop-rspec cops ported |
| M9: rubocop-rails | 98 | 251→364 | **Done** (was M6) | All 98 rubocop-rails cops ported in M6 |

**M6 is the "ship it" milestone.** At ~190 cops (core + performance) with
bin/lint hybrid mode, you can deploy to CI across all repos. RuboCop still
runs for rspec + rails + vendor + custom cops. Zero risk, immediate speedup.

**M8 and M9 are high priority, not optional.** The 12-second RuboCop floor
means every plugin ported to Rust brings you closer to eliminating the
Ruby boot tax entirely. After M9, RuboCop only runs ~18 cops (vendor +
custom) — and you're one small step from dropping it completely.

## Performance Targets

| Metric | RuboCop (current) | After M6 | After M9 | After full port |
|--------|-------------------|----------|----------|-----------------|
| Full CI lint, 13k files | 42s | ~13-16s | ~13s (12s floor) | <1s |
| rubylint portion | n/a | <1s | <1s | <1s |
| Single file (editor) | 2-3s (boot) | <50ms | <50ms | <50ms |
| Memory | ~1GB+ | ~1.2GB combined | ~1.2GB combined | <200MB |

Note: The 12s floor persists as long as ANY rubocop invocation is needed.
The only way to break through 12s is to port all enabled plugin cops so
RuboCop is no longer invoked at all.

## Testing Strategy

Each cop gets a pair of test fixtures:

```
testdata/cops/layout/trailing_whitespace/
├── offense.rb           # Contains violations with annotations
└── no_offense.rb        # Clean file, should produce zero diagnostics
```

Offense files use annotation comments (same pattern as RuboCop's specs):

```ruby
# testdata/cops/layout/trailing_whitespace/offense.rb
x = 1   
#     ^^ Layout/TrailingWhitespace: Trailing whitespace detected.
y = 2
z = 3	
#    ^ Layout/TrailingWhitespace: Trailing whitespace detected.
```

Test runner parses annotations, runs the cop, asserts diagnostics match.

Additionally:

- **Conformance test**: Run rubylint + rubocop on the same codebase,
  diff the results for cops that rubylint covers. Zero diff = passing.
  Run this on rubylint's own test fixtures AND on real repos.

- **Performance benchmark**: `hyperfine 'rubylint .' 'rubocop .'` on
  a large monorepo, tracked in CI.

## Open Questions

1. **Prism FFI complexity**: Prism's Rust bindings require linking to the
   Prism C library. This adds a build step but is well-documented. The
   `prism` crate handles most of this. Worth spiking in M0 to validate
   the build/parse workflow before writing any cops.

2. **Autocorrect safety**: RuboCop's autocorrect can chain corrections that
   conflict. Start with "safe" autocorrects only (whitespace, quotes, comments).

3. **Name**: rubylint, rblint, rb-ruff, ruff-rb, rubocop-rs, fastcop?
   Should signal "fast rubocop alternative" without trademark issues.

4. **Config caching**: Hash the resolved config and cache it. Invalidate on:
   .rubocop.yml mtime change, Gemfile.lock mtime change, style gem version
   change. This makes subsequent runs skip all config resolution.

5. **LSP server**: Eventually serve diagnostics via LSP for editor integration.
   This is where the <50ms single-file target matters most.

6. **Conformance testing at scale**: Run rubylint + rubocop on all repos
   in CI to catch any behavioral differences. The --rubocop-only flag
   makes this easy: if rubylint reports offense X and rubocop (running all cops)
   also reports offense X, they agree. Any discrepancy is a bug.

7. **Config resolution brittleness**: Pure Rust gem path detection covers
   mise/asdf/rbenv + GEM_HOME env var. For unusual environments (Docker,
   global Ruby, chruby, rvm, git/path gems in Gemfile), users add manual
   gem_paths to .rubylint.yml. If this proves too brittle across many repos,
   add a `rubylint --resolve-config` command that shells out to
   `ruby -e 'puts Gem::Specification.find_by_name("gem").gem_dir'` once
   and caches the result.

8. **Eliminating the 12s floor**: The ultimate goal is to port enough cops
   that RuboCop is no longer invoked. At that point, bin/lint is just
   `rubylint .` and CI lint drops from 42s to <1s. Track progress toward
   this by monitoring the output of `rubylint --rubocop-only | wc -l`.
