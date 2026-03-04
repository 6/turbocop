use crate::cop::node_type::{CALL_NODE, DEF_NODE};
use crate::cop::util::{collect_foldable_ranges, collect_heredoc_ranges, count_body_lines_ex};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-03)
///
/// Corpus oracle reported FP=714, FN=5,610.
///
/// FN=5,610: Fixed by removing unconditional heredoc folding (commit a1d41934).
/// Heredocs are now only folded when "heredoc" is in CountAsOne (default: []).
///
/// FP=714→PASS: Fixed BeginNode off-by-one for methods with rescue/ensure
/// (commit 1901c910). Prism's BeginNode.location() starts at the def keyword,
/// not the first body statement. When the method was inside a class/module,
/// this caused effective_start_offset to include the def line in the body count.
/// Fix: for BeginNode bodies, use the first child statement's location instead.
///
/// Remaining excess (9,721) is within file-drop noise (8,174 from jruby).
///
/// ## Corpus investigation (2026-03-04)
///
/// Corpus oracle currently reports FP-heavy divergence for this cop.
///
/// In this batch, additional suppression mismatches were traced to short-form
/// directives (`# rubocop:disable MethodLength`) that RuboCop resolves to this
/// cop. nitrocop now resolves short names in `parse::directives` to align with
/// RuboCop's directive handling.
///
/// Additional FP root cause: methods whose body is only a heredoc expression.
/// In Parser AST, bare heredoc bodies are `str`/`dstr` nodes whose source range
/// is only the opener line (`<<~SQL`), so RuboCop counts them as one line.
/// nitrocop previously counted full heredoc content lines for this shape.
pub struct MethodLength;

/// Parsed config values for MethodLength.
struct MethodLengthConfig {
    max: usize,
    count_comments: bool,
    count_as_one: Option<Vec<String>>,
    allowed_methods: Option<Vec<String>>,
    allowed_patterns: Option<Vec<String>>,
}

impl MethodLengthConfig {
    fn from_cop_config(config: &CopConfig) -> Self {
        Self {
            max: config.get_usize("Max", 10),
            count_comments: config.get_bool("CountComments", false),
            count_as_one: config.get_string_array("CountAsOne"),
            allowed_methods: config.get_string_array("AllowedMethods"),
            allowed_patterns: config.get_string_array("AllowedPatterns"),
        }
    }

    /// Check if a method name is allowed by AllowedMethods or AllowedPatterns.
    fn is_allowed(&self, method_name: &str) -> bool {
        if let Some(allowed) = &self.allowed_methods {
            if allowed.iter().any(|m| m == method_name) {
                return true;
            }
        }
        if let Some(patterns) = &self.allowed_patterns {
            for pat in patterns {
                if let Ok(re) = regex::Regex::new(pat) {
                    if re.is_match(method_name) {
                        return true;
                    }
                }
            }
        }
        false
    }
}

impl Cop for MethodLength {
    fn name(&self) -> &'static str {
        "Metrics/MethodLength"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[DEF_NODE, CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let cfg = MethodLengthConfig::from_cop_config(config);

        if let Some(def_node) = node.as_def_node() {
            self.check_def(source, def_node, &cfg, diagnostics);
        } else if let Some(call_node) = node.as_call_node() {
            self.check_define_method(source, call_node, &cfg, diagnostics);
        }
    }
}

impl MethodLength {
    fn check_def(
        &self,
        source: &SourceFile,
        def_node: ruby_prism::DefNode<'_>,
        cfg: &MethodLengthConfig,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let method_name_str = std::str::from_utf8(def_node.name().as_slice()).unwrap_or("");
        if cfg.is_allowed(method_name_str) {
            return;
        }

        let start_offset = def_node.def_keyword_loc().start_offset();
        let count = if let Some(end_loc) = def_node.end_keyword_loc() {
            let end_offset = end_loc.start_offset();
            count_method_lines(source, start_offset, end_offset, cfg, def_node.body())
        } else {
            // Endless methods (`def foo = ...`) have no `end` keyword.
            // RuboCop measures body.source lines for these definitions.
            match def_node.body() {
                Some(body) => count_endless_method_lines(source, &body, cfg),
                None => 0,
            }
        };

        if count > cfg.max {
            let (line, column) = source.offset_to_line_col(start_offset);
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!("Method has too many lines. [{count}/{}]", cfg.max),
            ));
        }
    }

    fn check_define_method(
        &self,
        source: &SourceFile,
        call_node: ruby_prism::CallNode<'_>,
        cfg: &MethodLengthConfig,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Handle define_method calls with or without receiver.
        if call_node.name().as_slice() != b"define_method" {
            return;
        }

        // Must have a block
        let block = match call_node.block() {
            Some(b) => match b.as_block_node() {
                Some(bn) => bn,
                None => return,
            },
            None => return,
        };

        // Extract method name from first argument for AllowedMethods/AllowedPatterns
        let method_name = extract_define_method_name(&call_node);
        if let Some(name) = &method_name {
            if cfg.is_allowed(name) {
                return;
            }
        }

        let start_offset = call_node.location().start_offset();
        let end_offset = block.closing_loc().start_offset();

        let count = count_method_lines(source, start_offset, end_offset, cfg, block.body());

        if count > cfg.max {
            let (line, column) = source.offset_to_line_col(start_offset);
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!("Method has too many lines. [{count}/{}]", cfg.max),
            ));
        }
    }
}

/// Count body lines for a method (def or define_method block), folding heredocs
/// and CountAsOne constructs.
///
/// RuboCop counts lines from `node.body.source` which starts at the first body
/// statement, AFTER any parameter list. We replicate this by using the body
/// node's start offset instead of the def keyword offset when a body exists.
fn count_method_lines(
    source: &SourceFile,
    start_offset: usize,
    end_offset: usize,
    cfg: &MethodLengthConfig,
    body: Option<ruby_prism::Node<'_>>,
) -> usize {
    let body = match body {
        Some(b) => b,
        // Empty method body = 0 lines, matching RuboCop's `return 0 unless body`
        None => return 0,
    };

    // Parser/RuboCop behavior: when a method body is a single heredoc
    // expression, code length is based on the heredoc opener node source,
    // so it counts as one line.
    if is_single_heredoc_expression(source, &body) {
        return 1;
    }

    // RuboCop uses `body.source.lines` which starts at the first statement.
    // count_body_lines_ex counts from start_line+1 to end_line-1, so we need
    // start_line = body_first_line - 1. We achieve this by using the offset of
    // the line just before the body's first line.
    //
    // For methods with rescue/ensure, Prism wraps the body in a BeginNode whose
    // location() starts at the def keyword (not the first statement). We must
    // dig into the BeginNode's children to find the actual first body line.
    let first_content_offset = if let Some(begin) = body.as_begin_node() {
        begin
            .statements()
            .and_then(|s| s.body().iter().next())
            .map(|n| n.location().start_offset())
            .or_else(|| begin.rescue_clause().map(|r| r.location().start_offset()))
            .or_else(|| begin.ensure_clause().map(|e| e.location().start_offset()))
            .unwrap_or(body.location().start_offset())
    } else {
        body.location().start_offset()
    };
    let (body_start_line, _) = source.offset_to_line_col(first_content_offset);
    let effective_start_offset = if body_start_line > 1 {
        // Use offset of the line before the body's first line
        source
            .line_col_to_offset(body_start_line - 1, 0)
            .unwrap_or(start_offset)
    } else {
        start_offset
    };

    // Collect foldable ranges from CountAsOne config. Heredocs are only
    // folded when "heredoc" is explicitly in CountAsOne (default: []).
    // For non-bare-heredoc bodies, RuboCop includes heredoc content lines via
    // source_from_node_with_heredoc. We replicate that here and only fold when
    // CountAsOne says to.
    let mut all_foldable: Vec<(usize, usize)> = Vec::new();
    if let Some(cao) = &cfg.count_as_one {
        if !cao.is_empty() {
            all_foldable.extend(collect_foldable_ranges(source, &body, cao));
            // collect_foldable_ranges can't fold heredocs correctly in Prism
            // (InterpolatedStringNode.location() only covers the opening).
            // Use collect_heredoc_ranges which uses closing_loc().
            if cao.iter().any(|s| s == "heredoc") {
                all_foldable.extend(collect_heredoc_ranges(source, &body));
            }
        }
    }
    all_foldable.sort();
    all_foldable.dedup();

    count_body_lines_ex(
        source,
        effective_start_offset,
        end_offset,
        cfg.count_comments,
        &all_foldable,
    )
}

fn count_endless_method_lines(
    source: &SourceFile,
    body: &ruby_prism::Node<'_>,
    cfg: &MethodLengthConfig,
) -> usize {
    if is_single_heredoc_expression(source, body) {
        return 1;
    }

    let mut all_foldable: Vec<(usize, usize)> = Vec::new();
    if let Some(cao) = &cfg.count_as_one {
        if !cao.is_empty() {
            all_foldable.extend(collect_foldable_ranges(source, body, cao));
            if cao.iter().any(|s| s == "heredoc") {
                all_foldable.extend(collect_heredoc_ranges(source, body));
            }
        }
    }
    all_foldable.sort();
    all_foldable.dedup();

    count_node_lines(source, body, cfg.count_comments, &all_foldable)
}

fn count_node_lines(
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
    count_comments: bool,
    foldable_ranges: &[(usize, usize)],
) -> usize {
    let loc = node.location();
    let (start_line, _) = source.offset_to_line_col(loc.start_offset());
    let end_off = loc.end_offset().saturating_sub(1).max(loc.start_offset());
    let (end_line, _) = source.offset_to_line_col(end_off);

    let mut folded_lines = std::collections::HashSet::new();
    for &(fold_start, fold_end) in foldable_ranges {
        for line in (fold_start + 1)..=fold_end {
            folded_lines.insert(line);
        }
    }

    let lines: Vec<&[u8]> = source.lines().collect();
    let mut count = 0;
    for line_num in start_line..=end_line {
        if line_num == 0 || line_num > lines.len() {
            continue;
        }
        if folded_lines.contains(&line_num) {
            continue;
        }

        let trimmed = trim_line(lines[line_num - 1]);
        if trimmed.is_empty() {
            continue;
        }
        if !count_comments && trimmed.starts_with(b"#") {
            continue;
        }
        count += 1;
    }

    count
}

fn trim_line(line: &[u8]) -> &[u8] {
    let start = line
        .iter()
        .position(|&c| c != b' ' && c != b'\t' && c != b'\r');
    match start {
        Some(s) => {
            let end = line
                .iter()
                .rposition(|&c| c != b' ' && c != b'\t' && c != b'\r')
                .unwrap_or(s);
            &line[s..=end]
        }
        None => &[],
    }
}

/// Extract the method name from a `define_method` call's first argument.
/// Handles symbol literals (:name), string literals ("name"), and returns
/// None for dynamic/interpolated names.
fn extract_define_method_name(call: &ruby_prism::CallNode<'_>) -> Option<String> {
    let args = call.arguments()?;
    let first = args.arguments().iter().next()?;

    if let Some(sym) = first.as_symbol_node() {
        return Some(String::from_utf8_lossy(sym.unescaped()).into_owned());
    }
    if let Some(s) = first.as_string_node() {
        return Some(String::from_utf8_lossy(s.unescaped()).into_owned());
    }
    None
}

fn is_single_heredoc_expression(source: &SourceFile, body: &ruby_prism::Node<'_>) -> bool {
    if is_heredoc_node(source, body) {
        return true;
    }

    if let Some(stmts) = body.as_statements_node() {
        let mut iter = stmts.body().iter();
        if let Some(first) = iter.next() {
            return iter.next().is_none() && is_heredoc_node(source, &first);
        }
    }

    false
}

fn is_heredoc_node(source: &SourceFile, node: &ruby_prism::Node<'_>) -> bool {
    if let Some(s) = node.as_string_node() {
        return s
            .opening_loc()
            .map(|o| source.as_bytes()[o.start_offset()..o.end_offset()].starts_with(b"<<"))
            .unwrap_or(false);
    }

    if let Some(s) = node.as_interpolated_string_node() {
        let is_heredoc = s
            .opening_loc()
            .map(|o| source.as_bytes()[o.start_offset()..o.end_offset()].starts_with(b"<<"))
            .unwrap_or(false);
        if !is_heredoc {
            return false;
        }

        // Keep the one-line special-case only for plain heredocs. Interpolated
        // heredocs (`#{...}` parts present) are counted by RuboCop as multiline.
        return !s
            .parts()
            .iter()
            .any(|part| part.as_embedded_statements_node().is_some());
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MethodLength, "cops/metrics/method_length");

    #[test]
    fn config_custom_max() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(5.into()))]),
            ..CopConfig::default()
        };
        // 6 body lines exceeds Max:5
        let source = b"def foo\n  a\n  b\n  c\n  d\n  e\n  f\nend\n";
        let diags = run_cop_full_with_config(&MethodLength, source, config);
        assert!(!diags.is_empty(), "Should fire with Max:5 on 6-line method");
        assert!(diags[0].message.contains("[6/5]"));
    }

    #[test]
    fn config_count_as_one_array() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        // With CountAsOne: ["array"], a multiline array counts as 1 line
        // Use Max:4 so it passes with folding but would fail without
        let config2 = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(4.into())),
                (
                    "CountAsOne".into(),
                    serde_yml::Value::Sequence(vec![serde_yml::Value::String("array".into())]),
                ),
            ]),
            ..CopConfig::default()
        };
        // Body: a, b, c, arr = [\n1,\n2,\n3\n] = 3 + 4 = 7 lines without folding, 3 + 1 = 4 with folding
        let source =
            b"def foo\n  a = 1\n  b = 2\n  c = 3\n  arr = [\n    1,\n    2,\n    3\n  ]\nend\n";
        let diags = run_cop_full_with_config(&MethodLength, source, config2);
        assert!(
            diags.is_empty(),
            "Should not fire when array is folded to 1 line (4/4)"
        );

        // Without CountAsOne, Max:4 should fire (7 lines > 4)
        let config3 = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(4.into()))]),
            ..CopConfig::default()
        };
        let diags2 = run_cop_full_with_config(&MethodLength, source, config3);
        assert!(
            !diags2.is_empty(),
            "Should fire without CountAsOne (7 lines > 4)"
        );
    }

    #[test]
    fn config_count_comments_true() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(3.into())),
                ("CountComments".into(), serde_yml::Value::Bool(true)),
            ]),
            ..CopConfig::default()
        };
        // RuboCop counts comments within the body (between statements), not before
        // the first statement. 4 body lines (a, comment, comment, b) exceeds Max:3.
        let source = b"def foo\n  a\n  # comment1\n  # comment2\n  b\nend\n";
        let diags = run_cop_full_with_config(&MethodLength, source, config);
        assert!(!diags.is_empty(), "Should fire with CountComments:true");
        assert!(diags[0].message.contains("[4/3]"));
    }

    #[test]
    fn define_method_offense() {
        use crate::testutil::run_cop_full;
        let source = b"define_method(:long_method) do\n  a = 1\n  b = 2\n  c = 3\n  d = 4\n  e = 5\n  f = 6\n  g = 7\n  h = 8\n  i = 9\n  j = 10\n  k = 11\nend\n";
        let diags = run_cop_full(&MethodLength, source);
        assert!(
            !diags.is_empty(),
            "Should fire on define_method with 11 lines"
        );
        assert!(diags[0].message.contains("[11/10]"));
    }

    #[test]
    fn define_method_no_offense() {
        use crate::testutil::run_cop_full;
        let source = b"define_method(:short) do\n  a = 1\n  b = 2\nend\n";
        let diags = run_cop_full(&MethodLength, source);
        assert!(diags.is_empty(), "Should not fire on short define_method");
    }

    #[test]
    fn allowed_methods_define_method() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(5.into())),
                (
                    "AllowedMethods".into(),
                    serde_yml::Value::Sequence(vec![serde_yml::Value::String("foo".into())]),
                ),
            ]),
            ..CopConfig::default()
        };
        let source =
            b"define_method(:foo) do\n  a = 1\n  a = 2\n  a = 3\n  a = 4\n  a = 5\n  a = 6\nend\n";
        let diags = run_cop_full_with_config(&MethodLength, source, config);
        assert!(
            diags.is_empty(),
            "Should skip define_method(:foo) when foo is allowed"
        );
    }

    #[test]
    fn multiline_params_not_counted() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(3.into()))]),
            ..CopConfig::default()
        };
        // Method with multiline params: body has 3 lines (a, b, c), params should NOT
        // be counted. RuboCop counts only body.source lines.
        let source = b"def initialize(\n  param1: nil,\n  param2: nil,\n  param3: nil\n)\n  a = 1\n  b = 2\n  c = 3\nend\n";
        let diags = run_cop_full_with_config(&MethodLength, source, config.clone());
        assert!(
            diags.is_empty(),
            "Should not fire: 3 body lines <= Max:3 (params not counted)"
        );

        // Same method but with 4 body lines should fire
        let source2 = b"def initialize(\n  param1: nil,\n  param2: nil,\n  param3: nil\n)\n  a = 1\n  b = 2\n  c = 3\n  d = 4\nend\n";
        let diags2 = run_cop_full_with_config(&MethodLength, source2, config);
        assert!(!diags2.is_empty(), "Should fire: 4 body lines > Max:3");
        assert!(diags2[0].message.contains("[4/3]"));
    }

    #[test]
    fn empty_method_no_count() {
        use crate::testutil::run_cop_full;
        // Empty method should have 0 lines
        let source = b"def foo\nend\n";
        let diags = run_cop_full(&MethodLength, source);
        assert!(diags.is_empty(), "Empty method should not fire");
    }

    #[test]
    fn multiline_params_borderline() {
        use crate::testutil::run_cop_full;
        // 10 param lines + 10 body lines. With old code this would be [20/10].
        // With fix, only body lines counted: [10/10] = no offense.
        let source = b"def initialize(\n\
            param1: nil,\n\
            param2: nil,\n\
            param3: nil,\n\
            param4: nil,\n\
            param5: nil,\n\
            param6: nil,\n\
            param7: nil,\n\
            param8: nil,\n\
            param9: nil,\n\
            param10: nil\n\
          )\n\
            a = 1\n\
            b = 2\n\
            c = 3\n\
            d = 4\n\
            e = 5\n\
            f = 6\n\
            g = 7\n\
            h = 8\n\
            i = 9\n\
            j = 10\n\
          end\n";
        let diags = run_cop_full(&MethodLength, source);
        assert!(
            diags.is_empty(),
            "10 body lines with multiline params should not fire (params not counted)"
        );
    }

    #[test]
    fn allowed_patterns_regex() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(5.into())),
                (
                    "AllowedPatterns".into(),
                    serde_yml::Value::Sequence(vec![serde_yml::Value::String("_name".into())]),
                ),
            ]),
            ..CopConfig::default()
        };
        // user_name matches /_name/ regex
        let source = b"def user_name\n  a = 1\n  a = 2\n  a = 3\n  a = 4\n  a = 5\n  a = 6\nend\n";
        let diags = run_cop_full_with_config(&MethodLength, source, config.clone());
        assert!(
            diags.is_empty(),
            "Should skip user_name matching /_name/ pattern"
        );

        // firstname does NOT match /_name/ regex (no underscore before name)
        let source2 = b"def firstname\n  a = 1\n  a = 2\n  a = 3\n  a = 4\n  a = 5\n  a = 6\nend\n";
        let diags2 = run_cop_full_with_config(&MethodLength, source2, config);
        assert!(
            !diags2.is_empty(),
            "Should fire on firstname which doesn't match /_name/ pattern"
        );
    }

    #[test]
    fn method_with_ensure_exact_boundary() {
        use crate::testutil::run_cop_full;
        // From corpus FP: method with ensure, exactly 10 body lines
        // Body: old_values={}, each do, send, send, end, yield, ensure, each do, send, end = 10
        let source = b"def swap(klass, new_values)\n  old_values = {}\n  new_values.each do |key, value|\n    old_values[key] = klass.public_send key\n    klass.public_send key\n  end\n  yield\nensure\n  old_values.each do |key, value|\n    klass.public_send key\n  end\nend\n";
        let diags = run_cop_full(&MethodLength, source);
        for d in &diags {
            eprintln!("  DIAG: {} at line {}", d.message, d.location.line);
        }
        assert!(
            diags.is_empty(),
            "10 body lines with ensure should NOT fire (Max:10)"
        );
    }

    #[test]
    fn method_with_ensure_inside_class() {
        use crate::testutil::run_cop_full;
        // When a method with ensure is inside a class, the BeginNode's location
        // starts at the def keyword, not the first statement. This caused an
        // off-by-one: body_start_line == def_line, effective_start becomes the
        // line BEFORE def, making us count the def line as a body line.
        let source = b"class Foo\n  def with_adapter_method_tracking(method_name, tracker)\n    original = MultiJson.method(method_name)\n    silence_warnings do\n      MultiJson.define_singleton_method(method_name) do\n        tracker.call\n        original.call\n      end\n    end\n    yield\n  ensure\n    silence_warnings { MultiJson.define_singleton_method(method_name, original) }\n  end\nend\n";
        // Body: lines 3-12 = original, silence do, define do, call, call, end, end, yield, ensure, silence = 10 lines
        let diags = run_cop_full(&MethodLength, source);
        for d in &diags {
            eprintln!("  DIAG: {} at line {}", d.message, d.location.line);
        }
        assert!(
            diags.is_empty(),
            "10 body lines with ensure inside class should NOT fire (Max:10)"
        );
    }
}
