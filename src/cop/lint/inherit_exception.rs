use crate::cop::shared::constant_predicates;
use crate::cop::shared::node_type::{CALL_NODE, CLASS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct InheritException;

impl Cop for InheritException {
    fn name(&self) -> &'static str {
        "Lint/InheritException"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, CLASS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let style = config.get_str("EnforcedStyle", "standard_error");
        let _supported = config.get_string_array("SupportedStyles");

        let prefer = match style {
            "runtime_error" => "RuntimeError",
            _ => "StandardError",
        };

        // Check class Foo < Exception
        if let Some(class_node) = node.as_class_node() {
            let parent = match class_node.superclass() {
                Some(p) => p,
                None => return,
            };

            if is_exception(&parent) {
                // Corpus investigation notes (2026-03-01):
                // - Added omitted-namespace guard for `class C < Exception` when a local
                //   sibling `Exception` is already defined in the same scope.
                // - This reduced check-cop excess from 53 -> 4.
                // - Remaining 4 excess offenses are unresolved and likely require a more
                //   complete lexical-constant resolution than this same-scope sibling scan.
                if is_omitted_namespace_exception(&parent)
                    && has_local_exception_sibling(
                        _parse_result,
                        class_node.location().start_offset(),
                    )
                {
                    return;
                }

                let loc = parent.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Inherit from `{prefer}` instead of `Exception`."),
                ));
            }
            return;
        }

        // Check Class.new(Exception)
        if let Some(call) = node.as_call_node() {
            if call.name().as_slice() != b"new" {
                return;
            }

            let receiver = match call.receiver() {
                Some(r) => r,
                None => return,
            };

            let recv_name = match constant_predicates::constant_short_name(&receiver) {
                Some(n) => n,
                None => return,
            };

            if recv_name != b"Class" {
                return;
            }

            let arguments = match call.arguments() {
                Some(a) => a,
                None => return,
            };

            let args = arguments.arguments();
            if let Some(first_arg) = args.first() {
                if is_exception(&first_arg) {
                    let loc = first_arg.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        format!("Inherit from `{prefer}` instead of `Exception`."),
                    ));
                }
            }
        }
    }
}

fn is_exception(node: &ruby_prism::Node<'_>) -> bool {
    // Bare `Exception` (ConstantReadNode)
    if let Some(cr) = node.as_constant_read_node() {
        return cr.name().as_slice() == b"Exception";
    }
    // `::Exception` but NOT `Foo::Exception` or `::Foo::Exception`
    if let Some(cp) = node.as_constant_path_node() {
        if let Some(name) = cp.name() {
            if name.as_slice() == b"Exception" {
                return cp.parent().is_none();
            }
        }
    }
    false
}

fn is_omitted_namespace_exception(node: &ruby_prism::Node<'_>) -> bool {
    node.as_constant_read_node()
        .is_some_and(|cr| cr.name().as_slice() == b"Exception")
}

fn has_local_exception_sibling(
    parse_result: &ruby_prism::ParseResult<'_>,
    target_class_offset: usize,
) -> bool {
    // Tracks whether a local `Exception` constant/class/module appears in the
    // same statements list before the target class definition.
    let mut finder = LocalExceptionSiblingFinder {
        target_class_offset,
        found: false,
        done: false,
    };
    finder.visit(&parse_result.node());
    finder.found
}

struct LocalExceptionSiblingFinder {
    target_class_offset: usize,
    found: bool,
    done: bool,
}

impl<'a> Visit<'a> for LocalExceptionSiblingFinder {
    fn visit_statements_node(&mut self, node: &ruby_prism::StatementsNode<'a>) {
        if self.done {
            return;
        }

        let mut seen_local_exception = false;
        for stmt in node.body().iter() {
            if self.done {
                return;
            }

            if let Some(class_node) = stmt.as_class_node() {
                if class_node.location().start_offset() == self.target_class_offset {
                    self.found = seen_local_exception;
                    self.done = true;
                    return;
                }
            }

            self.visit(&stmt);
            if self.done {
                return;
            }

            if defines_local_exception_constant(&stmt) {
                seen_local_exception = true;
            }
        }
    }
}

fn defines_local_exception_constant(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(class_node) = node.as_class_node() {
        return is_exception_identifier(&class_node.constant_path());
    }

    if let Some(module_node) = node.as_module_node() {
        return is_exception_identifier(&module_node.constant_path());
    }

    if let Some(const_write) = node.as_constant_write_node() {
        return const_write.name().as_slice() == b"Exception";
    }

    false
}

fn is_exception_identifier(node: &ruby_prism::Node<'_>) -> bool {
    node.as_constant_read_node()
        .is_some_and(|cr| cr.name().as_slice() == b"Exception")
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(InheritException, "cops/lint/inherit_exception");
}
