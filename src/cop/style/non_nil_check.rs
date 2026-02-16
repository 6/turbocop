use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct NonNilCheck;

impl Cop for NonNilCheck {
    fn name(&self) -> &'static str {
        "Style/NonNilCheck"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let include_semantic_changes = config.get_bool("IncludeSemanticChanges", false);

        // Pattern 1: x != nil
        if let Some(call) = node.as_call_node() {
            if call.name().as_slice() == b"!=" {
                if let Some(args) = call.arguments() {
                    let args_vec: Vec<_> = args.arguments().iter().collect();
                    if args_vec.len() == 1 && args_vec[0].as_nil_node().is_some() {
                        if call.receiver().is_some() {
                            let loc = call.location();
                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                            if include_semantic_changes {
                                return vec![self.diagnostic(
                                    source,
                                    line,
                                    column,
                                    "Explicit non-nil checks are usually redundant.".to_string(),
                                )];
                            } else {
                                // In non-semantic mode, suggest !x.nil? instead
                                let receiver_src = std::str::from_utf8(call.receiver().unwrap().location().as_slice()).unwrap_or("x");
                                let current_src = std::str::from_utf8(loc.as_slice()).unwrap_or("");
                                return vec![self.diagnostic(
                                    source,
                                    line,
                                    column,
                                    format!("Prefer `!{}.nil?` over `{}`.", receiver_src, current_src),
                                )];
                            }
                        }
                    }
                }
            }

            // Pattern 2: !x.nil? (only with IncludeSemanticChanges)
            if include_semantic_changes && call.name().as_slice() == b"!" {
                if let Some(receiver) = call.receiver() {
                    if let Some(inner_call) = receiver.as_call_node() {
                        if inner_call.name().as_slice() == b"nil?"
                            && inner_call.arguments().is_none()
                            && inner_call.receiver().is_some()
                        {
                            // Skip if inside a predicate method (method ending in ?)
                            // We don't have full parent tracking, so we just flag it
                            let loc = node.location();
                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                            return vec![self.diagnostic(
                                source,
                                line,
                                column,
                                "Explicit non-nil checks are usually redundant.".to_string(),
                            )];
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
    crate::cop_fixture_tests!(NonNilCheck, "cops/style/non_nil_check");
}
