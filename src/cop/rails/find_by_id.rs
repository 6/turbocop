use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct FindById;

impl Cop for FindById {
    fn name(&self) -> &'static str {
        "Rails/FindById"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let name = call.name().as_slice();

        // Pattern 1: find_by_id!(id)
        if name == b"find_by_id!" {
            if call.receiver().is_some() && call.arguments().is_some() {
                let loc = call.message_loc().unwrap_or(call.location());
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Use `find` instead of `find_by_id!`.".to_string(),
                )];
            }
            return Vec::new();
        }

        // Pattern 2: find_by!(id: value)
        if name == b"find_by!" {
            if call.receiver().is_some() {
                if let Some(val) = util::keyword_arg_value(&call, b"id") {
                    // Only flag if it's the sole keyword argument
                    let _ = val;
                    let loc = call.message_loc().unwrap_or(call.location());
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        "Use `find` instead of `find_by!`.".to_string(),
                    )];
                }
            }
            return Vec::new();
        }

        // Pattern 3: where(id: value).take!
        if name == b"take!" {
            let chain = match util::as_method_chain(node) {
                Some(c) => c,
                None => return Vec::new(),
            };
            if chain.inner_method != b"where" {
                return Vec::new();
            }
            // Check that `where` has an `id:` keyword arg
            if util::keyword_arg_value(&chain.inner_call, b"id").is_some() {
                let loc = chain.inner_call.message_loc().unwrap_or(chain.inner_call.location());
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Use `find` instead of `where(id: ...).take!`.".to_string(),
                )];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FindById, "cops/rails/find_by_id");
}
