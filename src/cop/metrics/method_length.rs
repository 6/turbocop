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
///
/// Follow-up fixes in this batch:
/// - Handle endless methods (`def foo = ...`) instead of skipping all defs
///   without `end`.
/// - Count `define_method` blocks even when the send has a receiver
///   (`klass.define_method`, `mod.define_method`), matching RuboCop's
///   `on_block` + `node.method?(:define_method)` behavior.
///
/// Local corpus rerun comparison against unchanged baseline binary:
/// only 5 repos changed, all in the expected FN direction (+6 total offenses):
/// `ruby__typeprof` (+2), `refinery__refinerycms` (+1), `natalie-lang__natalie` (+1),
/// `opal__opal` (+1), `theforeman__foreman` (+1).
///
/// Known remaining FN examples from corpus oracle: `chef` (powershell wrapper)
/// and `jruby` (`test_lje_structure`).
///
/// ## Corpus investigation (2026-03-07)
///
/// FP=112 across 31 repos. Root cause: when a method body contains heredocs,
/// RuboCop uses `source_from_node_with_heredoc(body)` which computes the line
/// range as `body.first_line..max(descendant.last_line)`. Since `each_descendant`
/// yields only children/grandchildren (not the body node itself), wrapper closing
/// keywords like block `end`s are excluded from the max. In contrast, nitrocop
/// used the method's `end` keyword line as the range boundary, which included
/// inner block `end` keywords.
///
/// Fix: when the body has heredoc descendants, compute `effective_end_offset`
/// from `max_descendant_end_line` (max of inner statements' end lines and
/// heredoc closing locations) rather than the method's `end` keyword line.
/// The function `inner_content_end_line` recursively digs into CallNode blocks
/// and StatementsNode wrappers to find the innermost content line, matching
/// RuboCop's descendant-based line range.
///
/// ## Corpus investigation (2026-03-08)
///
/// FP=38, FN=151. Root cause: `inner_content_end_line` was recursively digging
/// into ALL nested block bodies (including deeply nested ones), excluding their
/// `end` keywords. But in Parser AST, `body.each_descendant` only excludes the
/// root body node itself — nested block nodes ARE descendants whose `last_line`
/// includes their `end` keywords.
///
/// Key Parser/Prism structural mismatch:
/// - Parser single-statement: body = statement (block/send). `each_descendant`
///   yields statement's children, excluding the statement itself.
/// - Parser multi-statement: body = (begin stmts). `each_descendant` yields
///   all children, including block nodes with their `end`.
/// - Prism always wraps in StatementsNode, even for single statements.
///
/// Fix: `inner_content_end_line` now only unwraps one level of StatementsNode.
/// For single-child bodies, `descendants_max_end_line` visits the child's
/// immediate children (for CallNode with block: block body children; for
/// CallNode without block: args end line). For multi-child bodies, uses
/// `end_line_of` for each child. This includes nested block `end` keywords
/// while excluding the outermost body, matching `each_descendant` semantics.
///
/// ## Corpus investigation (2026-03-09)
///
/// Re-ran the cop under the repository's Ruby 3.4 toolchain:
/// `mise exec ruby@3.4 -- python3 scripts/check-cop.py Metrics/MethodLength
/// --verbose --rerun`.
///
/// Result:
/// - Expected: 107,344
/// - Actual:   113,392
/// - Excess:   0 over CI baseline after file-drop adjustment
/// - Missing:  0
///
/// No code change was taken in this run. The artifact-reported FP/FN counts
/// are dominated by jruby file-drop noise; with the correct rerun environment
/// the cop has no remaining missing offenses and no excess regression.
///
/// ## Corpus investigation (2026-03-10)
///
/// FP=28, FN=2. Root causes of FPs:
///
/// 1. **Heredoc-in-structured-node off-by-one (18 FPs):** When a method body
///    contains heredocs, nitrocop switches to `source_from_node_with_heredoc`
///    semantics, computing effective_end_offset from `max_descendant_end_line`.
///    For single-child bodies like `if`/`case`/`while` nodes,
///    `descendants_max_end_line` fell back to `end_line_of(node)` which
///    included the node's own `end` keyword. Parser's `each_descendant`
///    excludes the root node, so its `end` keyword is not counted.
///    Fix: use a Prism visitor to walk all descendants, skipping the root
///    node, matching `each_descendant` semantics.
///    Similarly, for BeginNode with ensure/rescue, the clause's
///    `location().end_offset()` included closing keywords. Fix: use the
///    clause's inner statements' end lines instead.
///
/// 2. **Directive handling (10 FPs):** thredded (6) and coreinfrastructure (4)
///    have `# rubocop:disable Metrics/MethodLength` directives that suppress
///    offenses in RuboCop. These are directive resolution issues, not cop logic.
///
/// The 2 FNs are in chef and jruby (known file-drop noise).
///
/// ## Corpus investigation (2026-03-10, second pass)
///
/// FP=13, FN=2. 6 FPs are config resolution issues (rails_admin Max:29,
/// super_diff Max:114). 7 FPs are heredoc off-by-one where nitrocop counts
/// [11/10] but RuboCop counts 10. All 7 are methods with `if/else` containing
/// heredocs.
///
/// Root cause: Prism's `ElseNode` location extends to the parent's closing
/// `end` keyword. In `MaxEndLineVisitor`, visiting the `ElseNode` descendant
/// inflated `max_line` by one. Parser AST has no `ElseNode` wrapper — the
/// else branch is just a child node whose `last_line` is the last statement,
/// not the `end` keyword. Fix: skip `ElseNode` in the visitor (let its children
/// be visited instead).
///
/// ## Corpus investigation (2026-03-10, third pass)
///
/// Corpus oracle reported FP=1, FN=2.
///
/// FP=1: rubyonjets/jets `git_dirty_message` [11/10]. Method body is
/// `if/elsif` containing heredocs. Prism's elsif IfNode location extends
/// to the parent's `end` keyword (same structural issue as ElseNode).
/// Fix: skip elsif IfNode in MaxEndLineVisitor by checking if
/// `if_keyword_loc` starts with `elsif`.
///
/// FN=2: chef (single-heredoc body) and jruby (=begin/=end block comment
/// file). Both are file-drop noise — nitrocop cannot process these files
/// due to missing project structure (no Gemfile/lockfile). Not cop logic bugs.
///
/// ## Extended corpus investigation (2026-03-23)
///
/// Extended corpus (5592 repos) reported FP=76, FN=2. Standard corpus is 0/0.
///
/// FP=76 root cause: cross-cutting file-level issue. 54/76 FP from Tubalr (32)
/// and stackneveroverflow (22) with vendored gems that RuboCop cannot parse.
/// Remaining 22 FP from 12 repos — likely config resolution differences (custom
/// Max values, AllowedMethods) or vendor path exclusion edge cases.
///
/// FN=2 from brixen/poetics (bin/poetics:21) — extensionless file that nitrocop
/// does not discover. This is a file discovery issue (shebang detection), not a
/// cop logic bug. RuboCop discovers and lints these files.
///
/// ## Corpus verification (2026-03-25)
///
/// verify_cop_locations.py: FP 0 fixed / 16 remain, FN 100 fixed / 0 remain.
/// All FN verified fixed. Remaining FP=16: noosfero (4, vendored plugins),
/// ruby/tk (3, config), auth0 (2, config), dradis (1, vendored plugin),
/// ConfigLMM (1), brandur (1), engineyard (1), gisiahq (1), samvera (1),
/// siberas (1) — all config resolution or vendored file issues.
/// No cop-level fix needed.
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

    // When the body contains heredocs, RuboCop switches from `body.source.lines`
    // to `source_from_node_with_heredoc(body)`, which computes lines from
    // body.first_line to the max descendant last_line. This excludes wrapper
    // closing keywords (block `end`s) that are part of the body node but not
    // individual descendants. We replicate this by adjusting end_offset.
    let effective_end_offset = if body_has_heredoc(source, &body) {
        let max_line = max_descendant_end_line(source, &body);
        if max_line > 0 {
            // Use the start of the line AFTER max_line as end_offset so
            // count_body_lines_impl's exclusive range includes max_line.
            source
                .line_col_to_offset(max_line + 1, 0)
                .unwrap_or(end_offset)
        } else {
            end_offset
        }
    } else {
        end_offset
    };

    count_body_lines_ex(
        source,
        effective_start_offset,
        effective_end_offset,
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

/// Check if a body node contains any heredoc descendants.
pub(crate) fn body_has_heredoc(source: &SourceFile, body: &ruby_prism::Node<'_>) -> bool {
    use ruby_prism::Visit;

    struct HeredocDetector<'a> {
        source: &'a SourceFile,
        found: bool,
    }

    impl<'pr> Visit<'pr> for HeredocDetector<'_> {
        fn visit_string_node(&mut self, node: &ruby_prism::StringNode<'pr>) {
            if !self.found {
                if let Some(o) = node.opening_loc() {
                    let bytes = &self.source.as_bytes()[o.start_offset()..o.end_offset()];
                    if bytes.starts_with(b"<<") {
                        self.found = true;
                    }
                }
            }
        }

        fn visit_interpolated_string_node(
            &mut self,
            node: &ruby_prism::InterpolatedStringNode<'pr>,
        ) {
            if !self.found {
                if let Some(o) = node.opening_loc() {
                    let bytes = &self.source.as_bytes()[o.start_offset()..o.end_offset()];
                    if bytes.starts_with(b"<<") {
                        self.found = true;
                        return;
                    }
                }
                ruby_prism::visit_interpolated_string_node(self, node);
            }
        }
    }

    let mut detector = HeredocDetector {
        source,
        found: false,
    };
    detector.visit(body);
    detector.found
}

/// Compute the max end line (1-indexed) among descendants of a body node,
/// considering heredoc closing locations. This replicates RuboCop's
/// `source_from_node_with_heredoc` behavior where the effective end
/// is the max descendant last_line (not the container node's last_line).
pub(crate) fn max_descendant_end_line(source: &SourceFile, body: &ruby_prism::Node<'_>) -> usize {
    let heredoc_ranges = collect_heredoc_ranges(source, body);
    let max_heredoc_line = heredoc_ranges
        .iter()
        .map(|&(_, end)| end)
        .max()
        .unwrap_or(0);
    let last_stmt_line = inner_content_end_line(source, body);
    last_stmt_line.max(max_heredoc_line)
}

/// Get the max end line among body's descendants, matching RuboCop's
/// `body.each_descendant` behavior for `source_from_node_with_heredoc`.
///
/// In Parser AST, `extract_body` returns the method's body node.
/// Single statement: body = the statement itself (block, send, etc.).
/// Multiple statements: body = (begin stmt1 stmt2 ...).
/// `body.each_descendant` yields all descendants but NOT body itself.
///
/// In Prism, body is always a StatementsNode (or BeginNode for rescue/ensure).
/// We unwrap the body's StatementsNode to find the equivalent Parser body,
/// then collect end_line_of for all its descendants.
///
/// This function does NOT recurse into nested blocks. It unwraps the body
/// exactly once (matching Parser's body extraction) and then uses end_line_of
/// for all children found. This ensures nested block `end` keywords are
/// included (as they would be in Parser's `each_descendant`).
fn inner_content_end_line(source: &SourceFile, body: &ruby_prism::Node<'_>) -> usize {
    let end_line_of = |node: &ruby_prism::Node<'_>| -> usize {
        let off = node
            .location()
            .end_offset()
            .saturating_sub(1)
            .max(node.location().start_offset());
        source.offset_to_line_col(off).0
    };

    // Unwrap Prism's StatementsNode/BeginNode to find the equivalent Parser body.
    // Then collect children of that body using end_line_of.
    if let Some(stmts) = body.as_statements_node() {
        let children: Vec<_> = stmts.body().iter().collect();
        if children.len() == 1 {
            // Single child: Parser would have body = this child.
            // each_descendant yields children of THIS node, not the node itself.
            return descendants_max_end_line(source, &children[0]);
        }
        // Multiple children: Parser would have body = (begin children...).
        // each_descendant yields all children (they ARE descendants of begin).
        children.iter().map(&end_line_of).max().unwrap_or(0)
    } else if let Some(begin) = body.as_begin_node() {
        let mut max = 0usize;
        // For ensure/rescue clauses, use the statements' end lines rather than
        // the clause's own location (which may include `end` keywords).
        // This matches Parser's each_descendant behavior where the clause
        // node's own closing keyword is excluded.
        if let Some(ensure_clause) = begin.ensure_clause() {
            if let Some(stmts) = ensure_clause.statements() {
                for child in stmts.body().iter() {
                    max = max.max(end_line_of(&child));
                }
            }
        }
        if let Some(rescue_clause) = begin.rescue_clause() {
            // Include rescue clause body statements
            if let Some(stmts) = rescue_clause.statements() {
                for child in stmts.body().iter() {
                    max = max.max(end_line_of(&child));
                }
            }
            // Follow rescue chain (else_clause, subsequent_clause)
            if let Some(else_clause) = rescue_clause.subsequent() {
                let off = else_clause.location().end_offset().saturating_sub(1);
                max = max.max(source.offset_to_line_col(off).0);
            }
        }
        if let Some(stmts) = begin.statements() {
            let children: Vec<_> = stmts.body().iter().collect();
            if children.len() == 1 {
                max = max.max(descendants_max_end_line(source, &children[0]));
            } else {
                for child in &children {
                    max = max.max(end_line_of(child));
                }
            }
        }
        max
    } else {
        // Body is a single expression — get max of its descendants
        descendants_max_end_line(source, body)
    }
}

/// Get the max end line among the descendants of a node, matching
/// Parser's `node.each_descendant` behavior where the node itself is
/// excluded. For a CallNode with block, the block corresponds to Parser's
/// body, so we visit the block's body children (not the block itself).
/// For other node types (IfNode, CaseNode, etc.), we use a visitor to
/// walk all descendants, skipping the root node and any direct block
/// children to exclude their closing `end` keywords.
fn descendants_max_end_line(source: &SourceFile, node: &ruby_prism::Node<'_>) -> usize {
    let end_line_of = |n: &ruby_prism::Node<'_>| -> usize {
        let off = n
            .location()
            .end_offset()
            .saturating_sub(1)
            .max(n.location().start_offset());
        source.offset_to_line_col(off).0
    };

    if let Some(call) = node.as_call_node() {
        if let Some(block_wrapper) = call.block() {
            if let Some(block) = block_wrapper.as_block_node() {
                // In Parser, for a single-statement body that is a block call,
                // body = block_node. `body.each_descendant` yields the block's
                // children: send, args, and body. The block's `end` keyword is
                // excluded (it's on the body node itself).
                //
                // In Prism, the block body is a StatementsNode. Its children
                // are the block's inner statements. We use end_line_of for each
                // child — nested blocks inside WILL include their `end` keywords
                // (they are descendants, not the root body).
                let mut max = 0usize;
                // Include the send part (method name, receiver, args before block)
                // which is on the opening line
                let send_line = source.offset_to_line_col(call.location().start_offset()).0;
                max = max.max(send_line);
                if let Some(inner_body) = block.body() {
                    if let Some(stmts) = inner_body.as_statements_node() {
                        for child in stmts.body().iter() {
                            max = max.max(end_line_of(&child));
                        }
                    } else {
                        max = max.max(end_line_of(&inner_body));
                    }
                }
                return max;
            }
        }
        // CallNode without block: use args end line (not call's own `)`)
        let mut max = 0usize;
        if let Some(recv) = call.receiver() {
            max = max.max(end_line_of(&recv));
        }
        if let Some(args) = call.arguments() {
            max = max.max(end_line_of(&args.as_node()));
        }
        return max;
    }

    // For non-CallNode types (IfNode, CaseNode, WhileNode, etc.),
    // visit all descendants using a Prism visitor, skipping the root
    // node itself. This matches Parser's `each_descendant` behavior
    // where the root is excluded, so the root's closing `end` keyword
    // is not counted.
    use ruby_prism::Visit;

    struct MaxEndLineVisitor<'a> {
        source: &'a SourceFile,
        max_line: usize,
        root_start: usize,
        root_end: usize,
        skipped_root: bool,
    }

    impl<'pr> Visit<'pr> for MaxEndLineVisitor<'_> {
        fn visit_branch_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
            // Skip the root node itself — we only want its descendants
            if !self.skipped_root
                && node.location().start_offset() == self.root_start
                && node.location().end_offset() == self.root_end
            {
                self.skipped_root = true;
                return;
            }
            // ElseNode is a Prism wrapper that has no equivalent in Parser AST.
            // Its location extends to the parent's closing `end` keyword, which
            // would inflate the max end line. Skip it — its children (the actual
            // else-branch statements) will be visited and counted correctly.
            if node.as_else_node().is_some() {
                return;
            }
            // elsif clauses are IfNode with `if_keyword_loc` = `elsif`.
            // In Prism, their location extends to the parent's `end` keyword.
            // In Parser, elsif's source range ends at its last body statement,
            // not at `end`. Skip tracking to avoid inflating the count.
            if let Some(if_node) = node.as_if_node() {
                if let Some(kw) = if_node.if_keyword_loc() {
                    let bytes = &self.source.as_bytes()[kw.start_offset()..kw.end_offset()];
                    if bytes.starts_with(b"elsif") {
                        return;
                    }
                }
            }
            let off = node
                .location()
                .end_offset()
                .saturating_sub(1)
                .max(node.location().start_offset());
            let line = self.source.offset_to_line_col(off).0;
            self.max_line = self.max_line.max(line);
        }

        fn visit_leaf_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
            let off = node
                .location()
                .end_offset()
                .saturating_sub(1)
                .max(node.location().start_offset());
            let line = self.source.offset_to_line_col(off).0;
            self.max_line = self.max_line.max(line);
        }
    }

    let mut visitor = MaxEndLineVisitor {
        source,
        max_line: 0,
        root_start: node.location().start_offset(),
        root_end: node.location().end_offset(),
        skipped_root: false,
    };
    visitor.visit(node);
    visitor.max_line
}

fn is_single_heredoc_expression(source: &SourceFile, body: &ruby_prism::Node<'_>) -> bool {
    // Find the single heredoc node in the body (either directly or unwrapped
    // from a StatementsNode).
    let node = if is_heredoc_node(source, body) {
        body
    } else if let Some(stmts) = body.as_statements_node() {
        let mut iter = stmts.body().iter();
        match (iter.next(), iter.next()) {
            (Some(first), None) if is_heredoc_node(source, &first) => {
                // A bare heredoc body counts as 1 line in RuboCop — unless the
                // heredoc contains nested heredocs in its interpolation blocks.
                // In that case, RuboCop's `body_has_heredoc?` finds the nested
                // heredoc descendants and switches to
                // `source_from_node_with_heredoc` which counts actual lines.
                return !has_nested_heredoc_in_node(source, &first);
            }
            _ => return false,
        }
    } else {
        return false;
    };

    !has_nested_heredoc_in_node(source, node)
}

/// Check if an interpolated heredoc node contains nested heredoc nodes
/// in its interpolation parts.
fn has_nested_heredoc_in_node(source: &SourceFile, node: &ruby_prism::Node<'_>) -> bool {
    use ruby_prism::Visit;

    let Some(interp) = node.as_interpolated_string_node() else {
        // Non-interpolated heredocs can't contain nested heredocs.
        return false;
    };

    struct NestedHeredocDetector<'a> {
        source: &'a SourceFile,
        found: bool,
    }

    impl<'pr> Visit<'pr> for NestedHeredocDetector<'_> {
        fn visit_string_node(&mut self, node: &ruby_prism::StringNode<'pr>) {
            if !self.found {
                if let Some(o) = node.opening_loc() {
                    let bytes = &self.source.as_bytes()[o.start_offset()..o.end_offset()];
                    if bytes.starts_with(b"<<") {
                        self.found = true;
                    }
                }
            }
        }

        fn visit_interpolated_string_node(
            &mut self,
            node: &ruby_prism::InterpolatedStringNode<'pr>,
        ) {
            if !self.found {
                if let Some(o) = node.opening_loc() {
                    let bytes = &self.source.as_bytes()[o.start_offset()..o.end_offset()];
                    if bytes.starts_with(b"<<") {
                        self.found = true;
                        return;
                    }
                }
            }
            if !self.found {
                ruby_prism::visit_interpolated_string_node(self, node);
            }
        }
    }

    let mut detector = NestedHeredocDetector {
        source,
        found: false,
    };
    // Visit only the parts (children), not the outer heredoc node itself.
    for part in interp.parts().iter() {
        detector.visit(&part);
        if detector.found {
            return true;
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
        return s
            .opening_loc()
            .map(|o| source.as_bytes()[o.start_offset()..o.end_offset()].starts_with(b"<<"))
            .unwrap_or(false);
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MethodLength, "cops/metrics/method_length");

    #[test]
    fn heredoc_in_block_no_offense() {
        use crate::testutil::run_cop_full;
        // Method with heredoc inside block: RuboCop's source_from_node_with_heredoc
        // excludes the block `end` from the line count. 10 non-blank body lines.
        let source = b"def test_method\n  in_tmpdir do\n    path = current_dir.join(\"config\")\n    path.write(<<~TEXT)\n      target :app do\n        collection_config \"test.yaml\"\n      end\n    TEXT\n    current_dir.join(\"test.yaml\").write(\"[]\")\n\n    Runner.new.load_config(path: path)\n    assert_match(/pattern/, output.string)\n  end\nend\n";
        let diags = run_cop_full(&MethodLength, source);
        assert!(
            diags.is_empty(),
            "Method with heredoc in block should not fire (10 body lines per RuboCop)"
        );
    }

    #[test]
    fn heredoc_with_nested_heredocs_counts_lines() {
        use crate::testutil::run_cop_full;
        // When a bare heredoc body contains nested heredocs in interpolation,
        // RuboCop uses source_from_node_with_heredoc and counts actual lines
        // instead of treating the whole thing as 1 line.
        let source = b"def wrapper_script\n  <<~OUTER\n    start\n#{if true\n    <<~INNER1\n      line1\n      line2\n      line3\n    INNER1\n  else\n    <<~INNER2\n      alt1\n      alt2\n      alt3\n    INNER2\n  end}\n    middle\n#{if true\n    <<~INNER3\n      more1\n      more2\n      more3\n    INNER3\n  else\n    <<~INNER4\n      other1\n      other2\n      other3\n    INNER4\n  end}\n    end_content\n  OUTER\nend\n";
        let diags = run_cop_full(&MethodLength, source);
        assert!(
            !diags.is_empty(),
            "Method with heredoc containing nested heredocs should fire (29 lines per RuboCop)"
        );
        assert!(
            diags[0].message.contains("[29/10]"),
            "Expected [29/10] but got: {}",
            diags[0].message
        );
    }

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
    fn heredoc_multi_stmt_body_should_fire() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(5.into()))]),
            ..CopConfig::default()
        };
        // Multi-statement body with heredoc: x = 1, then block call.
        // In Parser, body = begin(lvasgn, block), each_descendant yields block
        // whose last_line includes `end`. Prism must use child's full end line.
        let source = b"def test_method\n  x = 1\n  with_checker do\n    parse_ruby(<<-EOF)\nfoo\n    EOF\n    do_something\n  end\nend\n";
        // Lines: x=1(2), with_checker do(3), parse_ruby(4), foo(5), EOF(6), do_something(7), end(8)
        // = 7 body lines > Max:5
        let diags = run_cop_full_with_config(&MethodLength, source, config);
        assert!(
            !diags.is_empty(),
            "Multi-statement body with heredoc should fire (7 lines > Max:5)"
        );
    }

    #[test]
    fn heredoc_single_block_call_should_fire() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(10.into()))]),
            ..CopConfig::default()
        };
        // Single-statement body: one block call wrapping heredoc and multiple stmts.
        // Matches steep test pattern. RuboCop: body = block, each_descendant yields
        // block's inner stmts. Inner block `end` keywords ARE included as descendants.
        let source = b"def test_method\n  with_checker do |checker|\n    source = parse_ruby(<<-EOF)\nfoo\nbar\n    EOF\n    with_construction(checker, source) do |c, t|\n      c.synthesize(source.node)\n      assert_equal 2, t.errors.size\n      assert_all t.errors do |error|\n        error.is_a?(SomeError)\n      end\n    end\n  end\nend\n";
        // Body lines 2-14: with_checker(2), source=parse(3), foo(4), bar(5), EOF(6),
        // with_construction(7), synthesize(8), assert_equal(9), assert_all(10),
        // is_a(11), end(12), end(13), end(14). end_line computed = line of inner
        // with_construction end (13). Non-blank lines 2-13 = 12. > Max:10 → fires.
        let diags = run_cop_full_with_config(&MethodLength, source, config);
        assert!(
            !diags.is_empty(),
            "Single block call body with heredoc should fire (>10 lines per RuboCop)"
        );
    }

    #[test]
    fn heredoc_call_without_block_no_fp() {
        use crate::testutil::run_cop_full;
        // Single call without block that has a heredoc argument. RuboCop uses
        // source_from_node_with_heredoc, max descendant last_line excludes the
        // call's own `)`. Only args contribute. 10 body lines → no offense.
        let source = b"def test_method\n  assert_parse_only(\n    [\n      ['a', 'b', 'c'],\n      ['d', 'e', 'f'],\n      ['g', 'h', 'i']\n    ], <<EOY\nrow1\nrow2\nrow3\nEOY\n  )\nend\n";
        // Lines: assert(2), [(3), a(4), d(5), g(6), ](7), <<EOY → heredoc(8-11=EOY).
        // RuboCop max descendant = EOY on line 11. Range: 2-11 = 10 lines.
        // Non-blank: 10 → no offense at Max:10.
        let diags = run_cop_full(&MethodLength, source);
        assert!(
            diags.is_empty(),
            "Call without block + heredoc: should not fire at 10 body lines (Max:10)"
        );
    }

    #[test]
    fn method_with_begin_end_comment() {
        use crate::testutil::run_cop_full;
        // Method with =begin/=end multi-line comment block.
        // RuboCop counts =begin/=end content as body lines (not excluded by
        // CountComments: false, which only skips # comments).
        // Body has 13 code lines + 4 =begin/=end lines = 17 (above Max:10).
        let source = b"class Foo\n  def test_method\n    begin\n      break 1\n    rescue => e\n      handle(e)\n      log(e)\n      report(e)\n    end\n\n    begin\n      yield 1\n    rescue => e\n      handle(e)\n      log(e)\n    end\n\n=begin\n    This is a multi-line comment.\n    Should not count as code.\n=end\n  end\nend\n";
        let diags = run_cop_full(&MethodLength, source);
        assert!(
            !diags.is_empty(),
            "Method with =begin/=end comment should fire (17 body lines > Max:10)"
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

    #[test]
    fn if_elsif_with_heredoc_no_overcount() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        // Method with if/elsif containing heredocs. In Parser, the elsif
        // clause's source range ends at its last body statement, not at
        // the `end` keyword. In Prism, the elsif IfNode's location extends
        // to `end`. MaxEndLineVisitor must skip elsif IfNodes to avoid
        // inflating the count by 1.
        //
        // Lines:
        // 1: def git_dirty_message
        // 2:   if git_dirty?
        // 3:     <<~EOL.strip
        // 4:       Warning: Git is dirty.
        // 5:       Commit first.
        // 6:     EOL
        // 7:   elsif !git_changes_pushed?
        // 8:     <<~EOL.strip
        // 9:       Warning: Changes not pushed.
        // 10:      Push first.
        // 11:    EOL
        // 12:  end
        // 13: end
        //
        // RuboCop counts body lines 2-11 via source_from_node_with_heredoc.
        // Max descendant last_line = 11 (EOL terminator). The `end` at line 12
        // is part of the if node (excluded because it's the root body in Parser).
        // Non-blank lines 2-11 = 10 → exactly at threshold, no offense.
        let source = b"def git_dirty_message\n  if git_dirty?\n    <<~EOL.strip\n      Warning: Git is dirty.\n      Commit first.\n    EOL\n  elsif !git_changes_pushed?\n    <<~EOL.strip\n      Warning: Changes not pushed.\n      Push first.\n    EOL\n  end\nend\n";

        // With Max:10 should NOT fire (RuboCop counts exactly 10)
        let config10 = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(10.into()))]),
            ..CopConfig::default()
        };
        let diags = run_cop_full_with_config(&MethodLength, source, config10);
        assert!(
            diags.is_empty(),
            "Method with if/elsif+heredoc should count 10 body lines (elsif `end` excluded). Got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );

        // With Max:9 SHOULD fire [10/9]
        let config9 = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(9.into()))]),
            ..CopConfig::default()
        };
        let diags = run_cop_full_with_config(&MethodLength, source, config9);
        assert!(
            !diags.is_empty(),
            "Method with if/elsif+heredoc should fire at Max:9 (10 body lines)"
        );
        assert!(
            diags[0].message.contains("[10/9]"),
            "Expected [10/9] but got: {}",
            diags[0].message
        );
    }
}
