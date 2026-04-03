use crate::cop::shared::node_type::{ARRAY_NODE, CALL_NODE, SYMBOL_NODE};
use crate::cop::shared::util::keyword_arg_value;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Rails/MatchRoute - suggests using specific HTTP method instead of `match`.
///
/// Investigation findings:
/// - FN root cause: single-element arrays like `via: [:get]` were treated as
///   multi-method arrays and skipped. The cop only handled bare symbol values
///   (`via: :get`) but not single-element array values (`via: [:get]`).
/// - Hashrocket syntax (`:via => [:get]`) works correctly via `keyword_arg_value`
///   which already handles SymbolNode keys in both KeywordHashNode and HashNode.
/// - Fixed by extracting the symbol from single-element arrays and matching
///   it against known HTTP methods, same as the bare symbol case.
/// - FP root cause: when `via:` is absent, the cop defaulted to "get" even when
///   arguments were dynamic (variables/method calls that could contain `via:`).
///   Examples: `match url, opts` where `opts` is a variable, and
///   `match section => redirect(...)` where the hash key is a local variable.
///   RuboCop silently skips these because `p.key.value` raises NoMethodError
///   on non-literal hash keys. Fixed by skipping when arguments contain
///   non-literal values or hash keys that aren't symbols/strings.
/// - FN cause: interpolated string/symbol keys in hashrocket syntax
///   (e.g. `"login_#{role}/:id" => "sessions#login_#{role}"`) were not
///   recognized as valid literal keys. Fixed by also allowing
///   InterpolatedStringNode and InterpolatedSymbolNode in the key type check.
pub struct MatchRoute;

/// Check if a symbol's unescaped bytes match a known HTTP method.
/// Returns the method name as a static str, or None.
fn http_method_from_symbol(sym: &ruby_prism::SymbolNode<'_>) -> Option<&'static str> {
    let unescaped = sym.unescaped();
    if unescaped == b"get" {
        Some("get")
    } else if unescaped == b"post" {
        Some("post")
    } else if unescaped == b"put" {
        Some("put")
    } else if unescaped == b"patch" {
        Some("patch")
    } else if unescaped == b"delete" {
        Some("delete")
    } else {
        None
    }
}

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

        // Check for `via:` option (handles both `via: :get` and `:via => :get` syntax)
        let via_value = keyword_arg_value(&call, b"via");

        let http_method = match via_value {
            None => {
                // No via option -> defaults to GET, but only if we can confirm
                // there are no dynamic arguments that might contain route options.
                // Skip when:
                // - Any argument is a variable or method call (not a literal/hash),
                //   since it could contain `via:` or other options.
                // - A keyword hash has non-symbol/non-string keys (e.g. local
                //   variable keys like `match section => redirect(...)`), which
                //   indicates dynamic route definitions RuboCop doesn't flag.
                if let Some(args) = call.arguments() {
                    for arg in args.arguments().iter() {
                        if let Some(kw) = arg.as_keyword_hash_node() {
                            // Check for non-literal keys (local variable reads, method calls)
                            for elem in kw.elements().iter() {
                                if let Some(assoc) = elem.as_assoc_node() {
                                    let key = assoc.key();
                                    if key.as_symbol_node().is_none()
                                        && key.as_string_node().is_none()
                                        && key.as_interpolated_string_node().is_none()
                                        && key.as_interpolated_symbol_node().is_none()
                                    {
                                        return;
                                    }
                                }
                            }
                        } else if arg.as_string_node().is_none()
                            && arg.as_symbol_node().is_none()
                            && arg.as_interpolated_string_node().is_none()
                        {
                            // Argument is not a literal path — could be a variable
                            // holding route options (e.g. `match url, opts`)
                            return;
                        }
                    }
                }
                "get"
            }
            Some(ref val) => {
                // via: :get (single symbol) or :via => :get (hashrocket)
                if let Some(sym) = val.as_symbol_node() {
                    if sym.unescaped() == b"all" {
                        return; // via: :all is fine
                    }
                    match http_method_from_symbol(&sym) {
                        Some(m) => m,
                        None => return,
                    }
                } else if let Some(arr) = val.as_array_node() {
                    // via: [:get] - single-element array, extract the method
                    // via: [:get, :post] - multiple methods is fine
                    let elements: Vec<_> = arr.elements().iter().collect();
                    if elements.len() == 1 {
                        if let Some(sym) = elements[0].as_symbol_node() {
                            match http_method_from_symbol(&sym) {
                                Some(m) => m,
                                None => return,
                            }
                        } else {
                            return;
                        }
                    } else {
                        return;
                    }
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
