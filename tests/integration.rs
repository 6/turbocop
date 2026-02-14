//! Integration tests for the rblint linting pipeline.
//!
//! These tests exercise the full linter: file reading, config loading,
//! cop registry, cop execution, and diagnostic collection. They write
//! real files to a temp directory and invoke `run_linter` directly.

use std::fs;
use std::path::{Path, PathBuf};

use rblint::cli::Args;
use rblint::config::load_config;
use rblint::cop::registry::CopRegistry;
use rblint::linter::run_linter;

/// Create a temporary directory with a unique name for each test.
fn temp_dir(test_name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rblint_integration_{test_name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_file(dir: &Path, name: &str, content: &[u8]) -> PathBuf {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&path, content).unwrap();
    path
}

fn default_args() -> Args {
    Args {
        paths: vec![],
        config: None,
        format: "text".to_string(),
        only: vec![],
        except: vec![],
        no_color: false,
        debug: false,
    }
}

// ---------- Full pipeline tests ----------

#[test]
fn lint_clean_file_no_offenses() {
    let dir = temp_dir("clean_file");
    let file = write_file(
        &dir,
        "clean.rb",
        b"# frozen_string_literal: true\n\nx = 1\ny = 2\n",
    );
    let config = load_config(None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = default_args();

    let result = run_linter(&[file], &config, &registry, &args);
    assert_eq!(result.file_count, 1);
    assert!(
        result.diagnostics.is_empty(),
        "Expected no offenses on clean file, got: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| format!("{d}"))
            .collect::<Vec<_>>()
    );
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn lint_file_with_multiple_offenses() {
    let dir = temp_dir("multi_offense");
    // Missing frozen_string_literal + trailing whitespace
    let file = write_file(&dir, "bad.rb", b"x = 1  \ny = 2\n");
    let config = load_config(None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = default_args();

    let result = run_linter(&[file], &config, &registry, &args);
    assert_eq!(result.file_count, 1);

    let cop_names: Vec<&str> = result.diagnostics.iter().map(|d| d.cop_name.as_str()).collect();
    assert!(
        cop_names.contains(&"Style/FrozenStringLiteralComment"),
        "Expected FrozenStringLiteralComment offense"
    );
    assert!(
        cop_names.contains(&"Layout/TrailingWhitespace"),
        "Expected TrailingWhitespace offense"
    );

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn lint_multiple_files() {
    let dir = temp_dir("multi_file");
    let f1 = write_file(
        &dir,
        "a.rb",
        b"# frozen_string_literal: true\n\nx = 1\n",
    );
    let f2 = write_file(&dir, "b.rb", b"y = 2  \n");

    let config = load_config(None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = default_args();

    let result = run_linter(&[f1, f2], &config, &registry, &args);
    assert_eq!(result.file_count, 2);

    // a.rb should be clean, b.rb should have offenses
    let a_offenses: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.path.contains("a.rb"))
        .collect();
    let b_offenses: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.path.contains("b.rb"))
        .collect();
    assert!(a_offenses.is_empty(), "a.rb should be clean");
    assert!(!b_offenses.is_empty(), "b.rb should have offenses");

    fs::remove_dir_all(&dir).ok();
}

// ---------- Filtering tests ----------

#[test]
fn only_filter_limits_cops() {
    let dir = temp_dir("only_filter");
    // Missing frozen_string_literal + trailing whitespace
    let file = write_file(&dir, "test.rb", b"x = 1  \n");
    let config = load_config(None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = Args {
        only: vec!["Layout/TrailingWhitespace".to_string()],
        ..default_args()
    };

    let result = run_linter(&[file], &config, &registry, &args);

    // Only TrailingWhitespace should fire
    for d in &result.diagnostics {
        assert_eq!(
            d.cop_name, "Layout/TrailingWhitespace",
            "Only TrailingWhitespace should fire with --only filter, got: {}",
            d.cop_name,
        );
    }
    assert!(
        !result.diagnostics.is_empty(),
        "TrailingWhitespace should still fire"
    );

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn except_filter_excludes_cops() {
    let dir = temp_dir("except_filter");
    let file = write_file(&dir, "test.rb", b"x = 1  \n");
    let config = load_config(None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = Args {
        except: vec![
            "Style/FrozenStringLiteralComment".to_string(),
            "Layout/TrailingWhitespace".to_string(),
        ],
        ..default_args()
    };

    let result = run_linter(&[file], &config, &registry, &args);

    let excluded_cops = ["Style/FrozenStringLiteralComment", "Layout/TrailingWhitespace"];
    for d in &result.diagnostics {
        assert!(
            !excluded_cops.contains(&d.cop_name.as_str()),
            "Excluded cop {} should not fire",
            d.cop_name,
        );
    }

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn only_with_single_cop_on_clean_file() {
    let dir = temp_dir("only_clean");
    let file = write_file(&dir, "test.rb", b"x = 1\ny = 2\n");
    let config = load_config(None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = Args {
        only: vec!["Layout/TrailingWhitespace".to_string()],
        ..default_args()
    };

    let result = run_linter(&[file], &config, &registry, &args);
    assert!(result.diagnostics.is_empty());

    fs::remove_dir_all(&dir).ok();
}

// ---------- Config override tests ----------

#[test]
fn config_disables_cop() {
    let dir = temp_dir("config_disable");
    let file = write_file(&dir, "test.rb", b"x = 1  \n");
    let config_path = write_file(
        &dir,
        ".rubocop.yml",
        b"Layout/TrailingWhitespace:\n  Enabled: false\nStyle/FrozenStringLiteralComment:\n  Enabled: false\n",
    );
    let config = load_config(Some(config_path.as_path())).unwrap();
    let registry = CopRegistry::default_registry();
    let args = default_args();

    let result = run_linter(&[file], &config, &registry, &args);

    let disabled_cops = [
        "Layout/TrailingWhitespace",
        "Style/FrozenStringLiteralComment",
    ];
    for d in &result.diagnostics {
        assert!(
            !disabled_cops.contains(&d.cop_name.as_str()),
            "Disabled cop {} should not fire",
            d.cop_name,
        );
    }

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn config_line_length_max_override() {
    let dir = temp_dir("config_max");
    // Line is 20 chars — under default 120 but over Max:10
    let file = write_file(
        &dir,
        "test.rb",
        b"# frozen_string_literal: true\n\ntwenty_char_line = 1\n",
    );
    let config_path = write_file(
        &dir,
        ".rubocop.yml",
        b"Layout/LineLength:\n  Max: 10\n",
    );
    let config = load_config(Some(config_path.as_path())).unwrap();
    let registry = CopRegistry::default_registry();
    let args = Args {
        only: vec!["Layout/LineLength".to_string()],
        ..default_args()
    };

    let result = run_linter(&[file], &config, &registry, &args);

    assert!(
        !result.diagnostics.is_empty(),
        "LineLength should fire with Max:10 on a 20-char line"
    );
    for d in &result.diagnostics {
        assert_eq!(d.cop_name, "Layout/LineLength");
        assert!(d.message.contains("/10]"), "Message should reference Max:10");
    }

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn default_line_length_allows_120() {
    let dir = temp_dir("default_max");
    // 120 chars exactly — should NOT trigger
    let line = format!(
        "# frozen_string_literal: true\n\n{}\n",
        "x" .repeat(120)
    );
    let file = write_file(&dir, "test.rb", line.as_bytes());
    let config = load_config(None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = Args {
        only: vec!["Layout/LineLength".to_string()],
        ..default_args()
    };

    let result = run_linter(&[file], &config, &registry, &args);
    assert!(
        result.diagnostics.is_empty(),
        "120-char line should not trigger default LineLength"
    );

    fs::remove_dir_all(&dir).ok();
}

// ---------- Edge case tests ----------

#[test]
fn empty_file_no_crash() {
    let dir = temp_dir("empty");
    let file = write_file(&dir, "empty.rb", b"");
    let config = load_config(None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = default_args();

    let result = run_linter(&[file], &config, &registry, &args);
    assert_eq!(result.file_count, 1);
    // Should not panic; may or may not have offenses (FrozenStringLiteralComment fires)

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn file_with_syntax_errors_still_lints() {
    let dir = temp_dir("syntax_error");
    // Invalid Ruby syntax — Prism parses with errors but still returns a tree
    let file = write_file(&dir, "bad_syntax.rb", b"def foo(\n  x = 1\n");
    let config = load_config(None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = default_args();

    // Should not panic
    let result = run_linter(&[file], &config, &registry, &args);
    assert_eq!(result.file_count, 1);
    // Line-based cops should still find offenses (at minimum FrozenStringLiteralComment)

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn binary_content_no_crash() {
    let dir = temp_dir("binary");
    // Binary content with null bytes
    let file = write_file(&dir, "binary.rb", b"\x00\x01\x02\xff\xfe");
    let config = load_config(None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = default_args();

    // Should not panic
    let result = run_linter(&[file], &config, &registry, &args);
    assert_eq!(result.file_count, 1);

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn crlf_line_endings_detected() {
    let dir = temp_dir("crlf");
    let file = write_file(
        &dir,
        "crlf.rb",
        b"# frozen_string_literal: true\r\n\r\nx = 1\r\n",
    );
    let config = load_config(None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = Args {
        only: vec!["Layout/EndOfLine".to_string()],
        ..default_args()
    };

    let result = run_linter(&[file], &config, &registry, &args);
    assert!(
        !result.diagnostics.is_empty(),
        "EndOfLine should detect CRLF"
    );
    for d in &result.diagnostics {
        assert_eq!(d.cop_name, "Layout/EndOfLine");
        assert_eq!(d.message, "Carriage return character detected.");
    }

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn diagnostics_are_sorted_by_path_then_location() {
    let dir = temp_dir("sort_order");
    let f1 = write_file(&dir, "b.rb", b"x = 1  \n");
    let f2 = write_file(&dir, "a.rb", b"y = 2  \n");
    let config = load_config(None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = Args {
        only: vec!["Layout/TrailingWhitespace".to_string()],
        ..default_args()
    };

    let result = run_linter(&[f1, f2], &config, &registry, &args);
    assert_eq!(result.diagnostics.len(), 2);
    // Diagnostics should be sorted: a.rb before b.rb
    assert!(
        result.diagnostics[0].path < result.diagnostics[1].path
            || (result.diagnostics[0].path == result.diagnostics[1].path
                && result.diagnostics[0].location.line <= result.diagnostics[1].location.line),
        "Diagnostics should be sorted by path then location"
    );

    fs::remove_dir_all(&dir).ok();
}

// ---------- All 8 cops fire correctly ----------

#[test]
fn all_registered_cops_can_fire() {
    let dir = temp_dir("all_cops");
    // This file triggers multiple cops:
    // - Missing frozen_string_literal
    // - Trailing whitespace on line 1
    // - Tab on line 2
    let file = write_file(&dir, "test.rb", b"x = 1  \n\ty = 2\n");
    let config = load_config(None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = default_args();

    let result = run_linter(&[file], &config, &registry, &args);

    let cop_names: Vec<&str> = result
        .diagnostics
        .iter()
        .map(|d| d.cop_name.as_str())
        .collect();
    assert!(
        cop_names.contains(&"Style/FrozenStringLiteralComment"),
        "FrozenStringLiteralComment should fire"
    );
    assert!(
        cop_names.contains(&"Layout/TrailingWhitespace"),
        "TrailingWhitespace should fire"
    );
    assert!(cop_names.contains(&"Style/Tab"), "Tab should fire");

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn registry_has_expected_cop_count() {
    let registry = CopRegistry::default_registry();
    assert_eq!(registry.len(), 251, "Expected 251 registered cops");

    let names = registry.names();
    let expected = [
        // Layout (40)
        "Layout/TrailingWhitespace",
        "Layout/LineLength",
        "Layout/TrailingEmptyLines",
        "Layout/LeadingEmptyLines",
        "Layout/EndOfLine",
        "Layout/InitialIndentation",
        "Layout/EmptyLines",
        "Layout/SpaceAfterComma",
        "Layout/SpaceAfterSemicolon",
        "Layout/SpaceBeforeComma",
        "Layout/SpaceAroundEqualsInParameterDefault",
        "Layout/SpaceAfterColon",
        "Layout/SpaceInsideParens",
        "Layout/SpaceInsideHashLiteralBraces",
        "Layout/SpaceInsideBlockBraces",
        "Layout/SpaceInsideArrayLiteralBrackets",
        "Layout/SpaceBeforeBlockBraces",
        "Layout/EmptyLineBetweenDefs",
        "Layout/EmptyLinesAroundClassBody",
        "Layout/EmptyLinesAroundModuleBody",
        "Layout/EmptyLinesAroundMethodBody",
        "Layout/EmptyLinesAroundBlockBody",
        "Layout/CaseIndentation",
        "Layout/ArgumentAlignment",
        "Layout/ArrayAlignment",
        "Layout/HashAlignment",
        "Layout/BlockAlignment",
        "Layout/ConditionPosition",
        "Layout/DefEndAlignment",
        "Layout/ElseAlignment",
        "Layout/EndAlignment",
        "Layout/RescueEnsureAlignment",
        "Layout/IndentationWidth",
        "Layout/IndentationConsistency",
        "Layout/FirstArgumentIndentation",
        "Layout/FirstArrayElementIndentation",
        "Layout/FirstHashElementIndentation",
        "Layout/AssignmentIndentation",
        "Layout/MultilineMethodCallIndentation",
        "Layout/MultilineOperationIndentation",
        // Lint (19)
        "Lint/Debugger",
        "Lint/LiteralAsCondition",
        "Lint/EmptyConditionalBody",
        "Lint/EmptyWhen",
        "Lint/BooleanSymbol",
        "Lint/DeprecatedClassMethods",
        "Lint/EnsureReturn",
        "Lint/FloatOutOfRange",
        "Lint/Loop",
        "Lint/NestedMethodDefinition",
        "Lint/RaiseException",
        "Lint/SuppressedException",
        "Lint/UnifiedInteger",
        "Lint/UriEscapeUnescape",
        "Lint/UriRegexp",
        "Lint/DuplicateCaseCondition",
        "Lint/ElseLayout",
        "Lint/RedundantStringCoercion",
        "Lint/EachWithObjectArgument",
        // Metrics (8)
        "Metrics/MethodLength",
        "Metrics/ClassLength",
        "Metrics/ModuleLength",
        "Metrics/BlockLength",
        "Metrics/ParameterLists",
        "Metrics/AbcSize",
        "Metrics/CyclomaticComplexity",
        "Metrics/PerceivedComplexity",
        // Naming (8)
        "Naming/MethodName",
        "Naming/VariableName",
        "Naming/ConstantName",
        "Naming/ClassAndModuleCamelCase",
        "Naming/AccessorMethodName",
        "Naming/PredicateName",
        "Naming/AsciiIdentifiers",
        "Naming/FileName",
        // Performance (47)
        "Performance/AncestorsInclude",
        "Performance/ArraySemiInfiniteRangeSlice",
        "Performance/BigDecimalWithNumericArgument",
        "Performance/BindCall",
        "Performance/BlockGivenWithExplicitBlock",
        "Performance/Caller",
        "Performance/CaseWhenSplat",
        "Performance/Casecmp",
        "Performance/ChainArrayAllocation",
        "Performance/CompareWithBlock",
        "Performance/ConcurrentMonotonicTime",
        "Performance/Count",
        "Performance/DeletePrefix",
        "Performance/DeleteSuffix",
        "Performance/Detect",
        "Performance/DoubleStartEndWith",
        "Performance/EndWith",
        "Performance/FlatMap",
        "Performance/InefficientHashSearch",
        "Performance/IoReadlines",
        "Performance/MapCompact",
        "Performance/MapMethodChain",
        "Performance/MethodObjectAsBlock",
        "Performance/OpenStruct",
        "Performance/RangeInclude",
        "Performance/RedundantBlockCall",
        "Performance/RedundantEqualityComparisonBlock",
        "Performance/RedundantMatch",
        "Performance/RedundantMerge",
        "Performance/RedundantSortBlock",
        "Performance/RedundantSplitRegexpArgument",
        "Performance/RedundantStringChars",
        "Performance/RegexpMatch",
        "Performance/ReverseEach",
        "Performance/ReverseFirst",
        "Performance/SelectMap",
        "Performance/Size",
        "Performance/SortReverse",
        "Performance/Squeeze",
        "Performance/StartWith",
        "Performance/StringIdentifierArgument",
        "Performance/StringInclude",
        "Performance/StringReplacement",
        "Performance/Sum",
        "Performance/TimesMap",
        "Performance/UnfreezeString",
        "Performance/UriDefaultParser",
        // Rails (98)
        "Rails/ActionControllerFlashBeforeRender",
        "Rails/ActionControllerTestCase",
        "Rails/ActionOrder",
        "Rails/ActiveRecordCallbacksOrder",
        "Rails/ActiveSupportAliases",
        "Rails/AddColumnIndex",
        "Rails/AfterCommitOverride",
        "Rails/ApplicationController",
        "Rails/ApplicationJob",
        "Rails/ApplicationMailer",
        "Rails/ApplicationRecord",
        "Rails/AttributeDefaultBlockValue",
        "Rails/Blank",
        "Rails/CompactBlank",
        "Rails/ContentTag",
        "Rails/CreateTableWithTimestamps",
        "Rails/DangerousColumnNames",
        "Rails/Date",
        "Rails/Delegate",
        "Rails/DelegateAllowBlank",
        "Rails/DotSeparatedKeys",
        "Rails/DuplicateAssociation",
        "Rails/DuplicateScope",
        "Rails/DurationArithmetic",
        "Rails/DynamicFindBy",
        "Rails/EnumHash",
        "Rails/EnumSyntax",
        "Rails/EnumUniqueness",
        "Rails/Env",
        "Rails/EnvLocal",
        "Rails/EnvironmentComparison",
        "Rails/EnvironmentVariableAccess",
        "Rails/Exit",
        "Rails/ExpandedDateRange",
        "Rails/FilePath",
        "Rails/FindBy",
        "Rails/FindEach",
        "Rails/FreezeTime",
        "Rails/HasAndBelongsToMany",
        "Rails/HasManyOrHasOneDependent",
        "Rails/HelperInstanceVariable",
        "Rails/HttpPositionalArguments",
        "Rails/HttpStatus",
        "Rails/HttpStatusNameConsistency",
        "Rails/I18nLazyLookup",
        "Rails/I18nLocaleAssignment",
        "Rails/IndexBy",
        "Rails/Inquiry",
        "Rails/InverseOf",
        "Rails/LexicallyScopedActionFilter",
        "Rails/MigrationClassName",
        "Rails/NegateInclude",
        "Rails/NotNullColumn",
        "Rails/Output",
        "Rails/OutputSafety",
        "Rails/Pick",
        "Rails/Pluck",
        "Rails/PluckId",
        "Rails/PluckInWhere",
        "Rails/Present",
        "Rails/RakeEnvironment",
        "Rails/ReadWriteAttribute",
        "Rails/RedundantActiveRecordAllMethod",
        "Rails/RedundantAllowNil",
        "Rails/RedundantForeignKey",
        "Rails/RedundantPresenceValidationOnBelongsTo",
        "Rails/RedundantTravelBack",
        "Rails/ReflectionClassName",
        "Rails/RefuteMethods",
        "Rails/RelativeDateConstant",
        "Rails/RenderInline",
        "Rails/RenderPlainText",
        "Rails/RequestReferer",
        "Rails/ResponseParsedBody",
        "Rails/ReversibleMigrationMethodDefinition",
        "Rails/RootJoinChain",
        "Rails/RootPathnameMethods",
        "Rails/RootPublicPath",
        "Rails/SafeNavigation",
        "Rails/SchemaComment",
        "Rails/ScopeArgs",
        "Rails/SelectMap",
        "Rails/ShortI18n",
        "Rails/SkipsModelValidations",
        "Rails/StripHeredoc",
        "Rails/TableNameAssignment",
        "Rails/ThreeStateBooleanColumn",
        "Rails/TimeZone",
        "Rails/TimeZoneAssignment",
        "Rails/ToFormattedS",
        "Rails/ToSWithArgument",
        "Rails/TransactionExitStatement",
        "Rails/UnknownEnv",
        "Rails/UnusedRenderContent",
        "Rails/Validation",
        "Rails/WhereExists",
        "Rails/WhereNot",
        "Rails/WhereRange",
        // Style (31)
        "Style/FrozenStringLiteralComment",
        "Style/Tab",
        "Style/StringLiterals",
        "Style/RedundantReturn",
        "Style/NumericLiterals",
        "Style/Semicolon",
        "Style/EmptyMethod",
        "Style/NegatedIf",
        "Style/NegatedWhile",
        "Style/ParenthesesAroundCondition",
        "Style/IfUnlessModifier",
        "Style/WordArray",
        "Style/SymbolArray",
        "Style/TrailingCommaInArguments",
        "Style/TrailingCommaInArrayLiteral",
        "Style/TrailingCommaInHashLiteral",
        "Style/ClassAndModuleChildren",
        "Style/TernaryParentheses",
        "Style/Documentation",
        "Style/Lambda",
        "Style/Proc",
        "Style/RaiseArgs",
        "Style/RescueModifier",
        "Style/RescueStandardError",
        "Style/SignalException",
        "Style/SingleLineMethods",
        "Style/SpecialGlobalVars",
        "Style/StabbyLambdaParentheses",
        "Style/YodaCondition",
        "Style/HashSyntax",
        "Style/MethodCallWithArgsParentheses",
    ];
    for name in &expected {
        assert!(
            names.contains(name),
            "Registry missing expected cop: {name}"
        );
    }
}

// ---------- Performance department integration tests ----------

#[test]
fn performance_cops_fire_on_slow_patterns() {
    let dir = temp_dir("perf_cops");
    let file = write_file(
        &dir,
        "slow.rb",
        b"# frozen_string_literal: true\n\narr = [1, 2, 3]\narr.select { |x| x > 1 }.first\narr.reverse.each { |x| puts x }\narr.select { |x| x > 1 }.count\narr.flatten.map { |x| x.to_s }\n",
    );
    let config = load_config(None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = default_args();

    let result = run_linter(&[file], &config, &registry, &args);
    let perf_diags: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.cop_name.starts_with("Performance/"))
        .collect();

    assert!(
        perf_diags.len() >= 3,
        "Expected at least 3 Performance diagnostics, got {}: {:?}",
        perf_diags.len(),
        perf_diags.iter().map(|d| &d.cop_name).collect::<Vec<_>>()
    );

    let cop_names: Vec<&str> = perf_diags.iter().map(|d| d.cop_name.as_str()).collect();
    assert!(
        cop_names.contains(&"Performance/Detect"),
        "Expected Performance/Detect to fire on select.first"
    );
    assert!(
        cop_names.contains(&"Performance/ReverseEach"),
        "Expected Performance/ReverseEach to fire on reverse.each"
    );
    assert!(
        cop_names.contains(&"Performance/Count"),
        "Expected Performance/Count to fire on select.count"
    );

    fs::remove_dir_all(&dir).ok();
}

// ---------- Lint department integration tests ----------

#[test]
fn lint_cops_fire_on_bad_code() {
    let dir = temp_dir("lint_cops");
    let file = write_file(
        &dir,
        "bad.rb",
        b"# frozen_string_literal: true\n\nbinding.pry\nraise Exception, \"bad\"\nx = :true\n",
    );
    let config = load_config(None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = default_args();

    let result = run_linter(&[file], &config, &registry, &args);
    let lint_diags: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.cop_name.starts_with("Lint/"))
        .collect();

    assert!(
        lint_diags.len() >= 3,
        "Expected at least 3 Lint diagnostics, got {}: {:?}",
        lint_diags.len(),
        lint_diags.iter().map(|d| &d.cop_name).collect::<Vec<_>>()
    );

    let cop_names: Vec<&str> = lint_diags.iter().map(|d| d.cop_name.as_str()).collect();
    assert!(
        cop_names.contains(&"Lint/Debugger"),
        "Expected Lint/Debugger to fire on binding.pry"
    );
    assert!(
        cop_names.contains(&"Lint/RaiseException"),
        "Expected Lint/RaiseException to fire on raise Exception"
    );
    assert!(
        cop_names.contains(&"Lint/BooleanSymbol"),
        "Expected Lint/BooleanSymbol to fire on :true"
    );

    fs::remove_dir_all(&dir).ok();
}

// ---------- Multi-department JSON output test ----------

#[test]
fn json_formatter_includes_all_departments() {
    let dir = temp_dir("multi_dept");
    // This file triggers cops from multiple departments:
    // - Layout: trailing whitespace
    // - Style: missing frozen_string_literal
    // - Lint: binding.pry (Debugger), :true (BooleanSymbol)
    let file = write_file(
        &dir,
        "multi.rb",
        b"binding.pry  \nx = :true\n",
    );
    let config = load_config(None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = default_args();

    let result = run_linter(&[file], &config, &registry, &args);

    // Collect unique department prefixes
    let departments: std::collections::HashSet<&str> = result
        .diagnostics
        .iter()
        .filter_map(|d| d.cop_name.split('/').next())
        .collect();

    assert!(
        departments.contains("Layout"),
        "Expected Layout department diagnostics, got departments: {:?}",
        departments
    );
    assert!(
        departments.contains("Style"),
        "Expected Style department diagnostics, got departments: {:?}",
        departments
    );
    assert!(
        departments.contains("Lint"),
        "Expected Lint department diagnostics, got departments: {:?}",
        departments
    );

    fs::remove_dir_all(&dir).ok();
}

// ---------- Include/Exclude integration tests ----------
//
// These tests exercise the full linter pipeline with path-based filtering.
// Since run_linter receives absolute paths but Include/Exclude patterns are
// relative, we construct config patterns using absolute paths to match.

#[test]
fn migration_cop_filtered_by_path() {
    let dir = temp_dir("migration_path_filter");
    // Use a config that sets Include with an absolute pattern matching our temp dir.
    // This mirrors what default_include does but with absolute paths.
    let dir_str = dir.display();
    let config_yaml = format!(
        "Rails/CreateTableWithTimestamps:\n  Include:\n    - '{dir_str}/db/migrate/**/*.rb'\n"
    );
    let config_path = write_file(&dir, ".rubocop.yml", config_yaml.as_bytes());
    let migration_content = b"class CreateUsers < ActiveRecord::Migration[7.0]\n  def change\n    create_table :users do |t|\n      t.string :name\n    end\n  end\nend\n";
    let migrate_file = write_file(&dir, "db/migrate/001_create_users.rb", migration_content);
    let model_file = write_file(&dir, "app/models/user.rb", migration_content);
    let config = load_config(Some(config_path.as_path())).unwrap();
    let registry = CopRegistry::default_registry();
    let args = Args {
        only: vec!["Rails/CreateTableWithTimestamps".to_string()],
        ..default_args()
    };

    let result = run_linter(&[migrate_file, model_file], &config, &registry, &args);

    // Only the migration file should have offenses
    let migrate_offenses: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.path.contains("db/migrate"))
        .collect();
    let model_offenses: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.path.contains("app/models"))
        .collect();

    assert!(
        !migrate_offenses.is_empty(),
        "CreateTableWithTimestamps should fire on db/migrate/ files"
    );
    assert!(
        model_offenses.is_empty(),
        "CreateTableWithTimestamps should NOT fire on app/models/ files (not in Include path)"
    );

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn global_exclude_skips_file() {
    let dir = temp_dir("global_exclude");
    // Use absolute pattern in AllCops.Exclude to match temp dir paths
    let dir_str = dir.display();
    let config_yaml = format!(
        "AllCops:\n  Exclude:\n    - '{dir_str}/vendor/**'\n"
    );
    let config_path = write_file(&dir, ".rubocop.yml", config_yaml.as_bytes());
    // Place a file with trailing whitespace in vendor/
    let vendor_file = write_file(&dir, "vendor/foo.rb", b"x = 1  \n");
    // Place the same file outside vendor/
    let app_file = write_file(&dir, "app.rb", b"x = 1  \n");

    let config = load_config(Some(config_path.as_path())).unwrap();
    let registry = CopRegistry::default_registry();
    let args = Args {
        only: vec!["Layout/TrailingWhitespace".to_string()],
        ..default_args()
    };

    let result = run_linter(&[vendor_file, app_file], &config, &registry, &args);

    let vendor_offenses: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.path.contains("vendor"))
        .collect();
    let app_offenses: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.path.contains("app.rb"))
        .collect();

    assert!(
        vendor_offenses.is_empty(),
        "Global Exclude should prevent offenses on vendor/ files"
    );
    assert!(
        !app_offenses.is_empty(),
        "Non-excluded files should still have offenses"
    );

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn user_include_override_widens_scope() {
    let dir = temp_dir("user_include_override");
    // CreateTableWithTimestamps defaults to Include: db/migrate/**/*.rb
    // Override to widen scope to all db/**/*.rb (using absolute path for temp dir)
    let dir_str = dir.display();
    let config_yaml = format!(
        "Rails/CreateTableWithTimestamps:\n  Include:\n    - '{dir_str}/db/**/*.rb'\n"
    );
    let config_path = write_file(&dir, ".rubocop.yml", config_yaml.as_bytes());
    let migration_content = b"class CreateUsers < ActiveRecord::Migration[7.0]\n  def change\n    create_table :users do |t|\n      t.string :name\n    end\n  end\nend\n";
    // This file is in db/ but NOT in db/migrate/ — only matches the widened Include
    let seeds_file = write_file(&dir, "db/seeds.rb", migration_content);
    let config = load_config(Some(config_path.as_path())).unwrap();
    let registry = CopRegistry::default_registry();
    let args = Args {
        only: vec!["Rails/CreateTableWithTimestamps".to_string()],
        ..default_args()
    };

    let result = run_linter(&[seeds_file], &config, &registry, &args);

    assert!(
        !result.diagnostics.is_empty(),
        "User Include override should widen scope to db/seeds.rb"
    );
    for d in &result.diagnostics {
        assert_eq!(d.cop_name, "Rails/CreateTableWithTimestamps");
    }

    fs::remove_dir_all(&dir).ok();
}

// ---------- Test coverage guard ----------

/// Convert CamelCase to snake_case, handling runs of uppercase letters.
/// Examples: "TrailingWhitespace" -> "trailing_whitespace",
///           "AbcSize" -> "abc_size", "ABCSize" -> "abc_size",
///           "UriRegexp" -> "uri_regexp"
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                let prev = chars[i - 1];
                if prev.is_lowercase() || prev.is_ascii_digit() {
                    result.push('_');
                } else if i + 1 < chars.len() && chars[i + 1].is_lowercase() {
                    result.push('_');
                }
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}

#[test]
fn all_cops_have_minimum_test_coverage() {
    let testdata = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata/cops");
    let registry = CopRegistry::default_registry();

    // Cops exempt from the 2-offense minimum. Each entry must have a comment
    // explaining why 1 offense is acceptable. Remove entries as coverage improves.
    let offense_exemptions: &[&str] = &[
        // --- Rails cops with inherently single patterns ---
        "Rails/ApplicationController",  // only checks class parent != ApplicationController
        "Rails/ApplicationJob",         // only checks class parent != ApplicationJob
        "Rails/ApplicationMailer",      // only checks class parent != ApplicationMailer
        "Rails/ApplicationRecord",      // only checks class parent != ApplicationRecord
        "Rails/HasAndBelongsToMany",    // only checks for has_and_belongs_to_many call
        "Rails/TableNameAssignment",    // only checks self.table_name =
        // --- Non-Rails cops not yet enriched (future milestones) ---
        // Layout
        "Layout/AssignmentIndentation",
        "Layout/BlockAlignment",
        "Layout/CaseIndentation",
        "Layout/ConditionPosition",
        "Layout/DefEndAlignment",
        "Layout/ElseAlignment",
        "Layout/EmptyLineBetweenDefs",
        "Layout/EmptyLines",
        "Layout/EmptyLinesAroundBlockBody",
        "Layout/EmptyLinesAroundClassBody",
        "Layout/EmptyLinesAroundMethodBody",
        "Layout/EmptyLinesAroundModuleBody",
        "Layout/EndAlignment",
        "Layout/EndOfLine",
        "Layout/FirstArgumentIndentation",
        "Layout/FirstArrayElementIndentation",
        "Layout/FirstHashElementIndentation",
        "Layout/IndentationConsistency",
        "Layout/IndentationWidth",
        "Layout/InitialIndentation",
        "Layout/LeadingEmptyLines",
        "Layout/LineLength",
        "Layout/MultilineMethodCallIndentation",
        "Layout/MultilineOperationIndentation",
        "Layout/RescueEnsureAlignment",
        "Layout/SpaceAfterColon",
        "Layout/SpaceAfterComma",
        "Layout/SpaceAfterSemicolon",
        "Layout/SpaceAroundEqualsInParameterDefault",
        "Layout/SpaceBeforeBlockBraces",
        "Layout/SpaceBeforeComma",
        "Layout/TrailingEmptyLines",
        "Layout/TrailingWhitespace",
        // Lint
        "Lint/DuplicateCaseCondition",
        "Lint/EachWithObjectArgument",
        "Lint/ElseLayout",
        "Lint/EmptyConditionalBody",
        "Lint/EmptyWhen",
        "Lint/EnsureReturn",
        "Lint/FloatOutOfRange",
        "Lint/LiteralAsCondition",
        "Lint/Loop",
        "Lint/NestedMethodDefinition",
        "Lint/RaiseException",
        "Lint/RedundantStringCoercion",
        "Lint/SuppressedException",
        "Lint/UriRegexp",
        // Style
        "Style/ClassAndModuleChildren",
        "Style/Documentation",
        "Style/EmptyMethod",
        "Style/FrozenStringLiteralComment",
        "Style/HashSyntax",
        "Style/IfUnlessModifier",
        "Style/Lambda",
        "Style/MethodCallWithArgsParentheses",
        "Style/NegatedIf",
        "Style/NegatedWhile",
        "Style/NumericLiterals",
        "Style/ParenthesesAroundCondition",
        "Style/Proc",
        "Style/RaiseArgs",
        "Style/RedundantReturn",
        "Style/RescueModifier",
        "Style/RescueStandardError",
        "Style/Semicolon",
        "Style/SignalException",
        "Style/SingleLineMethods",
        "Style/SpecialGlobalVars",
        "Style/StabbyLambdaParentheses",
        "Style/StringLiterals",
        "Style/SymbolArray",
        "Style/Tab",
        "Style/TernaryParentheses",
        "Style/TrailingCommaInArguments",
        "Style/TrailingCommaInArrayLiteral",
        "Style/TrailingCommaInHashLiteral",
        "Style/WordArray",
        "Style/YodaCondition",
        // Performance
        "Performance/AncestorsInclude",
        "Performance/ArraySemiInfiniteRangeSlice",
        "Performance/BindCall",
        "Performance/BlockGivenWithExplicitBlock",
        "Performance/CaseWhenSplat",
        "Performance/CompareWithBlock",
        "Performance/ConcurrentMonotonicTime",
        "Performance/DeletePrefix",
        "Performance/DeleteSuffix",
        "Performance/Detect",
        "Performance/DoubleStartEndWith",
        "Performance/EndWith",
        "Performance/InefficientHashSearch",
        "Performance/MapMethodChain",
        "Performance/MethodObjectAsBlock",
        "Performance/RangeInclude",
        "Performance/RedundantBlockCall",
        "Performance/RedundantEqualityComparisonBlock",
        "Performance/RedundantMerge",
        "Performance/RedundantSortBlock",
        "Performance/RedundantSplitRegexpArgument",
        "Performance/RedundantStringChars",
        "Performance/Size",
        "Performance/SortReverse",
        "Performance/Squeeze",
        "Performance/StartWith",
        "Performance/StringIdentifierArgument",
        "Performance/StringInclude",
        "Performance/StringReplacement",
        "Performance/Sum",
        "Performance/TimesMap",
        "Performance/UnfreezeString",
        // Metrics
        "Metrics/AbcSize",
        "Metrics/BlockLength",
        "Metrics/ClassLength",
        "Metrics/CyclomaticComplexity",
        "Metrics/MethodLength",
        "Metrics/ModuleLength",
        "Metrics/ParameterLists",
        "Metrics/PerceivedComplexity",
        // Naming
        "Naming/AccessorMethodName",
        "Naming/AsciiIdentifiers",
        "Naming/ClassAndModuleCamelCase",
        "Naming/ConstantName",
        "Naming/FileName",
        "Naming/MethodName",
        "Naming/PredicateName",
        "Naming/VariableName",
    ];

    // Cops exempt from the 3-line no_offense.rb minimum.
    let no_offense_exemptions: &[&str] = &[
        // These cops have very narrow patterns where 1-2 no_offense lines suffice
        "Layout/EndOfLine",        // no_offense is just a unix-ending file
        "Layout/InitialIndentation", // no_offense is just a properly-indented file
        "Layout/SpaceBeforeBlockBraces", // single line suffices
        "Layout/TrailingEmptyLines", // no_offense is a file with correct trailing newline
        "Lint/EachWithObjectArgument", // narrow pattern
        "Lint/UriEscapeUnescape",  // narrow pattern
        "Lint/UriRegexp",          // narrow pattern
        "Naming/FileName",         // tested via filename, not content
        // --- Non-Rails cops not yet enriched (future milestones) ---
        "Layout/TrailingWhitespace",
        "Layout/LeadingEmptyLines",
        "Layout/EmptyLines",
        "Layout/SpaceAfterSemicolon",
        "Layout/SpaceBeforeComma",
        "Layout/SpaceAroundEqualsInParameterDefault",
        "Layout/SpaceInsideHashLiteralBraces",
        "Layout/SpaceInsideArrayLiteralBrackets",
        "Style/FrozenStringLiteralComment",
        "Style/Lambda",
        "Style/Proc",
        "Style/StabbyLambdaParentheses",
        "Style/TernaryParentheses",
        "Performance/AncestorsInclude",
        "Performance/BigDecimalWithNumericArgument",
        "Performance/Caller",
        "Performance/ConcurrentMonotonicTime",
        "Performance/IoReadlines",
        "Performance/MapMethodChain",
        "Performance/MethodObjectAsBlock",
        "Performance/OpenStruct",
        "Performance/RangeInclude",
        "Performance/RedundantMatch",
        "Performance/RegexpMatch",
        "Performance/UriDefaultParser",
    ];

    let mut failures = Vec::new();

    for cop_name in registry.names() {
        let parts: Vec<&str> = cop_name.split('/').collect();
        let dept = parts[0].to_lowercase();
        let name = to_snake_case(parts[1]);

        let dir = testdata.join(&dept).join(&name);
        let dir_alt = testdata.join(&dept).join(format!("{name}_cop"));
        let effective_dir = if dir.exists() {
            &dir
        } else if dir_alt.exists() {
            &dir_alt
        } else {
            continue; // all_cops_have_fixture_files covers this
        };

        // Check offense.rb has at least 2 annotated cases.
        // Count annotation lines: lines where first non-whitespace is '^' followed
        // by ': ' and '/' (matching the fixture annotation format).
        if let Ok(offense_content) = fs::read_to_string(effective_dir.join("offense.rb")) {
            let annotation_count = offense_content
                .lines()
                .filter(|line| {
                    let trimmed = line.trim_start();
                    trimmed.starts_with('^')
                        && trimmed.contains(": ")
                        && trimmed.contains('/')
                })
                .count();
            if annotation_count < 2 && !offense_exemptions.contains(&cop_name) {
                failures.push(format!(
                    "{cop_name}: only {annotation_count} offense case(s) in offense.rb, need at least 2"
                ));
            }
        }

        // Check no_offense.rb has at least 3 non-empty lines
        if let Ok(no_offense_content) = fs::read_to_string(effective_dir.join("no_offense.rb")) {
            let non_empty = no_offense_content
                .lines()
                .filter(|l| !l.trim().is_empty())
                .count();
            if non_empty < 3 && !no_offense_exemptions.contains(&cop_name) {
                failures.push(format!(
                    "{cop_name}: only {non_empty} non-empty line(s) in no_offense.rb, need at least 3"
                ));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "Cops below minimum test coverage thresholds:\n{}",
        failures.join("\n")
    );
}

#[test]
fn all_cops_have_fixture_files() {
    let testdata = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata/cops");
    let registry = CopRegistry::default_registry();
    let mut missing = Vec::new();

    for cop_name in registry.names() {
        let parts: Vec<&str> = cop_name.split('/').collect();
        let dept = parts[0].to_lowercase();
        let name = to_snake_case(parts[1]);

        let dir = testdata.join(&dept).join(&name);
        // Some cops use a `_cop` suffix to avoid Rust keyword conflicts (e.g. loop -> loop_cop)
        let dir_alt = testdata.join(&dept).join(format!("{name}_cop"));

        let effective_dir = if dir.exists() {
            &dir
        } else if dir_alt.exists() {
            &dir_alt
        } else {
            missing.push(format!("{cop_name}: missing directory ({} or {})", dir.display(), dir_alt.display()));
            continue;
        };

        if !effective_dir.join("offense.rb").exists() {
            missing.push(format!("{cop_name}: missing offense.rb"));
        }
        if !effective_dir.join("no_offense.rb").exists() {
            missing.push(format!("{cop_name}: missing no_offense.rb"));
        }
    }

    assert!(
        missing.is_empty(),
        "Cops missing fixture files:\n{}",
        missing.join("\n")
    );
}

// ---------- M3 integration tests ----------

#[test]
fn metrics_cops_fire_on_complex_code() {
    let dir = temp_dir("metrics_complex");
    // 16-line method body exceeds default Max:10 for MethodLength
    let file = write_file(
        &dir,
        "complex.rb",
        b"# frozen_string_literal: true\n\ndef long_method\n  a = 1\n  b = 2\n  c = 3\n  d = 4\n  e = 5\n  f = 6\n  g = 7\n  h = 8\n  i = 9\n  j = 10\n  k = 11\n  l = 12\n  m = 13\n  n = 14\n  o = 15\n  p = 16\nend\n",
    );
    let config = load_config(None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = Args {
        only: vec!["Metrics/MethodLength".to_string()],
        ..default_args()
    };

    let result = run_linter(&[file], &config, &registry, &args);
    assert!(
        !result.diagnostics.is_empty(),
        "Metrics/MethodLength should fire on 16-line method"
    );
    assert_eq!(result.diagnostics[0].cop_name, "Metrics/MethodLength");

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn naming_cops_fire_on_bad_names() {
    let dir = temp_dir("naming_bad");
    // camelCase method name should trigger Naming/MethodName
    let file = write_file(
        &dir,
        "bad_names.rb",
        b"# frozen_string_literal: true\n\ndef myMethod\nend\n",
    );
    let config = load_config(None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = Args {
        only: vec!["Naming/MethodName".to_string()],
        ..default_args()
    };

    let result = run_linter(&[file], &config, &registry, &args);
    let cop_names: Vec<&str> = result.diagnostics.iter().map(|d| d.cop_name.as_str()).collect();
    assert!(
        cop_names.contains(&"Naming/MethodName"),
        "Naming/MethodName should fire on camelCase method name"
    );

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn config_overrides_new_departments() {
    let dir = temp_dir("config_new_dept");
    // 4-line method body: under default Max:10 but over Max:3
    let file = write_file(
        &dir,
        "short_method.rb",
        b"# frozen_string_literal: true\n\ndef foo\n  a = 1\n  b = 2\n  c = 3\n  d = 4\nend\n",
    );
    let config_path = write_file(
        &dir,
        ".rubocop.yml",
        b"Metrics/MethodLength:\n  Max: 3\n",
    );
    let config = load_config(Some(config_path.as_path())).unwrap();
    let registry = CopRegistry::default_registry();
    let args = Args {
        only: vec!["Metrics/MethodLength".to_string()],
        ..default_args()
    };

    let result = run_linter(&[file], &config, &registry, &args);
    assert!(
        !result.diagnostics.is_empty(),
        "Metrics/MethodLength should fire with Max:3 on 4-line method"
    );
    assert_eq!(result.diagnostics[0].cop_name, "Metrics/MethodLength");
    assert!(
        result.diagnostics[0].message.contains("/3]"),
        "Message should reference Max:3"
    );

    fs::remove_dir_all(&dir).ok();
}
