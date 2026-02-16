use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct MemoizedInstanceVariableName;

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

        // Strip trailing ? or ! from method name for matching
        let base_name = method_name_str.trim_end_matches(|c| c == '?' || c == '!');

        let body = match def_node.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        // Look for @var ||= pattern
        // The body should be a StatementsNode with a single OrWriteNode
        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        // Check each statement for @var ||= pattern
        for stmt in stmts.body().iter() {
            if let Some(or_write) = stmt.as_instance_variable_or_write_node() {
                let ivar_name = or_write.name().as_slice();
                let ivar_str = std::str::from_utf8(ivar_name).unwrap_or("");
                // Strip leading @
                let ivar_base = ivar_str.strip_prefix('@').unwrap_or(ivar_str);

                // Check if ivar matches method name
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
            }
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
