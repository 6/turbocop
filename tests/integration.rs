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
    assert_eq!(registry.len(), 117, "Expected 117 registered cops");

    let names = registry.names();
    let expected = [
        // Layout (17)
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
        // Performance (39)
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
        // Style (18)
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
    ];
    for name in &expected {
        assert!(
            names.contains(name),
            "Registry missing expected cop: {name}"
        );
    }
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
