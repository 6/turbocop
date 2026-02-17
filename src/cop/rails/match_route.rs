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

        // Must be receiverless `match` call
        if call.receiver().is_some() || call.name().as_slice() != b"match" {
            return Vec::new();
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
                        return Vec::new(); // via: :all is fine
                    } else {
                        return Vec::new();
                    }
                } else if val.as_array_node().is_some() {
                    // via: [:get, :post] - multiple methods is fine
                    return Vec::new();
                } else {
                    return Vec::new();
                }
            }
        };

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            format!("Use `{http_method}` instead of `match` to define a route."),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MatchRoute, "cops/rails/match_route");
}
