use crate::cop::shared::node_type::{CALL_NODE, DEF_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-03)
///
/// Corpus oracle reported FP=0, FN=137.
///
/// FN=137: Fixed by implementing the COMBINATOR pattern — detecting `^`/`+`/`*`/`|`
/// operators (and their `^=`/`+=`/`*=`/`|=` op-asgn forms) inside `def hash` methods,
/// `define_method(:hash)` blocks, and `define_singleton_method(:hash)` blocks. Also
/// fixed the REDUNDANT pattern to flag each individual `.hash` element (ANY, not ALL).
/// Commits: 2f9d59d, ef273b9, 967ffa0.
///
/// After fix: FP=0, FN=2. The 2 remaining FN are within corpus file-drop noise (7
/// repos with RuboCop parser crashes produce noise offenses). No further cop logic
/// changes needed.
///
/// ## Corpus investigation (2026-03-20) — extended corpus
///
/// Extended corpus oracle reported FP=10, FN=4.
///
/// FP=8: Fixed by requiring `def hash` to have NO parameters before running
/// combinator finder. Methods like `def self.hash(uin, ptwebqq)` are custom
/// methods, not Object#hash overrides. Commit: this session.
///
/// FP=2: Fixed by skipping MONUPLE check when `[x].hash` is followed by a
/// combinator operator in source text (e.g., `[x].hash ^ BIG_VALUE` inside
/// `def hash` — only COMBINATOR should fire, not both).
///
/// FN=2: Fixed by allowing bare `hash` calls (no receiver) in REDUNDANT check.
/// Pattern: `[a, hash, b].hash` where `hash` is `self.hash`.
///
/// FN=1: Fixed by adding `IndexOperatorWriteNode` handling to combinator finder
/// for patterns like `h[:key] ^= value` inside `def hash`.
///
/// FN=1: workarea `results[id] += value` — insufficient context to determine if
/// inside `def hash`. May be a corpus artifact or deeper nesting issue.
///
/// ## Corpus investigation (2026-03-25) — full corpus verification
///
/// Corpus oracle reported FP=0, FN=1. FN verified FIXED by
/// `verify_cop_locations.py`. Cop logic is correct — detects combinator
/// patterns (`@local.hash ^ @domain.hash` inside `def hash`) correctly.
/// The FN gap was a corpus oracle config/path resolution artifact.
pub struct CompoundHash;

const COMBINATOR_MSG: &str = "Use `[...].hash` instead of combining hash values manually.";
const MONUPLE_MSG: &str =
    "Delegate hash directly without wrapping in an array when only using a single value.";
const REDUNDANT_MSG: &str = "Calling `.hash` on elements of a hashed array is redundant.";

/// Combinator operator names: ^, +, *, | (and their op-asgn forms ^=, +=, *=, |=)
fn is_combinator_op(name: &[u8]) -> bool {
    matches!(
        name,
        b"^" | b"+" | b"*" | b"|" | b"^=" | b"+=" | b"*=" | b"|="
    )
}

/// Walk the body of a hash method to find outermost combinator expressions.
/// "Outermost" means: if `a ^ b ^ c` parses as `(a ^ b) ^ c`, only the outer `^` is flagged.
fn find_outermost_combinators<'pr>(
    node: &ruby_prism::Node<'pr>,
    source: &SourceFile,
    results: &mut Vec<ruby_prism::Location<'pr>>,
) {
    use ruby_prism::Visit;

    struct CombinatorFinder<'a, 'pr> {
        source: &'a SourceFile,
        results: &'a mut Vec<ruby_prism::Location<'pr>>,
    }

    impl CombinatorFinder<'_, '_> {
        fn is_combinator_op_at(&self, loc: &ruby_prism::Location<'_>) -> bool {
            let op = &self.source.as_bytes()[loc.start_offset()..loc.end_offset()];
            is_combinator_op(op)
        }
    }

    impl<'pr> Visit<'pr> for CombinatorFinder<'_, 'pr> {
        fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
            if is_combinator_op(node.name().as_slice()) {
                // Flag the outermost combinator — do NOT recurse into children
                self.results.push(node.location());
                return;
            }
            // Continue visiting children for non-combinator calls
            ruby_prism::visit_call_node(self, node);
        }

        fn visit_local_variable_operator_write_node(
            &mut self,
            node: &ruby_prism::LocalVariableOperatorWriteNode<'pr>,
        ) {
            if self.is_combinator_op_at(&node.binary_operator_loc()) {
                self.results.push(node.location());
                return;
            }
            ruby_prism::visit_local_variable_operator_write_node(self, node);
        }

        fn visit_instance_variable_operator_write_node(
            &mut self,
            node: &ruby_prism::InstanceVariableOperatorWriteNode<'pr>,
        ) {
            if self.is_combinator_op_at(&node.binary_operator_loc()) {
                self.results.push(node.location());
                return;
            }
            ruby_prism::visit_instance_variable_operator_write_node(self, node);
        }

        fn visit_class_variable_operator_write_node(
            &mut self,
            node: &ruby_prism::ClassVariableOperatorWriteNode<'pr>,
        ) {
            if self.is_combinator_op_at(&node.binary_operator_loc()) {
                self.results.push(node.location());
                return;
            }
            ruby_prism::visit_class_variable_operator_write_node(self, node);
        }

        fn visit_global_variable_operator_write_node(
            &mut self,
            node: &ruby_prism::GlobalVariableOperatorWriteNode<'pr>,
        ) {
            if self.is_combinator_op_at(&node.binary_operator_loc()) {
                self.results.push(node.location());
                return;
            }
            ruby_prism::visit_global_variable_operator_write_node(self, node);
        }

        fn visit_index_operator_write_node(
            &mut self,
            node: &ruby_prism::IndexOperatorWriteNode<'pr>,
        ) {
            if self.is_combinator_op_at(&node.binary_operator_loc()) {
                self.results.push(node.location());
                return;
            }
            ruby_prism::visit_index_operator_write_node(self, node);
        }

        // Do not recurse into nested def nodes — they define a separate scope
        fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
    }

    let mut finder = CombinatorFinder { source, results };
    finder.visit(node);
}

impl Cop for CompoundHash {
    fn name(&self) -> &'static str {
        "Security/CompoundHash"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, DEF_NODE]
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
        // === COMBINATOR pattern: detect operators inside def hash ===

        // Handle `def hash` and `def object.hash` (DefNode)
        // Only flag methods named `hash` with NO parameters (Object#hash override).
        // Methods like `def hash(data)` are custom methods, not hash overrides.
        if let Some(def_node) = node.as_def_node() {
            if def_node.name().as_slice() == b"hash" && def_node.parameters().is_none() {
                if let Some(body) = def_node.body() {
                    let mut combinator_locs = Vec::new();
                    find_outermost_combinators(&body, source, &mut combinator_locs);
                    for loc in combinator_locs {
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            COMBINATOR_MSG.to_string(),
                        ));
                    }
                }
            }
            return;
        }

        // Handle CallNode: define_method(:hash), or .hash on arrays
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };
        let name = call.name().as_slice();

        // Check for define_method(:hash) or define_singleton_method(:hash)
        if name == b"define_method" || name == b"define_singleton_method" {
            if let Some(args) = call.arguments() {
                let arg_list: Vec<ruby_prism::Node<'_>> = args.arguments().iter().collect();
                if let Some(first_arg) = arg_list.first() {
                    if let Some(sym) = first_arg.as_symbol_node() {
                        if sym.unescaped() == b"hash" {
                            if let Some(block) = call.block() {
                                if let Some(block_node) = block.as_block_node() {
                                    if let Some(body) = block_node.body() {
                                        let mut combinator_locs = Vec::new();
                                        find_outermost_combinators(
                                            &body,
                                            source,
                                            &mut combinator_locs,
                                        );
                                        for loc in combinator_locs {
                                            let (line, column) =
                                                source.offset_to_line_col(loc.start_offset());
                                            diagnostics.push(self.diagnostic(
                                                source,
                                                line,
                                                column,
                                                COMBINATOR_MSG.to_string(),
                                            ));
                                        }
                                    }
                                }
                            }
                            return;
                        }
                    }
                }
            }
        }

        // === MONUPLE and REDUNDANT patterns ===
        // These are for `.hash` calls on arrays: `[x].hash` or `[a.hash, b].hash`

        if name != b"hash" {
            return;
        }

        // Must have no arguments
        if call.arguments().is_some() {
            return;
        }

        // Receiver must be an array literal
        let recv = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let array_node = match recv.as_array_node() {
            Some(a) => a,
            None => return,
        };

        let elements: Vec<ruby_prism::Node<'_>> = array_node.elements().iter().collect();

        // Monuple: [single_value].hash
        // Skip if the .hash result is used as a combinator operand (e.g., [x].hash ^ y),
        // because the COMBINATOR pattern already covers it inside def hash.
        if elements.len() == 1 {
            let end_offset = call.location().end_offset();
            let rest = &source.as_bytes()[end_offset..];
            let trimmed = rest.iter().skip_while(|b| b.is_ascii_whitespace()).copied();
            let next_char = trimmed.clone().next();
            let is_combinator_follow =
                matches!(next_char, Some(b'^') | Some(b'+') | Some(b'*') | Some(b'|'));
            if !is_combinator_follow {
                let msg_loc = call.message_loc().unwrap();
                let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
                diagnostics.push(self.diagnostic(source, line, column, MONUPLE_MSG.to_string()));
            }
        }

        // Redundant: flag EACH element that calls .hash (ANY, not ALL)
        // Includes both `foo.hash` (with receiver) and bare `hash` (self.hash, no receiver).
        if elements.len() >= 2 {
            for elem in &elements {
                if let Some(c) = elem.as_call_node() {
                    if c.name().as_slice() == b"hash" && c.arguments().is_none() {
                        let loc = c.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            REDUNDANT_MSG.to_string(),
                        ));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(CompoundHash, "cops/security/compound_hash");

    #[test]
    fn op_asgn_in_block_inside_def_hash() {
        crate::testutil::assert_cop_offenses_full(
            &CompoundHash,
            b"def hash\n  h = 0\n  things.each do |thing|\n    h ^= thing.hash\n    ^^^^^^^^^^^^^^^ Security/CompoundHash: Use `[...].hash` instead of combining hash values manually.\n  end\n  h\nend\n",
        );
    }

    #[test]
    fn index_op_asgn_plus_in_def_hash() {
        // h[:b] += 1 inside def self.hash (IndexOperatorWriteNode)
        crate::testutil::assert_cop_offenses_full(
            &CompoundHash,
            b"def self.hash\n  h = Hash[:a, 1, :b, 2]\n  h[:b] += 1\n  ^^^^^^^^^^ Security/CompoundHash: Use `[...].hash` instead of combining hash values manually.\nend\n",
        );
    }

    #[test]
    fn bare_hash_in_array_is_redundant() {
        // Bare `hash` (no receiver) inside hashed array should be flagged
        let src = b"[name, id, hash, updated_at].hash\n";
        let diags = crate::testutil::run_cop_full(&CompoundHash, src);
        assert!(
            !diags.is_empty(),
            "Expected a REDUNDANT offense for bare hash"
        );
        assert!(
            diags[0].message.contains("redundant"),
            "Expected REDUNDANT message"
        );
    }

    #[test]
    fn define_method_hash_combinator() {
        let source = b"define_method(:hash) do\n  1.hash ^ 2.hash\nend\n";
        crate::testutil::assert_cop_offenses_full(
            &CompoundHash,
            // Use nitrocop-expect to mark offenses
            b"define_method(:hash) do\n  1.hash ^ 2.hash\n  ^^^^^^^^^^^^^^^^ Security/CompoundHash: Use `[...].hash` instead of combining hash values manually.\nend\n",
        );
    }
}
