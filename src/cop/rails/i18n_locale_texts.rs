use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{ASSOC_NODE, CALL_NODE, HASH_NODE, KEYWORD_HASH_NODE, LOCAL_VARIABLE_READ_NODE, STRING_NODE, SYMBOL_NODE};

pub struct I18nLocaleTexts;

const MSG: &str =
    "Move locale texts to the locale files in the `config/locales` directory.";

/// Check if a node is a plain string literal (not a symbol, not interpolated).
fn is_string_literal(node: &ruby_prism::Node<'_>) -> bool {
    node.as_string_node().is_some()
}

/// Extract string value from keyword argument in a KeywordHashNode or HashNode.
fn find_keyword_string_value<'a>(
    call: &ruby_prism::CallNode<'a>,
    key: &[u8],
) -> Option<ruby_prism::Node<'a>> {
    let args = call.arguments()?;
    for arg in args.arguments().iter() {
        // Check KeywordHashNode
        if let Some(kw) = arg.as_keyword_hash_node() {
            for elem in kw.elements().iter() {
                if let Some(assoc) = elem.as_assoc_node() {
                    if let Some(sym) = assoc.key().as_symbol_node() {
                        if sym.unescaped() == key {
                            return Some(assoc.value());
                        }
                    }
                }
            }
        }
        // Check HashNode
        if let Some(hash) = arg.as_hash_node() {
            for elem in hash.elements().iter() {
                if let Some(assoc) = elem.as_assoc_node() {
                    if let Some(sym) = assoc.key().as_symbol_node() {
                        if sym.unescaped() == key {
                            return Some(assoc.value());
                        }
                    }
                }
            }
        }
    }
    None
}

/// Search inside nested hash values for `:message` keys with string literal values.
/// Used for `validates :email, presence: { message: "text" }`.
fn find_message_strings_in_validation_args(
    call: &ruby_prism::CallNode<'_>,
    source: &SourceFile,
    cop: &I18nLocaleTexts,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let args = match call.arguments() {
        Some(a) => a,
        None => return diagnostics,
    };

    for arg in args.arguments().iter() {
        let kw = match arg.as_keyword_hash_node() {
            Some(k) => k,
            None => continue,
        };
        for elem in kw.elements().iter() {
            let assoc = match elem.as_assoc_node() {
                Some(a) => a,
                None => continue,
            };
            // The value might be a hash with a :message key
            let value = assoc.value();
            if let Some(hash) = value.as_hash_node() {
                for inner_elem in hash.elements().iter() {
                    if let Some(inner_assoc) = inner_elem.as_assoc_node() {
                        if let Some(sym) = inner_assoc.key().as_symbol_node() {
                            if sym.unescaped() == b"message" && is_string_literal(&inner_assoc.value()) {
                                let loc = inner_assoc.value().location();
                                let (line, column) = source.offset_to_line_col(loc.start_offset());
                                diagnostics.push(cop.diagnostic(source, line, column, MSG.to_string()));
                            }
                        }
                    }
                }
            }
        }
    }
    diagnostics
}

impl Cop for I18nLocaleTexts {
    fn name(&self) -> &'static str {
        "Rails/I18nLocaleTexts"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ASSOC_NODE, CALL_NODE, HASH_NODE, KEYWORD_HASH_NODE, LOCAL_VARIABLE_READ_NODE, STRING_NODE, SYMBOL_NODE]
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

        let method_name = call.name().as_slice();

        match method_name {
            b"validates" => {
                // Check for string message values in nested hashes
                diagnostics.extend(find_message_strings_in_validation_args(&call, source, self));
                return;
            }
            b"redirect_to" | b"redirect_back" => {
                // Check :notice and :alert keyword args for string literals
                for key in &[b"notice" as &[u8], b"alert"] {
                    if let Some(val) = find_keyword_string_value(&call, key) {
                        if is_string_literal(&val) {
                            let loc = val.location();
                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                            diagnostics.push(self.diagnostic(source, line, column, MSG.to_string()));
                        }
                    }
                }
                return;
            }
            b"mail" => {
                // Check :subject keyword arg for string literal
                if let Some(val) = find_keyword_string_value(&call, b"subject") {
                    if is_string_literal(&val) {
                        let loc = val.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        diagnostics.push(self.diagnostic(source, line, column, MSG.to_string()));
                    }
                }
            }
            _ => {}
        }

        // Check flash[:notice] = "string" or flash.now[:notice] = "string"
        // This is `[]=` call on `flash` or `flash.now`
        if method_name == b"[]=" {
            if let Some(receiver) = call.receiver() {
                let is_flash = is_flash_receiver(&receiver);
                if is_flash {
                    // The last argument is the assigned value
                    if let Some(args) = call.arguments() {
                        let arg_list: Vec<_> = args.arguments().iter().collect();
                        if arg_list.len() == 2 {
                            if is_string_literal(&arg_list[1]) {
                                let loc = arg_list[1].location();
                                let (line, column) = source.offset_to_line_col(loc.start_offset());
                                diagnostics.push(self.diagnostic(
                                    source,
                                    line,
                                    column,
                                    MSG.to_string(),
                                ));
                            }
                        }
                    }
                }
            }
        }

    }
}

/// Check if a node is `flash` or `flash.now`.
fn is_flash_receiver(node: &ruby_prism::Node<'_>) -> bool {
    // Direct `flash` call
    if let Some(call) = node.as_call_node() {
        if call.name().as_slice() == b"flash" && call.receiver().is_none() {
            return true;
        }
        // `flash.now`
        if call.name().as_slice() == b"now" {
            if let Some(recv) = call.receiver() {
                if let Some(inner_call) = recv.as_call_node() {
                    if inner_call.name().as_slice() == b"flash" && inner_call.receiver().is_none() {
                        return true;
                    }
                }
            }
        }
    }
    // Also handle local variable `flash`
    if let Some(lvar) = node.as_local_variable_read_node() {
        return lvar.name().as_slice() == b"flash";
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(I18nLocaleTexts, "cops/rails/i18n_locale_texts");
}
