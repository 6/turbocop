//! Integration tests for the turbocop linting pipeline.
//!
//! These tests exercise the full linter: file reading, config loading,
//! cop registry, cop execution, and diagnostic collection. They write
//! real files to a temp directory and invoke `run_linter` directly.

use std::fs;
use std::path::{Path, PathBuf};

use turbocop::cli::Args;
use turbocop::config::load_config;
use turbocop::cop::registry::CopRegistry;
use turbocop::fs::DiscoveredFiles;
use turbocop::linter::run_linter;

/// Create a temporary directory with a unique name for each test.
fn temp_dir(test_name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("turbocop_integration_{test_name}"));
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
        rubocop_only: false,
        list_cops: false,
        stdin: None,
        init: false,
        no_cache: false,
        cache: "true".to_string(),
        cache_clear: false,
        fail_level: "convention".to_string(),
        fail_fast: false,
        force_exclusion: false,
    }
}

/// Wrap file paths as DiscoveredFiles with no explicit files (directory-discovered).
fn discovered(files: &[PathBuf]) -> DiscoveredFiles {
    DiscoveredFiles {
        files: files.to_vec(),
        explicit: std::collections::HashSet::new(),
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
    let config = load_config(None, None, None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = default_args();

    let result = run_linter(&discovered(&[file]), &config, &registry, &args);
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
    let config = load_config(None, None, None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = default_args();

    let result = run_linter(&discovered(&[file]), &config, &registry, &args);
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

    let config = load_config(None, None, None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = default_args();

    let result = run_linter(&discovered(&[f1, f2]), &config, &registry, &args);
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
    let config = load_config(None, None, None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = Args {
        only: vec!["Layout/TrailingWhitespace".to_string()],
        ..default_args()
    };

    let result = run_linter(&discovered(&[file]), &config, &registry, &args);

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
    let config = load_config(None, None, None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = Args {
        except: vec![
            "Style/FrozenStringLiteralComment".to_string(),
            "Layout/TrailingWhitespace".to_string(),
        ],
        ..default_args()
    };

    let result = run_linter(&discovered(&[file]), &config, &registry, &args);

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
    let config = load_config(None, None, None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = Args {
        only: vec!["Layout/TrailingWhitespace".to_string()],
        ..default_args()
    };

    let result = run_linter(&discovered(&[file]), &config, &registry, &args);
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
    let config = load_config(Some(config_path.as_path()), None, None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = default_args();

    let result = run_linter(&discovered(&[file]), &config, &registry, &args);

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
    let config = load_config(Some(config_path.as_path()), None, None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = Args {
        only: vec!["Layout/LineLength".to_string()],
        ..default_args()
    };

    let result = run_linter(&discovered(&[file]), &config, &registry, &args);

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
    let config = load_config(None, None, None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = Args {
        only: vec!["Layout/LineLength".to_string()],
        ..default_args()
    };

    let result = run_linter(&discovered(&[file]), &config, &registry, &args);
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
    let config = load_config(None, None, None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = default_args();

    let result = run_linter(&discovered(&[file]), &config, &registry, &args);
    assert_eq!(result.file_count, 1);
    // Should not panic; may or may not have offenses (FrozenStringLiteralComment fires)

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn file_with_syntax_errors_still_lints() {
    let dir = temp_dir("syntax_error");
    // Invalid Ruby syntax — Prism parses with errors but still returns a tree
    let file = write_file(&dir, "bad_syntax.rb", b"def foo(\n  x = 1\n");
    let config = load_config(None, None, None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = default_args();

    // Should not panic
    let result = run_linter(&discovered(&[file]), &config, &registry, &args);
    assert_eq!(result.file_count, 1);
    // Line-based cops should still find offenses (at minimum FrozenStringLiteralComment)

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn binary_content_no_crash() {
    let dir = temp_dir("binary");
    // Binary content with null bytes
    let file = write_file(&dir, "binary.rb", b"\x00\x01\x02\xff\xfe");
    let config = load_config(None, None, None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = default_args();

    // Should not panic
    let result = run_linter(&discovered(&[file]), &config, &registry, &args);
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
    let config = load_config(None, None, None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = Args {
        only: vec!["Layout/EndOfLine".to_string()],
        ..default_args()
    };

    let result = run_linter(&discovered(&[file]), &config, &registry, &args);
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
    let config = load_config(None, None, None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = Args {
        only: vec!["Layout/TrailingWhitespace".to_string()],
        ..default_args()
    };

    let result = run_linter(&discovered(&[f1, f2]), &config, &registry, &args);
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
    let config = load_config(None, None, None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = default_args();

    let result = run_linter(&discovered(&[file]), &config, &registry, &args);

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
    assert_eq!(registry.len(), 915, "Expected 915 registered cops");

    let names = registry.names();
    let expected = [
        // Bundler (7)
        "Bundler/DuplicatedGem",
        "Bundler/DuplicatedGroup",
        "Bundler/GemComment",
        "Bundler/GemFilename",
        "Bundler/GemVersion",
        "Bundler/InsecureProtocolSource",
        "Bundler/OrderedGems",
        // FactoryBot (11)
        "FactoryBot/AssociationStyle",
        "FactoryBot/AttributeDefinedStatically",
        "FactoryBot/ConsistentParenthesesStyle",
        "FactoryBot/CreateList",
        "FactoryBot/ExcessiveCreateList",
        "FactoryBot/FactoryAssociationWithStrategy",
        "FactoryBot/FactoryClassName",
        "FactoryBot/FactoryNameStyle",
        "FactoryBot/IdSequence",
        "FactoryBot/RedundantFactoryOption",
        "FactoryBot/SyntaxMethods",
        // Gemspec (10)
        "Gemspec/AddRuntimeDependency",
        "Gemspec/AttributeAssignment",
        "Gemspec/DependencyVersion",
        "Gemspec/DeprecatedAttributeAssignment",
        "Gemspec/DevelopmentDependencies",
        "Gemspec/DuplicatedAssignment",
        "Gemspec/OrderedDependencies",
        "Gemspec/RequireMFA",
        "Gemspec/RequiredRubyVersion",
        "Gemspec/RubyVersionGlobalsUsage",
        // Layout (100)
        "Layout/AccessModifierIndentation",
        "Layout/ArgumentAlignment",
        "Layout/ArrayAlignment",
        "Layout/AssignmentIndentation",
        "Layout/BeginEndAlignment",
        "Layout/BlockAlignment",
        "Layout/BlockEndNewline",
        "Layout/CaseIndentation",
        "Layout/ClassStructure",
        "Layout/ClosingHeredocIndentation",
        "Layout/ClosingParenthesisIndentation",
        "Layout/CommentIndentation",
        "Layout/ConditionPosition",
        "Layout/DefEndAlignment",
        "Layout/DotPosition",
        "Layout/ElseAlignment",
        "Layout/EmptyComment",
        "Layout/EmptyLineAfterGuardClause",
        "Layout/EmptyLineAfterMagicComment",
        "Layout/EmptyLineAfterMultilineCondition",
        "Layout/EmptyLineBetweenDefs",
        "Layout/EmptyLines",
        "Layout/EmptyLinesAfterModuleInclusion",
        "Layout/EmptyLinesAroundAccessModifier",
        "Layout/EmptyLinesAroundArguments",
        "Layout/EmptyLinesAroundAttributeAccessor",
        "Layout/EmptyLinesAroundBeginBody",
        "Layout/EmptyLinesAroundBlockBody",
        "Layout/EmptyLinesAroundClassBody",
        "Layout/EmptyLinesAroundExceptionHandlingKeywords",
        "Layout/EmptyLinesAroundMethodBody",
        "Layout/EmptyLinesAroundModuleBody",
        "Layout/EndAlignment",
        "Layout/EndOfLine",
        "Layout/ExtraSpacing",
        "Layout/FirstArgumentIndentation",
        "Layout/FirstArrayElementIndentation",
        "Layout/FirstArrayElementLineBreak",
        "Layout/FirstHashElementIndentation",
        "Layout/FirstHashElementLineBreak",
        "Layout/FirstMethodArgumentLineBreak",
        "Layout/FirstMethodParameterLineBreak",
        "Layout/FirstParameterIndentation",
        "Layout/HashAlignment",
        "Layout/HeredocArgumentClosingParenthesis",
        "Layout/HeredocIndentation",
        "Layout/IndentationConsistency",
        "Layout/IndentationStyle",
        "Layout/IndentationWidth",
        "Layout/InitialIndentation",
        "Layout/LeadingCommentSpace",
        "Layout/LeadingEmptyLines",
        "Layout/LineContinuationLeadingSpace",
        "Layout/LineContinuationSpacing",
        "Layout/LineEndStringConcatenationIndentation",
        "Layout/LineLength",
        "Layout/MultilineArrayBraceLayout",
        "Layout/MultilineArrayLineBreaks",
        "Layout/MultilineAssignmentLayout",
        "Layout/MultilineBlockLayout",
        "Layout/MultilineHashBraceLayout",
        "Layout/MultilineHashKeyLineBreaks",
        "Layout/MultilineMethodArgumentLineBreaks",
        "Layout/MultilineMethodCallBraceLayout",
        "Layout/MultilineMethodCallIndentation",
        "Layout/MultilineMethodDefinitionBraceLayout",
        "Layout/MultilineMethodParameterLineBreaks",
        "Layout/MultilineOperationIndentation",
        "Layout/ParameterAlignment",
        "Layout/RedundantLineBreak",
        "Layout/RescueEnsureAlignment",
        "Layout/SingleLineBlockChain",
        "Layout/SpaceAfterColon",
        "Layout/SpaceAfterComma",
        "Layout/SpaceAfterMethodName",
        "Layout/SpaceAfterNot",
        "Layout/SpaceAfterSemicolon",
        "Layout/SpaceAroundBlockParameters",
        "Layout/SpaceAroundEqualsInParameterDefault",
        "Layout/SpaceAroundKeyword",
        "Layout/SpaceAroundMethodCallOperator",
        "Layout/SpaceAroundOperators",
        "Layout/SpaceBeforeBlockBraces",
        "Layout/SpaceBeforeBrackets",
        "Layout/SpaceBeforeComma",
        "Layout/SpaceBeforeComment",
        "Layout/SpaceBeforeFirstArg",
        "Layout/SpaceBeforeSemicolon",
        "Layout/SpaceInLambdaLiteral",
        "Layout/SpaceInsideArrayLiteralBrackets",
        "Layout/SpaceInsideArrayPercentLiteral",
        "Layout/SpaceInsideBlockBraces",
        "Layout/SpaceInsideHashLiteralBraces",
        "Layout/SpaceInsideParens",
        "Layout/SpaceInsidePercentLiteralDelimiters",
        "Layout/SpaceInsideRangeLiteral",
        "Layout/SpaceInsideReferenceBrackets",
        "Layout/SpaceInsideStringInterpolation",
        "Layout/TrailingEmptyLines",
        "Layout/TrailingWhitespace",
        // Lint (116)
        "Lint/AmbiguousAssignment",
        "Lint/AmbiguousOperatorPrecedence",
        "Lint/AmbiguousRange",
        "Lint/AssignmentInCondition",
        "Lint/BigDecimalNew",
        "Lint/BinaryOperatorWithIdenticalOperands",
        "Lint/BooleanSymbol",
        "Lint/CircularArgumentReference",
        "Lint/CopDirectiveSyntax",
        "Lint/ConstantDefinitionInBlock",
        "Lint/ConstantOverwrittenInRescue",
        "Lint/ConstantReassignment",
        "Lint/Debugger",
        "Lint/DeprecatedClassMethods",
        "Lint/DeprecatedConstants",
        "Lint/DeprecatedOpenSSLConstant",
        "Lint/DisjunctiveAssignmentInConstructor",
        "Lint/DuplicateBranch",
        "Lint/DuplicateCaseCondition",
        "Lint/DuplicateElsifCondition",
        "Lint/DuplicateHashKey",
        "Lint/DuplicateMagicComment",
        "Lint/DuplicateMethods",
        "Lint/DuplicateRequire",
        "Lint/DuplicateRescueException",
        "Lint/EachWithObjectArgument",
        "Lint/ElseLayout",
        "Lint/EmptyBlock",
        "Lint/EmptyClass",
        "Lint/EmptyConditionalBody",
        "Lint/EmptyEnsure",
        "Lint/EmptyExpression",
        "Lint/EmptyFile",
        "Lint/EmptyInPattern",
        "Lint/EmptyInterpolation",
        "Lint/EmptyWhen",
        "Lint/EnsureReturn",
        "Lint/FlipFlop",
        "Lint/FloatComparison",
        "Lint/FloatOutOfRange",
        "Lint/FormatParameterMismatch",
        "Lint/HashCompareByIdentity",
        "Lint/IdentityComparison",
        "Lint/ImplicitStringConcatenation",
        "Lint/IneffectiveAccessModifier",
        "Lint/InheritException",
        "Lint/InterpolationCheck",
        "Lint/LiteralAsCondition",
        "Lint/LiteralAssignmentInCondition",
        "Lint/LiteralInInterpolation",
        "Lint/Loop",
        "Lint/MissingCopEnableDirective",
        "Lint/MissingSuper",
        "Lint/MultipleComparison",
        "Lint/NestedMethodDefinition",
        "Lint/NestedPercentLiteral",
        "Lint/NextWithoutAccumulator",
        "Lint/NoReturnInBeginEndBlocks",
        "Lint/NonAtomicFileOperation",
        "Lint/NonLocalExitFromIterator",
        "Lint/NumberedParameterAssignment",
        "Lint/OrAssignmentToConstant",
        "Lint/OrderedMagicComments",
        "Lint/OutOfRangeRegexpRef",
        "Lint/PercentStringArray",
        "Lint/PercentSymbolArray",
        "Lint/RaiseException",
        "Lint/RandOne",
        "Lint/RedundantCopDisableDirective",
        "Lint/RedundantCopEnableDirective",
        "Lint/RedundantDirGlobSort",
        "Lint/RedundantRegexpQuantifiers",
        "Lint/RedundantRequireStatement",
        "Lint/RedundantSafeNavigation",
        "Lint/RedundantSplatExpansion",
        "Lint/RedundantStringCoercion",
        "Lint/RedundantWithIndex",
        "Lint/RedundantWithObject",
        "Lint/RegexpAsCondition",
        "Lint/RequireParentheses",
        "Lint/RequireRangeParentheses",
        "Lint/RescueException",
        "Lint/RescueType",
        "Lint/ReturnInVoidContext",
        "Lint/SafeNavigationChain",
        "Lint/SafeNavigationConsistency",
        "Lint/ScriptPermission",
        "Lint/SelfAssignment",
        "Lint/SendWithMixinArgument",
        "Lint/ShadowedArgument",
        "Lint/ShadowedException",
        "Lint/ShadowingOuterLocalVariable",
        "Lint/StructNewOverride",
        "Lint/SuppressedException",
        "Lint/SymbolConversion",
        "Lint/Syntax",
        "Lint/ToEnumArguments",
        "Lint/ToJSON",
        "Lint/TopLevelReturnWithArgument",
        "Lint/TrailingCommaInAttributeDeclaration",
        "Lint/TripleQuotes",
        "Lint/UnescapedBracketInRegexp",
        "Lint/UnifiedInteger",
        "Lint/UnmodifiedReduceAccumulator",
        "Lint/UnreachableCode",
        "Lint/UnreachableLoop",
        "Lint/UnusedBlockArgument",
        "Lint/UnusedMethodArgument",
        "Lint/UriEscapeUnescape",
        "Lint/UriRegexp",
        "Lint/UselessAccessModifier",
        "Lint/UselessAssignment",
        "Lint/UselessElseWithoutRescue",
        "Lint/UselessMethodDefinition",
        "Lint/UselessSetterCall",
        "Lint/Void",
        // Metrics (10)
        "Metrics/AbcSize",
        "Metrics/BlockLength",
        "Metrics/BlockNesting",
        "Metrics/ClassLength",
        "Metrics/CollectionLiteralLength",
        "Metrics/CyclomaticComplexity",
        "Metrics/MethodLength",
        "Metrics/ModuleLength",
        "Metrics/ParameterLists",
        "Metrics/PerceivedComplexity",
        // Migration (1)
        "Migration/DepartmentName",
        // Naming (19)
        "Naming/AccessorMethodName",
        "Naming/AsciiIdentifiers",
        "Naming/BinaryOperatorParameterName",
        "Naming/BlockForwarding",
        "Naming/BlockParameterName",
        "Naming/ClassAndModuleCamelCase",
        "Naming/ConstantName",
        "Naming/FileName",
        "Naming/HeredocDelimiterCase",
        "Naming/HeredocDelimiterNaming",
        "Naming/InclusiveLanguage",
        "Naming/MemoizedInstanceVariableName",
        "Naming/MethodName",
        "Naming/MethodParameterName",
        "Naming/PredicateMethod",
        "Naming/PredicatePrefix",
        "Naming/RescuedExceptionsVariableName",
        "Naming/VariableName",
        "Naming/VariableNumber",
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
        // RSpec (113)
        "RSpec/AlignLeftLetBrace",
        "RSpec/AlignRightLetBrace",
        "RSpec/AnyInstance",
        "RSpec/AroundBlock",
        "RSpec/Be",
        "RSpec/BeEmpty",
        "RSpec/BeEq",
        "RSpec/BeEql",
        "RSpec/BeNil",
        "RSpec/BeforeAfterAll",
        "RSpec/ChangeByZero",
        "RSpec/ClassCheck",
        "RSpec/ContainExactly",
        "RSpec/ContextMethod",
        "RSpec/ContextWording",
        "RSpec/DescribeClass",
        "RSpec/DescribeMethod",
        "RSpec/DescribeSymbol",
        "RSpec/DescribedClass",
        "RSpec/DescribedClassModuleWrapping",
        "RSpec/Dialect",
        "RSpec/DuplicatedMetadata",
        "RSpec/EmptyExampleGroup",
        "RSpec/EmptyHook",
        "RSpec/EmptyLineAfterExample",
        "RSpec/EmptyLineAfterExampleGroup",
        "RSpec/EmptyLineAfterFinalLet",
        "RSpec/EmptyLineAfterHook",
        "RSpec/EmptyLineAfterSubject",
        "RSpec/EmptyMetadata",
        "RSpec/EmptyOutput",
        "RSpec/Eq",
        "RSpec/ExampleLength",
        "RSpec/ExampleWithoutDescription",
        "RSpec/ExampleWording",
        "RSpec/ExcessiveDocstringSpacing",
        "RSpec/ExpectActual",
        "RSpec/ExpectChange",
        "RSpec/ExpectInHook",
        "RSpec/ExpectInLet",
        "RSpec/ExpectOutput",
        "RSpec/Focus",
        "RSpec/HookArgument",
        "RSpec/HooksBeforeExamples",
        "RSpec/IdenticalEqualityAssertion",
        "RSpec/ImplicitBlockExpectation",
        "RSpec/ImplicitExpect",
        "RSpec/ImplicitSubject",
        "RSpec/IncludeExamples",
        "RSpec/IndexedLet",
        "RSpec/InstanceSpy",
        "RSpec/InstanceVariable",
        "RSpec/IsExpectedSpecify",
        "RSpec/ItBehavesLike",
        "RSpec/IteratedExpectation",
        "RSpec/LeadingSubject",
        "RSpec/LeakyConstantDeclaration",
        "RSpec/LeakyLocalVariable",
        "RSpec/LetBeforeExamples",
        "RSpec/LetSetup",
        "RSpec/MatchArray",
        "RSpec/MessageChain",
        "RSpec/MessageExpectation",
        "RSpec/MessageSpies",
        "RSpec/MetadataStyle",
        "RSpec/MissingExampleGroupArgument",
        "RSpec/MissingExpectationTargetMethod",
        "RSpec/MultipleDescribes",
        "RSpec/MultipleExpectations",
        "RSpec/MultipleMemoizedHelpers",
        "RSpec/MultipleSubjects",
        "RSpec/NamedSubject",
        "RSpec/NestedGroups",
        "RSpec/NoExpectationExample",
        "RSpec/NotToNot",
        "RSpec/Output",
        "RSpec/OverwritingSetup",
        "RSpec/Pending",
        "RSpec/PendingWithoutReason",
        "RSpec/PredicateMatcher",
        "RSpec/ReceiveCounts",
        "RSpec/ReceiveMessages",
        "RSpec/ReceiveNever",
        "RSpec/RedundantAround",
        "RSpec/RedundantPredicateMatcher",
        "RSpec/RemoveConst",
        "RSpec/RepeatedDescription",
        "RSpec/RepeatedExample",
        "RSpec/RepeatedExampleGroupBody",
        "RSpec/RepeatedExampleGroupDescription",
        "RSpec/RepeatedIncludeExample",
        "RSpec/RepeatedSubjectCall",
        "RSpec/ReturnFromStub",
        "RSpec/ScatteredLet",
        "RSpec/ScatteredSetup",
        "RSpec/SharedContext",
        "RSpec/SharedExamples",
        "RSpec/SingleArgumentMessageChain",
        "RSpec/SkipBlockInsideExample",
        "RSpec/SortMetadata",
        "RSpec/SpecFilePathFormat",
        "RSpec/SpecFilePathSuffix",
        "RSpec/StubbedMock",
        "RSpec/SubjectDeclaration",
        "RSpec/SubjectStub",
        "RSpec/UndescriptiveLiteralsDescription",
        "RSpec/UnspecifiedException",
        "RSpec/VariableDefinition",
        "RSpec/VariableName",
        "RSpec/VerifiedDoubleReference",
        "RSpec/VerifiedDoubles",
        "RSpec/VoidExpect",
        "RSpec/Yield",
        // Rails (110)
        "Rails/ActionControllerFlashBeforeRender",
        "Rails/ActionControllerTestCase",
        "Rails/ActionOrder",
        "Rails/ActiveRecordCallbacksOrder",
        "Rails/ActiveSupportAliases",
        "Rails/ActiveSupportOnLoad",
        "Rails/AddColumnIndex",
        "Rails/AfterCommitOverride",
        "Rails/ApplicationController",
        "Rails/ApplicationJob",
        "Rails/ApplicationMailer",
        "Rails/ApplicationRecord",
        "Rails/AttributeDefaultBlockValue",
        "Rails/Blank",
        "Rails/BulkChangeTable",
        "Rails/CompactBlank",
        "Rails/ContentTag",
        "Rails/CreateTableWithTimestamps",
        "Rails/DangerousColumnNames",
        "Rails/Date",
        "Rails/Delegate",
        "Rails/DelegateAllowBlank",
        "Rails/DeprecatedActiveModelErrorsMethods",
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
        "Rails/FindByOrAssignmentMemoization",
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
        "Rails/I18nLocaleTexts",
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
        "Rails/RedundantReceiverInWithOptions",
        "Rails/RedundantTravelBack",
        "Rails/ReflectionClassName",
        "Rails/RefuteMethods",
        "Rails/RelativeDateConstant",
        "Rails/RenderInline",
        "Rails/RenderPlainText",
        "Rails/RequestReferer",
        "Rails/ResponseParsedBody",
        "Rails/ReversibleMigration",
        "Rails/ReversibleMigrationMethodDefinition",
        "Rails/RootJoinChain",
        "Rails/RootPathnameMethods",
        "Rails/RootPublicPath",
        "Rails/SafeNavigation",
        "Rails/SaveBang",
        "Rails/SchemaComment",
        "Rails/ScopeArgs",
        "Rails/SelectMap",
        "Rails/ShortI18n",
        "Rails/SkipsModelValidations",
        "Rails/SquishedSQLHeredocs",
        "Rails/StripHeredoc",
        "Rails/StrongParametersExpect",
        "Rails/TableNameAssignment",
        "Rails/ThreeStateBooleanColumn",
        "Rails/TimeZone",
        "Rails/TimeZoneAssignment",
        "Rails/ToFormattedS",
        "Rails/ToSWithArgument",
        "Rails/TransactionExitStatement",
        // UniqueValidationWithoutIndex disabled (requires schema analysis)
        "Rails/UnknownEnv",
        "Rails/UnusedRenderContent",
        "Rails/Validation",
        "Rails/WhereExists",
        "Rails/WhereMissing",
        "Rails/WhereNot",
        "Rails/WhereRange",
        // Security (7)
        "Security/CompoundHash",
        "Security/Eval",
        "Security/IoMethods",
        "Security/JSONLoad",
        "Security/MarshalLoad",
        "Security/Open",
        "Security/YAMLLoad",
        // Style (80)
        "Style/AndOr",
        "Style/BlockDelimiters",
        "Style/ClassAndModuleChildren",
        "Style/ClassVars",
        "Style/ColonMethodCall",
        "Style/DefWithParentheses",
        "Style/Documentation",
        "Style/DoubleNegation",
        "Style/EmptyMethod",
        "Style/ExplicitBlockArgument",
        "Style/FrozenStringLiteralComment",
        "Style/GlobalVars",
        "Style/GuardClause",
        "Style/HashSyntax",
        "Style/HashTransformKeys",
        "Style/HashTransformValues",
        "Style/IfUnlessModifier",
        "Style/Lambda",
        "Style/MethodCallWithArgsParentheses",
        "Style/MethodDefParentheses",
        "Style/MutableConstant",
        "Style/NegatedIf",
        "Style/NegatedUnless",
        "Style/NegatedWhile",
        "Style/NilLambda",
        "Style/NonNilCheck",
        "Style/Not",
        "Style/NumericLiteralPrefix",
        "Style/NumericLiterals",
        "Style/NumericPredicate",
        "Style/ObjectThen",
        "Style/OneClassPerFile",
        "Style/OpenStructUse",
        "Style/OptionalBooleanParameter",
        "Style/OrAssignment",
        "Style/ParenthesesAroundCondition",
        "Style/PreferredHashMethods",
        "Style/Proc",
        "Style/RaiseArgs",
        "Style/RedundantBegin",
        "Style/RedundantCondition",
        "Style/RedundantConditional",
        "Style/RedundantException",
        "Style/RedundantFileExtensionInRequire",
        "Style/RedundantFreeze",
        "Style/RedundantInterpolation",
        "Style/RedundantPercentQ",
        "Style/RedundantReturn",
        "Style/RedundantSelf",
        "Style/RedundantSort",
        "Style/RescueModifier",
        "Style/RescueStandardError",
        "Style/SafeNavigation",
        "Style/Sample",
        "Style/SelectByRegexp",
        "Style/SelfAssignment",
        "Style/Semicolon",
        "Style/SignalException",
        "Style/SingleArgumentDig",
        "Style/SingleLineMethods",
        "Style/SlicingWithRange",
        "Style/SpecialGlobalVars",
        "Style/StabbyLambdaParentheses",
        "Style/StringConcatenation",
        "Style/StringLiterals",
        "Style/Strip",
        "Style/SymbolArray",
        "Style/SymbolLiteral",
        "Style/Tab",
        "Style/TernaryParentheses",
        "Style/TrailingCommaInArguments",
        "Style/TrailingCommaInArrayLiteral",
        "Style/TrailingCommaInHashLiteral",
        "Style/TrivialAccessors",
        "Style/UnlessElse",
        "Style/UnpackFirst",
        "Style/WhenThen",
        "Style/WordArray",
        "Style/YodaCondition",
        "Style/ZeroLengthPredicate",
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
    // Need plugins: entry so Performance department cops are enabled
    write_file(&dir, ".rubocop.yml", b"plugins:\n  - rubocop-performance\n");
    let config = load_config(None, Some(dir.as_ref()), None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = default_args();

    let result = run_linter(&discovered(&[file]), &config, &registry, &args);
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
    let config = load_config(None, None, None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = default_args();

    let result = run_linter(&discovered(&[file]), &config, &registry, &args);
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
    let config = load_config(None, None, None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = default_args();

    let result = run_linter(&discovered(&[file]), &config, &registry, &args);

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
    // Need plugins: entry so Rails department cops are enabled.
    let dir_str = dir.display();
    let config_yaml = format!(
        "plugins:\n  - rubocop-rails\nRails/CreateTableWithTimestamps:\n  Include:\n    - '{dir_str}/db/migrate/**/*.rb'\n"
    );
    let config_path = write_file(&dir, ".rubocop.yml", config_yaml.as_bytes());
    let migration_content = b"class CreateUsers < ActiveRecord::Migration[7.0]\n  def change\n    create_table :users do |t|\n      t.string :name\n    end\n  end\nend\n";
    let migrate_file = write_file(&dir, "db/migrate/001_create_users.rb", migration_content);
    let model_file = write_file(&dir, "app/models/user.rb", migration_content);
    let config = load_config(Some(config_path.as_path()), None, None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = Args {
        only: vec!["Rails/CreateTableWithTimestamps".to_string()],
        ..default_args()
    };

    let result = run_linter(&discovered(&[migrate_file, model_file]), &config, &registry, &args);

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

    let config = load_config(Some(config_path.as_path()), None, None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = Args {
        only: vec!["Layout/TrailingWhitespace".to_string()],
        ..default_args()
    };

    let result = run_linter(&discovered(&[vendor_file, app_file]), &config, &registry, &args);

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
    // Need plugins: entry so Rails department cops are enabled.
    let dir_str = dir.display();
    let config_yaml = format!(
        "plugins:\n  - rubocop-rails\nRails/CreateTableWithTimestamps:\n  Include:\n    - '{dir_str}/db/**/*.rb'\n"
    );
    let config_path = write_file(&dir, ".rubocop.yml", config_yaml.as_bytes());
    let migration_content = b"class CreateUsers < ActiveRecord::Migration[7.0]\n  def change\n    create_table :users do |t|\n      t.string :name\n    end\n  end\nend\n";
    // This file is in db/ but NOT in db/migrate/ — only matches the widened Include
    let seeds_file = write_file(&dir, "db/seeds.rb", migration_content);
    let config = load_config(Some(config_path.as_path()), None, None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = Args {
        only: vec!["Rails/CreateTableWithTimestamps".to_string()],
        ..default_args()
    };

    let result = run_linter(&discovered(&[seeds_file]), &config, &registry, &args);

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

/// Count annotations in fixture content: both `^` markers and `# turbocop-expect:` lines.
fn count_annotations(content: &str) -> usize {
    content
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            // Standard ^ annotations
            (trimmed.starts_with('^') && trimmed.contains(": ") && trimmed.contains('/'))
            // Explicit expect annotations
            || line.starts_with("# turbocop-expect: ")
        })
        .count()
}

#[test]
fn all_cops_have_minimum_test_coverage() {
    let testdata = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata/cops");
    let registry = CopRegistry::default_registry();

    // No-op stub cops that never produce offenses by design.
    // These exist for configuration compatibility only.
    let stub_cops: &[&str] = &[
        "Lint/RedundantCopDisableDirective", // requires post-processing after all cops run
        "Lint/Syntax",                       // syntax errors reported by parser, not this cop
    ];

    let mut failures = Vec::new();

    for cop_name in registry.names() {
        if stub_cops.contains(&cop_name) {
            continue;
        }
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

        // Check offense annotations: either from offense.rb or offense/ directory.
        let offense_dir = effective_dir.join("offense");
        let annotation_count = if offense_dir.exists() && offense_dir.is_dir() {
            // Sum annotations across all .rb files in offense/
            let mut count = 0;
            for entry in fs::read_dir(&offense_dir).unwrap() {
                let entry = entry.unwrap();
                if entry.path().extension().map_or(false, |e| e == "rb") {
                    let content = fs::read_to_string(entry.path()).unwrap();
                    count += count_annotations(&content);
                }
            }
            count
        } else if let Ok(content) = fs::read_to_string(effective_dir.join("offense.rb")) {
            count_annotations(&content)
        } else {
            continue; // all_cops_have_fixture_files covers this
        };

        if annotation_count < 3 {
            failures.push(format!(
                "{cop_name}: only {annotation_count} offense case(s), need at least 3"
            ));
        }

        // Check no_offense.rb has at least 5 non-empty lines
        if let Ok(no_offense_content) = fs::read_to_string(effective_dir.join("no_offense.rb")) {
            let non_empty = no_offense_content
                .lines()
                .filter(|l| !l.trim().is_empty())
                .count();
            if non_empty < 5 {
                failures.push(format!(
                    "{cop_name}: only {non_empty} non-empty line(s) in no_offense.rb, need at least 5"
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

        let has_offense_file = effective_dir.join("offense.rb").exists();
        let has_offense_dir = effective_dir.join("offense").is_dir();
        if !has_offense_file && !has_offense_dir {
            missing.push(format!("{cop_name}: missing offense.rb or offense/ directory"));
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
    let config = load_config(None, None, None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = Args {
        only: vec!["Metrics/MethodLength".to_string()],
        ..default_args()
    };

    let result = run_linter(&discovered(&[file]), &config, &registry, &args);
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
    let config = load_config(None, None, None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = Args {
        only: vec!["Naming/MethodName".to_string()],
        ..default_args()
    };

    let result = run_linter(&discovered(&[file]), &config, &registry, &args);
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
    let config = load_config(Some(config_path.as_path()), None, None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = Args {
        only: vec!["Metrics/MethodLength".to_string()],
        ..default_args()
    };

    let result = run_linter(&discovered(&[file]), &config, &registry, &args);
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

// ---------- M10: Config inheritance tests ----------

#[test]
fn inherit_from_merges_configs() {
    let child_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata/config/inherit_from/child.yml");
    let config = load_config(Some(child_path.as_path()), None, None).unwrap();

    // Child overrides Layout/LineLength Max from 100 to 120
    let cc = config.cop_config("Layout/LineLength");
    assert_eq!(
        cc.options.get("Max").and_then(|v| v.as_u64()),
        Some(120),
        "Child should override base's Max:100 with Max:120"
    );

    // Child disables FrozenStringLiteralComment (base had it enabled)
    assert!(
        !config.is_cop_enabled("Style/FrozenStringLiteralComment", Path::new("a.rb"), &[], &[]),
        "Child should disable FrozenStringLiteralComment"
    );

    // Global excludes: child's AllCops.Exclude replaces base's by default
    // (RuboCop only merges when inherit_mode: merge: [Exclude] is specified)
    let excludes = config.global_excludes();
    assert!(
        !excludes.contains(&"vendor/**".to_string()),
        "Base's vendor/** should be replaced by child's excludes"
    );
    assert!(
        excludes.contains(&"tmp/**".to_string()),
        "Child's tmp/** exclude should be present"
    );
}

#[test]
fn circular_inherit_from_is_detected() {
    let circular_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("testdata/config/inherit_from/circular_a.yml");
    let result = load_config(Some(circular_path.as_path()), None, None);
    assert!(result.is_err(), "Circular inheritance should fail");
    let err_msg = format!("{:#}", result.unwrap_err());
    assert!(
        err_msg.contains("Circular config inheritance"),
        "Error should mention circular inheritance, got: {err_msg}"
    );
}

// ---------- M10: --rubocop-only CLI tests ----------

#[test]
fn rubocop_only_outputs_uncovered_cops() {
    let config_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata/config/rubocop_only/mixed.yml");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_turbocop"))
        .args(["--rubocop-only", "--config", config_path.to_str().unwrap()])
        .output()
        .expect("Failed to execute turbocop");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "--rubocop-only should exit 0, stderr: {stderr}"
    );

    // Should contain uncovered cops
    assert!(
        stdout.contains("Custom/MyCop"),
        "Output should contain Custom/MyCop, got: {stdout}"
    );
    assert!(
        stdout.contains("Vendor/SpecialCop"),
        "Output should contain Vendor/SpecialCop, got: {stdout}"
    );

    // Should NOT contain turbocop-covered cops
    assert!(
        !stdout.contains("Style/FrozenStringLiteralComment"),
        "Output should NOT contain covered cop Style/FrozenStringLiteralComment"
    );
    assert!(
        !stdout.contains("Layout/TrailingWhitespace"),
        "Output should NOT contain covered cop Layout/TrailingWhitespace"
    );

    // Should NOT contain disabled cops
    assert!(
        !stdout.contains("Custom/DisabledCop"),
        "Output should NOT contain disabled cop Custom/DisabledCop"
    );
}

// ---------- M10: --stdin CLI tests ----------

#[test]
fn stdin_detects_trailing_whitespace() {
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_turbocop"))
        .args([
            "--stdin",
            "test.rb",
            "--only",
            "Layout/TrailingWhitespace",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start turbocop");

    // Write source with trailing whitespace to stdin
    {
        use std::io::Write;
        let stdin = child.stdin.as_mut().unwrap();
        stdin.write_all(b"x = 1   \n").unwrap();
    }

    let output = child.wait_with_output().expect("Failed to wait for turbocop");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        !output.status.success(),
        "--stdin should exit 1 when offenses found"
    );
    assert!(
        stdout.contains("Layout/TrailingWhitespace"),
        "Should detect trailing whitespace via stdin, got: {stdout}"
    );
    assert!(
        stdout.contains("test.rb"),
        "Display path should be test.rb, got: {stdout}"
    );
}

#[test]
fn stdin_clean_code_exits_zero() {
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_turbocop"))
        .args([
            "--stdin",
            "clean.rb",
            "--only",
            "Layout/TrailingWhitespace",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start turbocop");

    {
        use std::io::Write;
        let stdin = child.stdin.as_mut().unwrap();
        stdin.write_all(b"x = 1\ny = 2\n").unwrap();
    }

    let output = child.wait_with_output().expect("Failed to wait for turbocop");

    assert!(
        output.status.success(),
        "--stdin with clean code should exit 0"
    );
}

#[test]
fn stdin_display_path_affects_include_matching() {
    // RSpec cops should run when display path matches spec pattern.
    // Need a config dir with plugins: rubocop-rspec so the RSpec department is enabled.
    let config_dir = temp_dir("stdin_include_config");
    write_file(&config_dir, ".rubocop.yml", b"plugins:\n  - rubocop-rspec\n");

    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_turbocop"))
        .args([
            "--stdin",
            "spec/foo_spec.rb",
            "--only",
            "RSpec/Focus",
        ])
        .current_dir(&config_dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start turbocop");

    {
        use std::io::Write;
        let stdin = child.stdin.as_mut().unwrap();
        stdin
            .write_all(b"RSpec.describe Foo, :focus do\n  it 'works' do\n    expect(1).to eq(1)\n  end\nend\n")
            .unwrap();
    }

    let output = child.wait_with_output().expect("Failed to wait for turbocop");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("RSpec/Focus"),
        "RSpec/Focus should fire when display path matches spec pattern, got: {stdout}"
    );

    // Same code with non-spec display path — RSpec cops should NOT run
    let mut child2 = std::process::Command::new(env!("CARGO_BIN_EXE_turbocop"))
        .args(["--stdin", "app/foo.rb", "--only", "RSpec/Focus"])
        .current_dir(&config_dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start turbocop");

    {
        use std::io::Write;
        let stdin = child2.stdin.as_mut().unwrap();
        stdin
            .write_all(b"RSpec.describe Foo, :focus do\n  it 'works' do\n    expect(1).to eq(1)\n  end\nend\n")
            .unwrap();
    }

    let output2 = child2
        .wait_with_output()
        .expect("Failed to wait for turbocop");
    let stdout2 = String::from_utf8_lossy(&output2.stdout);

    assert!(
        !stdout2.contains("RSpec/Focus"),
        "RSpec/Focus should NOT fire when display path is app/foo.rb, got: {stdout2}"
    );
}

// ---------- M10: Config inheritance with linter pipeline ----------

#[test]
fn inherited_config_affects_linting() {
    let dir = temp_dir("inherited_linting");
    // base.yml disables TrailingWhitespace
    write_file(
        &dir,
        "base.yml",
        b"Layout/TrailingWhitespace:\n  Enabled: false\n",
    );
    let config_path = write_file(
        &dir,
        ".rubocop.yml",
        b"inherit_from: base.yml\n",
    );
    let file = write_file(&dir, "test.rb", b"x = 1   \n");

    let config = load_config(Some(config_path.as_path()), None, None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = Args {
        only: vec!["Layout/TrailingWhitespace".to_string()],
        ..default_args()
    };

    let result = run_linter(&discovered(&[file]), &config, &registry, &args);
    assert!(
        result.diagnostics.is_empty(),
        "TrailingWhitespace should be disabled by inherited config, got {} offenses",
        result.diagnostics.len()
    );

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn lint_source_directly() {
    use turbocop::linter::lint_source;
    use turbocop::parse::source::SourceFile;

    let source =
        SourceFile::from_string(PathBuf::from("test.rb"), "x = 1   \n".to_string());
    let config = load_config(None, None, None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = Args {
        only: vec!["Layout/TrailingWhitespace".to_string()],
        ..default_args()
    };

    let result = lint_source(&source, &config, &registry, &args);
    assert_eq!(result.file_count, 1);
    assert!(
        !result.diagnostics.is_empty(),
        "lint_source should detect trailing whitespace"
    );
    assert_eq!(
        result.diagnostics[0].cop_name,
        "Layout/TrailingWhitespace"
    );
    assert_eq!(result.diagnostics[0].path, "test.rb");
}

// ---------- --list-cops CLI tests ----------

#[test]
fn list_cops_prints_all_registered_cops() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_turbocop"))
        .args(["--list-cops"])
        .output()
        .expect("Failed to execute turbocop");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "--list-cops should exit 0, stderr: {stderr}"
    );

    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines.len(),
        915,
        "Expected 915 cop names, got {}",
        lines.len()
    );

    // Verify sorted order
    let mut sorted = lines.clone();
    sorted.sort();
    assert_eq!(lines, sorted, "--list-cops output should be sorted");

    // Spot-check a few cops from different departments
    assert!(
        lines.contains(&"Layout/TrailingWhitespace"),
        "Should contain Layout/TrailingWhitespace"
    );
    assert!(
        lines.contains(&"Style/FrozenStringLiteralComment"),
        "Should contain Style/FrozenStringLiteralComment"
    );
    assert!(
        lines.contains(&"Lint/Debugger"),
        "Should contain Lint/Debugger"
    );
    assert!(
        lines.contains(&"Performance/Detect"),
        "Should contain Performance/Detect"
    );
    assert!(
        lines.contains(&"Rails/FindBy"),
        "Should contain Rails/FindBy"
    );
    assert!(
        lines.contains(&"RSpec/Focus"),
        "Should contain RSpec/Focus"
    );
}

// ---------- Config audit ----------

#[test]
fn config_audit() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let registry = CopRegistry::default_registry();

    // Parse each vendor default YAML
    let yaml_paths = [
        manifest.join("vendor/rubocop/config/default.yml"),
        manifest.join("vendor/rubocop-performance/config/default.yml"),
        manifest.join("vendor/rubocop-rails/config/default.yml"),
        manifest.join("vendor/rubocop-rspec/config/default.yml"),
    ];

    let infrastructure_keys: std::collections::HashSet<&str> = [
        "Enabled",
        "Description",
        "StyleGuide",
        "Reference",
        "References",
        "VersionAdded",
        "VersionChanged",
        "SafeAutoCorrect",
        "AutoCorrect",
        "SupportedStyles",
        "SupportedStylesForMultiline",
        "Exclude",
        "Include",
        "Safe",
        "DocumentationBaseURL",
        "inherit_mode",
        "Severity",
    ]
    .into_iter()
    .collect();

    // Also filter Supported* prefix keys (e.g. SupportedStylesAlignWith)
    let is_infrastructure = |key: &str| -> bool {
        infrastructure_keys.contains(key) || key.starts_with("Supported")
    };

    // Build a map of cop name -> YAML child keys (config options only)
    let mut yaml_cop_keys: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for yaml_path in &yaml_paths {
        let content = fs::read_to_string(yaml_path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {e}", yaml_path.display()));
        let doc: serde_yml::Value = serde_yml::from_str(&content)
            .unwrap_or_else(|e| panic!("Failed to parse {}: {e}", yaml_path.display()));

        if let serde_yml::Value::Mapping(map) = doc {
            for (key, value) in &map {
                let key_str = match key.as_str() {
                    Some(s) => s,
                    None => continue,
                };
                // Only cop names contain '/'
                if !key_str.contains('/') {
                    continue;
                }
                if let serde_yml::Value::Mapping(child_map) = value {
                    let config_keys: Vec<String> = child_map
                        .keys()
                        .filter_map(|k| k.as_str())
                        .filter(|k| !is_infrastructure(k))
                        .map(|k| k.to_string())
                        .collect();
                    if !config_keys.is_empty() {
                        yaml_cop_keys.insert(key_str.to_string(), config_keys);
                    }
                }
            }
        }
    }

    // For each cop in the registry, check which YAML keys the Rust source reads
    let mut current_gaps: Vec<String> = Vec::new();

    for cop_name in registry.names() {
        let yaml_keys = match yaml_cop_keys.get(cop_name) {
            Some(keys) => keys,
            None => continue, // No YAML config for this cop
        };

        let parts: Vec<&str> = cop_name.split('/').collect();
        let dept = parts[0].to_lowercase();
        let name = to_snake_case(parts[1]);

        // Try to find the source file
        let src_path = manifest.join(format!("src/cop/{dept}/{name}.rs"));
        let src_path_alt = manifest.join(format!("src/cop/{dept}/{name}_cop.rs"));
        let source = if src_path.exists() {
            fs::read_to_string(&src_path).unwrap()
        } else if src_path_alt.exists() {
            fs::read_to_string(&src_path_alt).unwrap()
        } else {
            continue; // Source not found, skip
        };

        let mut missing: Vec<&str> = Vec::new();
        for key in yaml_keys {
            // Check if the Rust source references this config key via any get_ method
            let pattern = format!("\"{key}\"");
            if !source.contains(&pattern) {
                missing.push(key);
            }
        }

        if !missing.is_empty() {
            current_gaps.push(format!("{cop_name}: {}", missing.join(", ")));
        }
    }

    current_gaps.sort();

    assert!(
        current_gaps.is_empty(),
        "\n[config_audit] {} cop(s) have YAML config keys not referenced in Rust source:\n{}\n",
        current_gaps.len(),
        current_gaps.iter().map(|g| format!("  {g}")).collect::<Vec<_>>().join("\n")
    );
}

// ---------- Prism pitfalls ----------

#[test]
fn prism_pitfalls() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let registry = CopRegistry::default_registry();

    let mut current_gaps: Vec<String> = Vec::new();

    for cop_name in registry.names() {
        let parts: Vec<&str> = cop_name.split('/').collect();
        let dept = parts[0].to_lowercase();
        let name = to_snake_case(parts[1]);

        let src_path = manifest.join(format!("src/cop/{dept}/{name}.rs"));
        let src_path_alt = manifest.join(format!("src/cop/{dept}/{name}_cop.rs"));
        let source = if src_path.exists() {
            fs::read_to_string(&src_path).unwrap()
        } else if src_path_alt.exists() {
            fs::read_to_string(&src_path_alt).unwrap()
        } else {
            continue;
        };

        // Pitfall 1: KeywordHashNode gap
        if source.contains("as_hash_node") && !source.contains("keyword_hash_node") {
            current_gaps.push(format!(
                "{cop_name}: handles Hash literals (as_hash_node) but misses keyword arguments (keyword_hash_node)"
            ));
        }

        // Pitfall 2: ConstantPathNode gap
        if source.contains("as_constant_read_node") && !source.contains("constant_path_node") {
            current_gaps.push(format!(
                "{cop_name}: handles simple constants (as_constant_read_node) but misses qualified constants (constant_path_node)"
            ));
        }
    }

    current_gaps.sort();

    assert!(
        current_gaps.is_empty(),
        "\n[prism_pitfalls] {} pitfall(s) found:\n{}\n",
        current_gaps.len(),
        current_gaps.iter().map(|g| format!("  {g}")).collect::<Vec<_>>().join("\n")
    );
}

// ---------- NodePattern codegen integration tests ----------

#[test]
fn codegen_generates_output_for_vendor_file() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let vendor_file = manifest.join("vendor/rubocop-rails/lib/rubocop/cop/rails/inverse_of.rb");

    if !vendor_file.exists() {
        eprintln!("Skipping codegen integration test: vendor file not found");
        return;
    }

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_node_pattern_codegen"))
        .args(["generate", vendor_file.to_str().unwrap()])
        .output()
        .expect("Failed to execute node_pattern_codegen");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.status.success(),
        "codegen should exit 0 for a file with patterns, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // inverse_of.rb has multiple def_node_matcher patterns
    assert!(
        stdout.contains("Auto-generated by node_pattern_codegen"),
        "Output should contain header comment"
    );
    assert!(
        stdout.contains("fn "),
        "Output should contain generated Rust functions"
    );
    assert!(
        stdout.contains("as_call_node") || stdout.contains("as_"),
        "Output should contain Prism cast methods"
    );
    // Should extract several patterns
    assert!(
        stdout.matches("def_node_matcher").count() >= 3,
        "Should reference at least 3 patterns from inverse_of.rb"
    );
}

#[test]
fn codegen_handles_file_with_no_patterns() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Use a file that likely has no def_node_matcher
    let test_file = manifest.join("testdata/cops/layout/trailing_whitespace/offense.rb");

    if !test_file.exists() {
        eprintln!("Skipping codegen no-patterns test: test file not found");
        return;
    }

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_node_pattern_codegen"))
        .args(["generate", test_file.to_str().unwrap()])
        .output()
        .expect("Failed to execute node_pattern_codegen");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "codegen should exit 0 even with no patterns"
    );
    assert!(
        stdout.is_empty(),
        "No output expected for file without patterns, got: {stdout}"
    );
    assert!(
        stderr.contains("No def_node_matcher"),
        "Should print informational message to stderr, got: {stderr}"
    );
}

#[test]
fn codegen_exits_nonzero_for_missing_file() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_node_pattern_codegen"))
        .args(["generate", "/nonexistent/file.rb"])
        .output()
        .expect("Failed to execute node_pattern_codegen");

    assert!(
        !output.status.success(),
        "codegen should exit non-zero for missing file"
    );
}

#[test]
fn codegen_exits_nonzero_without_args() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_node_pattern_codegen"))
        .output()
        .expect("Failed to execute node_pattern_codegen");

    assert!(
        !output.status.success(),
        "codegen should exit non-zero without arguments"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Usage"),
        "Should print usage, got: {stderr}"
    );
}

// ---------- Lint/RedundantCopDisableDirective tests ----------
//
// This cop requires the full linter pipeline because it detects disable
// directives that didn't suppress any offense. It can't use the standard
// fixture test framework since that runs a single cop in isolation.
//
// The implementation is conservative: it only flags directives for cops
// that are UNKNOWN in a known department (likely renamed/removed) or
// cops that are DISABLED/EXCLUDED for this file. It does NOT flag
// directives for cops that ARE running, since turbocop might have gaps
// in its detection vs. RuboCop.

#[test]
fn no_redundant_disable_unknown_cop_in_known_department() {
    // Disable for a cop name that doesn't exist in a known department.
    // Style/ is a known department, but NonexistentFakeCop doesn't exist.
    // We do NOT flag this because it could be a project-local custom cop
    // (e.g., Style/MiddleDot in mastodon).
    let dir = temp_dir("redundant_disable_known_dept");
    let file = write_file(
        &dir,
        "test.rb",
        b"# frozen_string_literal: true\n\nx = 1 # rubocop:disable Style/NonexistentFakeCop\n",
    );
    let config = load_config(None, None, None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = default_args();

    let result = run_linter(&discovered(&[file]), &config, &registry, &args);
    let redundant: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.cop_name == "Lint/RedundantCopDisableDirective")
        .collect();

    assert!(
        redundant.is_empty(),
        "Should NOT flag unknown cop (could be custom), got: {:?}",
        redundant
    );

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn no_redundant_disable_unknown_department() {
    // Disable for a cop in a completely unknown department.
    // CustomDept/ doesn't exist in the registry, so it might be a
    // custom cop from a plugin gem — don't flag.
    let dir = temp_dir("redundant_disable_unknown_dept");
    let file = write_file(
        &dir,
        "test.rb",
        b"# frozen_string_literal: true\n\nx = 1 # rubocop:disable CustomDept/CustomCop\n",
    );
    let config = load_config(None, None, None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = default_args();

    let result = run_linter(&discovered(&[file]), &config, &registry, &args);
    let redundant: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.cop_name == "Lint/RedundantCopDisableDirective")
        .collect();

    assert!(
        redundant.is_empty(),
        "Should NOT flag disable for unknown department (might be custom cop), got: {:?}",
        redundant
    );

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn no_redundant_disable_running_cop_no_offense() {
    // Disable for a real cop that is running but doesn't fire on clean code.
    // Conservative approach: don't flag, since it might be a detection gap.
    let dir = temp_dir("redundant_disable_running_clean");
    let file = write_file(
        &dir,
        "test.rb",
        b"# frozen_string_literal: true\n\nx = 1 # rubocop:disable Layout/TrailingWhitespace\n",
    );
    let config = load_config(None, None, None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = default_args();

    let result = run_linter(&discovered(&[file]), &config, &registry, &args);
    let redundant: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.cop_name == "Lint/RedundantCopDisableDirective")
        .collect();

    assert!(
        redundant.is_empty(),
        "Should NOT flag disable for a running cop (conservative approach), got: {:?}",
        redundant
    );

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn no_redundant_disable_when_offense_suppressed() {
    // Block disable around a line with trailing whitespace -> used, not redundant.
    let dir = temp_dir("redundant_disable_used");
    let file = write_file(
        &dir,
        "test.rb",
        b"# frozen_string_literal: true\n\n# rubocop:disable Layout/TrailingWhitespace\nx = 1   \n# rubocop:enable Layout/TrailingWhitespace\n",
    );
    let config = load_config(None, None, None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = default_args();

    let result = run_linter(&discovered(&[file]), &config, &registry, &args);
    let redundant: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.cop_name == "Lint/RedundantCopDisableDirective")
        .collect();

    assert!(
        redundant.is_empty(),
        "Should NOT report redundant disable when it actually suppresses an offense, got: {:?}",
        redundant
    );

    // Also verify the trailing whitespace offense was suppressed
    let tw: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.cop_name == "Layout/TrailingWhitespace")
        .collect();
    assert!(
        tw.is_empty(),
        "TrailingWhitespace should be suppressed by disable directive"
    );

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn no_redundant_disable_with_only_flag() {
    // When --only is set, don't report redundant disables (cops are filtered).
    // Style/Copyright is disabled by default and WOULD be flagged as redundant
    // normally, but --only means cops are filtered so we skip redundancy checks.
    let dir = temp_dir("redundant_disable_only");
    let file = write_file(
        &dir,
        "test.rb",
        b"# frozen_string_literal: true\n\nx = 1 # rubocop:disable Style/Copyright\n",
    );
    let config = load_config(None, None, None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = Args {
        only: vec!["Layout/TrailingWhitespace".to_string()],
        ..default_args()
    };

    let result = run_linter(&discovered(&[file]), &config, &registry, &args);
    let redundant: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.cop_name == "Lint/RedundantCopDisableDirective")
        .collect();

    assert!(
        redundant.is_empty(),
        "Should NOT report redundant disables when --only is set, got: {:?}",
        redundant
    );

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn redundant_disable_disabled_cop() {
    // Disable for a cop that is explicitly disabled in config.
    // Style/Copyright is disabled by default (default_enabled = false).
    // A disable directive for it is redundant since the cop doesn't run.
    let dir = temp_dir("redundant_disable_disabled_cop");
    let file = write_file(
        &dir,
        "test.rb",
        b"# frozen_string_literal: true\n\nx = 1 # rubocop:disable Style/Copyright\n",
    );
    let config = load_config(None, None, None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = default_args();

    let result = run_linter(&discovered(&[file]), &config, &registry, &args);
    let redundant: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.cop_name == "Lint/RedundantCopDisableDirective")
        .collect();

    assert_eq!(
        redundant.len(),
        1,
        "Expected 1 redundant disable for disabled cop Style/Copyright, got: {:?}",
        redundant
    );
    assert!(
        redundant[0].message.contains("Style/Copyright"),
        "Message should mention the cop name: {}",
        redundant[0].message
    );

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn no_redundant_disable_department_used() {
    // Department-level disables are never flagged (conservative)
    let dir = temp_dir("redundant_disable_dept_used");
    let file = write_file(
        &dir,
        "test.rb",
        b"# frozen_string_literal: true\n\nx = 1   # rubocop:disable Layout\n",
    );
    let config = load_config(None, None, None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = default_args();

    let result = run_linter(&discovered(&[file]), &config, &registry, &args);
    let redundant: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.cop_name == "Lint/RedundantCopDisableDirective")
        .collect();

    assert!(
        redundant.is_empty(),
        "Department disable should NOT be redundant, got: {:?}",
        redundant
    );

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn redundant_disable_todo_directive() {
    // rubocop:todo is treated the same as disable — disabled cop is flagged
    let dir = temp_dir("redundant_disable_todo");
    let file = write_file(
        &dir,
        "test.rb",
        b"# frozen_string_literal: true\n\nx = 1 # rubocop:todo Style/Copyright\n",
    );
    let config = load_config(None, None, None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = default_args();

    let result = run_linter(&discovered(&[file]), &config, &registry, &args);
    let redundant: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.cop_name == "Lint/RedundantCopDisableDirective")
        .collect();

    assert_eq!(
        redundant.len(),
        1,
        "rubocop:todo for disabled cop should be flagged, got: {:?}",
        redundant
    );

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn no_redundant_disable_all() {
    // `disable all` is never flagged as redundant (too broad to check)
    let dir = temp_dir("redundant_disable_all_not_flagged");
    let file = write_file(
        &dir,
        "test.rb",
        b"# frozen_string_literal: true\n\n# rubocop:disable all\nx = 1\n# rubocop:enable all\n",
    );
    let config = load_config(None, None, None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = Args {
        except: vec!["Style/DisableCopsWithinSourceCodeDirective".to_string()],
        ..default_args()
    };

    let result = run_linter(&discovered(&[file]), &config, &registry, &args);
    let redundant: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.cop_name == "Lint/RedundantCopDisableDirective")
        .collect();

    assert!(
        redundant.is_empty(),
        "disable all should NOT be flagged as redundant (conservative), got: {:?}",
        redundant
    );

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn redundant_disable_renamed_cop_extended_format() {
    // Naming/PredicateName was renamed to Naming/PredicatePrefix (extended format
    // in obsoletion.yml with new_name key). A disable directive for the old name
    // should be flagged as redundant since the old cop name no longer exists.
    let dir = temp_dir("redundant_disable_renamed_cop");
    let file = write_file(
        &dir,
        "test.rb",
        b"# frozen_string_literal: true\n\ndef is_foo? # rubocop:disable Naming/PredicateName\n  true\nend\n",
    );
    let config = load_config(None, None, None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = default_args();

    let result = run_linter(&discovered(&[file]), &config, &registry, &args);
    let redundant: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.cop_name == "Lint/RedundantCopDisableDirective")
        .collect();

    assert_eq!(
        redundant.len(),
        1,
        "Expected 1 redundant disable for renamed cop Naming/PredicateName, got: {:?}",
        redundant
    );
    assert!(
        redundant[0].message.contains("Naming/PredicateName"),
        "Message should mention the old cop name: {}",
        redundant[0].message
    );

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn redundant_disable_renamed_cop_simple_value() {
    // Layout/Tab was renamed to Layout/IndentationStyle (simple key: value format
    // in obsoletion.yml). A disable directive for the old name should be flagged.
    let dir = temp_dir("redundant_disable_renamed_simple");
    let file = write_file(
        &dir,
        "test.rb",
        b"# frozen_string_literal: true\n\nx = 1 # rubocop:disable Layout/Tab\n",
    );
    let config = load_config(None, None, None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = default_args();

    let result = run_linter(&discovered(&[file]), &config, &registry, &args);
    let redundant: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.cop_name == "Lint/RedundantCopDisableDirective")
        .collect();

    assert_eq!(
        redundant.len(),
        1,
        "Expected 1 redundant disable for renamed cop Layout/Tab, got: {:?}",
        redundant
    );
    assert!(
        redundant[0].message.contains("Layout/Tab"),
        "Message should mention the old cop name: {}",
        redundant[0].message
    );

    fs::remove_dir_all(&dir).ok();
}

/// Ensure no cop still uses the old `-> Vec<Diagnostic>` return type for trait methods.
///
/// The Cop trait methods (check_lines, check_source, check_node) now take
/// `diagnostics: &mut Vec<Diagnostic>` and return `()`. This test scans all cop
/// source files to catch any new cop that accidentally uses the old signature pattern.
#[test]
fn no_cop_returns_vec_diagnostic() {
    let cop_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/cop");
    let mut failures = Vec::new();

    let skip_files = ["mod.rs", "walker.rs", "node_type.rs", "registry.rs", "util.rs"];

    fn collect_rs_files(dir: &Path, files: &mut Vec<PathBuf>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    collect_rs_files(&path, files);
                } else if path.extension().map_or(false, |e| e == "rs") {
                    files.push(path);
                }
            }
        }
    }

    let mut rs_files = Vec::new();
    collect_rs_files(&cop_dir, &mut rs_files);

    for path in &rs_files {
        let filename = path.file_name().unwrap().to_str().unwrap();
        if skip_files.contains(&filename) {
            continue;
        }

        let content = fs::read_to_string(path).unwrap();

        for (i, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if (trimmed.contains("fn check_node(")
                || trimmed.contains("fn check_lines(")
                || trimmed.contains("fn check_source("))
                && !trimmed.starts_with("//")
            {
                let remaining: String = content.lines().skip(i).take(10).collect::<Vec<_>>().join(" ");
                if remaining.contains("-> Vec<Diagnostic>") {
                    let rel = path.strip_prefix(&cop_dir).unwrap().display();
                    failures.push(format!(
                        "{rel}:{}: trait method still returns Vec<Diagnostic> — use `diagnostics: &mut Vec<Diagnostic>` parameter instead",
                        i + 1
                    ));
                }
            }
        }
    }

    assert!(
        failures.is_empty(),
        "Cops using old Vec<Diagnostic> return pattern (should use &mut Vec<Diagnostic> parameter):\n{}",
        failures.join("\n")
    );
}

// ---------- Result cache integration tests ----------

#[test]
fn cache_produces_same_results_as_uncached() {
    let dir = temp_dir("cache_same_results");
    // File with offenses
    let file1 = write_file(&dir, "trailing.rb", b"x = 1 \ny = 2\n");
    // File without offenses
    let file2 = write_file(&dir, "clean.rb", b"x = 1\ny = 2\n");

    let config = load_config(None, Some(dir.as_path()), None).unwrap();
    let registry = CopRegistry::default_registry();

    // Run without cache
    let args_no_cache = Args {
        only: vec!["Layout/TrailingWhitespace".to_string()],
        cache: "false".to_string(),
        ..default_args()
    };
    let result_no_cache = run_linter(&discovered(&[file1.clone(), file2.clone()]), &config, &registry, &args_no_cache);

    // Run with cache (cold)
    let cache_dir = dir.join("cache");
    fs::create_dir_all(&cache_dir).unwrap();
    // SAFETY: test-only, set env var for cache root
    unsafe { std::env::set_var("TURBOCOP_CACHE_DIR", &cache_dir) };
    let args_cached = Args {
        only: vec!["Layout/TrailingWhitespace".to_string()],
        cache: "true".to_string(),
        ..default_args()
    };
    let result_cold = run_linter(&discovered(&[file1.clone(), file2.clone()]), &config, &registry, &args_cached);

    // Run with cache (warm)
    let result_warm = run_linter(&discovered(&[file1.clone(), file2.clone()]), &config, &registry, &args_cached);

    unsafe { std::env::remove_var("TURBOCOP_CACHE_DIR") };

    // All three runs should produce identical diagnostics
    assert_eq!(
        result_no_cache.diagnostics.len(),
        result_cold.diagnostics.len(),
        "Cold cache should produce same offense count as uncached"
    );
    assert_eq!(
        result_no_cache.diagnostics.len(),
        result_warm.diagnostics.len(),
        "Warm cache should produce same offense count as uncached"
    );

    // Verify actual offenses match
    for (d1, d2) in result_no_cache.diagnostics.iter().zip(result_warm.diagnostics.iter()) {
        assert_eq!(d1.cop_name, d2.cop_name);
        assert_eq!(d1.location.line, d2.location.line);
        assert_eq!(d1.location.column, d2.location.column);
        assert_eq!(d1.message, d2.message);
    }

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn cache_invalidated_by_file_change() {
    let dir = temp_dir("cache_file_change");
    let file = write_file(&dir, "test.rb", b"x = 1 \n");

    let cache_dir = dir.join("cache");
    fs::create_dir_all(&cache_dir).unwrap();
    unsafe { std::env::set_var("TURBOCOP_CACHE_DIR", &cache_dir) };

    let config = load_config(None, Some(dir.as_path()), None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = Args {
        only: vec!["Layout/TrailingWhitespace".to_string()],
        cache: "true".to_string(),
        ..default_args()
    };

    // First run: should detect trailing whitespace
    let result1 = run_linter(&discovered(&[file.clone()]), &config, &registry, &args);
    assert_eq!(result1.diagnostics.len(), 1, "Should detect trailing whitespace");

    // Modify file to remove the offense
    fs::write(&file, b"x = 1\n").unwrap();

    // Second run: file changed, cache should miss, no offense
    let result2 = run_linter(&discovered(&[file.clone()]), &config, &registry, &args);
    assert_eq!(result2.diagnostics.len(), 0, "After fix, should find no offenses");

    unsafe { std::env::remove_var("TURBOCOP_CACHE_DIR") };
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn cache_invalidated_by_config_change() {
    let dir = temp_dir("cache_config_change");
    let file = write_file(&dir, "test.rb", b"x = 1 \n");

    let cache_dir = dir.join("cache");
    fs::create_dir_all(&cache_dir).unwrap();
    unsafe { std::env::set_var("TURBOCOP_CACHE_DIR", &cache_dir) };

    let config = load_config(None, Some(dir.as_path()), None).unwrap();
    let registry = CopRegistry::default_registry();

    // Run with --only TrailingWhitespace
    let args1 = Args {
        only: vec!["Layout/TrailingWhitespace".to_string()],
        cache: "true".to_string(),
        ..default_args()
    };
    let result1 = run_linter(&discovered(&[file.clone()]), &config, &registry, &args1);
    assert_eq!(result1.diagnostics.len(), 1);

    // Run with --only a different cop — different session hash, so cache miss
    let args2 = Args {
        only: vec!["Style/FrozenStringLiteralComment".to_string()],
        cache: "true".to_string(),
        ..default_args()
    };
    let result2 = run_linter(&discovered(&[file.clone()]), &config, &registry, &args2);
    // Should get different results (FrozenStringLiteralComment, not TrailingWhitespace)
    for d in &result2.diagnostics {
        assert_ne!(
            d.cop_name, "Layout/TrailingWhitespace",
            "Config change should use different session, not return stale cached results"
        );
    }

    unsafe { std::env::remove_var("TURBOCOP_CACHE_DIR") };
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn cache_preserves_all_severity_types() {
    use turbocop::cache::{CacheLookup, ResultCache};
    use turbocop::diagnostic::{Diagnostic, Location, Severity};

    let tmp = tempfile::tempdir().unwrap();
    let configs = vec![turbocop::cop::CopConfig::default()];
    let args = default_args();
    let cache = ResultCache::with_root(tmp.path(), "0.1.0-test", &configs, &args);

    // Create a real file so stat() works
    let rb_file = tmp.path().join("severity_test.rb");
    let content = b"test content";
    fs::write(&rb_file, content).unwrap();

    let diagnostics = vec![
        Diagnostic {
            path: rb_file.to_string_lossy().to_string(),
            location: Location { line: 1, column: 0 },
            severity: Severity::Convention,
            cop_name: "Style/A".to_string(),
            message: "convention".to_string(),
        },
        Diagnostic {
            path: rb_file.to_string_lossy().to_string(),
            location: Location { line: 2, column: 5 },
            severity: Severity::Warning,
            cop_name: "Lint/B".to_string(),
            message: "warning".to_string(),
        },
        Diagnostic {
            path: rb_file.to_string_lossy().to_string(),
            location: Location { line: 3, column: 10 },
            severity: Severity::Error,
            cop_name: "Security/C".to_string(),
            message: "error".to_string(),
        },
        Diagnostic {
            path: rb_file.to_string_lossy().to_string(),
            location: Location { line: 4, column: 0 },
            severity: Severity::Fatal,
            cop_name: "Lint/D".to_string(),
            message: "fatal".to_string(),
        },
    ];

    cache.put(&rb_file, content, &diagnostics);

    // Should get a stat hit since file hasn't changed
    match cache.get_by_stat(&rb_file) {
        CacheLookup::StatHit(cached) => {
            assert_eq!(cached.len(), 4);
            assert_eq!(cached[0].severity, Severity::Convention);
            assert_eq!(cached[1].severity, Severity::Warning);
            assert_eq!(cached[1].location.column, 5);
            assert_eq!(cached[2].severity, Severity::Error);
            assert_eq!(cached[2].location.line, 3);
            assert_eq!(cached[3].severity, Severity::Fatal);
        }
        _ => panic!("Expected StatHit"),
    }
}

#[test]
fn redundant_disable_excluded_cop() {
    // A cop that is enabled but excluded from the file's path via Exclude
    // patterns doesn't execute on that file. A disable directive for it is
    // therefore redundant and should be flagged.
    let dir = temp_dir("redundant_disable_excluded_cop");
    let sub = dir.join("app").join("controllers");
    fs::create_dir_all(&sub).unwrap();
    let file = write_file(
        &dir,
        "app/controllers/test_controller.rb",
        b"# frozen_string_literal: true\n\ndef edit # rubocop:disable Lint/UselessMethodDefinition\n  super\nend\n",
    );
    // Config that excludes Lint/UselessMethodDefinition from app/controllers/**
    write_file(
        &dir,
        ".rubocop.yml",
        b"Lint/UselessMethodDefinition:\n  Exclude:\n    - 'app/controllers/**/*'\n",
    );
    let config = load_config(None, Some(&dir), None).unwrap();
    let registry = CopRegistry::default_registry();
    let args = default_args();

    let result = run_linter(&discovered(&[file]), &config, &registry, &args);
    let redundant: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.cop_name == "Lint/RedundantCopDisableDirective")
        .collect();

    assert_eq!(
        redundant.len(),
        1,
        "Expected 1 redundant disable for excluded cop, got: {:?}",
        redundant
    );
    assert!(
        redundant[0]
            .message
            .contains("Lint/UselessMethodDefinition"),
        "Message should mention the excluded cop: {}",
        redundant[0].message
    );

    fs::remove_dir_all(&dir).ok();
}
