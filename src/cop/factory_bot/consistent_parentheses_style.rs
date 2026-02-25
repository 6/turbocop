use ruby_prism::Visit;

use crate::cop::factory_bot::{FACTORY_BOT_METHODS, FACTORY_BOT_SPEC_INCLUDE, is_factory_call};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ConsistentParenthesesStyle;

impl Cop for ConsistentParenthesesStyle {
    fn name(&self) -> &'static str {
        "FactoryBot/ConsistentParenthesesStyle"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        FACTORY_BOT_SPEC_INCLUDE
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::cop::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let style = config.get_str("EnforcedStyle", "require_parentheses");
        let explicit_only = config.get_bool("ExplicitOnly", false);

        let mut visitor = ParenStyleVisitor {
            source,
            cop: self,
            style,
            explicit_only,
            diagnostics: Vec::new(),
            parent_is_ambiguous: false,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct ParenStyleVisitor<'s> {
    source: &'s SourceFile,
    cop: &'s ConsistentParenthesesStyle,
    style: &'s str,
    explicit_only: bool,
    diagnostics: Vec<Diagnostic>,
    parent_is_ambiguous: bool,
}

impl<'s> ParenStyleVisitor<'s> {
    fn check_factory_call(&mut self, call: &ruby_prism::CallNode<'_>) {
        let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
        if !FACTORY_BOT_METHODS.contains(&method_name) {
            return;
        }

        if !is_factory_call(call.receiver(), self.explicit_only) {
            return;
        }

        // Skip if parent is an ambiguous context (send, pair, array, and, or, if)
        if self.parent_is_ambiguous {
            return;
        }

        // Must have arguments
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        // First argument must be a symbol, string, send, or local variable
        let first_arg = &arg_list[0];
        let valid_first_arg = first_arg.as_symbol_node().is_some()
            || first_arg.as_string_node().is_some()
            || first_arg.as_call_node().is_some()
            || first_arg.as_local_variable_read_node().is_some();

        if !valid_first_arg {
            return;
        }

        // `generate` with more than 1 argument is excluded
        if method_name == "generate" && arg_list.len() > 1 {
            return;
        }

        let has_parens = call.opening_loc().is_some();

        if self.style == "require_parentheses" && !has_parens {
            let msg_loc = call.message_loc().unwrap_or(call.location());
            let (line, column) = self.source.offset_to_line_col(msg_loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Prefer method call with parentheses".to_string(),
            ));
        }

        if self.style == "omit_parentheses" && has_parens {
            let call_loc = call.location();
            let (call_line, _) = self.source.offset_to_line_col(call_loc.start_offset());
            let first_arg_loc = first_arg.location();
            let (arg_line, _) = self.source.offset_to_line_col(first_arg_loc.start_offset());

            if call_line != arg_line {
                return;
            }

            if has_value_omission_hash(&arg_list) {
                return;
            }

            let msg_loc = call.message_loc().unwrap_or(call.location());
            let (line, column) = self.source.offset_to_line_col(msg_loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Prefer method call without parentheses".to_string(),
            ));
        }
    }
}

impl<'pr> Visit<'pr> for ParenStyleVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // Check this node for factory call offense
        self.check_factory_call(node);

        // Visit receiver (not ambiguous)
        if let Some(recv) = node.receiver() {
            let saved = self.parent_is_ambiguous;
            self.parent_is_ambiguous = false;
            self.visit(&recv);
            self.parent_is_ambiguous = saved;
        }

        // Visit arguments — children here have a CallNode (send) parent = ambiguous
        if let Some(args) = node.arguments() {
            let saved = self.parent_is_ambiguous;
            self.parent_is_ambiguous = true;
            for arg in args.arguments().iter() {
                self.visit(&arg);
            }
            self.parent_is_ambiguous = saved;
        }

        // Visit block (not ambiguous — block body is independent context)
        if let Some(block) = node.block() {
            let saved = self.parent_is_ambiguous;
            self.parent_is_ambiguous = false;
            self.visit(&block);
            self.parent_is_ambiguous = saved;
        }
    }

    fn visit_array_node(&mut self, node: &ruby_prism::ArrayNode<'pr>) {
        let saved = self.parent_is_ambiguous;
        self.parent_is_ambiguous = true;
        ruby_prism::visit_array_node(self, node);
        self.parent_is_ambiguous = saved;
    }

    fn visit_assoc_node(&mut self, node: &ruby_prism::AssocNode<'pr>) {
        let saved = self.parent_is_ambiguous;
        self.parent_is_ambiguous = true;
        ruby_prism::visit_assoc_node(self, node);
        self.parent_is_ambiguous = saved;
    }

    fn visit_and_node(&mut self, node: &ruby_prism::AndNode<'pr>) {
        let saved = self.parent_is_ambiguous;
        self.parent_is_ambiguous = true;
        ruby_prism::visit_and_node(self, node);
        self.parent_is_ambiguous = saved;
    }

    fn visit_or_node(&mut self, node: &ruby_prism::OrNode<'pr>) {
        let saved = self.parent_is_ambiguous;
        self.parent_is_ambiguous = true;
        ruby_prism::visit_or_node(self, node);
        self.parent_is_ambiguous = saved;
    }

    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        let saved = self.parent_is_ambiguous;
        self.parent_is_ambiguous = true;
        ruby_prism::visit_if_node(self, node);
        self.parent_is_ambiguous = saved;
    }
}

/// Check if any argument is a hash with value omission (Ruby 3.1+ `name:` syntax).
fn has_value_omission_hash(args: &[ruby_prism::Node<'_>]) -> bool {
    for arg in args {
        if let Some(hash) = arg.as_keyword_hash_node() {
            for elem in hash.elements().iter() {
                if let Some(pair) = elem.as_assoc_node() {
                    if pair.value().as_implicit_node().is_some() {
                        return true;
                    }
                }
            }
        }
        if let Some(hash) = arg.as_hash_node() {
            for elem in hash.elements().iter() {
                if let Some(pair) = elem.as_assoc_node() {
                    if pair.value().as_implicit_node().is_some() {
                        return true;
                    }
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        ConsistentParenthesesStyle,
        "cops/factorybot/consistent_parentheses_style"
    );
}
