use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

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
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // Pass 1: Collect all MatchData reference positions with their scope info
        let mut ref_collector = MatchDataRefCollector {
            refs: Vec::new(),
            current_scope: None,
        };
        ref_collector.visit(&parse_result.node());

        // Pass 2: Visit conditions and check for matches
        let mut visitor = ConditionVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            match_data_refs: ref_collector.refs,
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

struct ConditionVisitor<'a, 'src> {
    cop: &'a RegexpMatch,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    match_data_refs: Vec<MatchDataRef>,
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
            self.current_scope,
            &mut self.diagnostics,
        );
        ruby_prism::visit_unless_node(self, node);
    }

    // RuboCop only checks on_if (covers if/unless/elsif/ternary) and on_case.
    // It does NOT check while/until conditions.

    fn visit_case_node(&mut self, node: &ruby_prism::CaseNode<'pr>) {
        // RuboCop only checks case-less when (i.e., `case\n when cond\n ...`)
        if node.predicate().is_none() {
            let case_start = node.location().start_offset();
            for condition in node.conditions().iter() {
                if let Some(when_node) = condition.as_when_node() {
                    for when_cond in when_node.conditions().iter() {
                        check_condition(
                            self.cop,
                            self.source,
                            &when_cond,
                            case_start,
                            &self.match_data_refs,
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
fn check_condition(
    cop: &RegexpMatch,
    source: &SourceFile,
    cond: &ruby_prism::Node<'_>,
    if_node_offset: usize,
    match_data_refs: &[MatchDataRef],
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
/// `cond_offset` is the start of the condition expression (e.g. `x =~ /re/`).
/// `if_node_offset` is the start of the enclosing if/unless node (to handle modifier forms
/// where `return $1 if x =~ /re/` has `$1` before the condition but still on the same line).
fn last_match_used_in_scope(
    _cond_offset: usize,
    if_node_offset: usize,
    match_data_refs: &[MatchDataRef],
    current_scope: Option<ScopeId>,
) -> bool {
    for r in match_data_refs {
        if r.scope == current_scope {
            // MatchData ref in the same scope.
            // RuboCop checks from the match position (or if_branch start for modifier forms)
            // to the next match in the same scope.
            // We check: ref is at or after the if_node start (covers modifier `return $1 if x =~ /re/`)
            // or at or after the condition offset.
            if r.offset >= if_node_offset {
                return true;
            }
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
    current_scope: Option<ScopeId>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Skip if either side is nil (shouldn't happen for =~/!~ but be safe)
    if call.receiver().is_none() {
        return;
    }

    // Check if MatchData is used in the same scope
    if last_match_used_in_scope(
        call.location().start_offset(),
        if_node_offset,
        match_data_refs,
        current_scope,
    ) {
        return;
    }

    let op_str = if method == b"!~" { "!~" } else { "=~" };
    let loc = call.location();
    let (line, column) = source.offset_to_line_col(loc.start_offset());
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
fn check_match_method(
    cop: &RegexpMatch,
    source: &SourceFile,
    call: &ruby_prism::CallNode<'_>,
    if_node_offset: usize,
    match_data_refs: &[MatchDataRef],
    current_scope: Option<ScopeId>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Must have a receiver (x.match)
    let receiver = match call.receiver() {
        Some(r) => r,
        None => return,
    };

    // Must have arguments (x.match(y) or x.match(y, pos))
    let arguments = match call.arguments() {
        Some(a) => a,
        None => return,
    };

    let first_arg = match arguments.arguments().iter().next() {
        Some(a) => a,
        None => return,
    };

    // RuboCop requires at least one side to be a regexp, string, or symbol literal
    let recv_is_literal = is_match_literal(&receiver);
    let arg_is_literal = is_match_literal(&first_arg);

    if !recv_is_literal && !arg_is_literal {
        return;
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

    // Check if MatchData is used in the same scope
    if last_match_used_in_scope(
        call.location().start_offset(),
        if_node_offset,
        match_data_refs,
        current_scope,
    ) {
        return;
    }

    let loc = call.location();
    let (line, column) = source.offset_to_line_col(loc.start_offset());
    diagnostics.push(cop.diagnostic(
        source,
        line,
        column,
        "Use `match?` instead of `match` when `MatchData` is not used.".to_string(),
    ));
}

/// Check === with regexp literal on LHS.
fn check_threequals(
    cop: &RegexpMatch,
    source: &SourceFile,
    call: &ruby_prism::CallNode<'_>,
    if_node_offset: usize,
    match_data_refs: &[MatchDataRef],
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

    // Check if MatchData is used in the same scope
    if last_match_used_in_scope(
        call.location().start_offset(),
        if_node_offset,
        match_data_refs,
        current_scope,
    ) {
        return;
    }

    let loc = call.location();
    let (line, column) = source.offset_to_line_col(loc.start_offset());
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
}
