use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// ## Corpus investigation (2026-03-07)
///
/// FP=312, FN=59. Biggest FP source: `p { "..." }` in Phlex/Markaby views where
/// `p` is an HTML `<p>` tag builder, not Kernel#p. RuboCop skips calls with blocks
/// (`node.block_node`), block_pass args, and hash args. Fixed by adding block and
/// argument type checks.
///
/// ## Corpus investigation (2026-03-10)
///
/// FP=12, FN=59. Root causes:
/// - FP: Missing `node.parent&.call_type?` check — `p.do_something` was flagged
///   because `p` parses as a receiverless CallNode whose parent is another CallNode.
///   RuboCop skips this. Fixed with visitor-based `parent_is_call` tracking.
/// - FN: Missing methods `ap` and `pretty_print` from RESTRICT_ON_SEND. Fixed.
/// - FN: Missing IO output detection — `$stdout.write`, `$stderr.syswrite`,
///   `STDOUT.binwrite`, `STDERR.write_nonblock`, `::STDOUT.write`, `::STDERR.write`.
///   RuboCop matches `(send (gvar $stdout/$stderr) ...)` and
///   `(send (const nil?/cbase STDOUT/STDERR) ...)` with IO write methods. Fixed.
/// - FN: Location mismatch — was reporting at `node.location()` (full call expression)
///   but RuboCop reports at `node.loc.selector` (method name only) for receiverless
///   calls, and receiver start to selector end for receivered IO calls. Fixed using
///   `message_loc()` and range calculation.
/// - Message updated to match RuboCop: "Use Rails's logger if you want to log."
///
/// ## Corpus investigation (2026-03-15)
///
/// FP=0, FN=65. Root cause: `parent_is_call` flag leaked into block bodies
/// when the block's call was itself an argument of another call.
/// E.g. `bar(foo { puts "hello" })` or `formatter = proc { msg.tap { puts msg } }`.
/// When `foo` was visited as bar's argument, `parent_is_call` was set to true.
/// The block of `foo` restored `was` (which was true), so `puts` inside the block
/// incorrectly had `parent_is_call = true` and was skipped. Fixed by always
/// resetting `parent_is_call = false` when entering block bodies.
///
/// ## Corpus investigation (2026-03-16)
///
/// FP=0, FN=16. Root cause: `parent_is_call` flag leaked through non-CallNode
/// argument children into their nested code. When a CallNode's argument was a
/// LambdaNode, RescueModifierNode, or DefNode, the default visitor recursed into
/// their bodies without resetting the flag. E.g. `config.x = ->(w) { puts w }`
/// had `parent_is_call = true` when visiting `puts` inside the lambda body.
/// Fixed by adding `visit_lambda_node`, `visit_def_node`, and
/// `visit_rescue_modifier_node` overrides that reset `parent_is_call = false`
/// before recursing, matching the pattern already used in `rspec/output.rs`.
///
/// ## Corpus investigation (2026-03-19)
///
/// FP=15, FN=7. All 15 FPs are from vendored gem paths (heroku/ruby/1.9.1/gems/
/// in cjstewart88/Tubalr and vendor/bundle/ in liaoziyang/stackneveroverflow).
/// These are file-exclusion issues — RuboCop's AllCops.Exclude covers these
/// paths but nitrocop's path resolution doesn't relativize correctly.
/// The cop logic is correct; no code fix needed for FPs.
///
/// ## Corpus investigation (2026-03-23)
///
/// FP=15, FN=7 (extended). FPs confirmed as vendor-path exclusion issues (not cop logic).
/// FN root cause: `parent_is_call` flag leaked through non-CallNode intermediate nodes
/// that appeared as call arguments. RuboCop checks `node.parent&.call_type?` which only
/// matches when the DIRECT parent is a call node. Intermediate nodes like SplatNode,
/// CaseNode, IfNode, BeginNode, InterpolatedStringNode, and ParenthesesNode need to
/// reset the flag so nested output calls are detected. Corpus FN patterns:
/// - `*p` as splat argument: SplatNode wraps the `p` call
/// - `puts` inside `case/else` branch that is a call argument: CaseNode wraps branches
/// - `pp(act)` inside `#{}` interpolation in a call argument string
/// - `puts` inside `if/else` branch that is a call argument
pub struct Output;

const MSG: &str = "Do not write to stdout. Use Rails's logger if you want to log.";

/// Receiverless output methods (Kernel#p, Kernel#puts, etc.)
const OUTPUT_METHODS: &[&[u8]] = &[b"ap", b"p", b"pp", b"pretty_print", b"print", b"puts"];

/// IO write methods called on $stdout/$stderr/STDOUT/STDERR
const IO_WRITE_METHODS: &[&[u8]] = &[b"binwrite", b"syswrite", b"write", b"write_nonblock"];

/// Global variables that are IO targets
const GLOBAL_VARS: &[&[u8]] = &[b"$stdout", b"$stderr"];

/// Constants that are IO targets
const CONST_NAMES: &[&[u8]] = &[b"STDOUT", b"STDERR"];

impl Cop for Output {
    fn name(&self) -> &'static str {
        "Rails/Output"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        &[
            "**/app/**/*.rb",
            "**/config/**/*.rb",
            "db/**/*.rb",
            "**/lib/**/*.rb",
        ]
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[] // Using check_source with visitor instead
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
    /// Matches RuboCop's `return if node.parent&.call_type?`.
    parent_is_call: bool,
}

impl<'pr> Visit<'pr> for OutputVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let method = node.name().as_slice();

        // Only check when parent is NOT a call node
        // (matches RuboCop: `return if node.parent&.call_type?`)
        if !self.parent_is_call {
            if OUTPUT_METHODS.contains(&method) && node.receiver().is_none() {
                // Receiverless output call (e.g., `puts "hello"`, `p value`)
                if node.block().is_none() && !has_hash_or_block_pass_args(node) {
                    // Report at message_loc (method name only), matching RuboCop's node.loc.selector
                    if let Some(msg_loc) = node.message_loc() {
                        let (line, column) = self.source.offset_to_line_col(msg_loc.start_offset());
                        self.diagnostics.push(self.cop.diagnostic(
                            self.source,
                            line,
                            column,
                            MSG.to_string(),
                        ));
                    }
                }
            } else if IO_WRITE_METHODS.contains(&method) {
                // IO output call (e.g., `$stdout.write "data"`, `STDOUT.binwrite "data"`)
                if let Some(recv) = node.receiver() {
                    let is_io_target = if let Some(gv) = recv.as_global_variable_read_node() {
                        GLOBAL_VARS.contains(&gv.name().as_slice())
                    } else if let Some(c) = recv.as_constant_read_node() {
                        CONST_NAMES.contains(&c.name().as_slice())
                    } else if let Some(cp) = recv.as_constant_path_node() {
                        // ::STDOUT or ::STDERR (ConstantPathNode with no parent, i.e. cbase)
                        cp.parent().is_none()
                            && cp.name().is_some()
                            && CONST_NAMES.contains(&cp.name().unwrap().as_slice())
                    } else {
                        false
                    };

                    if is_io_target && node.block().is_none() {
                        // Report from receiver start (matches RuboCop's
                        // range_between(node.source_range.begin_pos, node.loc.selector.end_pos))
                        let recv_loc = recv.location();
                        let (line, column) =
                            self.source.offset_to_line_col(recv_loc.start_offset());
                        self.diagnostics.push(self.cop.diagnostic(
                            self.source,
                            line,
                            column,
                            MSG.to_string(),
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
            for arg in args.arguments().iter() {
                self.visit(&arg);
            }
        }
        self.parent_is_call = was;
        // Visit block body with parent_is_call = false (blocks are not "parent call" context).
        // Must explicitly set to false rather than relying on `was`, because `was` could be
        // `true` when this call is itself an argument of another call (e.g. `bar(foo { puts })`).
        if let Some(block) = node.block() {
            let was_for_block = self.parent_is_call;
            self.parent_is_call = false;
            self.visit(&block);
            self.parent_is_call = was_for_block;
        }
    }

    // Lambda bodies are new statement scopes — reset parent_is_call so that
    // output calls inside `config.x = ->(w) { puts w }` are not suppressed by the
    // outer assignment call treating the lambda as its argument.
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

    // Rescue modifier expressions are new contexts — reset parent_is_call so
    // that `expr rescue (puts "msg")` detects the puts even when the rescue
    // expression is an argument of a call.
    fn visit_rescue_modifier_node(&mut self, node: &ruby_prism::RescueModifierNode<'pr>) {
        let was = self.parent_is_call;
        self.parent_is_call = false;
        ruby_prism::visit_rescue_modifier_node(self, node);
        self.parent_is_call = was;
    }

    // Array literals reset parent_is_call so that output calls inside nested arrays
    // (which are arguments of calls) are properly detected. Without this, `p` in
    //   [["key", p], ...].map { ... }
    // would be suppressed because `parent_is_call` is set when visiting the array as the
    // receiver of `.map`, and the flag propagates into nested array elements.
    //
    // RuboCop's check `node.parent&.call_type?` checks the DIRECT parent, so `p` inside
    // an array literal is NOT considered "parent is call" (array is parent, not the call).
    fn visit_array_node(&mut self, node: &ruby_prism::ArrayNode<'pr>) {
        let was = self.parent_is_call;
        self.parent_is_call = false;
        ruby_prism::visit_array_node(self, node);
        self.parent_is_call = was;
    }

    // Hash literals similarly reset parent_is_call so output calls inside hash values
    // (passed as arguments to calls) are detected.
    fn visit_hash_node(&mut self, node: &ruby_prism::HashNode<'pr>) {
        let was = self.parent_is_call;
        self.parent_is_call = false;
        ruby_prism::visit_hash_node(self, node);
        self.parent_is_call = was;
    }

    // Splat nodes reset parent_is_call so that `*p` inside a call argument
    // detects the `p` call (RuboCop's parent of `p` is the splat, not the call).
    fn visit_splat_node(&mut self, node: &ruby_prism::SplatNode<'pr>) {
        let was = self.parent_is_call;
        self.parent_is_call = false;
        ruby_prism::visit_splat_node(self, node);
        self.parent_is_call = was;
    }

    // Case expressions reset parent_is_call so that output calls inside
    // when/else branches are detected even when the case is a call argument.
    fn visit_case_node(&mut self, node: &ruby_prism::CaseNode<'pr>) {
        let was = self.parent_is_call;
        self.parent_is_call = false;
        ruby_prism::visit_case_node(self, node);
        self.parent_is_call = was;
    }

    // If/unless expressions reset parent_is_call so that output calls inside
    // branches are detected even when the if is a call argument.
    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        let was = self.parent_is_call;
        self.parent_is_call = false;
        ruby_prism::visit_if_node(self, node);
        self.parent_is_call = was;
    }

    // Begin/rescue blocks reset parent_is_call so that output calls inside
    // begin blocks that are call arguments are detected.
    fn visit_begin_node(&mut self, node: &ruby_prism::BeginNode<'pr>) {
        let was = self.parent_is_call;
        self.parent_is_call = false;
        ruby_prism::visit_begin_node(self, node);
        self.parent_is_call = was;
    }

    // Interpolated strings reset parent_is_call so that output calls inside
    // string interpolation (e.g., `"#{pp(x)}"`) are detected even when the
    // string is a call argument.
    fn visit_interpolated_string_node(&mut self, node: &ruby_prism::InterpolatedStringNode<'pr>) {
        let was = self.parent_is_call;
        self.parent_is_call = false;
        ruby_prism::visit_interpolated_string_node(self, node);
        self.parent_is_call = was;
    }

    // Parentheses reset parent_is_call so that output calls inside
    // parenthesized expressions that are call arguments are detected.
    fn visit_parentheses_node(&mut self, node: &ruby_prism::ParenthesesNode<'pr>) {
        let was = self.parent_is_call;
        self.parent_is_call = false;
        ruby_prism::visit_parentheses_node(self, node);
        self.parent_is_call = was;
    }
}

/// Check if any argument is a hash (keyword args) or block_pass
fn has_hash_or_block_pass_args(node: &ruby_prism::CallNode<'_>) -> bool {
    if let Some(args) = node.arguments() {
        for arg in args.arguments().iter() {
            if arg.as_hash_node().is_some() || arg.as_keyword_hash_node().is_some() {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;
    crate::cop_fixture_tests!(Output, "cops/rails/output");

    #[test]
    fn p_inside_array_inside_method_chain_fires() {
        // p inside an array inside a method chain receiver should fire
        // (matches ransack/nodes/condition.rb corpus FN pattern)
        let source = b"data = [['pred', p], ['attr', a]].map { |k, v| k }.join\n";
        let diags = run_cop_full(&Output, source);
        assert_eq!(
            diags.len(),
            1,
            "p inside nested array inside method chain should fire: {:?}",
            diags
        );
    }
}
