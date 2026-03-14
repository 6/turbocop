use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Lint/OutOfRangeRegexpRef — detects `$N` back-references that exceed
/// the number of capture groups in the most recently seen regexp.
///
/// ## Investigation (2026-03-08)
///
/// **Root cause of 140 FPs:** Two issues in capture-count state management:
///
/// 1. **case/when with constant matchers:** When a `when` clause uses a constant
///    (e.g., `when SOME_PATTERN`) instead of a literal regexp, RuboCop's `on_when`
///    returns `@valid_ref = nil` (since `[].max` returns nil in Ruby). Our code was
///    not updating `current_capture_count` at all, so the capture count from a
///    previous `when` clause with a literal regexp leaked into subsequent non-literal
///    when clauses. This was the primary FP source (e.g., xcpretty parser.rb with 56 FPs
///    from a large case/when matching against constant patterns).
///
/// 2. **None vs Some(0) initial state:** RuboCop initializes `@valid_ref = 0` (any
///    `$N > 0` is an offense) but sets `@valid_ref = nil` after non-literal regexp
///    methods (no offense). Our code used `None` for both states, which conflated
///    "no regexp seen yet" (should flag) with "non-literal regexp seen" (should not flag).
///
/// **Fix:** Changed initial state to `Some(0)`, changed `None` to mean "unknown/don't flag",
/// replaced all `Some(usize::MAX)` with `None` for non-literal regexp cases, and added
/// `None` reset in `visit_case_node` for when clauses without literal regexp conditions.
///
/// ## Investigation (2026-03-11)
///
/// **Root cause of remaining 18 FPs:** Three additional state-management gaps:
///
/// 1. **`sets_backref` methods with no arguments:** Methods like `str.gsub` (no args,
///    returns enumerator) didn't reset `current_capture_count`. RuboCop's `after_send`
///    unconditionally sets `@valid_ref = nil` before checking for regexp args, so any
///    RESTRICT_ON_SEND method without a literal regexp arg resets state. Fixed by adding
///    else branch when `node.arguments()` is None.
///
/// 2. **`visit_case_match_node` zero-capture patterns:** `case/in` clauses with non-regexp
///    patterns (e.g., `in [x, y]`, `in Integer`) didn't reset `current_capture_count`.
///    RuboCop's `on_in_pattern` returns `[].max` (nil) when no regexp patterns exist.
///    Fixed by replacing `count_captures_in_pattern` with `has_regexp_in_pattern` that
///    distinguishes "no regexp" (reset to None) from "regexp with 0 captures" (Some(0)).
///
/// 3. **`visit_match_write_node` defensive fix:** Added else branch to reset to None
///    when the regexp has interpolation (can't count captures statically).
///
/// ## Investigation (2026-03-11, round 3)
///
/// **Root cause of remaining 18→13 FPs:** After-send timing mismatch in `=~`, `===`,
/// and `match` handlers.
///
/// RuboCop uses `after_send` which fires AFTER all child nodes (including the
/// receiver chain) have been visited. Nitrocop's handlers were setting the capture
/// count BEFORE calling `ruby_prism::visit_call_node()`, which re-visits the
/// receiver chain. This caused nested regexp-setting calls in the receiver (e.g.,
/// `"foo".gsub(/(a)/, "") =~ /(b)(c)/`) to overwrite the outer call's capture
/// count with the inner call's count.
///
/// **Fix:** Changed `=~`, `===`, and `match` handlers to use the same manual-visit
/// pattern as `sets_backref` methods: visit receiver and args first, THEN set the
/// capture count, THEN visit the block. This matches RuboCop's `after_send`
/// semantics. Also fixed `match` to always reset to None unconditionally (matching
/// RuboCop's `@valid_ref = nil` at the start of `after_send`), including when
/// called with no arguments.
///
/// ## Investigation (2026-03-14, round 4)
///
/// **Root cause of remaining 13 FPs:** Save/restore of `current_capture_count`
/// around `visit_case_node` and `visit_case_match_node`.
///
/// RuboCop's `on_when` / `on_in_pattern` just set `@valid_ref` for each clause
/// and let the last clause's value persist after the case statement ends. There
/// is no save/restore mechanism. Nitrocop was saving `current_capture_count`
/// before the case and restoring it after, which meant:
/// - Before case: `Some(N)` (from a previous regexp or initial `Some(0)`)
/// - Last when/in clause: non-literal condition → `None`
/// - After case: restored to `Some(N)` instead of `None`
/// - `$M` references after the case: flagged by nitrocop (M > N) but not by
///   RuboCop (nil = don't flag)
///
/// **Fix:** Removed save/restore of `current_capture_count` in both
/// `visit_case_node` and `visit_case_match_node`, matching RuboCop's behavior
/// where the last clause's capture state leaks out.
pub struct OutOfRangeRegexpRef;

impl Cop for OutOfRangeRegexpRef {
    fn name(&self) -> &'static str {
        "Lint/OutOfRangeRegexpRef"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = RegexpRefVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            // Start at Some(0) to match RuboCop's @valid_ref = 0 in on_new_investigation.
            // Any $N > 0 before the first regexp match is an offense.
            current_capture_count: Some(0),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct RegexpRefVisitor<'a, 'src> {
    cop: &'a OutOfRangeRegexpRef,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    /// Number of capture groups in the most recent regexp match.
    /// Some(n) means n capture groups were detected (0 = no groups).
    /// None means a non-literal regexp was used and captures are unknown — do not flag $N.
    /// Starts at Some(0) to match RuboCop's @valid_ref = 0 initialization.
    current_capture_count: Option<usize>,
}

impl<'pr> Visit<'pr> for RegexpRefVisitor<'_, '_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let method = node.name().as_slice();

        // `=~` operator
        // Uses after_send pattern: visit children first, then set capture count.
        // This prevents nested regexp calls in the receiver chain from clobbering
        // the count (e.g., `"foo".gsub(/(a)/, "") =~ /(b)(c)/` should use /(b)(c)/).
        if method == b"=~" {
            // Visit receiver and args first (matches RuboCop's after_send timing)
            if let Some(recv) = node.receiver() {
                self.visit(&recv);
            }
            if let Some(args) = node.arguments() {
                self.visit(&args.as_node());
            }
            // Now set capture count (after children visited, like after_send)
            // Reset first, then set from regexp if found (matches RuboCop's @valid_ref = nil)
            self.current_capture_count = None;
            if let Some(args) = node.arguments() {
                let arg_list: Vec<ruby_prism::Node<'pr>> = args.arguments().iter().collect();
                if let Some(arg) = arg_list.first() {
                    // RHS regexp takes precedence
                    if let Some(count) = count_captures_in_node(arg) {
                        self.current_capture_count = Some(count);
                    } else if let Some(recv) = node.receiver() {
                        // LHS regexp (only if RHS is not a regexp)
                        if let Some(count) = count_captures_in_node(&recv) {
                            self.current_capture_count = Some(count);
                        }
                    }
                }
            }
            // Visit block (if any) with the correct capture count
            if let Some(block) = node.block() {
                self.visit(&block);
            }
            return;
        }

        // `===` operator with regexp receiver
        // Uses after_send pattern: visit children first, then set capture count.
        if method == b"===" {
            // Visit receiver and args first
            if let Some(recv) = node.receiver() {
                self.visit(&recv);
            }
            if let Some(args) = node.arguments() {
                self.visit(&args.as_node());
            }
            // Reset, then set from regexp receiver if found
            self.current_capture_count = None;
            if let Some(recv) = node.receiver() {
                if let Some(count) = count_captures_in_node(&recv) {
                    self.current_capture_count = Some(count);
                }
            }
            // Visit block (if any)
            if let Some(block) = node.block() {
                self.visit(&block);
            }
            return;
        }

        // `match` method with regexp receiver or argument (but not `match?`)
        // Uses after_send pattern: visit children first, then set capture count.
        if method == b"match" {
            // Visit receiver and args first
            if let Some(recv) = node.receiver() {
                self.visit(&recv);
            }
            if let Some(args) = node.arguments() {
                self.visit(&args.as_node());
            }
            // Reset first (matches RuboCop's @valid_ref = nil in after_send)
            self.current_capture_count = None;
            if let Some(recv) = node.receiver() {
                if let Some(count) = count_captures_in_node(&recv) {
                    // Regexp receiver: /re/.match(str) or /re/.match
                    self.current_capture_count = Some(count);
                } else if let Some(args) = node.arguments() {
                    // Non-regexp receiver, check if arg is regexp: str.match(/re/)
                    let arg_list: Vec<ruby_prism::Node<'pr>> = args.arguments().iter().collect();
                    if let Some(arg) = arg_list.first() {
                        if let Some(count) = count_captures_in_node(arg) {
                            self.current_capture_count = Some(count);
                        }
                    }
                }
            }
            // Visit block (if any) with the correct capture count
            if let Some(block) = node.block() {
                self.visit(&block);
            }
            return;
        }

        // `match?` does NOT update $1, $2, etc.
        if method == b"match?" {
            ruby_prism::visit_call_node(self, node);
            return;
        }

        // Methods that take a regexp arg and set backreferences:
        // gsub, gsub!, sub, sub!, scan, slice, slice!, index, rindex,
        // partition, rpartition, start_with?, end_with?, []
        let sets_backref = matches!(
            method,
            b"gsub"
                | b"gsub!"
                | b"sub"
                | b"sub!"
                | b"scan"
                | b"slice"
                | b"slice!"
                | b"index"
                | b"rindex"
                | b"partition"
                | b"rpartition"
                | b"start_with?"
                | b"end_with?"
                | b"[]"
                | b"grep"
        );

        if sets_backref {
            // Visit the receiver chain first, so inner regexp calls don't
            // clobber the capture count set by THIS call's argument.
            if let Some(recv) = node.receiver() {
                self.visit(&recv);
            }
            // Visit arguments (which recurses into arg nodes)
            if let Some(args) = node.arguments() {
                self.visit(&args.as_node());
            }
            // Now set the capture count from this call's regexp argument.
            // Matches RuboCop's after_send which sets @valid_ref = nil first,
            // then only sets it to a number if a literal regexp arg is found.
            if let Some(args) = node.arguments() {
                let arg_list: Vec<ruby_prism::Node<'pr>> = args.arguments().iter().collect();
                if let Some(arg) = arg_list.first() {
                    if let Some(count) = count_captures_in_node(arg) {
                        self.current_capture_count = Some(count);
                    } else {
                        // Non-literal regexp argument (variable, constant, etc.) —
                        // captures can't be determined statically, mark as unknown
                        self.current_capture_count = None;
                    }
                }
            } else {
                // No arguments (e.g., `str.gsub` returning an enumerator) —
                // captures are unknown, mark as such to match RuboCop behavior
                self.current_capture_count = None;
            }
            // Now visit the block (if any) with the correct capture count.
            if let Some(block) = node.block() {
                self.visit(&block);
            }
            return;
        }

        ruby_prism::visit_call_node(self, node);
    }

    fn visit_match_write_node(&mut self, node: &ruby_prism::MatchWriteNode<'pr>) {
        // This is for `/(?<named>regexp)/ =~ string` where the regexp is on the LHS
        // with named captures. The receiver should always be a literal regexp,
        // but handle the else case defensively (e.g., interpolated regexp).
        let call = node.call();
        if let Some(recv) = call.receiver() {
            if let Some(count) = count_captures_in_node(&recv) {
                self.current_capture_count = Some(count);
            } else {
                // Interpolated regexp with named captures — can't count statically
                self.current_capture_count = None;
            }
        }
        ruby_prism::visit_match_write_node(self, node);
    }

    fn visit_case_node(&mut self, node: &ruby_prism::CaseNode<'pr>) {
        // For case/when, each when clause with regexp conditions sets capture count.
        // Matches RuboCop's on_when behavior: literal regexp conditions set @valid_ref
        // to max captures; non-literal conditions (constants, variables) set @valid_ref
        // to nil (= None here), meaning $N references won't be flagged.
        //
        // Note: RuboCop does NOT save/restore @valid_ref around case/when — the last
        // when clause's capture state persists after the case statement. We match this
        // by not saving/restoring here.
        for condition in node.conditions().iter() {
            if let Some(when_node) = condition.as_when_node() {
                let mut has_literal_regexp = false;
                let mut max_captures = 0;
                for cond in when_node.conditions().iter() {
                    if let Some(count) = count_captures_in_node(&cond) {
                        max_captures = max_captures.max(count);
                        has_literal_regexp = true;
                    }
                }
                if has_literal_regexp {
                    self.current_capture_count = Some(max_captures);
                } else {
                    // No literal regexp conditions — captures are unknown.
                    // Matches RuboCop's behavior where [].max returns nil.
                    self.current_capture_count = None;
                }
                if let Some(body) = when_node.statements() {
                    self.visit_statements_node(&body);
                }
            }
        }
        if let Some(else_clause) = node.else_clause() {
            self.visit_else_node(&else_clause);
        }
    }

    fn visit_case_match_node(&mut self, node: &ruby_prism::CaseMatchNode<'pr>) {
        // Matches RuboCop's on_in_pattern behavior — no save/restore, the last
        // in clause's capture state persists after the case statement.
        for condition in node.conditions().iter() {
            if let Some(in_node) = condition.as_in_node() {
                let (has_regexp, max_captures) = has_regexp_in_pattern(&in_node.pattern());
                if has_regexp {
                    self.current_capture_count = Some(max_captures);
                } else {
                    // No regexp in pattern — captures are unknown.
                    // Matches RuboCop's on_in_pattern where [].max returns nil.
                    self.current_capture_count = None;
                }
                if let Some(body) = in_node.statements() {
                    self.visit_statements_node(&body);
                }
            }
        }
        if let Some(else_clause) = node.else_clause() {
            self.visit_else_node(&else_clause);
        }
    }

    fn visit_numbered_reference_read_node(
        &mut self,
        node: &ruby_prism::NumberedReferenceReadNode<'pr>,
    ) {
        // None means a non-literal regexp was used or captures are unknown — don't flag.
        // This matches RuboCop's behavior where @valid_ref = nil causes early return.
        let Some(max_captures) = self.current_capture_count else {
            return;
        };
        let ref_num = node.number() as usize;
        if ref_num > max_captures {
            let loc = node.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            let message = if max_captures == 0 {
                format!(
                    "${} is out of range (no regexp capture groups detected).",
                    ref_num
                )
            } else if max_captures == 1 {
                format!(
                    "${} is out of range ({} regexp capture group detected).",
                    ref_num, max_captures
                )
            } else {
                format!(
                    "${} is out of range ({} regexp capture groups detected).",
                    ref_num, max_captures
                )
            };
            self.diagnostics
                .push(self.cop.diagnostic(self.source, line, column, message));
        }
    }
}

/// Count capture groups in a regexp node. Returns None if not a literal regexp.
fn count_captures_in_node(node: &ruby_prism::Node<'_>) -> Option<usize> {
    if let Some(regexp) = node.as_regular_expression_node() {
        // Check for interpolation — skip if present
        let content = regexp.unescaped();
        let content_str = std::str::from_utf8(content).ok()?;
        Some(count_capture_groups(content_str))
    } else if let Some(interp_regexp) = node.as_interpolated_regular_expression_node() {
        // If it has interpolation, we can't reliably count captures
        let mut has_interp = false;
        let mut pattern = String::new();
        for part in interp_regexp.parts().iter() {
            if let Some(s) = part.as_string_node() {
                let val = s.unescaped();
                pattern.push_str(&String::from_utf8_lossy(val));
            } else {
                has_interp = true;
            }
        }
        if has_interp {
            return None; // Can't count with interpolation
        }
        Some(count_capture_groups(&pattern))
    } else {
        None
    }
}

/// Count capture groups in a regexp pattern string.
fn count_capture_groups(pattern: &str) -> usize {
    let bytes = pattern.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut count = 0;
    let mut named_count = 0;

    while i < len {
        if bytes[i] == b'\\' {
            i += 2; // skip escaped char
            continue;
        }

        // Skip character classes
        if bytes[i] == b'[' {
            i += 1;
            if i < len && bytes[i] == b'^' {
                i += 1;
            }
            if i < len && bytes[i] == b']' {
                i += 1;
            }
            while i < len && bytes[i] != b']' {
                if bytes[i] == b'\\' {
                    i += 2;
                } else {
                    i += 1;
                }
            }
            if i < len {
                i += 1;
            }
            continue;
        }

        if bytes[i] == b'(' && i + 1 < len {
            if bytes[i + 1] == b'?' {
                if i + 2 < len {
                    match bytes[i + 2] {
                        b'<' => {
                            if i + 3 < len && bytes[i + 3] != b'=' && bytes[i + 3] != b'!' {
                                named_count += 1;
                            }
                        }
                        b'\'' => {
                            named_count += 1;
                        }
                        _ => {} // non-capturing
                    }
                }
            } else {
                count += 1;
            }
        }

        i += 1;
    }

    // If there are named captures, only named captures count for $N references
    // Named captures disable numbered captures in Ruby
    if named_count > 0 { named_count } else { count }
}

/// Check if a pattern matching expression contains any regexp nodes,
/// and return the max capture count. Returns (has_regexp, max_captures).
fn has_regexp_in_pattern(node: &ruby_prism::Node<'_>) -> (bool, usize) {
    let mut found = false;
    let mut max = 0;

    if let Some(count) = count_captures_in_node(node) {
        found = true;
        max = max.max(count);
    }

    // Check array patterns
    if let Some(arr) = node.as_array_pattern_node() {
        for elem in arr.requireds().iter() {
            let (f, c) = has_regexp_in_pattern(&elem);
            found |= f;
            max = max.max(c);
        }
    }

    // Check hash patterns
    if let Some(hash) = node.as_hash_pattern_node() {
        for elem in hash.elements().iter() {
            if let Some(assoc) = elem.as_assoc_node() {
                let (f, c) = has_regexp_in_pattern(&assoc.value());
                found |= f;
                max = max.max(c);
            }
        }
    }

    // Check alternation patterns
    if let Some(alt) = node.as_alternation_pattern_node() {
        let (f1, c1) = has_regexp_in_pattern(&alt.left());
        let (f2, c2) = has_regexp_in_pattern(&alt.right());
        found |= f1 | f2;
        max = max.max(c1).max(c2);
    }

    // Check capture patterns (=> var)
    if let Some(cap) = node.as_capture_pattern_node() {
        let (f, c) = has_regexp_in_pattern(&cap.value());
        found |= f;
        max = max.max(c);
    }

    // Check pinned patterns
    if let Some(pin) = node.as_pinned_variable_node() {
        let (f, c) = has_regexp_in_pattern(&pin.variable());
        found |= f;
        max = max.max(c);
    }

    (found, max)
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(OutOfRangeRegexpRef, "cops/lint/out_of_range_regexp_ref");
}
