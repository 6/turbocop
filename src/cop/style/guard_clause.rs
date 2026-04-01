use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Detects both RuboCop guard-clause forms:
/// - terminal `if`/`unless` bodies without `else`, which become `return` guards
/// - block-form `if ... else` / `unless ... else` where one branch is already a
///   scope-exiting guard (`return`, `raise`, `fail`, `next`, `break`, or
///   `and`/`or` with such a RHS)
///
/// FN fixes applied:
/// 1. Removed incorrect `branch_is_trivial` gate from `register_branch_guard_clause`.
///    For if-else patterns where the inline guard exceeds max line length, we were
///    skipping when the remaining branch was "trivial" (single simple statement).
///    RuboCop's `trivial?` returns false for two-branch nodes, so if-else guard
///    clauses are always flagged regardless of remaining-branch complexity.
/// 2. Added recursion into the if/unless-branch after registering an ending guard
///    clause offense, matching RuboCop's `check_ending_body(node.if_branch)`.
///    This detects nested bare `if`/`unless` that are the last statement inside
///    another if/unless branch at the end of a method body.
/// 3. Matched RuboCop's descendant-only local-variable check for assignment-in-
///    condition suppression. nitrocop was counting the root condition/statement
///    node itself, which incorrectly accepted `if foo = bar` endings and bare
///    `foo` branches that RuboCop still flags. We now mirror
///    `each_descendant(:lvasgn)` / `each_descendant(:lvar)` and ignore the root.
/// 4. Matched RuboCop's parser-shape checks for assignment conditions inside
///    multi-statement branches and `||=` / `&&=` local writes. Prism wraps
///    branches in `StatementsNode` and uses distinct local write node kinds,
///    so nitrocop was missing real offenses while also over-flagging accepted
///    parenthesized assignments like multi-statement deprecation helpers.
pub struct GuardClause;

const GUARD_METHODS: &[&[u8]] = &[b"raise", b"fail"];

impl Cop for GuardClause {
    fn name(&self) -> &'static str {
        "Style/GuardClause"
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
        let min_body_length = config.get_usize("MinBodyLength", 1);
        let _allow_consecutive = config.get_bool("AllowConsecutiveConditionals", false);
        let max_line_length = config.get_usize("MaxLineLength", 120);
        let mut visitor = GuardClauseVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            min_body_length,
            max_line_length,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct GuardClauseVisitor<'a, 'src> {
    cop: &'a GuardClause,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    min_body_length: usize,
    max_line_length: usize,
}

impl GuardClauseVisitor<'_, '_> {
    /// Check if the ending of a method body is an if/unless that could be a guard clause.
    fn check_ending_body(&mut self, body: &ruby_prism::Node<'_>) {
        if let Some(if_node) = body.as_if_node() {
            self.check_ending_if_node(&if_node);
        } else if let Some(unless_node) = body.as_unless_node() {
            self.check_ending_unless_node(&unless_node);
        } else if let Some(stmts) = body.as_statements_node() {
            // Body is a StatementsNode (begin block) - check last statement
            let body_nodes: Vec<_> = stmts.body().iter().collect();
            if let Some(last) = body_nodes.last() {
                if let Some(if_node) = last.as_if_node() {
                    self.check_ending_if_node(&if_node);
                } else if let Some(unless_node) = last.as_unless_node() {
                    self.check_ending_unless_node(&unless_node);
                }
            }
        }
    }

    fn check_ending_if_node(&mut self, node: &ruby_prism::IfNode<'_>) {
        // if_keyword_loc() is None for ternary
        let if_keyword_loc = match node.if_keyword_loc() {
            Some(loc) => loc,
            None => return, // ternary
        };

        // Check that the keyword is actually "if" (not elsif)
        if if_keyword_loc.as_slice() != b"if" {
            return;
        }

        // Modifier if: the node location starts before the keyword (at the body expression)
        if node.location().start_offset() != if_keyword_loc.start_offset() {
            return;
        }

        if self.node_is_single_line(&node.as_node()) {
            return;
        }

        // If it has a subsequent branch (else/elsif), skip for ending guard clause check
        if node.subsequent().is_some() {
            return;
        }

        // Skip if condition spans multiple lines
        let predicate = node.predicate();
        if self.is_multiline(&predicate) {
            return;
        }

        // Skip if condition assigns a local variable used in the if body
        if self.assigned_lvar_used_in_branch(&predicate, node.statements()) {
            return;
        }

        // Check min body length
        let end_offset = node
            .end_keyword_loc()
            .map(|l| l.start_offset())
            .unwrap_or(node.location().end_offset());
        if !self.meets_min_body_length(if_keyword_loc.start_offset(), end_offset) {
            return;
        }

        let condition_src = self.node_source(&predicate);
        let example = format!("return unless {}", condition_src);
        let (line, column) = self
            .source
            .offset_to_line_col(if_keyword_loc.start_offset());

        // Skip if guard clause would be too long and body is trivial
        if self.too_long_and_trivial(
            column,
            &example,
            node.statements(),
            node.subsequent().is_some(),
        ) {
            return;
        }

        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            format!(
                "Use a guard clause (`{}`) instead of wrapping the code inside a conditional expression.",
                example
            ),
        ));

        // Recurse into the if-branch to check its ending body (matches RuboCop behavior)
        if let Some(body_stmts) = node.statements() {
            let body_nodes: Vec<_> = body_stmts.body().iter().collect();
            if let Some(last) = body_nodes.last() {
                if let Some(inner_if) = last.as_if_node() {
                    self.check_ending_if_node(&inner_if);
                } else if let Some(inner_unless) = last.as_unless_node() {
                    self.check_ending_unless_node(&inner_unless);
                }
            }
        }
    }

    fn check_if_else_guard_clause(&mut self, node: &ruby_prism::IfNode<'_>) {
        let if_keyword_loc = match node.if_keyword_loc() {
            Some(loc) => loc,
            None => return,
        };

        if if_keyword_loc.as_slice() != b"if" {
            return;
        }

        if node.location().start_offset() != if_keyword_loc.start_offset() {
            return;
        }

        let else_node = match node.subsequent().and_then(|sub| sub.as_else_node()) {
            Some(node) => node,
            None => return,
        };

        // Skip if else branch has no actual statements (comment-only else)
        if !Self::else_has_statements(&else_node) {
            return;
        }

        if self.keyword_has_code_before(if_keyword_loc.start_offset())
            || self.keyword_has_multiline_assignment_before(if_keyword_loc.start_offset())
        {
            return;
        }

        let predicate = node.predicate();
        if self.is_multiline(&predicate) {
            return;
        }

        if self.assigned_lvar_used_in_branch(&predicate, node.statements()) {
            return;
        }

        if let Some(guard_stmt) = self.single_guard_statement(node.statements()) {
            self.register_branch_guard_clause(
                if_keyword_loc.start_offset(),
                &predicate,
                &guard_stmt,
                "if",
                else_node.statements(),
            );
            return;
        }

        if let Some(guard_stmt) = self.single_guard_statement(else_node.statements()) {
            self.register_branch_guard_clause(
                if_keyword_loc.start_offset(),
                &predicate,
                &guard_stmt,
                "unless",
                node.statements(),
            );
        }
    }

    fn check_ending_unless_node(&mut self, node: &ruby_prism::UnlessNode<'_>) {
        // Check for modifier form: in modifier unless, the node location starts
        // before the keyword (at the expression). If the node start != keyword start,
        // it's a modifier form.
        let keyword_loc = node.keyword_loc();
        if node.location().start_offset() != keyword_loc.start_offset() {
            return;
        }

        if self.node_is_single_line(&node.as_node()) {
            return;
        }

        // If it has an else branch, skip
        if node.else_clause().is_some() {
            return;
        }

        // Skip if condition spans multiple lines
        let predicate = node.predicate();
        if self.is_multiline(&predicate) {
            return;
        }

        // Skip if condition assigns a local variable used in the body
        if self.assigned_lvar_used_in_branch(&predicate, node.statements()) {
            return;
        }

        // Check min body length
        let end_offset = node
            .end_keyword_loc()
            .map(|l| l.start_offset())
            .unwrap_or(node.location().end_offset());
        if !self.meets_min_body_length(keyword_loc.start_offset(), end_offset) {
            return;
        }

        let condition_src = self.node_source(&predicate);
        let example = format!("return if {}", condition_src);
        let (line, column) = self.source.offset_to_line_col(keyword_loc.start_offset());

        // Skip if guard clause would be too long and body is trivial
        if self.too_long_and_trivial(
            column,
            &example,
            node.statements(),
            node.else_clause().is_some(),
        ) {
            return;
        }

        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            format!(
                "Use a guard clause (`{}`) instead of wrapping the code inside a conditional expression.",
                example
            ),
        ));

        // Recurse into the unless-branch to check its ending body (matches RuboCop behavior)
        if let Some(body_stmts) = node.statements() {
            let body_nodes: Vec<_> = body_stmts.body().iter().collect();
            if let Some(last) = body_nodes.last() {
                if let Some(inner_if) = last.as_if_node() {
                    self.check_ending_if_node(&inner_if);
                } else if let Some(inner_unless) = last.as_unless_node() {
                    self.check_ending_unless_node(&inner_unless);
                }
            }
        }
    }

    fn check_unless_else_guard_clause(&mut self, node: &ruby_prism::UnlessNode<'_>) {
        let keyword_loc = node.keyword_loc();
        if node.location().start_offset() != keyword_loc.start_offset() {
            return;
        }

        let else_node = match node.else_clause() {
            Some(node) => node,
            None => return,
        };

        // Skip if else branch has no actual statements (comment-only else)
        if !Self::else_has_statements(&else_node) {
            return;
        }

        if self.keyword_has_code_before(keyword_loc.start_offset())
            || self.keyword_has_multiline_assignment_before(keyword_loc.start_offset())
        {
            return;
        }

        let predicate = node.predicate();
        if self.is_multiline(&predicate) {
            return;
        }

        if self.assigned_lvar_used_in_branch(&predicate, node.statements()) {
            return;
        }

        if let Some(guard_stmt) = self.single_guard_statement(node.statements()) {
            self.register_branch_guard_clause(
                keyword_loc.start_offset(),
                &predicate,
                &guard_stmt,
                "unless",
                else_node.statements(),
            );
            return;
        }

        if let Some(guard_stmt) = self.single_guard_statement(else_node.statements()) {
            self.register_branch_guard_clause(
                keyword_loc.start_offset(),
                &predicate,
                &guard_stmt,
                "if",
                node.statements(),
            );
        }
    }

    /// Check if an else node has actual code statements (not just comments).
    /// Prism emits an ElseNode even for comment-only else branches, but RuboCop's
    /// Parser gem treats those as no-else. We must match that behavior.
    fn else_has_statements(else_node: &ruby_prism::ElseNode<'_>) -> bool {
        else_node
            .statements()
            .is_some_and(|s| s.body().iter().next().is_some())
    }

    /// Check if a node spans multiple lines.
    fn is_multiline(&self, node: &ruby_prism::Node<'_>) -> bool {
        let loc = node.location();
        let (start_line, _) = self.source.offset_to_line_col(loc.start_offset());
        let (end_line, _) = self.source.offset_to_line_col(loc.end_offset());
        end_line > start_line
    }

    /// Check if descendant local variable assignments in the condition are used
    /// by descendant nodes in the branch.
    ///
    /// This mirrors RuboCop's `each_descendant(:lvasgn)` / `each_descendant(:lvar)`
    /// behavior and intentionally ignores the root condition/statement node.
    fn assigned_lvar_used_in_branch(
        &self,
        condition: &ruby_prism::Node<'_>,
        statements: Option<ruby_prism::StatementsNode<'_>>,
    ) -> bool {
        let assigned_names = collect_descendant_lvar_write_names(condition);
        if assigned_names.is_empty() {
            return false;
        }
        let used_names = collect_parser_equivalent_lvar_read_names(statements);
        assigned_names.iter().any(|name| used_names.contains(name))
    }

    fn register_branch_guard_clause(
        &mut self,
        keyword_offset: usize,
        condition: &ruby_prism::Node<'_>,
        guard_stmt: &ruby_prism::Node<'_>,
        conditional_keyword: &str,
        _remaining_branch: Option<ruby_prism::StatementsNode<'_>>,
    ) {
        if !self.guard_stmt_is_single_line(guard_stmt) {
            return;
        }

        let guard_src = self.node_source(guard_stmt);
        let condition_src = self.node_source(condition);
        let inline_example = format!("{} {} {}", guard_src, conditional_keyword, condition_src);
        let (line, column) = self.source.offset_to_line_col(keyword_offset);

        let example = if self.too_long_for_single_line(column, &inline_example) {
            format!(
                "{} {}; {}; end",
                conditional_keyword, condition_src, guard_src
            )
        } else {
            inline_example
        };

        self.push_offense(line, column, &example);
    }

    fn push_offense(&mut self, line: usize, column: usize, example: &str) {
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            format!(
                "Use a guard clause (`{}`) instead of wrapping the code inside a conditional expression.",
                example
            ),
        ));
    }

    /// Check if the guard clause would exceed max line length AND the body is trivial.
    /// "Trivial" means a single-branch if/unless with a body that is not itself an
    /// if/unless or begin block. In this case, RuboCop skips the offense.
    fn too_long_and_trivial(
        &self,
        column: usize,
        example: &str,
        statements: Option<ruby_prism::StatementsNode<'_>>,
        has_else: bool,
    ) -> bool {
        let total_len = column + example.len();
        if total_len <= self.max_line_length {
            return false;
        }
        // Too long -- check if body is trivial
        if has_else {
            return false;
        }
        let stmts = match statements {
            Some(s) => s,
            None => return true, // empty body is trivial
        };
        let body_nodes: Vec<_> = stmts.body().iter().collect();
        if body_nodes.len() != 1 {
            return false;
        }
        let single = &body_nodes[0];
        // Not trivial if the body is itself an if/unless or begin
        if single.as_if_node().is_some()
            || single.as_unless_node().is_some()
            || single.as_begin_node().is_some()
        {
            return false;
        }
        true
    }

    fn too_long_for_single_line(&self, column: usize, example: &str) -> bool {
        self.max_line_length > 0 && column + example.len() > self.max_line_length
    }

    fn single_guard_statement<'pr>(
        &self,
        statements: Option<ruby_prism::StatementsNode<'pr>>,
    ) -> Option<ruby_prism::Node<'pr>> {
        let stmts = statements?;
        let mut body = stmts.body().iter();
        let stmt = body.next()?;
        if body.next().is_some() {
            return None;
        }
        if is_guard_stmt(&stmt) {
            Some(stmt)
        } else {
            None
        }
    }

    fn guard_stmt_is_single_line(&self, guard_stmt: &ruby_prism::Node<'_>) -> bool {
        let (check_start, check_end) = guard_clause_check_location(guard_stmt);
        let end_offset = check_end.saturating_sub(1).max(check_start);
        let start_line = self.source.offset_to_line_col(check_start).0;
        let end_line = self.source.offset_to_line_col(end_offset).0;
        start_line == end_line
    }

    fn keyword_has_code_before(&self, keyword_offset: usize) -> bool {
        let (line, _) = self.source.offset_to_line_col(keyword_offset);
        let line_start_offset = self.source.line_col_to_offset(line, 0).unwrap_or(0);
        self.source.as_bytes()[line_start_offset..keyword_offset]
            .iter()
            .any(|&b| b != b' ' && b != b'\t')
    }

    fn keyword_has_multiline_assignment_before(&self, keyword_offset: usize) -> bool {
        let (line, _) = self.source.offset_to_line_col(keyword_offset);
        if line <= 1 {
            return false;
        }

        let lines: Vec<&[u8]> = self.source.lines().collect();
        let mut idx = line - 2;
        loop {
            let prev = trim_ascii_whitespace(lines[idx]);
            if prev.is_empty() || prev.starts_with(b"#") {
                if idx == 0 {
                    return false;
                }
                idx -= 1;
                continue;
            }
            return line_ends_with_assignment(prev);
        }
    }

    fn node_is_single_line(&self, node: &ruby_prism::Node<'_>) -> bool {
        let loc = node.location();
        let start_line = self.source.offset_to_line_col(loc.start_offset()).0;
        let end_offset = loc.end_offset().saturating_sub(1).max(loc.start_offset());
        let end_line = self.source.offset_to_line_col(end_offset).0;
        start_line == end_line
    }

    fn meets_min_body_length(&self, start_offset: usize, end_offset: usize) -> bool {
        let (start_line, _) = self.source.offset_to_line_col(start_offset);
        let (end_line, _) = self.source.offset_to_line_col(end_offset);
        let body_lines = if end_line > start_line + 1 {
            end_line - start_line - 1
        } else if end_line > start_line {
            0
        } else {
            1
        };
        body_lines >= self.min_body_length
    }

    fn node_source(&self, node: &ruby_prism::Node<'_>) -> String {
        let loc = node.location();
        let bytes = &self.source.as_bytes()[loc.start_offset()..loc.end_offset()];
        String::from_utf8_lossy(bytes).to_string()
    }
}

/// Visitor to collect local variable write names from a node tree.
struct LvarWriteCollector {
    names: Vec<Vec<u8>>,
}

impl<'pr> Visit<'pr> for LvarWriteCollector {
    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        self.names.push(node.name().as_slice().to_vec());
        ruby_prism::visit_local_variable_write_node(self, node);
    }

    fn visit_local_variable_and_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableAndWriteNode<'pr>,
    ) {
        self.names.push(node.name().as_slice().to_vec());
        ruby_prism::visit_local_variable_and_write_node(self, node);
    }

    fn visit_local_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOperatorWriteNode<'pr>,
    ) {
        self.names.push(node.name().as_slice().to_vec());
        ruby_prism::visit_local_variable_operator_write_node(self, node);
    }

    fn visit_local_variable_or_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOrWriteNode<'pr>,
    ) {
        self.names.push(node.name().as_slice().to_vec());
        ruby_prism::visit_local_variable_or_write_node(self, node);
    }

    fn visit_local_variable_target_node(
        &mut self,
        node: &ruby_prism::LocalVariableTargetNode<'pr>,
    ) {
        // Multi-assignment targets: (var, obj = ...)
        self.names.push(node.name().as_slice().to_vec());
    }
}

/// Visitor to collect local variable read names from a node tree.
struct LvarReadCollector {
    names: Vec<Vec<u8>>,
}

impl<'pr> Visit<'pr> for LvarReadCollector {
    fn visit_local_variable_read_node(&mut self, node: &ruby_prism::LocalVariableReadNode<'pr>) {
        self.names.push(node.name().as_slice().to_vec());
    }
}

fn collect_descendant_lvar_write_names(node: &ruby_prism::Node<'_>) -> Vec<Vec<u8>> {
    let mut collector = LvarWriteCollector { names: Vec::new() };
    collector.visit(node);
    if let Some(write) = node.as_local_variable_write_node() {
        remove_first_name(&mut collector.names, write.name().as_slice());
    }
    collector.names
}

fn collect_descendant_lvar_read_names(node: &ruby_prism::Node<'_>) -> Vec<Vec<u8>> {
    let mut collector = LvarReadCollector { names: Vec::new() };
    collector.visit(node);
    if let Some(read) = node.as_local_variable_read_node() {
        remove_first_name(&mut collector.names, read.name().as_slice());
    }
    collector.names
}

fn remove_first_name(names: &mut Vec<Vec<u8>>, name: &[u8]) {
    if let Some(index) = names
        .iter()
        .position(|candidate| candidate.as_slice() == name)
    {
        names.remove(index);
    }
}

fn collect_parser_equivalent_lvar_read_names(
    statements: Option<ruby_prism::StatementsNode<'_>>,
) -> Vec<Vec<u8>> {
    let Some(statements) = statements else {
        return Vec::new();
    };

    let body_nodes: Vec<_> = statements.body().iter().collect();
    match body_nodes.as_slice() {
        [] => Vec::new(),
        [single] => collect_descendant_lvar_read_names(single),
        _ => collect_descendant_lvar_read_names(&statements.as_node()),
    }
}

fn is_guard_stmt(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();
        if GUARD_METHODS.contains(&name) && call.receiver().is_none() {
            return true;
        }
    }

    if node.as_return_node().is_some()
        || node.as_break_node().is_some()
        || node.as_next_node().is_some()
    {
        return true;
    }

    if let Some(and_node) = node.as_and_node() {
        return is_guard_stmt(&and_node.right());
    }
    if let Some(or_node) = node.as_or_node() {
        return is_guard_stmt(&or_node.right());
    }

    false
}

fn guard_clause_check_location<'a>(node: &'a ruby_prism::Node<'a>) -> (usize, usize) {
    if let Some(and_node) = node.as_and_node() {
        let right = and_node.right();
        return guard_clause_check_location(&right);
    }
    if let Some(or_node) = node.as_or_node() {
        let right = or_node.right();
        return guard_clause_check_location(&right);
    }
    (node.location().start_offset(), node.location().end_offset())
}

fn trim_ascii_whitespace(bytes: &[u8]) -> &[u8] {
    let start = bytes
        .iter()
        .position(|&b| b != b' ' && b != b'\t' && b != b'\r' && b != b'\n')
        .unwrap_or(bytes.len());
    let end = bytes
        .iter()
        .rposition(|&b| b != b' ' && b != b'\t' && b != b'\r' && b != b'\n')
        .map(|idx| idx + 1)
        .unwrap_or(start);
    &bytes[start..end]
}

fn line_ends_with_assignment(bytes: &[u8]) -> bool {
    if bytes.len() < 2 || !bytes.ends_with(b"=") {
        return false;
    }

    !bytes.ends_with(b"==")
        && !bytes.ends_with(b"!=")
        && !bytes.ends_with(b">=")
        && !bytes.ends_with(b"<=")
        && !bytes.ends_with(b"=>")
        && !bytes.ends_with(b"=~")
}

impl<'pr> Visit<'pr> for GuardClauseVisitor<'_, '_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        if let Some(body) = node.body() {
            self.check_ending_body(&body);
        }
        ruby_prism::visit_def_node(self, node);
    }

    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        self.check_if_else_guard_clause(node);
        ruby_prism::visit_if_node(self, node);
    }

    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        self.check_unless_else_guard_clause(node);
        ruby_prism::visit_unless_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(GuardClause, "cops/style/guard_clause");
}
