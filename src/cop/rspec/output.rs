use crate::cop::shared::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// RSpec/Output: flags output calls (p, puts, print, pp, ap, pretty_print,
/// $stdout.write, etc.) in specs.
///
/// Investigation: 81 FPs caused by flagging `p(...)` when used as a method argument
/// (e.g., `expect(p("abc/").normalized_pattern)`) or as a receiver of a chained call
/// (e.g., `p.trigger`). RuboCop checks `node.parent&.call_type?` and skips when the
/// output call's parent is another call node. Switched from check_node to check_source
/// with a visitor that tracks parent-is-call context. This matches the RuboCop behavior
/// at vendor/rubocop-rspec/lib/rubocop/cop/rspec/output.rb:61.
///
/// Fix (110 FN): Added missing kernel methods `ap` and `pretty_print`, missing IO
/// method `write_nonblock`, and applied hash/block_pass argument skip to ALL kernel
/// methods (was previously only applied to `p`).
///
/// Fix (105 FN): The `parent_is_call` flag was not being reset when entering
/// scope-introducing nodes (LambdaNode, DefNode, block bodies). This caused
/// output calls like `p x` inside `-> { p x }.should(...)` or `Proc.new { puts "x" }.call`
/// to be suppressed because the lambda/block was the receiver of a call, and the
/// `parent_is_call = true` flag propagated through intermediate non-CallNode visitors
/// into the nested output call. Fixed by: (1) resetting `parent_is_call = false` when
/// visiting block bodies of CallNodes, (2) overriding `visit_lambda_node` and
/// `visit_def_node` to reset the flag. This matches RuboCop's `node.parent&.call_type?`
/// which only checks the immediate parent, not transitive ancestors.
///
/// Fix (2 FP, 11 FN): Two issues: (1) Hash argument skip only checked
/// `KeywordHashNode` but not `HashNode` — `ap({ a: 1 })` has a `HashNode` arg.
/// Added `as_hash_node()` to the skip condition. (2) `parent_is_call` propagated
/// through intermediate container nodes (Array, Hash, Begin, Parentheses, etc.)
/// to nested calls. E.g., `match_array [p, c]` — `p` is inside an ArrayNode
/// argument, but `p`'s parent is Array not Call, so RuboCop flags it. Added
/// visitor overrides for container nodes to reset `parent_is_call = false`.
pub struct Output;

/// Output methods without a receiver (Kernel print methods)
const PRINT_METHODS: &[&[u8]] = &[b"ap", b"p", b"pp", b"pretty_print", b"print", b"puts"];

/// IO write methods called on $stdout, $stderr, STDOUT, STDERR
const IO_WRITE_METHODS: &[&[u8]] = &[b"binwrite", b"syswrite", b"write", b"write_nonblock"];

/// Global variable names for stdout/stderr
const GLOBAL_VARS: &[&[u8]] = &[b"$stdout", b"$stderr"];

/// Constant names for stdout/stderr
const CONST_NAMES: &[&[u8]] = &[b"STDOUT", b"STDERR"];

impl Cop for Output {
    fn name(&self) -> &'static str {
        "RSpec/Output"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = OutputVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            parent_is_call: false,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct OutputVisitor<'a> {
    cop: &'a Output,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    /// True when the current node is a direct child (receiver/argument) of a CallNode.
    parent_is_call: bool,
}

impl<'pr> Visit<'pr> for OutputVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let method = node.name().as_slice();

        // Check for output calls only when parent is NOT a call
        // (matches RuboCop: `return if node.parent&.call_type?`)
        if !self.parent_is_call {
            if PRINT_METHODS.contains(&method) && node.receiver().is_none() {
                // Skip if it has a block (p { ... } is DSL usage like phlex)
                if node.block().is_none() {
                    // Skip if any argument is a hash or block_pass
                    // (matches RuboCop: `node.arguments.any? { |arg| arg.type?(:hash, :block_pass) }`)
                    let mut skip = false;
                    if let Some(args) = node.arguments() {
                        for arg in args.arguments().iter() {
                            if arg.as_hash_node().is_some()
                                || arg.as_keyword_hash_node().is_some()
                                || arg.as_block_argument_node().is_some()
                            {
                                skip = true;
                                break;
                            }
                        }
                    }
                    if !skip {
                        let loc = node.location();
                        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                        self.diagnostics.push(self.cop.diagnostic(
                            self.source,
                            line,
                            column,
                            "Do not write to stdout in specs.".to_string(),
                        ));
                    }
                }
            } else if IO_WRITE_METHODS.contains(&method) {
                if let Some(recv) = node.receiver() {
                    let is_io_target = if let Some(gv) = recv.as_global_variable_read_node() {
                        GLOBAL_VARS.contains(&gv.name().as_slice())
                    } else if let Some(c) = recv.as_constant_read_node() {
                        CONST_NAMES.contains(&c.name().as_slice())
                    } else if let Some(cp) = recv.as_constant_path_node() {
                        cp.parent().is_none()
                            && cp.name().is_some()
                            && CONST_NAMES.contains(&cp.name().unwrap().as_slice())
                    } else {
                        false
                    };

                    if is_io_target && node.block().is_none() {
                        let loc = node.location();
                        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                        self.diagnostics.push(self.cop.diagnostic(
                            self.source,
                            line,
                            column,
                            "Do not write to stdout in specs.".to_string(),
                        ));
                    }
                }
            }
        }

        // Visit children with parent_is_call = true for receiver/arguments,
        // but preserve default visiting for the block body
        let was = self.parent_is_call;
        self.parent_is_call = true;
        if let Some(recv) = node.receiver() {
            self.visit(&recv);
        }
        if let Some(args) = node.arguments() {
            self.visit_arguments_node(&args);
        }
        self.parent_is_call = was;

        // Visit block with parent_is_call = false: a block body is a new
        // statement scope, not a call argument. Without this reset,
        // `Proc.new { puts "x" }.call` would suppress the offense because
        // the Proc.new block inherits parent_is_call from `.call`'s receiver.
        if let Some(block) = node.block() {
            let was_block = self.parent_is_call;
            self.parent_is_call = false;
            self.visit(&block);
            self.parent_is_call = was_block;
        }
    }

    // Lambda bodies are new statement scopes — reset parent_is_call so that
    // output calls inside `-> { p(o) }.should(...)` are not suppressed by the
    // outer `.should` call treating the lambda as its receiver.
    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        let was = self.parent_is_call;
        self.parent_is_call = false;
        ruby_prism::visit_lambda_node(self, node);
        self.parent_is_call = was;
    }

    // Method definition bodies are new scopes — reset parent_is_call so that
    // output calls inside `def foo; puts "x"; end` are detected even when the
    // def node is nested inside a call's receiver/arguments.
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        let was = self.parent_is_call;
        self.parent_is_call = false;
        ruby_prism::visit_def_node(self, node);
        self.parent_is_call = was;
    }

    // Intermediate container nodes reset parent_is_call so that output calls
    // nested inside arrays, hashes, begin/rescue, etc. are not suppressed.
    // RuboCop checks `node.parent&.call_type?` (immediate parent only), so
    // `[p, 42]` as an argument should still flag `p` because p's parent is
    // ArrayNode, not CallNode.

    fn visit_array_node(&mut self, node: &ruby_prism::ArrayNode<'pr>) {
        let was = self.parent_is_call;
        self.parent_is_call = false;
        ruby_prism::visit_array_node(self, node);
        self.parent_is_call = was;
    }

    fn visit_hash_node(&mut self, node: &ruby_prism::HashNode<'pr>) {
        let was = self.parent_is_call;
        self.parent_is_call = false;
        ruby_prism::visit_hash_node(self, node);
        self.parent_is_call = was;
    }

    fn visit_keyword_hash_node(&mut self, node: &ruby_prism::KeywordHashNode<'pr>) {
        let was = self.parent_is_call;
        self.parent_is_call = false;
        ruby_prism::visit_keyword_hash_node(self, node);
        self.parent_is_call = was;
    }

    fn visit_begin_node(&mut self, node: &ruby_prism::BeginNode<'pr>) {
        let was = self.parent_is_call;
        self.parent_is_call = false;
        ruby_prism::visit_begin_node(self, node);
        self.parent_is_call = was;
    }

    fn visit_parentheses_node(&mut self, node: &ruby_prism::ParenthesesNode<'pr>) {
        let was = self.parent_is_call;
        self.parent_is_call = false;
        ruby_prism::visit_parentheses_node(self, node);
        self.parent_is_call = was;
    }

    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        let was = self.parent_is_call;
        self.parent_is_call = false;
        ruby_prism::visit_if_node(self, node);
        self.parent_is_call = was;
    }

    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        let was = self.parent_is_call;
        self.parent_is_call = false;
        ruby_prism::visit_unless_node(self, node);
        self.parent_is_call = was;
    }

    fn visit_case_node(&mut self, node: &ruby_prism::CaseNode<'pr>) {
        let was = self.parent_is_call;
        self.parent_is_call = false;
        ruby_prism::visit_case_node(self, node);
        self.parent_is_call = was;
    }

    fn visit_rescue_node(&mut self, node: &ruby_prism::RescueNode<'pr>) {
        let was = self.parent_is_call;
        self.parent_is_call = false;
        ruby_prism::visit_rescue_node(self, node);
        self.parent_is_call = was;
    }

    fn visit_ensure_node(&mut self, node: &ruby_prism::EnsureNode<'pr>) {
        let was = self.parent_is_call;
        self.parent_is_call = false;
        ruby_prism::visit_ensure_node(self, node);
        self.parent_is_call = was;
    }

    fn visit_interpolated_string_node(&mut self, node: &ruby_prism::InterpolatedStringNode<'pr>) {
        let was = self.parent_is_call;
        self.parent_is_call = false;
        ruby_prism::visit_interpolated_string_node(self, node);
        self.parent_is_call = was;
    }

    fn visit_interpolated_symbol_node(&mut self, node: &ruby_prism::InterpolatedSymbolNode<'pr>) {
        let was = self.parent_is_call;
        self.parent_is_call = false;
        ruby_prism::visit_interpolated_symbol_node(self, node);
        self.parent_is_call = was;
    }

    fn visit_embedded_statements_node(&mut self, node: &ruby_prism::EmbeddedStatementsNode<'pr>) {
        let was = self.parent_is_call;
        self.parent_is_call = false;
        ruby_prism::visit_embedded_statements_node(self, node);
        self.parent_is_call = was;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Output, "cops/rspec/output");
}
