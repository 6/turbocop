use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Corpus investigation (round 1): FP=2, FN=250. Root cause: last_match_used_in_scope checked
/// if ANY MatchData ref in the same scope had offset >= if_node_offset, without
/// limiting the search to the range before the next match node. Fixed by collecting
/// all match condition positions upfront and computing per-match ranges.
///
/// Corpus investigation (round 2): FP=11, FN=31. Three root causes:
/// 1. FP: next_match boundary used if_node_offset instead of match_expr_offset.
///    For modifier-if `raise "#{$1}" if x =~ /re/`, the if_node_offset includes the
///    body, so a `$1` in the NEXT statement's body was included in THIS match's range.
///    RuboCop uses the match expression offset as the upper boundary, not the if_node
///    offset. Fixed by storing both offsets and using match_expr_offset for boundaries.
/// 2. FP/FN: match positions only collected from if/unless/case conditions, not bare
///    match expressions (`x =~ /re/` as a statement). Bare matches reset MatchData
///    and serve as boundaries. Fixed by collecting ALL match expressions in Pass 2.
/// 3. FN: bare `match(/re/)` without receiver was not flagged. RuboCop's pattern
///    `(send _recv :match ...)` matches nil receiver (implicit self). Fixed by allowing
///    receiverless match() when the argument is a literal.
/// 4. FN: case/when used case_start as if_node_offset for all when conditions, making
///    them indistinguishable. Fixed by using each when condition's own offset.
///
/// Corpus investigation (round 3): FP=2, FN=0. Root cause: when a MatchData reference
/// (e.g., `$``, `Regexp.last_match`) is used as the receiver of the NEXT match expression,
/// its byte offset equals the next match expression's start offset. The boundary check
/// `r.offset < upper_bound` excluded it. Fixed by using `<=` so refs at the exact boundary
/// are included. Examples: `$` =~ MGR0` after `w =~ /eed$/`, and
/// `Regexp.last_match[0] =~ /re/` after `str =~ /pattern/`.
///
/// ## Extended corpus investigation (2026-03-23)
///
/// Extended corpus reported FP=38, FN=0. All 38 FPs from vendored gem files in
/// repos cjstewart88__Tubalr (heroku/ruby/1.9.1/gems/rdoc-*),
/// liaoziyang__stackneveroverflow (vendor/bundle/ruby/2.3.0/gems/rdoc-*),
/// infochimps-labs__wukong, and pitluga__supply_drop. Root cause: AllCops.Exclude
/// patterns like `vendor/**/*` failed to match when the scanned directory was
/// outside the config's base_dir (corpus oracle scenario). Fixed by adding
/// scan_roots support to CopFilterSet for out-of-tree path relativization.
/// The TargetRubyVersion check (minimum 2.4) remains as an additional guard.
pub struct RegexpMatch;

impl Cop for RegexpMatch {
    fn name(&self) -> &'static str {
        "Performance/RegexpMatch"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // RuboCop: minimum_target_ruby_version 2.4
        // match? was added in Ruby 2.4, so this cop only applies for 2.4+.
        let ruby_version = config
            .options
            .get("TargetRubyVersion")
            .and_then(|v| v.as_f64().or_else(|| v.as_u64().map(|u| u as f64)))
            .unwrap_or(2.7);
        if ruby_version < 2.4 {
            return;
        }

        // Pass 1: Collect all MatchData reference positions with their scope info
        let mut ref_collector = MatchDataRefCollector {
            refs: Vec::new(),
            current_scope: None,
        };
        ref_collector.visit(&parse_result.node());

        // Pass 2: Collect ALL match expression positions (=~, !~, .match, ===)
        // anywhere in the body — not just in conditions. Non-condition matches also
        // reset MatchData and serve as boundaries for next_match_pos computation.
        let mut match_collector = AllMatchExprCollector {
            positions: Vec::new(),
            current_scope: None,
        };
        match_collector.visit(&parse_result.node());

        // Pass 3: Visit conditions and check for matches
        let mut visitor = ConditionVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            match_data_refs: ref_collector.refs,
            match_positions: match_collector.positions,
            current_scope: None,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

/// A scope boundary (def, class, module) identified by byte offset range.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct ScopeId {
    start: usize,
    end: usize,
}

/// A reference to MatchData ($~, $1, $&, etc.) with its scope info.
struct MatchDataRef {
    offset: usize,
    scope: Option<ScopeId>,
}

/// A match expression position (=~, !~, .match, ===) anywhere in the body.
/// Used to compute the "next match" boundary — the upper limit of the range
/// where MatchData references are searched for each match.
struct MatchExprPos {
    /// Start of the actual match expression (=~, match, ===).
    offset: usize,
    scope: Option<ScopeId>,
}

/// Pass 1: Collect all MatchData references ($~, $1, $&, $', $`, $+,
/// $MATCH, $PREMATCH, $POSTMATCH, $LAST_PAREN_MATCH, Regexp.last_match).
struct MatchDataRefCollector {
    refs: Vec<MatchDataRef>,
    current_scope: Option<ScopeId>,
}

impl<'pr> Visit<'pr> for MatchDataRefCollector {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        let old = self.current_scope;
        let loc = node.location();
        self.current_scope = Some(ScopeId {
            start: loc.start_offset(),
            end: loc.end_offset(),
        });
        ruby_prism::visit_def_node(self, node);
        self.current_scope = old;
    }

    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        let old = self.current_scope;
        let loc = node.location();
        self.current_scope = Some(ScopeId {
            start: loc.start_offset(),
            end: loc.end_offset(),
        });
        ruby_prism::visit_class_node(self, node);
        self.current_scope = old;
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        let old = self.current_scope;
        let loc = node.location();
        self.current_scope = Some(ScopeId {
            start: loc.start_offset(),
            end: loc.end_offset(),
        });
        ruby_prism::visit_module_node(self, node);
        self.current_scope = old;
    }

    fn visit_back_reference_read_node(&mut self, node: &ruby_prism::BackReferenceReadNode<'pr>) {
        // $&, $`, $', $+, $~
        self.refs.push(MatchDataRef {
            offset: node.location().start_offset(),
            scope: self.current_scope,
        });
    }

    fn visit_numbered_reference_read_node(
        &mut self,
        node: &ruby_prism::NumberedReferenceReadNode<'pr>,
    ) {
        // $1, $2, ..., $100, etc.
        self.refs.push(MatchDataRef {
            offset: node.location().start_offset(),
            scope: self.current_scope,
        });
    }

    fn visit_global_variable_read_node(&mut self, node: &ruby_prism::GlobalVariableReadNode<'pr>) {
        // $~, $MATCH, $PREMATCH, $POSTMATCH, $LAST_PAREN_MATCH, $LAST_MATCH_INFO
        let name = node.name().as_slice();
        if name == b"$~"
            || name == b"$MATCH"
            || name == b"$PREMATCH"
            || name == b"$POSTMATCH"
            || name == b"$LAST_PAREN_MATCH"
            || name == b"$LAST_MATCH_INFO"
        {
            self.refs.push(MatchDataRef {
                offset: node.location().start_offset(),
                scope: self.current_scope,
            });
        }
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // Regexp.last_match or ::Regexp.last_match
        if node.name().as_slice() == b"last_match" {
            if let Some(recv) = node.receiver() {
                let is_regexp_const = recv
                    .as_constant_read_node()
                    .is_some_and(|c| c.name().as_slice() == b"Regexp")
                    || recv.as_constant_path_node().is_some_and(|cp| {
                        cp.name().is_some_and(|n| n.as_slice() == b"Regexp")
                            && cp.parent().is_none()
                    });
                if is_regexp_const {
                    self.refs.push(MatchDataRef {
                        offset: node.location().start_offset(),
                        scope: self.current_scope,
                    });
                }
            }
        }
        ruby_prism::visit_call_node(self, node);
    }
}

/// Pass 2: Collect ALL match expressions (=~, !~, .match, ===, match_with_lvasgn)
/// anywhere in the body. Used to compute the "next match" boundary. This includes
/// matches outside of conditions (bare statements), which also reset MatchData.
struct AllMatchExprCollector {
    positions: Vec<MatchExprPos>,
    current_scope: Option<ScopeId>,
}

/// Check if a call node is a match expression that we track (=~, !~, .match, ===).
fn is_match_call(call: &ruby_prism::CallNode<'_>) -> bool {
    let method = call.name().as_slice();
    if method == b"=~" || method == b"!~" {
        call.receiver().is_some()
    } else if method == b"match" {
        if let Some(args) = call.arguments() {
            let first_arg = args.arguments().iter().next();
            if let Some(recv) = call.receiver() {
                let recv_lit = is_match_literal(&recv);
                let arg_lit = first_arg.as_ref().is_some_and(is_match_literal);
                (recv_lit || arg_lit) && call.block().is_none()
            } else {
                // Bare match() without receiver — check if arg is a literal
                first_arg.as_ref().is_some_and(is_match_literal) && call.block().is_none()
            }
        } else {
            false
        }
    } else if method == b"===" {
        call.receiver()
            .is_some_and(|r| r.as_regular_expression_node().is_some())
            && call.arguments().is_some()
    } else {
        false
    }
}

impl<'pr> Visit<'pr> for AllMatchExprCollector {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        let old = self.current_scope;
        let loc = node.location();
        self.current_scope = Some(ScopeId {
            start: loc.start_offset(),
            end: loc.end_offset(),
        });
        ruby_prism::visit_def_node(self, node);
        self.current_scope = old;
    }

    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        let old = self.current_scope;
        let loc = node.location();
        self.current_scope = Some(ScopeId {
            start: loc.start_offset(),
            end: loc.end_offset(),
        });
        ruby_prism::visit_class_node(self, node);
        self.current_scope = old;
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        let old = self.current_scope;
        let loc = node.location();
        self.current_scope = Some(ScopeId {
            start: loc.start_offset(),
            end: loc.end_offset(),
        });
        ruby_prism::visit_module_node(self, node);
        self.current_scope = old;
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if is_match_call(node) {
            self.positions.push(MatchExprPos {
                offset: node.location().start_offset(),
                scope: self.current_scope,
            });
        }
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_match_write_node(&mut self, node: &ruby_prism::MatchWriteNode<'pr>) {
        // Named captures (/(?<name>...)/ =~ expr) — these reset MatchData too.
        self.positions.push(MatchExprPos {
            offset: node.location().start_offset(),
            scope: self.current_scope,
        });
        ruby_prism::visit_match_write_node(self, node);
    }
}

struct ConditionVisitor<'a, 'src> {
    cop: &'a RegexpMatch,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    match_data_refs: Vec<MatchDataRef>,
    match_positions: Vec<MatchExprPos>,
    current_scope: Option<ScopeId>,
}

impl<'pr> Visit<'pr> for ConditionVisitor<'_, '_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        let old = self.current_scope;
        let loc = node.location();
        self.current_scope = Some(ScopeId {
            start: loc.start_offset(),
            end: loc.end_offset(),
        });
        ruby_prism::visit_def_node(self, node);
        self.current_scope = old;
    }

    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        let old = self.current_scope;
        let loc = node.location();
        self.current_scope = Some(ScopeId {
            start: loc.start_offset(),
            end: loc.end_offset(),
        });
        ruby_prism::visit_class_node(self, node);
        self.current_scope = old;
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        let old = self.current_scope;
        let loc = node.location();
        self.current_scope = Some(ScopeId {
            start: loc.start_offset(),
            end: loc.end_offset(),
        });
        ruby_prism::visit_module_node(self, node);
        self.current_scope = old;
    }

    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        let if_start = node.location().start_offset();
        check_condition(
            self.cop,
            self.source,
            &node.predicate(),
            if_start,
            &self.match_data_refs,
            &self.match_positions,
            self.current_scope,
            &mut self.diagnostics,
        );
        ruby_prism::visit_if_node(self, node);
    }

    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        let unless_start = node.location().start_offset();
        check_condition(
            self.cop,
            self.source,
            &node.predicate(),
            unless_start,
            &self.match_data_refs,
            &self.match_positions,
            self.current_scope,
            &mut self.diagnostics,
        );
        ruby_prism::visit_unless_node(self, node);
    }

    // RuboCop only checks on_if (covers if/unless/elsif/ternary) and on_case.
    // It does NOT check while/until conditions.

    // In pattern matching `case/in`, the guard `if`/`unless` is embedded as an
    // IfNode/UnlessNode inside InNode.pattern(). The default visitor would descend
    // into these and treat the guard condition as a regular if-condition. RuboCop's
    // `on_if` does NOT fire for pattern matching guards, so we skip the pattern
    // and only visit the body (statements).
    fn visit_in_node(&mut self, node: &ruby_prism::InNode<'pr>) {
        if let Some(stmts) = node.statements() {
            self.visit(&stmts.as_node());
        }
    }

    fn visit_case_node(&mut self, node: &ruby_prism::CaseNode<'pr>) {
        // RuboCop only checks case-less when (i.e., `case\n when cond\n ...`)
        if node.predicate().is_none() {
            for condition in node.conditions().iter() {
                if let Some(when_node) = condition.as_when_node() {
                    for when_cond in when_node.conditions().iter() {
                        // RuboCop uses the condition expression's begin_pos as the
                        // range start (not case_start), so MatchData refs from
                        // earlier when bodies don't affect later when conditions.
                        let cond_start = when_cond.location().start_offset();
                        check_condition(
                            self.cop,
                            self.source,
                            &when_cond,
                            cond_start,
                            &self.match_data_refs,
                            &self.match_positions,
                            self.current_scope,
                            &mut self.diagnostics,
                        );
                    }
                }
            }
        }
        ruby_prism::visit_case_node(self, node);
    }
}

/// Check a condition expression for =~, !~, .match(), or === usage.
/// `if_node_offset` is the start of the enclosing if/unless/case node,
/// used for modifier-form MatchData detection.
#[allow(clippy::too_many_arguments)]
fn check_condition(
    cop: &RegexpMatch,
    source: &SourceFile,
    cond: &ruby_prism::Node<'_>,
    if_node_offset: usize,
    match_data_refs: &[MatchDataRef],
    match_positions: &[MatchExprPos],
    current_scope: Option<ScopeId>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(call) = cond.as_call_node() {
        let method = call.name().as_slice();

        if method == b"=~" || method == b"!~" {
            check_match_operator(
                cop,
                source,
                &call,
                method,
                if_node_offset,
                match_data_refs,
                match_positions,
                current_scope,
                diagnostics,
            );
        } else if method == b"match" {
            check_match_method(
                cop,
                source,
                &call,
                if_node_offset,
                match_data_refs,
                match_positions,
                current_scope,
                diagnostics,
            );
        } else if method == b"===" {
            check_threequals(
                cop,
                source,
                &call,
                if_node_offset,
                match_data_refs,
                match_positions,
                current_scope,
                diagnostics,
            );
        }
    }
    // MatchWriteNode (/(?<name>...)/ =~ expr) is handled by NOT matching it here —
    // named captures create local vars, so they should not be flagged.
    // NOTE: RuboCop only checks the top-level condition expression, not
    // sub-expressions within && or || chains. We match that behavior.
}

/// Check if MatchData is used in the same scope as a match at the given offset.
///
/// RuboCop's logic: search for MatchData refs in [if_node_offset, next_match_offset).
/// `if_node_offset` is the START of the search range (enclosing if/unless/case node
/// start — for modifier-if this includes the body before `if`).
/// `match_expr_offset` is the actual match expression start, used to find the NEXT
/// match boundary. The range END is the next match expression's offset in the same
/// scope. This matches RuboCop's `next_match_pos` which returns
/// `node.source_range.begin_pos`.
fn last_match_used_in_scope(
    if_node_offset: usize,
    match_expr_offset: usize,
    match_data_refs: &[MatchDataRef],
    match_positions: &[MatchExprPos],
    current_scope: Option<ScopeId>,
) -> bool {
    // Find the next match expression in the same scope after this one.
    let upper_bound = match_positions
        .iter()
        .filter(|m| m.scope == current_scope && m.offset > match_expr_offset)
        .map(|m| m.offset)
        .min()
        .unwrap_or(usize::MAX);

    // Check if any MatchData ref in the same scope falls within
    // [if_node_offset, upper_bound]. We use <= for the upper bound because
    // a MatchData ref can be the receiver of the next match expression
    // (e.g., `$` =~ MGR0` or `Regexp.last_match[0] =~ /re/`), in which
    // case the ref's offset equals the next match expression's start offset.
    for r in match_data_refs {
        if r.scope == current_scope && r.offset >= if_node_offset && r.offset <= upper_bound {
            return true;
        }
    }
    false
}

/// Check =~ or !~ operator usage.
#[allow(clippy::too_many_arguments)]
fn check_match_operator(
    cop: &RegexpMatch,
    source: &SourceFile,
    call: &ruby_prism::CallNode<'_>,
    method: &[u8],
    if_node_offset: usize,
    match_data_refs: &[MatchDataRef],
    match_positions: &[MatchExprPos],
    current_scope: Option<ScopeId>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Skip if either side is nil (shouldn't happen for =~/!~ but be safe)
    if call.receiver().is_none() {
        return;
    }

    let match_expr_offset = call.location().start_offset();

    // Check if MatchData is used in the same scope
    if last_match_used_in_scope(
        if_node_offset,
        match_expr_offset,
        match_data_refs,
        match_positions,
        current_scope,
    ) {
        return;
    }

    let op_str = if method == b"!~" { "!~" } else { "=~" };
    let (line, column) = source.offset_to_line_col(match_expr_offset);
    diagnostics.push(cop.diagnostic(
        source,
        line,
        column,
        format!(
            "Use `match?` instead of `{}` when `MatchData` is not used.",
            op_str
        ),
    ));
}

/// Check .match() method call usage.
#[allow(clippy::too_many_arguments)]
fn check_match_method(
    cop: &RegexpMatch,
    source: &SourceFile,
    call: &ruby_prism::CallNode<'_>,
    if_node_offset: usize,
    match_data_refs: &[MatchDataRef],
    match_positions: &[MatchExprPos],
    current_scope: Option<ScopeId>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Must have arguments (x.match(y) or match(y))
    let arguments = match call.arguments() {
        Some(a) => a,
        None => return,
    };

    let first_arg = match arguments.arguments().iter().next() {
        Some(a) => a,
        None => return,
    };

    // RuboCop's pattern: (send _recv :match {regexp str sym}) matches any receiver
    // including nil (bare match() calls). When there IS a receiver, at least one side
    // must be a literal. When there is NO receiver (bare match()), the arg must be a literal.
    if let Some(receiver) = call.receiver() {
        let recv_is_literal = is_match_literal(&receiver);
        let arg_is_literal = is_match_literal(&first_arg);
        if !recv_is_literal && !arg_is_literal {
            return;
        }
    } else {
        // Bare match() — RuboCop matches (send nil :match {regexp str sym})
        // The first arg must be a regexp, string, or symbol literal.
        if !is_match_literal(&first_arg) {
            return;
        }
    }

    // Don't flag if the call has a block
    if call.block().is_some() {
        return;
    }

    // Skip safe navigation (&.match)
    if let Some(op) = call.call_operator_loc() {
        let bytes = &source.as_bytes()[op.start_offset()..op.end_offset()];
        if bytes == b"&." {
            return;
        }
    }

    let match_expr_offset = call.location().start_offset();

    // Check if MatchData is used in the same scope
    if last_match_used_in_scope(
        if_node_offset,
        match_expr_offset,
        match_data_refs,
        match_positions,
        current_scope,
    ) {
        return;
    }

    let (line, column) = source.offset_to_line_col(match_expr_offset);
    diagnostics.push(cop.diagnostic(
        source,
        line,
        column,
        "Use `match?` instead of `match` when `MatchData` is not used.".to_string(),
    ));
}

/// Check === with regexp literal on LHS.
#[allow(clippy::too_many_arguments)]
fn check_threequals(
    cop: &RegexpMatch,
    source: &SourceFile,
    call: &ruby_prism::CallNode<'_>,
    if_node_offset: usize,
    match_data_refs: &[MatchDataRef],
    match_positions: &[MatchExprPos],
    current_scope: Option<ScopeId>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // RuboCop only flags /re/ === foo (regexp literal on LHS)
    let receiver = match call.receiver() {
        Some(r) => r,
        None => return,
    };

    // Must have an argument
    if call.arguments().is_none() {
        return;
    }

    // Check receiver is a regexp literal (simple or with flags, not interpolated)
    if receiver.as_regular_expression_node().is_none() {
        return;
    }

    let match_expr_offset = call.location().start_offset();

    // Check if MatchData is used in the same scope
    if last_match_used_in_scope(
        if_node_offset,
        match_expr_offset,
        match_data_refs,
        match_positions,
        current_scope,
    ) {
        return;
    }

    let (line, column) = source.offset_to_line_col(match_expr_offset);
    diagnostics.push(cop.diagnostic(
        source,
        line,
        column,
        "Use `match?` instead of `===` when `MatchData` is not used.".to_string(),
    ));
}

/// Check if a node is a regexp, string, or symbol literal.
fn is_match_literal(node: &ruby_prism::Node<'_>) -> bool {
    node.as_string_node().is_some()
        || node.as_regular_expression_node().is_some()
        || node.as_interpolated_regular_expression_node().is_some()
        || node.as_symbol_node().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(RegexpMatch, "cops/performance/regexp_match");

    #[test]
    fn matchdata_used_in_if_body() {
        use crate::testutil::run_cop_full;
        let source = b"if str =~ /(\\d+)/\n  puts $1\nend\n";
        let diags = run_cop_full(&RegexpMatch, source);
        assert!(
            diags.is_empty(),
            "Should NOT flag when $1 is used. Got: {:?}",
            diags
                .iter()
                .map(|d| format!("{}:{} {}", d.location.line, d.location.column, d.message))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn bare_match_in_if() {
        use crate::testutil::run_cop_full;
        let source = b"if match(/pattern/)\n  do_something\nend\n";
        let diags = run_cop_full(&RegexpMatch, source);
        assert_eq!(
            diags.len(),
            1,
            "Should flag bare match(). Diags: {:?}",
            diags
                .iter()
                .map(|d| format!("{}:{} {}", d.location.line, d.location.column, d.message))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn regexp_last_match_used_in_if_body() {
        use crate::testutil::run_cop_full;
        let source = b"if x =~ /pattern/\n  Regexp.last_match(1).downcase\nend\n";
        let diags = run_cop_full(&RegexpMatch, source);
        assert!(
            diags.is_empty(),
            "Should NOT flag when Regexp.last_match is used. Got: {:?}",
            diags
                .iter()
                .map(|d| format!("{}:{} {}", d.location.line, d.location.column, d.message))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn prematch_used_as_receiver_of_next_match() {
        use crate::testutil::run_cop_full;
        // $` (prematch) is used as the receiver of the next =~ expression.
        // The MatchData ref offset equals the next match expr start offset,
        // so the boundary check must use <= not <.
        let source = b"if w =~ /eed$/\n  w.chop! if $` =~ MGR0\nend\n";
        let diags = run_cop_full(&RegexpMatch, source);
        assert!(
            diags.is_empty(),
            "Should NOT flag when $` is used as receiver of next =~. Got: {:?}",
            diags
                .iter()
                .map(|d| format!("{}:{} {}", d.location.line, d.location.column, d.message))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn regexp_last_match_used_as_receiver_of_next_match() {
        use crate::testutil::run_cop_full;
        // Regexp.last_match[0] is used as receiver of the next =~ expression.
        let source = b"def conformance?(protocol)\n  return false unless str =~ /pattern/\n  Regexp.last_match[0] =~ /test/\nend\n";
        let diags = run_cop_full(&RegexpMatch, source);
        assert!(
            diags.is_empty(),
            "Should NOT flag when Regexp.last_match is used as receiver of next =~. Got: {:?}",
            diags
                .iter()
                .map(|d| format!("{}:{} {}", d.location.line, d.location.column, d.message))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn sequential_modifier_if_matchdata_suppresses_prior_match() {
        use crate::testutil::run_cop_full;
        // When $1 is between the first and second =~ (in the second match's modifier
        // body), RuboCop conservatively suppresses the first match.
        let source = b"def test\n  raise \"no db\" if out =~ /no_db/\n  raise \"missing #{$1}\" if out =~ /Unknown '(.+)'/\nend\n";
        let diags = run_cop_full(&RegexpMatch, source);
        assert_eq!(
            diags.len(),
            0,
            "Should not flag when $1 falls in range. Got: {:?}",
            diags
                .iter()
                .map(|d| format!("{}:{} {}", d.location.line, d.location.column, d.message))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn skips_when_target_ruby_below_2_4() {
        use crate::cop::CopConfig;
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;
        let source = b"if x =~ /pattern/\n  do_something\nend\n";
        let mut options = HashMap::new();
        options.insert(
            "TargetRubyVersion".to_string(),
            serde_yml::Value::Number(serde_yml::Number::from(2.3)),
        );
        let config = CopConfig {
            options,
            ..CopConfig::default()
        };
        let diags = run_cop_full_with_config(&RegexpMatch, source, config);
        assert!(
            diags.is_empty(),
            "Should NOT flag when TargetRubyVersion < 2.4. Got: {:?}",
            diags
                .iter()
                .map(|d| format!("{}:{} {}", d.location.line, d.location.column, d.message))
                .collect::<Vec<_>>()
        );
    }
}
