use crate::cop::node_type::{ARRAY_NODE, CALL_NODE, SYMBOL_NODE};
use crate::cop::util::keyword_arg_value;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct MatchRoute;

impl Cop for MatchRoute {
    fn name(&self) -> &'static str {
        "Rails/MatchRoute"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/config/routes.rb", "**/config/routes/**/*.rb"]
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE, CALL_NODE, SYMBOL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must be receiverless `match` call
        if call.receiver().is_some() || call.name().as_slice() != b"match" {
            return;
        }

        // Check for `via:` option
        let via_value = keyword_arg_value(&call, b"via");

        let http_method = match via_value {
            None => {
                // No via option -> defaults to GET
                "get"
            }
            Some(ref val) => {
                // via: :get (single symbol)
                if let Some(sym) = val.as_symbol_node() {
                    let unescaped = sym.unescaped();
                    if unescaped == b"get" {
                        "get"
                    } else if unescaped == b"post" {
                        "post"
                    } else if unescaped == b"put" {
                        "put"
                    } else if unescaped == b"patch" {
                        "patch"
                    } else if unescaped == b"delete" {
                        "delete"
                    } else if unescaped == b"all" {
                        return; // via: :all is fine
                    } else {
                        return;
                    }
                } else if val.as_array_node().is_some() {
                    // via: [:get, :post] - multiple methods is fine
                    return;
                } else {
                    return;
                }
            }
        };

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use `{http_method}` instead of `match` to define a route."),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MatchRoute, "cops/rails/match_route");
}
