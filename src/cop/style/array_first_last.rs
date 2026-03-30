use ruby_prism::Visit;
use std::path::{Component, Path};

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Corpus investigation (2026-03-23):
///
/// FP=23: All false positives were `arr[0]` used as an argument inside
/// `IndexOrWriteNode` (`h[arr[0]] ||= val`), `IndexAndWriteNode`
/// (`h[arr[0]] &&= val`), or `IndexOperatorWriteNode` (`h[arr[0]] += val`).
/// These Prism node types represent compound assignment on indexed access
/// and are equivalent to `[]=` in RuboCop's Parser-gem AST. The visitor was
/// only suppressing `[]` call arguments when the parent was a `CallNode`
/// with `[]`/`[]=`, missing these index-write node types.
///
/// FN=138: Most false negatives were `arr[0] += val`, `arr[-1] ||= default`,
/// etc. In Prism these are `IndexOperatorWriteNode`/`IndexOrWriteNode`/
/// `IndexAndWriteNode` — NOT `CallNode`. The visitor only handled `CallNode`,
/// so these patterns were never checked. Also missing: explicit method call
/// syntax `arr.[](0)` and safe-navigation `arr&.[](0)`.
///
/// Fix: Added `visit_index_or_write_node`, `visit_index_and_write_node`, and
/// `visit_index_operator_write_node` to (a) suppress `[]` calls in arguments
/// and (b) check the node's own index for `0`/`-1` offenses.
///
/// Corpus investigation (2026-03-24):
///
/// FP=5: Remaining false positives were `arr[0]` used as the RECEIVER of
/// index-write nodes, e.g. `remaining_fragments[0][:from_page] ||= val`
/// or `values[0][1] += value`. The index-write node acts as `[]=`, so
/// `[0]` is a child of a bracket operation and should be suppressed —
/// matching RuboCop's `brace_method?(parent)` check.
///
/// Fix: Added `suppress_index_write_receiver()` to suppress the receiver
/// of IndexOrWriteNode / IndexAndWriteNode / IndexOperatorWriteNode when
/// it is a `[]` call.
///
/// FN=67: Could not reproduce with available patterns. All tested patterns
/// (basic `arr[0]`, explicit `.[]()`, safe-nav `&.[]()`, space-syntax
/// `.[] 0`, multiline, method args, method chains) are correctly detected.
/// Remaining FNs likely involve project-specific edge cases in the corpus.
///
/// Corpus investigation (2026-03-27):
///
/// FN=105: Chained send expressions such as `result[0].content[0][:text]`,
/// `doc.blocks[0].rows.body[0][0]`, and `tokentype[0].split(":")[1]`
/// were still missed. The suppression set keyed off `call.location()`
/// start offsets, but in Prism every send in a chain starts at the
/// receiver's beginning. That made inner `[]` calls share the same key
/// as earlier valid `[]` calls, so suppressing the inner index also
/// suppressed the offense we actually needed to report.
///
/// Fix: Track suppression by the `[]` selector/message start instead of
/// the whole call span start. This makes each bracket call in a chain
/// unique and matches RuboCop's per-send behavior.
///
/// FP=2 in the sampled corpus gate: after fixing the chained-send misses,
/// nitrocop started reporting offenses in `.github/workflows/scripts/*.rb`
/// files inside `newrelic-ruby-agent`. RuboCop repo-root scans skip files
/// under hidden directories, but it still inspects root dotfiles like
/// `.watchr` and hidden basenames in visible directories like `.toys.rb`.
/// The earlier stopgap skipped any hidden path component, which hid real
/// offenses in those files. Fix: only skip files that live under a hidden
/// directory component, not files whose basename happens to start with `.`.
///
/// Corpus investigation (2026-03-30):
///
/// FN=29: Remaining misses were non-decimal integer literals like
/// `cart_4K[0x0000]`. The cop reparsed `IntegerNode` source text with
/// `parse::<i64>()`, which only matched plain decimal spellings even though
/// RuboCop keys off the normalized integer value. Fix: compare Prism's parsed
/// integer value against zero and negative one so `0`, `00`, `0x0000`, and
/// similar forms behave like RuboCop.
///
/// Corpus investigation (2026-03-30):
///
/// FN=19: Remaining misses were `[0]` calls used as the receiver or argument
/// of safe-navigation explicit bracket sends such as
/// `requirements[0]&.[](:requirement)`. RuboCop only suppresses offenses when
/// the parent is a regular `[]`/`[]=` send; a `&.[]` parent is a `csend`, so
/// the inner `[0]` should still be flagged. Fix: only suppress nested bracket
/// calls for regular bracket sends, not safe-navigation bracket sends.
pub struct ArrayFirstLast;

impl Cop for ArrayFirstLast {
    fn name(&self) -> &'static str {
        "Style/ArrayFirstLast"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if path_has_hidden_directory(&source.path) {
            return;
        }

        let mut visitor = ArrayFirstLastVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            suppressed_offsets: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct ArrayFirstLastVisitor<'a> {
    cop: &'a ArrayFirstLast,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    /// Selector/message start offsets of `[]` call nodes that should NOT be
    /// flagged because they are a direct child (receiver or argument) of
    /// another `[]/[]=` call. We key off the selector, not `call.location()`,
    /// because Prism gives chained sends the same overall start offset.
    suppressed_offsets: Vec<usize>,
}

/// Check if a call node is a `[]` method call.
fn is_bracket_call(call: &ruby_prism::CallNode<'_>) -> bool {
    call.name().as_slice() == b"[]"
}

fn bracket_call_offset(call: &ruby_prism::CallNode<'_>) -> usize {
    call.message_loc().unwrap_or(call.location()).start_offset()
}

fn is_safe_navigation_call(call: &ruby_prism::CallNode<'_>) -> bool {
    call.call_operator_loc()
        .is_some_and(|loc| loc.as_slice() == b"&.")
}

fn path_has_hidden_directory(path: &Path) -> bool {
    let mut components = path.components().peekable();

    while let Some(component) = components.next() {
        let is_last = components.peek().is_none();
        if is_last {
            break;
        }

        if matches!(
            component,
            Component::Normal(name)
                if name.to_str().is_some_and(|s| s.starts_with('.') && s != "." && s != "..")
        ) {
            return true;
        }
    }

    false
}

fn preferred_message_for_integer(int_node: ruby_prism::IntegerNode<'_>) -> Option<&'static str> {
    let value = int_node.value();
    let (negative, digits) = value.to_u32_digits();

    if !negative && digits.iter().all(|digit| *digit == 0) {
        Some("Use `first`.")
    } else if negative
        && digits.first().copied() == Some(1)
        && digits.iter().skip(1).all(|digit| *digit == 0)
    {
        Some("Use `last`.")
    } else {
        None
    }
}

/// Walk down the receiver chain of `[]` calls, adding each intermediate
/// `[]` node's offset to the suppressed set. For `a[0][1][2]`, visiting
/// the outermost `[2]` adds `a[0][1]` and `a[0]` as suppressed.
fn suppress_bracket_receiver_chain(node: &ruby_prism::CallNode<'_>, suppressed: &mut Vec<usize>) {
    let mut current_recv = node.receiver();
    while let Some(recv) = current_recv {
        if let Some(recv_call) = recv.as_call_node() {
            if is_bracket_call(&recv_call) {
                suppressed.push(bracket_call_offset(&recv_call));
                current_recv = recv_call.receiver();
                continue;
            }
        }
        break;
    }
}

/// Suppress a `[]` argument node and walk its receiver chain.
fn suppress_bracket_arg(arg_call: &ruby_prism::CallNode<'_>, suppressed: &mut Vec<usize>) {
    suppressed.push(bracket_call_offset(arg_call));
    suppress_bracket_receiver_chain(arg_call, suppressed);
}

/// Suppress `[]` call arguments inside an index-write node's argument list.
/// This handles `h[arr[0]] ||= val`, `h[arr[0]] += val`, etc.
fn suppress_index_write_args(
    args: Option<ruby_prism::ArgumentsNode<'_>>,
    suppressed: &mut Vec<usize>,
) {
    if let Some(args) = args {
        for arg in args.arguments().iter() {
            if let Some(arg_call) = arg.as_call_node() {
                if is_bracket_call(&arg_call) {
                    suppress_bracket_arg(&arg_call, suppressed);
                }
            }
        }
    }
}

/// Suppress the receiver of an index-write node if it is a `[]` call.
/// Index-write nodes (IndexOrWriteNode, IndexAndWriteNode, IndexOperatorWriteNode)
/// are semantically `[]=` operations. When the receiver is `arr[0]` (a `[]` call),
/// as in `arr[0][:key] ||= val`, the `[0]` is a child of a bracket operation and
/// must not be flagged — matching RuboCop's `brace_method?(parent)` suppression.
fn suppress_index_write_receiver(
    receiver: Option<ruby_prism::Node<'_>>,
    suppressed: &mut Vec<usize>,
) {
    if let Some(recv) = receiver {
        if let Some(recv_call) = recv.as_call_node() {
            if is_bracket_call(&recv_call) {
                suppressed.push(bracket_call_offset(&recv_call));
                suppress_bracket_receiver_chain(&recv_call, suppressed);
            }
        }
    }
}

/// Check if an index-write node's arguments contain integer 0 or -1,
/// and if so, produce a diagnostic. This handles `arr[0] += val`,
/// `arr[-1] ||= default`, etc.
fn check_index_write_args<'a>(
    args: Option<ruby_prism::ArgumentsNode<'_>>,
    receiver: Option<ruby_prism::Node<'_>>,
    opening_loc: ruby_prism::Location<'_>,
    source: &'a SourceFile,
    cop: &'a ArrayFirstLast,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Must have a receiver
    let recv = match receiver {
        Some(r) => r,
        None => return,
    };

    // Skip if receiver is itself a [] call (chained indexing)
    if let Some(recv_call) = recv.as_call_node() {
        if recv_call.name().as_slice() == b"[]" {
            return;
        }
    }

    // Must have exactly one argument
    let args = match args {
        Some(a) => a,
        None => return,
    };
    let arg_list: Vec<_> = args.arguments().iter().collect();
    if arg_list.len() != 1 {
        return;
    }

    if let Some(int_node) = arg_list[0].as_integer_node() {
        if let Some(message) = preferred_message_for_integer(int_node) {
            // Use opening bracket location as the offense location
            let (line, column) = source.offset_to_line_col(opening_loc.start_offset());
            diagnostics.push(cop.diagnostic(source, line, column, message.to_string()));
        }
    }
}

impl<'pr> Visit<'pr> for ArrayFirstLastVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let method_name = node.name().as_slice();
        let is_bracket = method_name == b"[]" || method_name == b"[]=";

        // When entering a []/[]= call, suppress [] calls that are direct
        // children (receiver or arguments). This matches RuboCop's behavior:
        // only suppress arr[0] when its immediate parent in the AST is []/[]=.
        if is_bracket && !is_safe_navigation_call(node) {
            // Suppress receiver if it's a [] call (chained: arr[0][:key])
            // Also walk the chain deeper (arr[0][1][:key] → suppress arr[0][1] and arr[0])
            suppress_bracket_receiver_chain(node, &mut self.suppressed_offsets);

            // Suppress arguments that are [] calls (nested: hash[arr[0]])
            if let Some(args) = node.arguments() {
                for arg in args.arguments().iter() {
                    if let Some(arg_call) = arg.as_call_node() {
                        if is_bracket_call(&arg_call) {
                            suppress_bracket_arg(&arg_call, &mut self.suppressed_offsets);
                        }
                    }
                }
            }
        }

        // Check if this [] call should produce a diagnostic.
        if method_name == b"[]" {
            self.check_call(node);
        }

        // Recurse into children
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_index_or_write_node(&mut self, node: &ruby_prism::IndexOrWriteNode<'pr>) {
        // Suppress [] call arguments (FP fix: h[arr[0]] ||= val)
        suppress_index_write_args(node.arguments(), &mut self.suppressed_offsets);
        // Suppress [] receiver (FP fix: arr[0][:key] ||= val)
        suppress_index_write_receiver(node.receiver(), &mut self.suppressed_offsets);
        // Check own index for 0/-1 (FN fix: arr[0] ||= val)
        check_index_write_args(
            node.arguments(),
            node.receiver(),
            node.opening_loc(),
            self.source,
            self.cop,
            &mut self.diagnostics,
        );
        ruby_prism::visit_index_or_write_node(self, node);
    }

    fn visit_index_and_write_node(&mut self, node: &ruby_prism::IndexAndWriteNode<'pr>) {
        // Suppress [] call arguments (FP fix: h[arr[0]] &&= val)
        suppress_index_write_args(node.arguments(), &mut self.suppressed_offsets);
        // Suppress [] receiver (FP fix: arr[0][:key] &&= val)
        suppress_index_write_receiver(node.receiver(), &mut self.suppressed_offsets);
        // Check own index for 0/-1 (FN fix: arr[0] &&= val)
        check_index_write_args(
            node.arguments(),
            node.receiver(),
            node.opening_loc(),
            self.source,
            self.cop,
            &mut self.diagnostics,
        );
        ruby_prism::visit_index_and_write_node(self, node);
    }

    fn visit_index_operator_write_node(&mut self, node: &ruby_prism::IndexOperatorWriteNode<'pr>) {
        // Suppress [] call arguments (FP fix: h[arr[0]] += val)
        suppress_index_write_args(node.arguments(), &mut self.suppressed_offsets);
        // Suppress [] receiver (FP fix: values[0][1] += val)
        suppress_index_write_receiver(node.receiver(), &mut self.suppressed_offsets);
        // Check own index for 0/-1 (FN fix: arr[0] += val)
        check_index_write_args(
            node.arguments(),
            node.receiver(),
            node.opening_loc(),
            self.source,
            self.cop,
            &mut self.diagnostics,
        );
        ruby_prism::visit_index_operator_write_node(self, node);
    }
}

impl ArrayFirstLastVisitor<'_> {
    fn check_call(&mut self, call: &ruby_prism::CallNode<'_>) {
        // Must have a receiver
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Skip if receiver is itself a [] call (chained indexing like hash[:key][0])
        if let Some(recv_call) = receiver.as_call_node() {
            if recv_call.name().as_slice() == b"[]" {
                return;
            }
        }

        // Skip if this call is suppressed (it's a direct child of another []/[]= call)
        if self.suppressed_offsets.contains(&bracket_call_offset(call)) {
            return;
        }

        // Must have exactly one argument
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return;
        }

        let arg = &arg_list[0];

        // Check for integer literal 0 or -1
        if let Some(int_node) = arg.as_integer_node() {
            if let Some(message) = preferred_message_for_integer(int_node) {
                let loc = call.message_loc().unwrap_or(call.location());
                let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    message.to_string(),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cop::CopConfig;
    use crate::testutil::run_cop_full_internal;
    crate::cop_fixture_tests!(ArrayFirstLast, "cops/style/array_first_last");

    fn run(source: &[u8]) -> Vec<crate::diagnostic::Diagnostic> {
        run_cop_full_internal(&ArrayFirstLast, source, CopConfig::default(), "test.rb")
    }

    fn run_with_path(path: &str, source: &[u8]) -> Vec<crate::diagnostic::Diagnostic> {
        run_cop_full_internal(&ArrayFirstLast, source, CopConfig::default(), path)
    }

    #[test]
    fn detects_explicit_bracket_no_parens() {
        assert_eq!(run(b"arr.[] 0\n").len(), 1, "Should detect arr.[] 0");
    }

    #[test]
    fn detects_explicit_bracket_negative_no_parens() {
        assert_eq!(run(b"arr.[] -1\n").len(), 1, "Should detect arr.[] -1");
    }

    #[test]
    fn detects_safe_nav_bracket_no_parens() {
        assert_eq!(run(b"arr&.[] 0\n").len(), 1, "Should detect arr&.[] 0");
    }

    #[test]
    fn detects_safe_nav_bracket_negative_no_parens() {
        assert_eq!(run(b"arr&.[] -1\n").len(), 1, "Should detect arr&.[] -1");
    }

    #[test]
    fn detects_multiline_bracket() {
        assert_eq!(
            run(b"arr[\n  0\n]\n").len(),
            1,
            "Should detect multiline arr[0]"
        );
    }

    #[test]
    fn detects_in_method_argument() {
        assert_eq!(
            run(b"foo(arr[0])\n").len(),
            1,
            "Should detect arr[0] in method arg"
        );
    }

    #[test]
    fn detects_with_method_chain() {
        assert_eq!(run(b"arr[0].to_s\n").len(), 1, "Should detect arr[0].to_s");
    }

    #[test]
    fn detects_zero_index_as_receiver_of_safe_navigation_bracket_call() {
        let d = run(b"requirements[0]&.[](:requirement)\n");
        assert_eq!(
            d.len(),
            1,
            "Should flag requirements[0] under &.[] parent: {:?}",
            d
        );
    }

    #[test]
    fn detects_zero_index_as_argument_of_safe_navigation_bracket_call() {
        let d = run(b"foo&.[](arr[0])\n");
        assert_eq!(d.len(), 1, "Should flag arr[0] under &.[] parent: {:?}", d);
    }

    #[test]
    fn detects_outer_bracket_before_nested_bracket_chain() {
        let d = run(b"result[0].content[0][:text]\n");
        assert_eq!(d.len(), 1, "Should only flag result[0]: {:?}", d);
    }

    #[test]
    fn detects_outer_bracket_before_nonzero_index() {
        let d = run(b"tokentype[0].split(\":\")[1]\n");
        assert_eq!(d.len(), 1, "Should flag tokentype[0]: {:?}", d);
    }

    #[test]
    fn no_offense_when_zero_index_has_bracket_receiver() {
        let d = run(b"sql_traces[1].params[:explain_plan][0].sort\n");
        assert!(
            d.is_empty(),
            "Should not flag [:explain_plan][0] because receiver is []: {:?}",
            d
        );
    }

    #[test]
    fn no_offense_when_zero_index_is_after_nonzero_bracket_chain() {
        let d = run(b"sql_traces[1].params[:explain_plan][1][0].sort\n");
        assert!(
            d.is_empty(),
            "Should not flag trailing [0] in [1][0] chain: {:?}",
            d
        );
    }

    #[test]
    fn detects_offense_in_root_dotfile_path() {
        let d = run_with_path(".watchr", b"run_spec match[0]\n");
        assert_eq!(
            d.len(),
            1,
            "Should lint root dotfiles like .watchr: {:?}",
            d
        );
    }

    #[test]
    fn detects_offense_in_hidden_basename_path() {
        let d = run_with_path("common-tools/ci/.toys.rb", b"collection[0]\n");
        assert_eq!(
            d.len(),
            1,
            "Should lint hidden basenames in visible directories: {:?}",
            d
        );
    }

    #[test]
    fn no_offense_in_hidden_directory_repo_scan() {
        let d = run_with_path(
            ".github/workflows/scripts/rubygems-publish.rb",
            b"ARGV[0]\n",
        );
        assert!(
            d.is_empty(),
            "Should skip hidden-path files during repo scans: {:?}",
            d
        );
    }

    #[test]
    fn no_offense_receiver_of_index_or_write() {
        let d = run(b"arr[0][:key] ||= val\n");
        assert!(
            d.is_empty(),
            "Should not flag arr[0] as receiver of ||= index-write: {:?}",
            d
        );
    }

    #[test]
    fn no_offense_receiver_of_index_operator_write() {
        let d = run(b"values[0][1] += value\n");
        assert!(
            d.is_empty(),
            "Should not flag values[0] as receiver of += index-write: {:?}",
            d
        );
    }
}
