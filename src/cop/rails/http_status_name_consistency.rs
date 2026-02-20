use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{ASSOC_NODE, CALL_NODE, ELSE_NODE, HASH_NODE, IF_NODE, KEYWORD_HASH_NODE, SYMBOL_NODE, UNLESS_NODE};

pub struct HttpStatusNameConsistency;

/// Deprecated HTTP status names and their preferred replacements (Rack >= 3.1).
const PREFERRED_STATUSES: &[(&[u8], &str)] = &[
    (b"unprocessable_entity", "unprocessable_content"),
    (b"payload_too_large", "content_too_large"),
];

impl Cop for HttpStatusNameConsistency {
    fn name(&self) -> &'static str {
        "Rails/HttpStatusNameConsistency"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ASSOC_NODE, CALL_NODE, ELSE_NODE, HASH_NODE, IF_NODE, KEYWORD_HASH_NODE, SYMBOL_NODE, UNLESS_NODE]
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

        let method = call.name().as_slice();
        // RESTRICT_ON_SEND = %i[render redirect_to head assert_response assert_redirected_to]
        if !matches!(
            method,
            b"render" | b"redirect_to" | b"head" | b"assert_response" | b"assert_redirected_to"
        ) {
            return;
        }

        // Must be receiverless
        if call.receiver().is_some() {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        // Look for deprecated status symbols in arguments
        for arg in args.arguments().iter() {
            self.check_for_deprecated_status(source, &arg, diagnostics);
        }

    }
}

impl HttpStatusNameConsistency {
    fn check_for_deprecated_status(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Check symbol nodes
        if let Some(sym) = node.as_symbol_node() {
            let name = sym.unescaped();
            for &(deprecated, preferred) in PREFERRED_STATUSES {
                if AsRef::<[u8]>::as_ref(&*name) == deprecated {
                    let loc = node.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        format!(
                            "Prefer `:{preferred}` over `:{}`.",
                            String::from_utf8_lossy(deprecated)
                        ),
                    ));
                    return;
                }
            }
        }

        // Check hash nodes for `status: :deprecated_name`
        if let Some(hash) = node.as_hash_node() {
            for element in hash.elements().iter() {
                if let Some(pair) = element.as_assoc_node() {
                    if let Some(key_sym) = pair.key().as_symbol_node() {
                        if AsRef::<[u8]>::as_ref(&*key_sym.unescaped()) == b"status" {
                            self.check_for_deprecated_status(source, &pair.value(), diagnostics);
                        }
                    }
                }
            }
        }

        // Check keyword hash nodes (inline keyword args)
        if let Some(hash) = node.as_keyword_hash_node() {
            for element in hash.elements().iter() {
                if let Some(pair) = element.as_assoc_node() {
                    if let Some(key_sym) = pair.key().as_symbol_node() {
                        if AsRef::<[u8]>::as_ref(&*key_sym.unescaped()) == b"status" {
                            self.check_for_deprecated_status(source, &pair.value(), diagnostics);
                        }
                    }
                }
            }
        }

        // Check conditional expressions (ternary: condition ? val1 : val2)
        if let Some(if_node) = node.as_if_node() {
            if let Some(stmts) = if_node.statements() {
                for stmt in stmts.body().iter() {
                    self.check_for_deprecated_status(source, &stmt, diagnostics);
                }
            }
            if let Some(subsequent) = if_node.subsequent() {
                if let Some(else_node) = subsequent.as_else_node() {
                    if let Some(stmts) = else_node.statements() {
                        for stmt in stmts.body().iter() {
                            self.check_for_deprecated_status(source, &stmt, diagnostics);
                        }
                    }
                } else if let Some(elsif_node) = subsequent.as_if_node() {
                    self.check_for_deprecated_status(source, &elsif_node.as_node(), diagnostics);
                }
            }
        }

        // Check unless expressions
        if let Some(unless_node) = node.as_unless_node() {
            if let Some(stmts) = unless_node.statements() {
                for stmt in stmts.body().iter() {
                    self.check_for_deprecated_status(source, &stmt, diagnostics);
                }
            }
            if let Some(else_clause) = unless_node.else_clause() {
                if let Some(stmts) = else_clause.statements() {
                    for stmt in stmts.body().iter() {
                        self.check_for_deprecated_status(source, &stmt, diagnostics);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        HttpStatusNameConsistency,
        "cops/rails/http_status_name_consistency"
    );
}
