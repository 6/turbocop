use crate::cop::shared::node_type::{IF_NODE, UNLESS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use regex::Regex;
use ruby_prism::Visit;

/// Style/IfUnlessModifier: Checks for `if` and `unless` statements that would
/// fit on one line if written as modifier `if`/`unless`.
///
/// ## Investigation findings (2026-03-15)
///
/// FP root causes (301 FPs):
/// 1. **Chained calls after `end`**: `if test; something; end.inspect` — RuboCop
///    skips via `node.chained?`. nitrocop was missing this check entirely. Fix:
///    detect non-whitespace after `end` keyword on the same line.
/// 2. **Comment on `end` line**: `end # comment` — RuboCop checks
///    `line_with_comment?(node.loc.last_line)`. nitrocop checked comments between
///    body and end but not on the end line itself. Fix: check end line for comments.
/// 3. **Named regexp captures**: `/(?<name>\d)/ =~ str` — RuboCop's
///    `named_capture_in_condition?` checks `match_with_lvasgn_type?`. Fix: detect
///    `MatchWriteNode` in condition (Prism equivalent).
/// 4. **Endless method def in body**: `def method_name = body` — RuboCop's
///    `endless_method?` skips these to avoid `Style/AmbiguousEndlessMethodDefinition`.
///    Fix: check if body is a DefNode with `equal_loc()`.
/// 5. **Pattern matching in condition**: `if [42] in [x]` — RuboCop skips
///    `any_match_pattern_type?`. Fix: detect MatchPredicateNode/MatchRequiredNode.
/// 6. **nonempty_line_count > 3**: Multiline conditions like `if a &&\n  b\n  body\nend`
///    have 4+ non-empty lines. RuboCop skips these. Fix: count non-empty lines in
///    the entire if/unless node source range.
/// 7. **Bare regexp literal on the LHS of `=~`**: `if /foo/ =~ bar` is accepted
///    by RuboCop, but parenthesized conditions like `if(/foo/ =~ bar)`,
///    interpolated regexps like `if /#{foo}/ =~ bar`, and modifier-form lines
///    that become too long are still offenses. Fix: skip only bare
///    non-modifier predicates whose top-level condition is an `=~` call with a
///    plain regexp literal receiver.
///
/// FN root causes (2026-04-01): The biggest remaining cluster was long
/// modifier-form statements like `raise '...' if condition`. The old Rust cop
/// returned immediately for any modifier-form `if`/`unless`, so it never
/// reached RuboCop's `too_long_due_to_modifier?` branch. Fixed by checking
/// modifier-form nodes separately, measuring the rendered line length with the
/// same `Layout/LineLength` allowances RuboCop uses here, and skipping only the
/// narrow `foo if bar; baz` same-line sibling case that RuboCop also ignores.
///
/// FN root cause (2026-04-04): `condition_contains_defined` was a blanket skip
/// for any `defined?()` in the condition. RuboCop only skips when the argument
/// is a local variable or method call (`:lvar`/`:send`) that hasn't been
/// previously assigned (`defined_argument_is_undefined?`). For constants
/// (`JRUBY_VERSION`), class variables (`@@logger`), instance variables, and
/// global variables, `defined?` doesn't change scoping semantics in modifier
/// form, so the cop should still flag. Fixed by checking the DefinedNode's
/// `value()` type in the visitor — only set found=true for
/// LocalVariableReadNode or CallNode arguments.
///
/// Remaining FN: `{ if user then body end }` (one-line if inside block)
/// — the `}` after `end` is treated as "code after end", skipping the
/// detection. Fixing this by allowing `}` would cause FPs for
/// `#{if cond then val end}` inside string interpolation. Needs AST-level
/// parent context check (e.g., detecting EmbeddedStatementsNode) to
/// distinguish block closing from interpolation closing.
pub struct IfUnlessModifier;

/// Check if a node (or any descendant) contains a heredoc.
/// Heredoc locations in Prism only cover the delimiter, so the actual
/// source spans more lines than the node location suggests.
fn node_contains_heredoc(node: &ruby_prism::Node<'_>) -> bool {
    let mut finder = HeredocFinder { found: false };
    finder.visit(node);
    finder.found
}

struct HeredocFinder {
    found: bool,
}

impl<'pr> Visit<'pr> for HeredocFinder {
    fn visit_interpolated_string_node(&mut self, node: &ruby_prism::InterpolatedStringNode<'pr>) {
        if let Some(open) = node.opening_loc() {
            if open.as_slice().starts_with(b"<<") {
                self.found = true;
                return;
            }
        }
        ruby_prism::visit_interpolated_string_node(self, node);
    }

    fn visit_string_node(&mut self, node: &ruby_prism::StringNode<'pr>) {
        if let Some(open) = node.opening_loc() {
            if open.as_slice().starts_with(b"<<") {
                self.found = true;
                return;
            }
        }
        ruby_prism::visit_string_node(self, node);
    }
}

/// Check if a node (or any descendant) contains a `defined?()` call.
///
/// RuboCop skips `if defined?(x)` when the argument is a local variable
/// or method call that might be undefined — converting to modifier form
/// changes the semantics of `defined?` with respect to local variable
/// scoping.  We conservatively skip any condition that contains `defined?`.
fn condition_contains_defined(node: &ruby_prism::Node<'_>) -> bool {
    let mut finder = DefinedFinder { found: false };
    finder.visit(node);
    finder.found
}

struct DefinedFinder {
    found: bool,
}

impl<'pr> Visit<'pr> for DefinedFinder {
    fn visit_defined_node(&mut self, node: &ruby_prism::DefinedNode<'pr>) {
        // RuboCop only skips `defined?` when the argument is a local variable
        // or method call (`:lvar` or `:send`) that hasn't been previously assigned.
        // For constants (`JRUBY_VERSION`), class variables (`@@logger`), instance
        // variables, and global variables, `defined?` doesn't change semantics
        // in modifier form, so the cop should still flag those.
        let value = node.value();
        if value.as_local_variable_read_node().is_some() || value.as_call_node().is_some() {
            self.found = true;
        }
    }
}

/// Check if a node (or any descendant) contains a local variable assignment (lvasgn).
///
/// RuboCop's `non_eligible_condition?` skips conditions that assign local
/// variables, because the modifier form may change scoping semantics.
fn condition_contains_lvasgn(node: &ruby_prism::Node<'_>) -> bool {
    let mut finder = LvasgnFinder { found: false };
    finder.visit(node);
    finder.found
}

struct LvasgnFinder {
    found: bool,
}

impl<'pr> Visit<'pr> for LvasgnFinder {
    fn visit_local_variable_write_node(&mut self, _node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        self.found = true;
    }
}

/// Check if the condition contains a named regexp capture (`/(?<x>...)/ =~ str`).
///
/// RuboCop's `named_capture_in_condition?` checks `match_with_lvasgn_type?`.
/// In Prism, this is represented as a `MatchWriteNode`.
fn condition_contains_named_capture(node: &ruby_prism::Node<'_>) -> bool {
    let mut finder = NamedCaptureFinder { found: false };
    finder.visit(node);
    finder.found
}

struct NamedCaptureFinder {
    found: bool,
}

impl<'pr> Visit<'pr> for NamedCaptureFinder {
    fn visit_match_write_node(&mut self, _node: &ruby_prism::MatchWriteNode<'pr>) {
        self.found = true;
    }
}

/// Check whether the top-level condition is a bare regexp literal on the left
/// side of `=~`, e.g. `if /foo/ =~ bar`.
///
/// RuboCop still flags parenthesized conditions like `if(/foo/ =~ bar)`,
/// interpolated regexps like `if /#{foo}/ =~ bar`, and modifier-form lines
/// that are too long, so this intentionally stays narrow.
fn condition_is_bare_regexp_lhs_match(node: &ruby_prism::Node<'_>) -> bool {
    let Some(call) = node.as_call_node() else {
        return false;
    };

    if call.name().as_slice() != b"=~" {
        return false;
    }

    let Some(receiver) = call.receiver() else {
        return false;
    };

    receiver.as_regular_expression_node().is_some()
}

/// Check if the condition contains pattern matching (`in` operator).
///
/// RuboCop's `pattern_matching_nodes` checks `any_match_pattern_type?`.
/// In Prism, `[42] in [x]` is a `MatchPredicateNode` and `[42] => x` is
/// `MatchRequiredNode`.
fn condition_contains_pattern_matching(node: &ruby_prism::Node<'_>) -> bool {
    let mut finder = PatternMatchFinder { found: false };
    finder.visit(node);
    finder.found
}

struct PatternMatchFinder {
    found: bool,
}

impl<'pr> Visit<'pr> for PatternMatchFinder {
    fn visit_match_predicate_node(&mut self, _node: &ruby_prism::MatchPredicateNode<'pr>) {
        self.found = true;
    }
    fn visit_match_required_node(&mut self, _node: &ruby_prism::MatchRequiredNode<'pr>) {
        self.found = true;
    }
}

/// Check if a body node is an endless method definition (`def method_name = body`).
///
/// RuboCop skips these to avoid conflict with `Style/AmbiguousEndlessMethodDefinition`.
fn body_is_endless_method(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(def_node) = node.as_def_node() {
        return def_node.equal_loc().is_some();
    }
    false
}

/// Check if a node (or any descendant) contains a nested conditional
/// (if/unless/ternary). RuboCop's `nested_conditional?` on IfNode checks
/// whether any branch contains a nested `:if` node (which includes ternaries).
/// We check the body for any descendant IfNode or UnlessNode.
fn body_contains_nested_conditional(node: &ruby_prism::Node<'_>) -> bool {
    let mut finder = NestedConditionalFinder { found: false };
    finder.visit(node);
    finder.found
}

struct NestedConditionalFinder {
    found: bool,
}

impl<'pr> Visit<'pr> for NestedConditionalFinder {
    fn visit_if_node(&mut self, _node: &ruby_prism::IfNode<'pr>) {
        self.found = true;
    }
    fn visit_unless_node(&mut self, _node: &ruby_prism::UnlessNode<'pr>) {
        self.found = true;
    }
}

fn normalize_ruby_regex(pattern: &str) -> String {
    let mut s = pattern.trim().to_string();

    if s.starts_with('/') {
        s.remove(0);
        if let Some(last_slash) = s.rfind('/') {
            s.truncate(last_slash);
        }
    }

    s.replace("\\A", "^")
        .replace("\\z", "$")
        .replace("\\Z", "$")
}

fn indentation_difference(line: &[u8], indentation_width: usize) -> usize {
    if indentation_width <= 1 || line.first() != Some(&b'\t') {
        return 0;
    }

    let leading_tabs = line.iter().take_while(|&&b| b == b'\t').count();

    leading_tabs * (indentation_width - 1)
}

fn uri_extends_to_end(
    line: &str,
    schemes: &[String],
    max: usize,
    indentation_width: usize,
) -> bool {
    let mut all_starts = Vec::new();
    for scheme in schemes {
        for prefix in [format!("{scheme}://"), format!(r"{scheme}:\/\/")] {
            let mut search_from = 0;
            while let Some(pos) = line[search_from..].find(&prefix) {
                let abs_pos = search_from + pos;
                all_starts.push(abs_pos);
                search_from = abs_pos + prefix.len();
            }
        }
    }

    if all_starts.is_empty() {
        return false;
    }

    let indentation_diff = indentation_difference(line.as_bytes(), indentation_width);

    for start in all_starts {
        let uri_end = start
            + line[start..]
                .find(|c: char| c.is_whitespace())
                .unwrap_or(line.len() - start);

        let mut end_pos = uri_end;
        if line.contains('{') && line.ends_with('}') {
            if let Some(brace_pos) = line[end_pos..].rfind('}') {
                end_pos += brace_pos + 1;
            }
        }

        let rest = &line[end_pos..];
        let non_ws_len = rest.find(|c: char| c.is_whitespace()).unwrap_or(rest.len());
        end_pos += non_ws_len;

        let start_chars = line[..start].chars().count() + indentation_diff;
        if start_chars < max && end_pos >= line.len() {
            return true;
        }
    }

    false
}

fn modifier_form_too_long(
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
    config: &CopConfig,
) -> bool {
    let max_line_length = config.get_usize("MaxLineLength", 120);
    if max_line_length == 0 || !config.get_bool("LineLengthEnabled", max_line_length > 0) {
        return false;
    }

    let node_src = &source.as_bytes()[node.location().start_offset()..node.location().end_offset()];
    if node_src.contains(&b'\n') {
        return false;
    }

    let (line_num, _) = source.offset_to_line_col(node.location().start_offset());
    let lines: Vec<&[u8]> = source.lines().collect();
    if line_num == 0 || line_num > lines.len() {
        return false;
    }

    let raw_line = lines[line_num - 1];
    let line = raw_line.strip_suffix(b"\r").unwrap_or(raw_line);
    let line_str = match std::str::from_utf8(line) {
        Ok(s) => s,
        Err(_) => return line.len() > max_line_length,
    };

    let indentation_width = config.get_usize("IndentationWidth", 2);
    let effective_len = line_str.chars().count() + indentation_difference(line, indentation_width);
    if effective_len <= max_line_length {
        return false;
    }

    if config.get_bool("AllowCopDirectives", true) {
        if let Some(comment_start) = line_str.find("# rubocop:") {
            let without_directive_chars = line_str[..comment_start].trim_end().chars().count();
            if without_directive_chars <= max_line_length {
                return false;
            }
        }
    }

    let allowed_patterns = config
        .get_string_array("AllowedPatterns")
        .unwrap_or_default();
    if !allowed_patterns.is_empty() {
        let compiled_patterns: Vec<Regex> = allowed_patterns
            .iter()
            .filter_map(|pattern| Regex::new(&normalize_ruby_regex(pattern)).ok())
            .collect();
        if compiled_patterns
            .iter()
            .any(|regex| regex.is_match(line_str))
        {
            return false;
        }
    }

    if config.get_bool("AllowURI", true) {
        let uri_schemes = config
            .get_string_array("URISchemes")
            .unwrap_or_else(|| vec!["http".into(), "https".into()]);
        if uri_extends_to_end(line_str, &uri_schemes, max_line_length, indentation_width) {
            return false;
        }
    }

    true
}

fn has_another_statement_on_same_line(source: &SourceFile, node: &ruby_prism::Node<'_>) -> bool {
    let (line_num, _) = source.offset_to_line_col(node.location().end_offset());
    let lines: Vec<&[u8]> = source.lines().collect();
    if line_num == 0 || line_num > lines.len() {
        return false;
    }

    let line_start = source.line_start_offset(line_num);
    let line = lines[line_num - 1];
    let after_start = node.location().end_offset().saturating_sub(line_start);
    if after_start >= line.len() {
        return false;
    }

    let after = &line[after_start..];
    let trimmed = after
        .iter()
        .copied()
        .skip_while(|&b| b == b' ' || b == b'\t')
        .collect::<Vec<_>>();

    trimmed.first() == Some(&b';')
}

/// Check if an IfNode or UnlessNode is a pattern matching guard (e.g., `in "a" if cond`).
/// In Prism, pattern matching guards are IfNode/UnlessNode inside InNode.pattern.
/// We detect this by checking if the text from line start to the node's start is just `in`.
fn is_pattern_matching_guard(source: &SourceFile, node: &ruby_prism::Node<'_>) -> bool {
    let loc = node.location();
    let start = loc.start_offset();
    let (line, _col) = source.offset_to_line_col(start);
    if let Some(line_start) = source.line_col_to_offset(line, 0) {
        if let Some(prefix) = source.try_byte_slice(line_start, start) {
            let trimmed = prefix.trim();
            return trimmed == "in";
        }
    }
    false
}

/// Check if a `# rubocop:disable` or `# rubocop:todo` comment disables
/// `Style/IfUnlessModifier` specifically (or `all`). Comments that disable
/// OTHER cops should still be counted in modifier-form line length.
fn comment_disables_this_cop(comment: &str) -> bool {
    // Match patterns like:
    //   # rubocop:disable Style/IfUnlessModifier
    //   # rubocop:todo Style/IfUnlessModifier
    //   # rubocop:disable all
    //   # rubocop:disable Foo, Style/IfUnlessModifier, Bar
    for keyword in ["rubocop:disable", "rubocop:todo"] {
        if let Some(pos) = comment.find(keyword) {
            let after = &comment[pos + keyword.len()..];
            let after = after.trim_start();
            // Check if any of the comma-separated cop names is "all" or our cop
            for cop in after.split(',') {
                let cop = cop.trim();
                if cop == "all" || cop == "Style/IfUnlessModifier" {
                    return true;
                }
            }
        }
    }
    false
}

fn first_line_comment_len(
    source: &SourceFile,
    kw_line: usize,
    predicate: &ruby_prism::Node<'_>,
) -> usize {
    let lines: Vec<&[u8]> = source.lines().collect();
    if kw_line == 0 || kw_line > lines.len() {
        return 0;
    }

    let kw_line_start = source.line_start_offset(kw_line);
    let predicate_end_in_line = predicate
        .location()
        .end_offset()
        .saturating_sub(kw_line_start);
    let kw_line_bytes = lines[kw_line - 1];
    if predicate_end_in_line >= kw_line_bytes.len() {
        return 0;
    }

    let after_predicate = &kw_line_bytes[predicate_end_in_line..];
    let trimmed = after_predicate
        .iter()
        .copied()
        .skip_while(|&b| b == b' ' || b == b'\t')
        .collect::<Vec<_>>();
    if !trimmed.starts_with(b"#") {
        return 0;
    }

    let comment = match std::str::from_utf8(&trimmed) {
        Ok(comment) => comment,
        Err(_) => return 0,
    };

    // Only exclude comments that disable THIS cop (Style/IfUnlessModifier) or
    // all cops. Comments disabling OTHER cops carry over to the modifier form
    // and must be counted in the line length (matching RuboCop's behavior).
    if comment_disables_this_cop(comment) {
        return 0;
    }

    1 + comment.chars().count()
}

impl Cop for IfUnlessModifier {
    fn name(&self) -> &'static str {
        "Style/IfUnlessModifier"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[IF_NODE, UNLESS_NODE]
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
        // Extract keyword location, predicate, statements, has_else, and keyword name
        // from either IfNode or UnlessNode
        let (kw_loc, predicate, statements, has_else, keyword) =
            if let Some(if_node) = node.as_if_node() {
                let kw_loc = match if_node.if_keyword_loc() {
                    Some(loc) => loc,
                    None => return, // ternary
                };
                // Skip elsif nodes — they are visited as IfNode but can't be
                // converted to modifier form independently
                if kw_loc.as_slice() == b"elsif" {
                    return;
                }
                (
                    kw_loc,
                    if_node.predicate(),
                    if_node.statements(),
                    if_node.subsequent().is_some(),
                    "if",
                )
            } else if let Some(unless_node) = node.as_unless_node() {
                (
                    unless_node.keyword_loc(),
                    unless_node.predicate(),
                    unless_node.statements(),
                    unless_node.else_clause().is_some(),
                    "unless",
                )
            } else {
                return;
            };

        // Skip pattern matching guards (e.g., `in "a" if condition`).
        // Prism wraps the pattern + guard as IfNode/UnlessNode inside InNode.
        if is_pattern_matching_guard(source, node) {
            return;
        }

        // Must not have an else clause
        if has_else {
            return;
        }

        let body = match statements {
            Some(stmts) => stmts,
            None => return,
        };

        let body_stmts = body.body();

        // Must have exactly one statement
        if body_stmts.len() != 1 {
            return;
        }

        let body_node = match body_stmts.iter().next() {
            Some(n) => n,
            None => return,
        };

        let modifier_form = kw_loc.start_offset() > body_node.location().start_offset();

        // Skip if the body is an endless method definition — conflict with
        // Style/AmbiguousEndlessMethodDefinition (RuboCop: endless_method?).
        if body_is_endless_method(&body_node) {
            return;
        }

        // Skip if the condition contains `defined?()` — converting to modifier
        // form changes semantics for undefined variables/methods.
        if condition_contains_defined(&predicate) {
            return;
        }

        // Skip if the condition contains pattern matching (in/=>) — modifier form
        // changes variable scoping semantics (RuboCop: pattern_matching_nodes).
        if condition_contains_pattern_matching(&predicate) {
            return;
        }

        if modifier_form {
            if !modifier_form_too_long(source, node, config)
                || has_another_statement_on_same_line(source, node)
            {
                return;
            }

            let (line, column) = source.offset_to_line_col(node.location().start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!("Modifier form of `{keyword}` makes the line too long."),
            ));
            return;
        }

        // RuboCop accepts bare non-modifier predicates like `if /foo/ =~ bar`,
        // but still flags parenthesized/interpolated variants and modifier-form
        // lines that become too long.
        if condition_is_bare_regexp_lhs_match(&predicate) {
            return;
        }

        // Skip if the condition contains a local variable assignment — modifier
        // form may change scoping semantics (RuboCop: non_eligible_condition?).
        if condition_contains_lvasgn(&predicate) {
            return;
        }

        // Skip if the condition contains a named regexp capture — modifier form
        // changes semantics (RuboCop: named_capture_in_condition?).
        if condition_contains_named_capture(&predicate) {
            return;
        }

        // Skip if the body contains any nested conditional (if/unless/ternary).
        // RuboCop's `nested_conditional?` checks if any branch contains a nested
        // `:if` node, which includes ternaries (e.g., `a = x ? y : z`).
        if body_contains_nested_conditional(&body_node) {
            return;
        }

        // Body must be on a single line to be eligible for modifier form
        let (body_start_line, _) = source.offset_to_line_col(body_node.location().start_offset());
        let body_end_off = body_node
            .location()
            .end_offset()
            .saturating_sub(1)
            .max(body_node.location().start_offset());
        let (body_end_line, _) = source.offset_to_line_col(body_end_off);
        if body_start_line != body_end_line {
            return;
        }

        // If there are standalone comment lines between keyword and body, don't suggest
        // modifier form — converting would lose the comments. But blank lines and
        // multiline condition continuation lines are OK.
        let (kw_line, _) = source.offset_to_line_col(kw_loc.start_offset());
        if body_start_line > kw_line + 1 {
            let lines: Vec<&[u8]> = source.lines().collect();
            for line_num in (kw_line + 1)..body_start_line {
                if line_num > 0 && line_num <= lines.len() {
                    let line = lines[line_num - 1];
                    let trimmed: Vec<u8> = line
                        .iter()
                        .skip_while(|&&b| b == b' ' || b == b'\t')
                        .copied()
                        .collect();
                    if trimmed.starts_with(b"#") {
                        return;
                    }
                }
            }
        }

        // Check if body contains a heredoc argument. Prism's node location for heredoc
        // references only covers the opening delimiter (<<~FOO), not the heredoc content.
        // The actual output would span more lines than the AST suggests.
        if node_contains_heredoc(&body_node) {
            return;
        }

        // Skip if body line has an EOL comment — converting to modifier would lose it
        {
            let lines: Vec<&[u8]> = source.lines().collect();
            if body_start_line > 0 && body_start_line <= lines.len() {
                let body_line = lines[body_start_line - 1];
                let body_end_in_line = body_node.location().end_offset();
                let (_, body_end_col) = source.offset_to_line_col(body_end_in_line);
                // Check if there's a comment after the body on the same line
                if body_end_col < body_line.len() {
                    let after_body = &body_line[body_end_col..];
                    let trimmed = after_body
                        .iter()
                        .skip_while(|&&b| b == b' ' || b == b'\t')
                        .copied()
                        .collect::<Vec<_>>();
                    if trimmed.starts_with(b"#") {
                        return;
                    }
                }
            }
        }

        // Skip if there's a comment before `end` on its own line, a comment on the
        // `end` line, or code after `end` on the same line (chained calls like
        // `end.inspect`, `end&.foo`, `end + 2`).
        {
            let end_loc: Option<ruby_prism::Location<'_>> = if let Some(if_node) = node.as_if_node()
            {
                if_node.end_keyword_loc()
            } else if let Some(unless_node) = node.as_unless_node() {
                unless_node.end_keyword_loc()
            } else {
                None
            };
            if let Some(end_loc) = end_loc {
                let end_off = end_loc.start_offset();
                let (end_line, end_col) = source.offset_to_line_col(end_off);
                if end_line > body_start_line + 1 {
                    // There are lines between body and end — check for comments
                    let lines: Vec<&[u8]> = source.lines().collect();
                    for line_num in (body_start_line + 1)..end_line {
                        if line_num > 0 && line_num <= lines.len() {
                            let line = lines[line_num - 1];
                            let trimmed: Vec<u8> = line
                                .iter()
                                .skip_while(|&&b| b == b' ' || b == b'\t')
                                .copied()
                                .collect();
                            if trimmed.starts_with(b"#") {
                                return;
                            }
                        }
                    }
                }

                // Check if the `end` line has a comment or code after `end`
                // (chained calls, binary operators, etc.)
                let lines: Vec<&[u8]> = source.lines().collect();
                if end_line > 0 && end_line <= lines.len() {
                    let end_line_bytes = lines[end_line - 1];
                    let after_end_col = end_col + 3; // "end" is 3 bytes
                    if after_end_col < end_line_bytes.len() {
                        let after_end = &end_line_bytes[after_end_col..];
                        let trimmed = after_end
                            .iter()
                            .copied()
                            .skip_while(|&b| b == b' ' || b == b'\t')
                            .collect::<Vec<_>>();
                        // Any non-empty content after `end` (comment or code) means
                        // we can't simply convert to modifier form
                        if !trimmed.is_empty() && trimmed[0] != b'\n' && trimmed[0] != b'\r' {
                            return;
                        }
                    }
                }
            }
        }

        // Skip if the entire if/unless node has more than 3 non-empty lines.
        // RuboCop's `non_eligible_node?` checks `node.nonempty_line_count > 3`.
        // This catches multiline conditions like `if a &&\n  b\n  body\nend`.
        {
            let node_src =
                &source.as_bytes()[node.location().start_offset()..node.location().end_offset()];
            let nonempty_count = node_src
                .split(|&b| b == b'\n')
                .filter(|line| line.iter().any(|&b| b != b' ' && b != b'\t' && b != b'\r'))
                .count();
            if nonempty_count > 3 {
                return;
            }
        }

        let max_line_length = config.get_usize("MaxLineLength", 120);
        // When MaxLineLength is 0, Layout/LineLength is disabled — skip line length check
        // (matches RuboCop's behavior: return true unless max_line_length)
        let line_length_enabled = config.get_bool("LineLengthEnabled", max_line_length > 0);

        // Estimate modifier line length: body + " " + keyword + " " + condition
        let body_text = &source.as_bytes()
            [body_node.location().start_offset()..body_node.location().end_offset()];
        let cond_text = &source.as_bytes()
            [predicate.location().start_offset()..predicate.location().end_offset()];

        // Include indentation in the modifier line length estimate.
        // The modifier form `body keyword condition` would be placed at the
        // indentation level of the original `if`/`unless` keyword, not at the
        // body's (deeper) indentation.
        let (_, kw_col) = source.offset_to_line_col(kw_loc.start_offset());

        // Account for tab expansion: the visual width of the indentation before
        // the keyword may be wider than the byte count if tabs are used.
        // RuboCop's `line_length` adds `indentation_difference` for leading tabs.
        let indentation_width = config.get_usize("IndentationWidth", 2);
        let tab_expansion = if indentation_width > 1 {
            let kw_line_start = kw_loc.start_offset() - kw_col;
            let before_kw = &source.as_bytes()[kw_line_start..kw_loc.start_offset()];
            let leading_tabs = before_kw.iter().take_while(|&&b| b == b'\t').count();
            leading_tabs * (indentation_width - 1)
        } else {
            0
        };

        // When the if/unless is used as the value of an assignment (e.g.,
        // `x = if cond; body; end`), RuboCop's `parenthesize?` wraps the modifier
        // form in parens: `x = (body if cond)`. This adds 2 chars to the line.
        // Check if the line before the keyword contains an assignment operator.
        let parens_overhead = {
            let kw_line_start = kw_loc.start_offset() - kw_col;
            let before_kw = &source.as_bytes()[kw_line_start..kw_loc.start_offset()];
            // Check if the content before keyword on the same line is just whitespace;
            // if not, it might contain assignment context. But the real case is when
            // the assignment is on the PREVIOUS line (multi-line assignment).
            // We check the previous non-blank line for a trailing `=`.
            let before_kw_trimmed = before_kw
                .iter()
                .copied()
                .filter(|&b| b != b' ' && b != b'\t')
                .count();
            if before_kw_trimmed == 0 && kw_line_start > 0 {
                // Check the previous line for trailing `=`
                let lines: Vec<&[u8]> = source.lines().collect();
                let (kw_line_num, _) = source.offset_to_line_col(kw_loc.start_offset());
                if kw_line_num >= 2 {
                    let prev_line = lines[kw_line_num - 2];
                    let trimmed = prev_line
                        .iter()
                        .copied()
                        .rev()
                        .skip_while(|&b| b == b' ' || b == b'\t')
                        .collect::<Vec<_>>();
                    if trimmed.first() == Some(&b'=') {
                        2 // add 2 for parentheses: "(" and ")"
                    } else {
                        0
                    }
                } else {
                    0
                }
            } else {
                0
            }
        };

        // For multiline conditions, normalize whitespace (newlines + runs of spaces)
        // into single spaces to estimate the modifier form length accurately.
        let cond_len = {
            let mut len = 0usize;
            let mut in_ws = false;
            for &b in cond_text {
                if b == b' ' || b == b'\t' || b == b'\n' || b == b'\r' {
                    if !in_ws {
                        len += 1;
                        in_ws = true;
                    }
                } else {
                    len += 1;
                    in_ws = false;
                }
            }
            len
        };
        let modifier_len = kw_col
            + tab_expansion
            + parens_overhead
            + body_text.len()
            + 1
            + keyword.len()
            + 1
            + cond_len
            + first_line_comment_len(source, kw_line, &predicate);

        if !line_length_enabled || modifier_len <= max_line_length {
            let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!("Favor modifier `{keyword}` usage when having a single-line body."),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(IfUnlessModifier, "cops/style/if_unless_modifier");

    #[test]
    fn config_max_line_length() {
        use crate::testutil::{assert_cop_no_offenses_full_with_config, run_cop_full_with_config};
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("MaxLineLength".into(), serde_yml::Value::Number(40.into()))]),
            ..CopConfig::default()
        };
        // Short body + condition fits in 40 chars as modifier => should suggest modifier
        let source = b"if x\n  y\nend\n";
        let diags = run_cop_full_with_config(&IfUnlessModifier, source, config.clone());
        assert!(
            !diags.is_empty(),
            "Should fire with MaxLineLength:40 on short if"
        );

        // Longer body that would exceed 40 chars as modifier => should NOT suggest
        let source2 =
            b"if some_very_long_condition_variable_name\n  do_something_important_here\nend\n";
        assert_cop_no_offenses_full_with_config(&IfUnlessModifier, source2, config);
    }

    #[test]
    fn config_line_length_disabled() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        // When LineLengthEnabled is false (Layout/LineLength disabled),
        // modifier form should always be suggested regardless of line length.
        // This matches RuboCop behavior where `max_line_length` returns nil
        // when the cop is disabled.
        let config = CopConfig {
            options: HashMap::from([
                ("LineLengthEnabled".into(), serde_yml::Value::Bool(false)),
                ("MaxLineLength".into(), serde_yml::Value::Number(40.into())),
            ]),
            ..CopConfig::default()
        };
        // This body + condition would exceed 40 chars, but since line length is
        // disabled, it should still suggest modifier form.
        let source =
            b"if some_very_long_condition_variable_name\n  do_something_important_here\nend\n";
        let diags = run_cop_full_with_config(&IfUnlessModifier, source, config);
        assert!(
            !diags.is_empty(),
            "Should fire when LineLengthEnabled is false regardless of line length"
        );
    }
}
