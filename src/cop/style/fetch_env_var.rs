use std::collections::HashSet;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct FetchEnvVar;

impl FetchEnvVar {
    fn is_env_receiver(node: &ruby_prism::Node<'_>) -> bool {
        // Simple constant: ENV
        if node
            .as_constant_read_node()
            .map_or(false, |c| c.name().as_slice() == b"ENV")
        {
            return true;
        }
        // Qualified constant: ::ENV (constant_path_node with no parent)
        if let Some(cp) = node.as_constant_path_node() {
            if cp.parent().is_none() && cp.name().map_or(false, |n| n.as_slice() == b"ENV") {
                return true;
            }
        }
        false
    }

    fn is_env_bracket_call(node: &ruby_prism::Node<'_>) -> bool {
        if let Some(call) = node.as_call_node() {
            if call.name().as_slice() == b"[]" {
                if let Some(receiver) = call.receiver() {
                    return Self::is_env_receiver(&receiver);
                }
            }
        }
        false
    }

    /// Collect start offsets of all ENV['X'] nodes that appear inside a given
    /// subtree. Used to suppress ENV['X'] nodes that are part of an if/unless
    /// condition or the LHS of `||`.
    fn collect_env_bracket_offsets(node: &ruby_prism::Node<'_>, offsets: &mut HashSet<usize>) {
        struct Collector<'a> {
            offsets: &'a mut HashSet<usize>,
        }
        impl<'pr> Visit<'pr> for Collector<'_> {
            fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
                if node.name().as_slice() == b"[]" {
                    if let Some(receiver) = node.receiver() {
                        if FetchEnvVar::is_env_receiver(&receiver) {
                            self.offsets.insert(node.location().start_offset());
                        }
                    }
                }
                ruby_prism::visit_call_node(self, node);
            }
        }
        let mut collector = Collector { offsets };
        collector.visit(node);
    }

    /// Extract the byte ranges of ENV key arguments from ENV['X'] calls in a subtree.
    /// Returns a set of (start_offset, end_offset) tuples for the key arguments.
    fn collect_env_key_ranges(node: &ruby_prism::Node<'_>) -> HashSet<(usize, usize)> {
        struct KeyCollector {
            key_ranges: HashSet<(usize, usize)>,
        }
        impl<'pr> Visit<'pr> for KeyCollector {
            fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
                if node.name().as_slice() == b"[]" {
                    if let Some(receiver) = node.receiver() {
                        if FetchEnvVar::is_env_receiver(&receiver) {
                            if let Some(args) = node.arguments() {
                                let arg_list: Vec<_> = args.arguments().iter().collect();
                                if arg_list.len() == 1 {
                                    let loc = arg_list[0].location();
                                    self.key_ranges
                                        .insert((loc.start_offset(), loc.end_offset()));
                                }
                            }
                        }
                    }
                }
                ruby_prism::visit_call_node(self, node);
            }
        }
        let mut collector = KeyCollector {
            key_ranges: HashSet::new(),
        };
        collector.visit(node);
        collector.key_ranges
    }

    /// Check if a method name is a comparison method (==, !=, <, >, <=, >=, <=>).
    fn is_comparison_method(name: &[u8]) -> bool {
        matches!(name, b"==" | b"!=" | b"<" | b">" | b"<=" | b">=" | b"<=>")
    }
}

impl Cop for FetchEnvVar {
    fn name(&self) -> &'static str {
        "Style/FetchEnvVar"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let allowed_vars = config.get_string_array("AllowedVars");
        let default_to_nil = config.get_bool("DefaultToNil", true);

        let mut visitor = FetchEnvVarVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            allowed_vars,
            default_to_nil,
            suppressed_offsets: HashSet::new(),
            condition_key_ranges: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct FetchEnvVarVisitor<'a> {
    cop: &'a FetchEnvVar,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    allowed_vars: Option<Vec<String>>,
    default_to_nil: bool,
    /// Start offsets of ENV['X'] nodes that should NOT be reported
    /// (used as flag in if/unless condition, or LHS of `||`).
    suppressed_offsets: HashSet<usize>,
    /// ENV key byte ranges from conditions — ENV['X'] in bodies with matching
    /// keys are also suppressed (RuboCop's `used_if_condition_in_body?`).
    condition_key_ranges: Vec<(usize, usize)>,
}

impl FetchEnvVarVisitor<'_> {
    /// Suppress all ENV['X'] nodes that appear inside an if/unless condition,
    /// AND collect ENV key ranges so that body ENV['same_key'] can be suppressed.
    fn suppress_env_in_condition(&mut self, condition: &ruby_prism::Node<'_>) {
        FetchEnvVar::collect_env_bracket_offsets(condition, &mut self.suppressed_offsets);
        let key_ranges = FetchEnvVar::collect_env_key_ranges(condition);
        for range in key_ranges {
            self.condition_key_ranges.push(range);
        }
    }

    /// Check if an ENV key (given by its byte range) matches any key from
    /// an ancestor if/unless condition.
    fn key_matches_condition(&self, key_start: usize, key_end: usize) -> bool {
        let key_bytes = &self.source.as_bytes()[key_start..key_end];
        for &(cond_start, cond_end) in &self.condition_key_ranges {
            let cond_bytes = &self.source.as_bytes()[cond_start..cond_end];
            if key_bytes == cond_bytes {
                return true;
            }
        }
        false
    }
}

impl<'pr> Visit<'pr> for FetchEnvVarVisitor<'_> {
    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        // Suppress ENV['X'] used in the condition of if/elsif/ternary,
        // AND suppress ENV['same_key'] in the body (used_if_condition_in_body).
        let prev_len = self.condition_key_ranges.len();
        self.suppress_env_in_condition(&node.predicate());
        ruby_prism::visit_if_node(self, node);
        self.condition_key_ranges.truncate(prev_len);
    }

    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        // Suppress ENV['X'] used in the condition of unless,
        // AND suppress ENV['same_key'] in the body.
        let prev_len = self.condition_key_ranges.len();
        self.suppress_env_in_condition(&node.predicate());
        ruby_prism::visit_unless_node(self, node);
        self.condition_key_ranges.truncate(prev_len);
    }

    fn visit_or_node(&mut self, node: &ruby_prism::OrNode<'pr>) {
        // ENV['X'] || default — suppress ENV['X'] on the LHS of ||
        // Also suppress if this or_node is nested inside another or_node
        // (e.g., ENV['A'] || ENV['B'] || default)
        FetchEnvVar::collect_env_bracket_offsets(&node.left(), &mut self.suppressed_offsets);
        ruby_prism::visit_or_node(self, node);
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let name = node.name();
        let method_bytes = name.as_slice();

        // For comparison methods (==, !=, etc.), suppress any ENV[] in arguments.
        // RuboCop's `used_as_flag?` suppresses ENV['X'] when parent is a
        // comparison_method? call — this covers both receiver and argument positions.
        if FetchEnvVar::is_comparison_method(method_bytes) {
            if let Some(args) = node.arguments() {
                for arg in args.arguments().iter() {
                    FetchEnvVar::collect_env_bracket_offsets(&arg, &mut self.suppressed_offsets);
                }
            }
            // Also suppress ENV[] in receiver position of comparison
            if let Some(receiver) = node.receiver() {
                FetchEnvVar::collect_env_bracket_offsets(&receiver, &mut self.suppressed_offsets);
            }
        }

        if method_bytes == b"[]" {
            let receiver = match node.receiver() {
                Some(r) => r,
                None => {
                    ruby_prism::visit_call_node(self, node);
                    return;
                }
            };

            if !FetchEnvVar::is_env_receiver(&receiver) {
                ruby_prism::visit_call_node(self, node);
                return;
            }

            // Check if this ENV['X'] is suppressed (used as flag or LHS of ||)
            if self
                .suppressed_offsets
                .contains(&node.location().start_offset())
            {
                return;
            }

            let args = match node.arguments() {
                Some(a) => a,
                None => {
                    ruby_prism::visit_call_node(self, node);
                    return;
                }
            };

            let arg_list: Vec<_> = args.arguments().iter().collect();
            if arg_list.len() != 1 {
                ruby_prism::visit_call_node(self, node);
                return;
            }

            let arg_loc = arg_list[0].location();

            // Check if this ENV key matches a condition key (body-in-condition suppression)
            if self.key_matches_condition(arg_loc.start_offset(), arg_loc.end_offset()) {
                return;
            }

            let arg_src = &self.source.as_bytes()[arg_loc.start_offset()..arg_loc.end_offset()];
            let arg_str = String::from_utf8_lossy(arg_src);

            // Check AllowedVars
            if let Some(ref allowed) = self.allowed_vars {
                let var_name = arg_str.trim_matches('\'').trim_matches('"');
                if allowed.iter().any(|v| v == var_name) {
                    ruby_prism::visit_call_node(self, node);
                    return;
                }
            }

            let loc = node.location();
            let call_src = &self.source.as_bytes()[loc.start_offset()..loc.end_offset()];
            let call_str = String::from_utf8_lossy(call_src);

            let replacement = if self.default_to_nil {
                format!("ENV.fetch({}, nil)", arg_str)
            } else {
                format!("ENV.fetch({})", arg_str)
            };

            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                format!("Use `{}` instead of `{}`.", replacement, call_str),
            ));

            // Don't recurse into this node (we already processed it)
            return;
        }

        // For non-[] calls, check if their receiver is ENV['X'].
        // If so, the ENV['X'] should NOT be flagged (it receives a message).
        if let Some(receiver) = node.receiver() {
            if FetchEnvVar::is_env_bracket_call(&receiver) {
                // Skip visiting the receiver — we handled the suppression by
                // NOT recursing into it.
                // Visit arguments and block only.
                if let Some(args) = node.arguments() {
                    self.visit(&args.as_node());
                }
                if let Some(block) = node.block() {
                    self.visit(&block);
                }
                return;
            }
        }

        ruby_prism::visit_call_node(self, node);
    }

    fn visit_call_operator_write_node(&mut self, node: &ruby_prism::CallOperatorWriteNode<'pr>) {
        ruby_prism::visit_call_operator_write_node(self, node);
    }

    fn visit_call_or_write_node(&mut self, node: &ruby_prism::CallOrWriteNode<'pr>) {
        // ENV['X'] ||= y  — don't flag it.
        if let Some(receiver) = node.receiver() {
            if FetchEnvVar::is_env_receiver(&receiver) {
                self.visit(&node.value());
                return;
            }
        }
        ruby_prism::visit_call_or_write_node(self, node);
    }

    fn visit_call_and_write_node(&mut self, node: &ruby_prism::CallAndWriteNode<'pr>) {
        // ENV['X'] &&= y  — don't flag it.
        if let Some(receiver) = node.receiver() {
            if FetchEnvVar::is_env_receiver(&receiver) {
                self.visit(&node.value());
                return;
            }
        }
        ruby_prism::visit_call_and_write_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FetchEnvVar, "cops/style/fetch_env_var");
}
