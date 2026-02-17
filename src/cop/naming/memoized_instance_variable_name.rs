use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct MemoizedInstanceVariableName;

impl MemoizedInstanceVariableName {
    fn check_or_write(
        &self,
        source: &SourceFile,
        or_write: ruby_prism::InstanceVariableOrWriteNode<'_>,
        base_name: &str,
        method_name_str: &str,
    ) -> Vec<Diagnostic> {
        let ivar_name = or_write.name().as_slice();
        let ivar_str = std::str::from_utf8(ivar_name).unwrap_or("");
        let ivar_base = ivar_str.strip_prefix('@').unwrap_or(ivar_str);

        if ivar_base != base_name {
            let loc = or_write.name_loc();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                format!(
                    "Memoized variable `@{ivar_base}` does not match method name `{method_name_str}`."
                ),
            )];
        }

        Vec::new()
    }
}

impl Cop for MemoizedInstanceVariableName {
    fn name(&self) -> &'static str {
        "Naming/MemoizedInstanceVariableName"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let _enforced_style = config.get_str("EnforcedStyleForLeadingUnderscores", "disallowed");

        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return Vec::new(),
        };

        let method_name = def_node.name().as_slice();
        let method_name_str = std::str::from_utf8(method_name).unwrap_or("");

        // RuboCop skips initialize methods — `||=` there is default initialization, not memoization
        if matches!(
            method_name_str,
            "initialize" | "initialize_clone" | "initialize_copy" | "initialize_dup"
        ) {
            return Vec::new();
        }

        // Strip trailing ? or ! from method name for matching
        let base_name = method_name_str.trim_end_matches(|c| c == '?' || c == '!');

        let body = match def_node.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        // Look for @var ||= pattern — only when it's the entire body or the last statement.
        // This is a memoization pattern; a `||=` in the middle of a method is just assignment.

        // Body could be a bare InstanceVariableOrWriteNode (single statement)
        if let Some(or_write) = body.as_instance_variable_or_write_node() {
            return self.check_or_write(source, or_write, base_name, method_name_str);
        }

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let body_nodes: Vec<_> = stmts.body().iter().collect();
        if body_nodes.is_empty() {
            return Vec::new();
        }

        // Only check the last statement — vendor requires ||= be the sole or last statement
        let last = &body_nodes[body_nodes.len() - 1];
        if let Some(or_write) = last.as_instance_variable_or_write_node() {
            return self.check_or_write(source, or_write, base_name, method_name_str);
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        MemoizedInstanceVariableName,
        "cops/naming/memoized_instance_variable_name"
    );
}
