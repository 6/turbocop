use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;
use std::collections::HashSet;
use std::sync::LazyLock;

pub struct CollectionLiteralInLoop;

const ENUMERABLE_METHODS: &[&[u8]] = &[
    b"all?", b"any?", b"chunk", b"chunk_while", b"collect", b"collect_concat",
    b"count", b"cycle", b"detect", b"drop", b"drop_while", b"each",
    b"each_cons", b"each_entry", b"each_slice", b"each_with_index",
    b"each_with_object", b"entries", b"filter", b"filter_map", b"find",
    b"find_all", b"find_index", b"first", b"flat_map", b"grep", b"grep_v",
    b"group_by", b"include?", b"inject", b"lazy", b"map", b"max", b"max_by",
    b"member?", b"min", b"min_by", b"minmax", b"minmax_by", b"none?", b"one?",
    b"partition", b"reduce", b"reject", b"reverse_each", b"select",
    b"slice_after", b"slice_before", b"slice_when", b"sort", b"sort_by",
    b"sum", b"take", b"take_while", b"tally", b"to_a", b"to_h",
    b"uniq", b"zip",
];

/// Non-mutating Array methods (safe to call on a literal without modifying it)
const NONMUTATING_ARRAY_METHODS: &[&[u8]] = &[
    b"&", b"*", b"+", b"-", b"<=>", b"==", b"[]", b"all?", b"any?", b"assoc",
    b"at", b"bsearch", b"bsearch_index", b"collect", b"combination", b"compact",
    b"count", b"cycle", b"deconstruct", b"difference", b"dig", b"drop",
    b"drop_while", b"each", b"each_index", b"empty?", b"eql?", b"fetch",
    b"filter", b"find_index", b"first", b"flatten", b"hash", b"include?",
    b"index", b"inspect", b"intersection", b"join", b"last", b"length", b"map",
    b"max", b"min", b"minmax", b"none?", b"one?", b"pack", b"permutation",
    b"product", b"rassoc", b"reject", b"repeated_combination",
    b"repeated_permutation", b"reverse", b"reverse_each", b"rindex", b"rotate",
    b"sample", b"select", b"shuffle", b"size", b"slice", b"sort", b"sum",
    b"take", b"take_while", b"to_a", b"to_ary", b"to_h", b"to_s", b"transpose",
    b"union", b"uniq", b"values_at", b"zip", b"|",
];

/// Non-mutating Hash methods
const NONMUTATING_HASH_METHODS: &[&[u8]] = &[
    b"<", b"<=", b"==", b">", b">=", b"[]", b"any?", b"assoc", b"compact",
    b"dig", b"each", b"each_key", b"each_pair", b"each_value", b"empty?",
    b"eql?", b"fetch", b"fetch_values", b"filter", b"flatten", b"has_key?",
    b"has_value?", b"hash", b"include?", b"inspect", b"invert", b"key", b"key?",
    b"keys?", b"length", b"member?", b"merge", b"rassoc", b"rehash", b"reject",
    b"select", b"size", b"slice", b"to_a", b"to_h", b"to_hash", b"to_proc",
    b"to_s", b"transform_keys", b"transform_values", b"value?", b"values",
    b"values_at",
];

fn build_method_set(methods: &[&[u8]]) -> HashSet<Vec<u8>> {
    methods.iter().map(|m| m.to_vec()).collect()
}

/// Pre-compiled method sets — built once, reused across all files.
static ARRAY_METHOD_SET: LazyLock<HashSet<Vec<u8>>> = LazyLock::new(|| {
    let mut set = build_method_set(ENUMERABLE_METHODS);
    for m in NONMUTATING_ARRAY_METHODS {
        set.insert(m.to_vec());
    }
    set
});

static HASH_METHOD_SET: LazyLock<HashSet<Vec<u8>>> = LazyLock::new(|| {
    let mut set = build_method_set(ENUMERABLE_METHODS);
    for m in NONMUTATING_HASH_METHODS {
        set.insert(m.to_vec());
    }
    set
});

static ENUMERABLE_METHOD_SET: LazyLock<HashSet<Vec<u8>>> =
    LazyLock::new(|| build_method_set(ENUMERABLE_METHODS));

impl Cop for CollectionLiteralInLoop {
    fn name(&self) -> &'static str {
        "Performance/CollectionLiteralInLoop"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let min_size = config.get_usize("MinSize", 1);

        let mut visitor = CollectionLiteralVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            loop_depth: 0,
            min_size,
            array_methods: &ARRAY_METHOD_SET,
            hash_methods: &HASH_METHOD_SET,
            enumerable_methods: &ENUMERABLE_METHOD_SET,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct CollectionLiteralVisitor<'a, 'src> {
    cop: &'a CollectionLiteralInLoop,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    loop_depth: usize,
    min_size: usize,
    array_methods: &'a HashSet<Vec<u8>>,
    hash_methods: &'a HashSet<Vec<u8>>,
    enumerable_methods: &'a HashSet<Vec<u8>>,
}

impl<'pr> Visit<'pr> for CollectionLiteralVisitor<'_, '_> {
    fn visit_while_node(&mut self, node: &ruby_prism::WhileNode<'pr>) {
        self.loop_depth += 1;
        ruby_prism::visit_while_node(self, node);
        self.loop_depth -= 1;
    }

    fn visit_until_node(&mut self, node: &ruby_prism::UntilNode<'pr>) {
        self.loop_depth += 1;
        ruby_prism::visit_until_node(self, node);
        self.loop_depth -= 1;
    }

    fn visit_for_node(&mut self, node: &ruby_prism::ForNode<'pr>) {
        self.loop_depth += 1;
        ruby_prism::visit_for_node(self, node);
        self.loop_depth -= 1;
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let method_name = node.name().as_slice();

        // Check if this call has a block and is a loop-like method
        let is_loop_call = if let Some(block) = node.block() {
            if block.as_block_node().is_some() {
                self.is_loop_method(node)
            } else {
                false
            }
        } else {
            false
        };

        // Check if this call's receiver is a collection literal inside a loop
        if self.loop_depth > 0 {
            self.check_call(node, method_name);
        }

        // Visit receiver
        if let Some(recv) = node.receiver() {
            self.visit(&recv);
        }
        // Visit arguments
        if let Some(args) = node.arguments() {
            self.visit(&args.as_node());
        }

        // Visit block body with loop context if needed
        if let Some(block) = node.block() {
            if let Some(block_node) = block.as_block_node() {
                if is_loop_call {
                    self.loop_depth += 1;
                }
                // Visit block parameters
                if let Some(params) = block_node.parameters() {
                    self.visit(&params);
                }
                // Visit block body
                if let Some(body) = block_node.body() {
                    self.visit(&body);
                }
                if is_loop_call {
                    self.loop_depth -= 1;
                }
            } else {
                self.visit(&block);
            }
        }
    }
}

impl CollectionLiteralVisitor<'_, '_> {
    /// Check if a call node is a loop-like method (Kernel.loop or enumerable method)
    fn is_loop_method(&self, call: &ruby_prism::CallNode<'_>) -> bool {
        let method_name = call.name().as_slice();

        // Check for Kernel.loop or bare `loop`
        // Handle both simple constant (Kernel) and qualified constant (::Kernel)
        if method_name == b"loop" {
            match call.receiver() {
                None => return true,
                Some(recv) => {
                    if let Some(cr) = recv.as_constant_read_node() {
                        if cr.name().as_slice() == b"Kernel" {
                            return true;
                        }
                    }
                    if let Some(cp) = recv.as_constant_path_node() {
                        if let Some(cp_name) = cp.name() {
                            if cp_name.as_slice() == b"Kernel" {
                                return true;
                            }
                        }
                    }
                }
            }
        }

        // Enumerable methods
        self.enumerable_methods.contains(method_name)
    }

    fn check_call(&mut self, call: &ruby_prism::CallNode<'_>, method_name: &[u8]) {
        let recv = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Check if receiver is an Array literal with a non-mutating array method
        if let Some(array) = recv.as_array_node() {
            if !self.array_methods.contains(method_name) {
                return;
            }
            if array.elements().len() < self.min_size {
                return;
            }
            if !is_recursive_basic_literal(&recv) {
                return;
            }
            let loc = recv.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Avoid immutable Array literals in loops. It is better to extract it into a local variable or a constant.".to_string(),
            ));
            return;
        }

        // Check if receiver is a Hash literal with a non-mutating hash method
        if let Some(hash) = recv.as_hash_node() {
            if !self.hash_methods.contains(method_name) {
                return;
            }
            if hash.elements().len() < self.min_size {
                return;
            }
            if !is_recursive_basic_literal(&recv) {
                return;
            }
            let loc = recv.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Avoid immutable Hash literals in loops. It is better to extract it into a local variable or a constant.".to_string(),
            ));
        }
    }
}

/// Check if a node is a recursive basic literal (all children are basic literals too).
fn is_recursive_basic_literal(node: &ruby_prism::Node<'_>) -> bool {
    if node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_string_node().is_some()
        || node.as_symbol_node().is_some()
        || node.as_nil_node().is_some()
        || node.as_true_node().is_some()
        || node.as_false_node().is_some()
    {
        return true;
    }

    if let Some(array) = node.as_array_node() {
        return array
            .elements()
            .iter()
            .all(|e| is_recursive_basic_literal(&e));
    }

    if let Some(hash) = node.as_hash_node() {
        return hash.elements().iter().all(|e| {
            if let Some(assoc) = e.as_assoc_node() {
                is_recursive_basic_literal(&assoc.key())
                    && is_recursive_basic_literal(&assoc.value())
            } else {
                false
            }
        });
    }

    // KeywordHashNode (keyword args like `foo(a: 1)`) cannot appear as a
    // method receiver, so this branch is unreachable in practice, but we
    // handle as_keyword_hash_node to satisfy the prism pitfalls check.
    if let Some(kh) = node.as_keyword_hash_node() {
        return kh.elements().iter().all(|e| {
            if let Some(assoc) = e.as_assoc_node() {
                is_recursive_basic_literal(&assoc.key())
                    && is_recursive_basic_literal(&assoc.value())
            } else {
                false
            }
        });
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(CollectionLiteralInLoop, "cops/performance/collection_literal_in_loop");
}
