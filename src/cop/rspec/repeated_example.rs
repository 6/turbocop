use crate::cop::node_type::{BLOCK_NODE, CALL_NODE};
use crate::cop::util::{RSPEC_DEFAULT_INCLUDE, is_rspec_example};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;
use std::collections::HashMap;

/// RSpec/RepeatedExample: Don't repeat examples (same body) within an example group.
///
/// **Investigation (2026-03-04):** 88 FPs caused by `its()` calls with different string
/// attributes but same block body being treated as duplicates. The `example_body_signature()`
/// function was skipping the first string arg (treating it as a description like `it`), but
/// for `its`, the first string arg is the attribute accessor (e.g., `its('Server.Version')`).
/// Fix: include the first string arg in the signature when the method is `its`.
///
/// **Investigation (2026-03-05):** 893 FNs and 22 FPs caused by raw source-text comparison
/// for example body signatures. RuboCop uses AST structural equality, meaning examples with
/// the same AST but different formatting (e.g., `do..end` vs `{ }`, different indentation,
/// semicolons vs newlines) are correctly identified as duplicates. Raw source comparison
/// missed all of these.
///
/// Root cause of FNs: identical example bodies with different whitespace/formatting produced
/// different raw source signatures, so they were not detected as duplicates.
///
/// Root cause of FPs: metadata args (like `:focus` tags) were compared as raw source text
/// which could accidentally match in edge cases.
///
/// Fix: replaced raw source comparison with AST-based structural fingerprinting. The new
/// `AstFingerprinter` walks the AST recursively, emitting node type tags and literal values
/// (strings, symbols, integers, identifiers) but ignoring whitespace and source locations.
/// This produces a canonical representation matching RuboCop's AST equality semantics.
///
/// The signature now consists of:
/// 1. Metadata args (everything after the first string description arg) — AST fingerprint
/// 2. Block body (the implementation) — AST fingerprint
/// 3. For `its()` calls, the first arg (attribute accessor) is also included
pub struct RepeatedExample;

impl Cop for RepeatedExample {
    fn name(&self) -> &'static str {
        "RSpec/RepeatedExample"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, CALL_NODE]
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

        let name = call.name().as_slice();
        if !is_example_group(name) {
            return;
        }

        let block = match call.block() {
            Some(b) => b,
            None => return,
        };
        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return,
        };
        let body = match block_node.body() {
            Some(b) => b,
            None => return,
        };
        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        // Collect examples: body_signature -> list of (line, col)
        let mut body_map: HashMap<Vec<u8>, Vec<(usize, usize)>> = HashMap::new();

        for stmt in stmts.body().iter() {
            if let Some(c) = stmt.as_call_node() {
                let m = c.name().as_slice();
                if is_rspec_example(m) || m == b"its" {
                    if let Some(sig) = example_body_signature(&c, m) {
                        let loc = c.location();
                        let (line, col) = source.offset_to_line_col(loc.start_offset());
                        body_map.entry(sig).or_default().push((line, col));
                    }
                }
            }
        }

        for locs in body_map.values() {
            if locs.len() > 1 {
                for (idx, &(line, col)) in locs.iter().enumerate() {
                    let other_lines: Vec<String> = locs
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| *i != idx)
                        .map(|(_, (l, _))| l.to_string())
                        .collect();
                    let msg = format!(
                        "Don't repeat examples within an example group. Repeated on line(s) {}.",
                        other_lines.join(", ")
                    );
                    diagnostics.push(self.diagnostic(source, line, col, msg));
                }
            }
        }
    }
}

/// Build a structural AST signature from the example's metadata + block body.
///
/// Two examples with the same AST structure (ignoring whitespace/formatting) and
/// same metadata are considered duplicates, matching RuboCop's behavior.
///
/// RuboCop's `build_example_signature` returns `[metadata, implementation]` where:
/// - `metadata` = args after the first string description (tags like `:focus`)
/// - `implementation` = block body AST node
///
/// Both are compared using Ruby's AST structural equality.
///
/// For `its()` calls, the first arg (attribute accessor) is included per RuboCop behavior.
fn example_body_signature(call: &ruby_prism::CallNode<'_>, method_name: &[u8]) -> Option<Vec<u8>> {
    let mut fp = AstFingerprinter::new();

    // Separator between metadata and body sections
    const SECTION_SEP: u8 = 0xFF;

    // Include metadata args (skip the first string/symbol description if present).
    // For `its()`, the first string arg is an attribute accessor, not a description,
    // so we include it in the signature.
    let is_its = method_name == b"its";
    if let Some(args) = call.arguments() {
        let arg_list: Vec<_> = args.arguments().iter().collect();
        for (i, arg) in arg_list.iter().enumerate() {
            // Skip first argument if it's a string (description) — but not for `its()`
            if i == 0
                && !is_its
                && (arg.as_string_node().is_some() || arg.as_interpolated_string_node().is_some())
            {
                continue;
            }
            fp.fingerprint_node(arg);
            fp.buf.push(b',');
        }
    }

    fp.buf.push(SECTION_SEP);

    // Include block body AST fingerprint
    if let Some(block) = call.block() {
        if let Some(block_node) = block.as_block_node() {
            // Fingerprint the body (StatementsNode), not the entire block
            // (which includes do/end or { } delimiters that differ by formatting)
            if let Some(ref body) = block_node.body() {
                fp.fingerprint_node(body);
            }
        }
    }

    if fp.buf.len() <= 1 {
        // Only the section separator — no meaningful content
        return None;
    }

    Some(fp.buf)
}

/// AST fingerprinter that produces a canonical byte representation of an AST subtree.
///
/// Walks the AST recursively, emitting:
/// - Node type tag (u8) for structural comparison
/// - Literal content for leaf nodes (string values, symbol names, integer literals, etc.)
/// - Child count markers for composite nodes
///
/// This is whitespace-independent: `do\n  expr\nend` and `{ expr }` produce
/// the same fingerprint because they have the same AST structure.
struct AstFingerprinter {
    buf: Vec<u8>,
}

impl AstFingerprinter {
    fn new() -> Self {
        Self {
            buf: Vec::with_capacity(128),
        }
    }

    fn fingerprint_node(&mut self, node: &ruby_prism::Node<'_>) {
        // Emit node type tag
        self.buf.push(crate::cop::node_type::node_type_tag(node));

        // For leaf nodes with literal content, emit the content
        // For composite nodes, the Visit traversal handles children
        self.visit(node);
    }

    fn emit_bytes(&mut self, bytes: &[u8]) {
        // Length-prefixed to avoid ambiguity
        let len = bytes.len() as u32;
        self.buf.extend_from_slice(&len.to_le_bytes());
        self.buf.extend_from_slice(bytes);
    }
}

impl<'pr> Visit<'pr> for AstFingerprinter {
    // For most nodes, the default visit implementation recurses into children,
    // and we emit the node type tag for each child we visit.

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // Emit method name for method calls
        self.emit_bytes(node.name().as_slice());
        // Emit whether there's a call operator (&. vs .)
        if node.call_operator_loc().is_some() {
            self.buf.push(1);
        } else {
            self.buf.push(0);
        }
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_string_node(&mut self, node: &ruby_prism::StringNode<'pr>) {
        self.emit_bytes(node.unescaped());
        ruby_prism::visit_string_node(self, node);
    }

    fn visit_symbol_node(&mut self, node: &ruby_prism::SymbolNode<'pr>) {
        self.emit_bytes(node.unescaped());
        ruby_prism::visit_symbol_node(self, node);
    }

    fn visit_integer_node(&mut self, node: &ruby_prism::IntegerNode<'pr>) {
        // Use the source representation for integer values
        self.emit_bytes(node.location().as_slice());
        ruby_prism::visit_integer_node(self, node);
    }

    fn visit_float_node(&mut self, node: &ruby_prism::FloatNode<'pr>) {
        self.emit_bytes(node.location().as_slice());
        ruby_prism::visit_float_node(self, node);
    }

    fn visit_constant_read_node(&mut self, node: &ruby_prism::ConstantReadNode<'pr>) {
        self.emit_bytes(node.name().as_slice());
        ruby_prism::visit_constant_read_node(self, node);
    }

    fn visit_constant_path_node(&mut self, node: &ruby_prism::ConstantPathNode<'pr>) {
        if let Some(name) = node.name() {
            self.emit_bytes(name.as_slice());
        }
        ruby_prism::visit_constant_path_node(self, node);
    }

    fn visit_local_variable_read_node(&mut self, node: &ruby_prism::LocalVariableReadNode<'pr>) {
        self.emit_bytes(node.name().as_slice());
        ruby_prism::visit_local_variable_read_node(self, node);
    }

    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        self.emit_bytes(node.name().as_slice());
        ruby_prism::visit_local_variable_write_node(self, node);
    }

    fn visit_instance_variable_read_node(
        &mut self,
        node: &ruby_prism::InstanceVariableReadNode<'pr>,
    ) {
        self.emit_bytes(node.name().as_slice());
        ruby_prism::visit_instance_variable_read_node(self, node);
    }

    fn visit_instance_variable_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableWriteNode<'pr>,
    ) {
        self.emit_bytes(node.name().as_slice());
        ruby_prism::visit_instance_variable_write_node(self, node);
    }

    fn visit_class_variable_read_node(&mut self, node: &ruby_prism::ClassVariableReadNode<'pr>) {
        self.emit_bytes(node.name().as_slice());
        ruby_prism::visit_class_variable_read_node(self, node);
    }

    fn visit_global_variable_read_node(&mut self, node: &ruby_prism::GlobalVariableReadNode<'pr>) {
        self.emit_bytes(node.name().as_slice());
        ruby_prism::visit_global_variable_read_node(self, node);
    }

    fn visit_interpolated_string_node(&mut self, node: &ruby_prism::InterpolatedStringNode<'pr>) {
        ruby_prism::visit_interpolated_string_node(self, node);
    }

    fn visit_interpolated_symbol_node(&mut self, node: &ruby_prism::InterpolatedSymbolNode<'pr>) {
        ruby_prism::visit_interpolated_symbol_node(self, node);
    }

    fn visit_regular_expression_node(&mut self, node: &ruby_prism::RegularExpressionNode<'pr>) {
        self.emit_bytes(node.unescaped());
        ruby_prism::visit_regular_expression_node(self, node);
    }

    fn visit_true_node(&mut self, _node: &ruby_prism::TrueNode<'pr>) {
        self.buf.push(1);
    }

    fn visit_false_node(&mut self, _node: &ruby_prism::FalseNode<'pr>) {
        self.buf.push(0);
    }

    fn visit_nil_node(&mut self, _node: &ruby_prism::NilNode<'pr>) {
        self.buf.push(0);
    }

    fn visit_self_node(&mut self, _node: &ruby_prism::SelfNode<'pr>) {
        self.buf.push(0);
    }

    // For block nodes, we only want to fingerprint the body, not the delimiters
    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        // Fingerprint parameters if present
        if let Some(ref params) = node.parameters() {
            self.buf.push(crate::cop::node_type::node_type_tag(params));
            self.visit(params);
        }
        // Fingerprint body if present
        if let Some(ref body) = node.body() {
            self.buf.push(crate::cop::node_type::node_type_tag(body));
            self.visit(body);
        }
    }

    // For nodes we visit by default traversal, we need to emit child type tags
    fn visit_statements_node(&mut self, node: &ruby_prism::StatementsNode<'pr>) {
        for child in node.body().iter() {
            self.buf.push(crate::cop::node_type::node_type_tag(&child));
            self.visit(&child);
        }
    }

    fn visit_arguments_node(&mut self, node: &ruby_prism::ArgumentsNode<'pr>) {
        for child in node.arguments().iter() {
            self.buf.push(crate::cop::node_type::node_type_tag(&child));
            self.visit(&child);
        }
    }

    fn visit_array_node(&mut self, node: &ruby_prism::ArrayNode<'pr>) {
        for child in node.elements().iter() {
            self.buf.push(crate::cop::node_type::node_type_tag(&child));
            self.visit(&child);
        }
    }

    fn visit_hash_node(&mut self, node: &ruby_prism::HashNode<'pr>) {
        for child in node.elements().iter() {
            self.buf.push(crate::cop::node_type::node_type_tag(&child));
            self.visit(&child);
        }
    }

    fn visit_keyword_hash_node(&mut self, node: &ruby_prism::KeywordHashNode<'pr>) {
        for child in node.elements().iter() {
            self.buf.push(crate::cop::node_type::node_type_tag(&child));
            self.visit(&child);
        }
    }

    fn visit_assoc_node(&mut self, node: &ruby_prism::AssocNode<'pr>) {
        self.buf
            .push(crate::cop::node_type::node_type_tag(&node.key()));
        self.visit(&node.key());
        self.buf
            .push(crate::cop::node_type::node_type_tag(&node.value()));
        self.visit(&node.value());
    }

    fn visit_parentheses_node(&mut self, node: &ruby_prism::ParenthesesNode<'pr>) {
        // Parentheses are transparent — just visit the body
        if let Some(ref body) = node.body() {
            self.buf.push(crate::cop::node_type::node_type_tag(body));
            self.visit(body);
        }
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        self.emit_bytes(node.name().as_slice());
        ruby_prism::visit_def_node(self, node);
    }
}

fn is_example_group(name: &[u8]) -> bool {
    // RuboCop only checks ExampleGroups (describe/context/feature),
    // NOT SharedGroups (shared_examples/shared_context).
    matches!(
        name,
        b"describe"
            | b"context"
            | b"feature"
            | b"example_group"
            | b"xdescribe"
            | b"xcontext"
            | b"xfeature"
            | b"fdescribe"
            | b"fcontext"
            | b"ffeature"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(RepeatedExample, "cops/rspec/repeated_example");
}
