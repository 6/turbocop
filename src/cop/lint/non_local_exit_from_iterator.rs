use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct NonLocalExitFromIterator;

const ITERATOR_METHODS: &[&[u8]] = &[
    b"each",
    b"each_with_index",
    b"each_with_object",
    b"map",
    b"flat_map",
    b"collect",
    b"select",
    b"filter",
    b"reject",
    b"find",
    b"detect",
    b"any?",
    b"all?",
    b"none?",
    b"count",
    b"sum",
    b"min_by",
    b"max_by",
    b"sort_by",
    b"group_by",
    b"partition",
    b"zip",
    b"take_while",
    b"drop_while",
    b"reduce",
    b"inject",
    b"times",
    b"upto",
    b"downto",
];

impl Cop for NonLocalExitFromIterator {
    fn name(&self) -> &'static str {
        "Lint/NonLocalExitFromIterator"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = call.name().as_slice();
        if !ITERATOR_METHODS.iter().any(|&m| m == method_name) {
            return Vec::new();
        }

        let block = match call.block() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let body = match block_node.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let mut finder = ReturnFinder {
            offsets: Vec::new(),
        };
        finder.visit(&body);

        finder
            .offsets
            .iter()
            .map(|&offset| {
                let (line, column) = source.offset_to_line_col(offset);
                self.diagnostic(
                    source,
                    line,
                    column,
                    "Non-local exit from iterator detected. Use `next` or `break` instead of `return`."
                        .to_string(),
                )
            })
            .collect()
    }
}

struct ReturnFinder {
    offsets: Vec<usize>,
}

impl<'pr> Visit<'pr> for ReturnFinder {
    fn visit_return_node(&mut self, node: &ruby_prism::ReturnNode<'pr>) {
        self.offsets.push(node.location().start_offset());
        // Don't recurse into the return's value — there can't be another return there
    }

    // Don't recurse into nested def/class/module/lambda
    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
    fn visit_class_node(&mut self, _node: &ruby_prism::ClassNode<'pr>) {}
    fn visit_module_node(&mut self, _node: &ruby_prism::ModuleNode<'pr>) {}
    fn visit_lambda_node(&mut self, _node: &ruby_prism::LambdaNode<'pr>) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NonLocalExitFromIterator, "cops/lint/non_local_exit_from_iterator");
}
