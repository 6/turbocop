use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct FixedSize;

const MSG: &str = "Do not compute the size of statically sized objects.";

impl Cop for FixedSize {
    fn name(&self) -> &'static str {
        "Performance/FixedSize"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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
        let mut visitor = FixedSizeVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            in_constant_assignment: false,
            in_block: false,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct FixedSizeVisitor<'a, 'src> {
    cop: &'a FixedSize,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    in_constant_assignment: bool,
    in_block: bool,
}

impl<'pr> Visit<'pr> for FixedSizeVisitor<'_, '_> {
    fn visit_constant_write_node(&mut self, node: &ruby_prism::ConstantWriteNode<'pr>) {
        let prev = self.in_constant_assignment;
        self.in_constant_assignment = true;
        ruby_prism::visit_constant_write_node(self, node);
        self.in_constant_assignment = prev;
    }

    fn visit_constant_path_write_node(&mut self, node: &ruby_prism::ConstantPathWriteNode<'pr>) {
        let prev = self.in_constant_assignment;
        self.in_constant_assignment = true;
        ruby_prism::visit_constant_path_write_node(self, node);
        self.in_constant_assignment = prev;
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        let prev = self.in_block;
        self.in_block = true;
        ruby_prism::visit_block_node(self, node);
        self.in_block = prev;
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        self.check_call(node);
        ruby_prism::visit_call_node(self, node);
    }
}

impl FixedSizeVisitor<'_, '_> {
    fn check_call(&mut self, call: &ruby_prism::CallNode<'_>) {
        let method_name = call.name().as_slice();
        match method_name {
            b"size" | b"length" | b"count" => {}
            _ => return,
        }

        let recv = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Check receiver is a static-size literal: string, symbol, array, or hash
        if !is_static_size_receiver(&recv) {
            return;
        }

        // Check for splat in arrays or double-splat in hashes
        if contains_splat(&recv) || contains_double_splat(&recv) {
            return;
        }

        // If method is `count` with arguments, check if they're valid
        if method_name == b"count" {
            if let Some(args) = call.arguments() {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if let Some(first_arg) = arg_list.first() {
                    // Allow count with string argument (e.g., "foo".count('o'))
                    // but reject count with non-string argument (e.g., "foo".count(bar))
                    if first_arg.as_string_node().is_none() {
                        return;
                    }
                }
            }
            // If count has a block, skip
            if call.block().is_some() {
                return;
            }
        }

        // Skip if inside constant assignment or block
        if self.in_constant_assignment || self.in_block {
            return;
        }

        let loc = call.location();
        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
        self.diagnostics.push(
            self.cop
                .diagnostic(self.source, line, column, MSG.to_string()),
        );
    }
}

fn is_static_size_receiver(node: &ruby_prism::Node<'_>) -> bool {
    // String literals (not interpolated)
    if node.as_string_node().is_some() {
        return true;
    }
    // Symbol literals (not interpolated)
    if node.as_symbol_node().is_some() {
        return true;
    }
    // Array literals
    if node.as_array_node().is_some() {
        return true;
    }
    // Hash literals (not keyword hash)
    if node.as_hash_node().is_some() {
        return true;
    }
    // KeywordHashNode (keyword args like `foo(a: 1)`) cannot appear as a
    // method receiver, so this is unreachable in practice. We explicitly
    // exclude it to acknowledge the hash_node/keyword_hash_node distinction.
    if node.as_keyword_hash_node().is_some() {
        return false;
    }
    false
}

fn contains_splat(node: &ruby_prism::Node<'_>) -> bool {
    let array = match node.as_array_node() {
        Some(a) => a,
        None => return false,
    };
    for elem in array.elements().iter() {
        if elem.as_splat_node().is_some() {
            return true;
        }
    }
    false
}

fn contains_double_splat(node: &ruby_prism::Node<'_>) -> bool {
    let hash = match node.as_hash_node() {
        Some(h) => h,
        None => return false,
    };
    for elem in hash.elements().iter() {
        if elem.as_assoc_splat_node().is_some() {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FixedSize, "cops/performance/fixed_size");
}
