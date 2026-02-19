use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, PARENTHESES_NODE, STATEMENTS_NODE};

pub struct ArrayIntersect;

impl Cop for ArrayIntersect {
    fn name(&self) -> &'static str {
        "Style/ArrayIntersect"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, PARENTHESES_NODE, STATEMENTS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // intersect? requires Ruby >= 3.1
        let ruby_version = config
            .options
            .get("TargetRubyVersion")
            .and_then(|v| v.as_f64().or_else(|| v.as_u64().map(|u| u as f64)))
            .unwrap_or(3.4);
        if ruby_version < 3.1 {
            return Vec::new();
        }

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");

        // Pattern: (array1 & array2).any? / .empty? / .none?
        if matches!(method_name, "any?" | "empty?" | "none?") {
            // Skip if the call has arguments or a block (any? with block)
            if call.arguments().is_some() || call.block().is_some() {
                return Vec::new();
            }

            if let Some(receiver) = call.receiver() {
                // Check for parenthesized expression containing &
                if let Some(paren) = receiver.as_parentheses_node() {
                    if let Some(body) = paren.body() {
                        if let Some(stmts) = body.as_statements_node() {
                            let stmt_list: Vec<_> = stmts.body().iter().collect();
                            if stmt_list.len() == 1 {
                                if let Some(inner_call) = stmt_list[0].as_call_node() {
                                    let inner_method = std::str::from_utf8(inner_call.name().as_slice()).unwrap_or("");
                                    if inner_method == "&" {
                                        let loc = node.location();
                                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                                        let msg = format!(
                                            "Use `intersect?` instead of `({}).{}`.",
                                            std::str::from_utf8(inner_call.location().as_slice()).unwrap_or("array1 & array2"),
                                            method_name
                                        );
                                        return vec![self.diagnostic(source, line, column, msg)];
                                    }
                                }
                            }
                        }
                    }
                }

                // Check for a.intersection(b).any? / .empty? / .none?
                if let Some(recv_call) = receiver.as_call_node() {
                    let recv_method = std::str::from_utf8(recv_call.name().as_slice()).unwrap_or("");
                    if recv_method == "intersection" {
                        // Must have exactly 1 argument and a receiver
                        if recv_call.receiver().is_some() {
                            if let Some(args) = recv_call.arguments() {
                                let arg_list: Vec<_> = args.arguments().iter().collect();
                                if arg_list.len() == 1 {
                                    let loc = node.location();
                                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                                    let msg = format!(
                                        "Use `intersect?` instead of `intersection(...).{}`.",
                                        method_name
                                    );
                                    return vec![self.diagnostic(source, line, column, msg)];
                                }
                            }
                        }
                    }
                }
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ArrayIntersect, "cops/style/array_intersect");
}
